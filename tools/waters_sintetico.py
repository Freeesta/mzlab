"""Synthetic Waters MassLynx .raw folder (_FUNCTNS.INF, _FUNC001/002 .DAT + .IDX, _HEADER.TXT), written with the layout of docs/agenti/waters.md: no laboratory data.
    python3 tools/waters_sintetico.py <out folder> [name.raw] [scans]
Function 1 scans m/z 100-1000 (12-byte records, centroids): a chromatographic peak of m/z 150.1 (and a flat m/z 300.3) at 1.5 min; function 2 is an MRM function with two channels
(200.1 > 100.2 and 300.3 > 150.4, 4-byte values) with a peak at the same time. The same folder as `crates/mzlab-lettori/tests/comune_waters/mod.rs` builds, with more scans."""
import math
import struct
import sys
from pathlib import Path


def word_mz(mz: float) -> int:
    e = math.floor(math.log2(mz)) + 1
    return (e << 27) | (round(mz / 2.0**e * (1 << 27)) & 0x07FFFFFF)


def word_int(i: int) -> int:
    e = i.bit_length()
    return (e << 22) | (round(i * 2.0 ** (21 - e)) & 0x1FFFFF)


def idx_row(offset: int, count: int, tic: float, rt: float) -> bytes:
    return struct.pack("<IIff", offset, count | 0x18000000, tic, rt) + bytes(6)


def func_record(tipo: int, sotto: int, cycle: float, slots) -> bytes:
    r = bytearray(416)
    r[0], r[1] = tipo, sotto
    r[2:6] = struct.pack("<f", cycle)
    for k, (q1, q3, dwell) in enumerate(slots):
        r[0x20 + 4 * k : 0x24 + 4 * k] = struct.pack("<f", dwell)
        r[0xA0 + 4 * k : 0xA4 + 4 * k] = struct.pack("<f", q1)
        r[0x120 + 4 * k : 0x124 + 4 * k] = struct.pack("<f", q3)
    return bytes(r)


def write(root: Path, name: str = "synt.raw", n: int = 40) -> Path:
    d = root / name
    d.mkdir(parents=True, exist_ok=True)
    dat1, idx1, dat2, idx2 = bytearray(), bytearray(), bytearray(), bytearray()
    for k in range(n):
        rt = 0.5 + 2.0 * k / max(1, n - 1)
        g = math.exp(-(((rt - 1.5) / 0.15) ** 2))
        peaks = [(150.1, 100 + round(8000 * g)), (300.3, 500), (450.4, 300)]
        off = len(dat1)
        for mz, it in peaks:
            dat1 += struct.pack("<III", word_int(it), word_mz(mz), 0)
        idx1 += idx_row(off, len(peaks), float(sum(p[1] for p in peaks)), rt)
        vals = [50 + round(4000 * g), 20 + round(1500 * g)]
        off = len(dat2)
        for v in vals:
            dat2 += struct.pack("<I", (21 << 22) | v)
        idx2 += idx_row(off, 2, float(sum(vals)), rt)
    inf = func_record(0x12, 0x25, 1.0, [(100.0, 1000.0, 0.0)]) + func_record(0x29, 0x01, 1.0, [(200.1, 100.2, 0.05), (300.3, 150.4, 0.05)])
    head = (
        "$$ Version: 01.00\r\n$$ Acquired Name: SYNT_001\r\n$$ Acquired Date: 30-Apr-2022\r\n$$ Acquired Time: 00:26:58\r\n"
        "$$ Instrument: XEVO-TQXS#SYN0001\r\n$$ Sample Description: synthetic sample\r\n$$ Bottle Number: 1:34\r\n$$ Cal Function 1: 0,1,T1\r\n"
    )
    for f, b in (("_FUNCTNS.INF", inf), ("_FUNC001.DAT", dat1), ("_FUNC001.IDX", idx1), ("_FUNC002.DAT", dat2), ("_FUNC002.IDX", idx2), ("_HEADER.TXT", head.encode("latin-1"))):
        (d / f).write_bytes(bytes(b))
    return d


if __name__ == "__main__":
    out = Path(sys.argv[1])
    print(write(out, *(sys.argv[2:3] or ["synt.raw"]), *(int(x) for x in sys.argv[3:4])))
