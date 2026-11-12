"""Synthetic .wiff + .wiff.scan of sparse product-ion scans as a 3200 QTRAP writes them (crates/mzlab-core/tests/wiff_sparse_product.rs, tests/test_lettori_parita.py).

    python3 tools/wiff_prodotto_rado.py OUT [--odd]

Writes OUT/rado.wiff(.scan) and OUT/rado.json. One sample, one product-ion experiment (scan code 6, precursor 251.1), 60 profile scans of version-1 blocks,
each with 0 to 5 points (a few with up to 300), and two traits of the real files the reference writer lacks: the nibble stream of `u` bytes is padded to
`u + 4 - u % 4` bytes, and the index row holds at +26 a value that is not the point count (1 for a scan with points, 2 where the head has one point, 0 for
an empty one). rado.json lists the spectra the reader must give (id, m/z, intensity), computed with the encoder of tools/wiff_riferimento.py. No laboratory data.
With --odd, OUT/storto.*: the same scans with a few blocks their bytes contradict (5 to 8 bytes after the points, a head that declares more points
than the block holds); storto.json leaves them out and names how many ("damaged"): the reader must count them, never refuse the sample.
"""
import json
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import RANGES, Writer, encode, expected_scan, header, idx_row, ion_spray  # noqa: E402

SCANS = 60


def block(points, extra=0, more=0, start=b"\xfe\xff\xff\xff", version=1) -> bytes:
    """A version-1 block; `extra` bytes after the 3200 padding, `more` points declared beyond the ones written; `start` and `version` make an odd head."""
    data = encode(points)
    head = start + struct.pack("<I", version) + struct.pack("<4d", *RANGES[1][0]) + struct.pack("<I", len(points) + more)
    return head + data + bytes(4 - len(data) % 4 + extra)


def main():
    out, odd = Path(sys.argv[1]), "--odd" in sys.argv[2:]
    name, bad = ("storto", 0) if odd else ("rado", 0)
    out.mkdir(parents=True, exist_ok=True)
    rnd = random.Random(11)
    w = Writer()
    w.sample(1, [], {"Experiment0/ExperimentHeader": header(6, 251.1), "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)})
    # sample() wrote the 44 bytes before the first block and the table row; the blocks follow
    idx, off, want = bytearray(32), 0, []
    for k in range(SCANS):
        n = 300 if k % 20 == 7 else rnd.randrange(6)
        bins = sorted(rnd.sample(range(0, 3000), n))
        pts = [(b, 1 + rnd.randrange(300 if j % 3 else 7)) for j, b in enumerate(bins)]
        # --odd: every 10th block keeps 5 to 8 bytes after its points, one head declares 50 points more than its stream holds, one head is of version 4
        # and one starts as a TOF block
        extra, more = (1 + k // 10 % 4 + 4, 0) if odd and k % 10 == 3 else (0, 50) if odd and k == 25 else (0, 0)
        start, version = (b"\xff" * 4, 1) if odd and k == 37 else (b"\xfe\xff\xff\xff", 4) if odd and k == 48 else (b"\xfe\xff\xff\xff", 1)
        weird = bool(extra or more or start[0] == 0xFF or version != 1)
        bad += weird
        b = block(pts, extra, more, start, version)
        t = 1500.0 * (k + 1)
        idx += idx_row(off, len(b), t, float(sum(c for _, c in pts)), 0 if not pts else 2 if n == 1 else 1)
        w.scan += b
        off += len(b)
        if pts and not weird:
            mz, inten = expected_scan(pts, 1)
            want.append({"id": f"sample=1 period=1 cycle={k + 1} experiment=1", "mz": mz, "intensity": inten})
    w.streams["SampleSubtree/Sample1/Idx"] = bytes(idx)
    w.save(out, name)
    (out / f"{name}.json").write_text(json.dumps({"spectra": want, "damaged": bad}))


if __name__ == "__main__":
    main()
