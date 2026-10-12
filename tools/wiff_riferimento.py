"""Reference for the parity test of the Rust `.wiff` / `.wiff.scan` reader (crates/mzlab-core/tests/wiff_parity.rs).

    python3 tools/wiff_riferimento.py OUT

Needs `opensxraw` (pip install opensxraw; Apache-2.0), run by hand as the oracle. Writes into OUT/ synthetic pairs (`one.wiff` + `one.wiff.scan`: one
sample; `two.wiff` + `two.wiff.scan`: two samples; `lonely.wiff`: no `.scan`), built with tools/cfb_scrivi.py, and OUT/riferimento.json: for each pair and sample
the scans as OpenSXRaw reads them (RT in seconds, level, TIC, points with intensity > 0) and the TIC chromatogram. No data of the laboratory is used.
"""
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cfb_scrivi  # noqa: E402


def token(gap: int, count: int) -> bytes:
    """Gaps of at most 127, then the shortest intensity token."""
    out = bytearray()
    while gap > 127:
        out.append(127)
        gap -= 127
    out.append(gap)
    if count < 124:
        out.append(0x80 | count)
    elif count < 256:
        out += bytes([0xFC, count])
    elif count < 1 << 16:
        out += bytes([0xFD]) + struct.pack("<H", count)
    elif count < 1 << 24:
        out += bytes([0xFE]) + struct.pack("<I", count)[:3]
    else:
        out += bytes([0xFF]) + struct.pack("<I", count)
    return bytes(out)


def payload(points) -> bytes:
    out, bin_ = bytearray(), 0
    for b, c in points:
        out += token(b - bin_, c)
        bin_ = b
    return bytes(out)


def scan_file(blocks):
    """Returns (bytes of the .scan, [(offset, size)]); each block's terminator lies in the head window of the next one."""
    data, rows, term = bytearray(), [], bytes(56)
    for pts in blocks:
        off = len(data)
        pl = payload(pts)
        data += term + pl
        rows.append((off, 56 + len(pl) + 4))
        term = b"\xff\xff\xff\xff" + bytes(52)
    data += b"\xff\xff\xff\xff"
    return bytes(data), rows


def idx_stream(rows, scans):
    out = bytearray(32)
    for (off, size), (rt, level, tic) in zip(rows, scans):
        out += struct.pack("<IIIfBBdd", off, size, 0, rt, 1 if level == 1 else 0, 0, tic, 0.0) + bytes(20)
    return bytes(out)


SAMPLE_A = [
    [(41, 1), (50, 200), (300, 341), (301, 70000), (1500, 20000000), (9000, 5000000000 % (1 << 32))],
    [(10, 3), (20, 123), (21, 124), (22, 255), (23, 256), (2000, 65535), (2001, 65536), (2002, (1 << 24) - 1)],
    [(1000, 0xFFFFFF), (1000 + 400, 0xFFFF)],
    [(5, 0xFFFFFFFF), (6, 7)],
]
INFO_A = [(0.0123, 1, 1.5e6), (0.0345, 2, 2.25e4), (0.0567, 2, 0.0), (0.0789, 1, 3.0e9)]
SAMPLE_B = [[(3, 9), (80, 99)], [(7, 130), (700, 3000)]]
INFO_B = [(0.5, 1, 12345.5), (0.75, 2, 678.25)]


def write_pair(out: Path, name: str, samples: dict):
    scan = bytearray()
    streams = {}
    for sample, (blocks, info) in samples.items():
        base = len(scan)
        data, rows = scan_file(blocks)
        scan += data
        streams[f"SampleSubtree/{sample}/Idx"] = idx_stream([(o + base, s) for o, s in rows], info)
    cfb_scrivi.write(streams, out / f"{name}.wiff")
    (out / f"{name}.wiff.scan").write_bytes(bytes(scan))


def oracle(path: Path):
    import opensxraw as o
    res = {}
    for s in o.list_samples(str(path)):
        r = o.open_sample(str(path), s)
        scans = []
        for sp in (r.read_spectrum(i) for i in range(r.scan_count)):
            scans.append({"rt": sp.retention_time_sec, "level": sp.ms_level,
                          "mz": [float(x) for x in sp.mz], "intensity": [float(x) for x in sp.intensity]})
        c = r.read_chromatograms()[0]
        res[s] = {"scans": scans, "tic": {"time_sec": c["time_sec"], "intensity": c["intensity"]}}
    return res


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    write_pair(out, "one", {"Sample1": (SAMPLE_A, INFO_A)})
    write_pair(out, "two", {"Sample1": (SAMPLE_A, INFO_A), "Sample2": (SAMPLE_B, INFO_B)})
    cfb_scrivi.write({"SampleSubtree/Sample1/Idx": bytes(32)}, out / "lonely.wiff")
    ref = {n: oracle(out / f"{n}.wiff") for n in ("one", "two")}
    (out / "riferimento.json").write_text(json.dumps(ref))


if __name__ == "__main__":
    main()
