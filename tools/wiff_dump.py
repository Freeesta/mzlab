"""A short text description of a SCIEX .wiff, to find where a file of the laboratory keeps what the reader does not show yet (sample names, dates, vials):
standard Python only (mzlab/reader/ole.py), no scan data printed. Run on the computer that holds the file; the output stays outside the repository.

    python3 tools/wiff_dump.py FILE.wiff [--all] [--log]

Prints: the streams of the container (those of the samples and the methods grouped by name, with how many samples have each and their sizes); then for each sample
its number, the scans (rows of `Idx`), the experiment types of its method (full scan, MS2 with the precursor, MRM, ...) and the readable strings of its small streams
(UTF-16 and ASCII, at most 8 per stream and 80 characters each; `--all` prints them all). The `Log` stream may hold the operator's name: only its size, unless `--log`.
"""
import argparse
import re
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from mzlab.reader.ole import Ole

TYPES = {15: "full scan", 11: "MS2", 4: "MRM", 5: "precursor ion", 7: "neutral loss"}
HEADER_PRECURSOR, HEADER_CODE = 42, 122  # crates/mzlab-core/src/wiff/mod.rs
IDX_HEAD, IDX_ROW = 32, 54  # crates/mzlab-core/src/wiff/idx.rs
SMALL = 1 << 20  # streams up to 1 MB are searched for strings
U16 = re.compile(rb"(?:[\x20-\x7e]\x00){3,}")
ASC = re.compile(rb"[\x20-\x7e]{4,}")


def strings(b: bytes, cap: int | None) -> list[str]:
    """The readable runs of `b` in order of position, without repeats: UTF-16LE first where both match."""
    found = sorted([(m.start(), m.group().decode("utf-16le")) for m in U16.finditer(b)] + [(m.start(), m.group().decode("ascii")) for m in ASC.finditer(b)])
    out: list[str] = []
    for _, s in found:
        s = s.strip()
        if s and s not in out and not any(s in o for o in out):
            out.append(s if cap is None else s[:80])
        if cap is not None and len(out) >= cap:
            break
    return out


def experiments(ole: Ole, n: int) -> list[str]:
    out = []
    while (p := f"MethodSubtree/Method{n}/DeviceMethod0/Period0/Experiment{len(out)}/ExperimentHeader") in ole.streams:
        d = ole.read(p)
        code = struct.unpack_from("<I", d, HEADER_CODE)[0] if len(d) >= HEADER_CODE + 4 else None
        prec = struct.unpack_from("<d", d, HEADER_PRECURSOR)[0] if len(d) >= HEADER_PRECURSOR + 8 else 0.0
        t = TYPES.get(code, f"code {code}")
        out.append(f"{t} {prec:.2f}" if code == 11 and prec > 0 else t)
    return out


def grouped(paths: dict, prefix: str) -> list[str]:
    """`prefix<N>/rest` streams by `rest`: how many numbers have it, and the smallest and largest size."""
    rx, by = re.compile(re.escape(prefix) + r"(\d+)/(.+)"), {}
    for p, e in paths.items():
        if m := rx.fullmatch(p):
            by.setdefault(m.group(2), []).append(e["size"])
    return [f"  {prefix}N/{k}  x{len(v)}  {min(v)}..{max(v)} B" for k, v in sorted(by.items())]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("wiff")
    ap.add_argument("--all", action="store_true", help="every string, whole")
    ap.add_argument("--log", action="store_true", help="also the strings of the Log stream")
    a = ap.parse_args()
    path = Path(a.wiff)
    scan = path.with_name(path.name + ".scan")
    ole = Ole(path)
    print(f"{path.name}: {path.stat().st_size} B; {scan.name}: {scan.stat().st_size if scan.exists() else 'missing'} B; {len(ole.streams)} streams")
    rest = {p: e for p, e in ole.streams.items() if not re.match(r"(SampleSubtree/Sample|MethodSubtree/Method)\d+/", p)}
    print("streams:")
    print("\n".join(f"  {p}  {e['size']} B" for p, e in sorted(rest.items())))
    print("\n".join(grouped(ole.streams, "SampleSubtree/Sample") + grouped(ole.streams, "MethodSubtree/Method")))
    cap = None if a.all else 8
    for p in sorted(rest):
        if rest[p]["size"] <= SMALL and (s := strings(ole.read(p), cap)):
            print(f"strings of {p}: " + " | ".join(s))
    nums = sorted({int(m.group(1)) for p in ole.streams if (m := re.match(r"SampleSubtree/Sample(\d+)/", p))})
    print(f"samples: {len(nums)}")
    for n in nums:
        idx = ole.streams.get(f"SampleSubtree/Sample{n}/Idx")
        scans = max(0, (idx["size"] - IDX_HEAD) // IDX_ROW) if idx else 0
        print(f"sample {n}: {scans} scans; experiments: {', '.join(experiments(ole, n)) or 'none'}")
        for p in sorted(q for q in ole.streams if q.startswith(f"SampleSubtree/Sample{n}/")):
            name, size = p.rsplit("/", 1)[-1], ole.streams[p]["size"]
            if name == "Idx" or size > SMALL or (name == "Log" and not a.log):
                continue
            if s := strings(ole.read(p), cap):
                print(f"  {p[len(f'SampleSubtree/Sample{n}/'):]}: " + " | ".join(s))
    ole.close()


if __name__ == "__main__":
    main()
