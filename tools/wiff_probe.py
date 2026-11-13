"""Metadata-only probe of the scan blocks of a SCIEX .wiff pair, to check the point decoding against the file's own index (no m/z or intensity is printed).
Standard Python only (mzlab/reader/ole.py) and its own decoder of the nibble stream (docs/agenti/wiff.md), independent of the Rust one. Run on the computer that holds
the file; the output stays outside the repository.

    python3 tools/wiff_probe.py FILE.wiff [--sample N]

For each sample: the block versions, the point tokens (1..7 count, 0 long count, skips), the counts by size, the bytes a block keeps after its points, and per scan
the ratio of the sum of the decoded intensities to the TIC of the index row (+18) as quantiles: a ratio far below 1 means the decoder drops most of the signal.
"""
import argparse
import re
import struct
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from mzlab.reader.ole import Ole

IDX_HEAD, IDX_ROW, TABLE_HEAD, TABLE_ROW, TABLE_LEAD, PAYLOAD_SKIP = 32, 54, 32, 16, 20, 44  # crates/mzlab-core/src/wiff/idx.rs, scan.rs
MARKER = b"\xfe\xff\xff\xff"


def decode(buf: bytes, n: int, tokens: Counter):
    """`(bin, count)` of up to `n` points and the bytes taken; `tokens` counts the token kinds."""
    out, at, pos = [], 0, 0

    def nib():
        nonlocal at
        if at >> 1 >= len(buf):
            return None
        b = buf[at >> 1]
        at += 1
        return b >> 4 if (at - 1) & 1 == 0 else b & 15

    def long():
        nonlocal at
        at += at & 1
        i = at >> 1
        if i >= len(buf):
            return None
        at += 2
        if buf[i]:
            return buf[i]
        if i + 5 > len(buf):
            return None
        at += 8
        return struct.unpack_from("<I", buf, i + 1)[0]

    while len(out) < n:
        t = nib()
        if t is None:
            break
        if t == 0:
            tokens["long count"] += 1
            c = long()
            if c is None:
                break
            out.append((pos, c))
        elif t <= 7:
            tokens[f"count {t}"] += 1
            out.append((pos, t))
        elif t == 8:
            tokens["long skip"] += 1
            s = long()
            if s is None:
                break
            pos += s
            continue
        else:
            tokens["skip"] += 1
            pos += t - 8
            continue
        pos += 1
    return out, (at + 1) // 2


def quantiles(v):
    if not v:
        return "none"
    v = sorted(v)
    return " ".join(f"{q}%={v[min(len(v) - 1, len(v) * q // 100)]:.3g}" for q in (0, 10, 50, 90, 100))


def probe(ole: Ole, scan: bytes, n: int, start: int):
    idx = ole.read(f"SampleSubtree/Sample{n}/Idx")
    rows = [idx[o : o + IDX_ROW] for o in range(IDX_HEAD, len(idx) - IDX_ROW + 1, IDX_ROW)]
    offs = [start + struct.unpack_from("<I", r, 0)[0] + PAYLOAD_SKIP for r in rows]
    versions, tokens, counts, rest, flags, scales = Counter(), Counter(), Counter(), Counter(), Counter(), Counter()
    ratio, empty_with_tic, short = [], 0, 0
    for k, r in enumerate(rows):
        at, size = offs[k], struct.unpack_from("<I", r, 4)[0]
        tic, flags[r[16]] = struct.unpack_from("<d", r, 18)[0], flags[r[16]] + 1
        b = scan[at : at + size] if size else scan[at : (offs[k + 1] if k + 1 < len(offs) else len(scan))]
        if b[:4] != MARKER:
            versions["no scan head"] += 1
            continue
        v = struct.unpack_from("<I", b, 4)[0]
        versions[f"version {v}"] += 1
        if not 1 <= v <= 3 or len(b) < 12 + 32 * v:
            continue
        hl = 12 + 32 * v
        ranges = [struct.unpack_from("<4d", b, 8 + 32 * j) for j in range(v)]
        for rg in ranges:
            scales[round(rg[3], 6)] += 1
        pts, used = decode(b[hl:], struct.unpack_from("<I", b, hl - 4)[0], tokens)
        short += len(pts) != struct.unpack_from("<I", b, hl - 4)[0]
        rest[len(b) - hl - used] += 1
        for _, c in pts:
            counts["1" if c == 1 else "2" if c == 2 else "3..7" if c < 8 else "8..255" if c < 256 else ">255"] += 1
        total = 0.0
        for bin_, c in pts:
            mz = ranges[0][0] + bin_ * ranges[0][2]
            total += c * next((rg[3] for rg in reversed(ranges[1:]) if mz > rg[0]), ranges[0][3])
        if tic > 0:
            ratio.append(total / tic)
            empty_with_tic += not pts
    print(f"sample {n}: {len(rows)} rows; {dict(versions)}; flag +16 {dict(flags)}")
    print(f"  tokens {dict(tokens)}; counts {dict(counts)}; blocks short of their head count {short}")
    print(f"  bytes after the points {dict(sorted(rest.items())[:12])}; distinct scales {len(scales)} (most used {scales.most_common(3)})")
    print(f"  decoded sum / index TIC per scan: {quantiles(ratio)}; scans with TIC > 0 and no point {empty_with_tic}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("wiff")
    ap.add_argument("--sample", type=int)
    a = ap.parse_args()
    path = Path(a.wiff)
    scan = path.with_name(path.name + ".scan").read_bytes()
    ole = Ole(path)
    table = ole.read("SampleSubtree/SampleIdxTable")
    for o in range(TABLE_HEAD, len(table) - TABLE_ROW + 1, TABLE_ROW):
        n, pos = struct.unpack_from("<II", table, o)
        if (a.sample is None or a.sample == n) and f"SampleSubtree/Sample{n}/Idx" in ole.streams:
            probe(ole, scan, n, pos - TABLE_LEAD)
    ole.close()


if __name__ == "__main__":
    main()
