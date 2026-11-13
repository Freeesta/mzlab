"""Synthetic `.wiff` pairs for the isolation tests of the `.wiff` reader (one broken unit never takes the healthy ones with it).

    python3 tools/wiff_isolation.py OUT

Writes into OUT/ (layout and writer of tools/wiff_riferimento.py, invented numbers, no laboratory data):
`sano`: three samples of three full scans each; `guasto`: the same bytes with every block of sample 1 marked `ffffffff` (a TOF block, damaged);
`mrm`: two MRM samples (10 and 4 cycles of 3 transitions); `mrm_nan`: the same with one f32 of cycle 5 of sample 1 a NaN; `mrm_taglio`: one MRM
sample of 10 cycles whose `.wiff.scan` ends inside cycle 7; `zeri`: sample 1 two full scans, sample 2 one scan block whose Idx rows are all zero;
`vuoto0`: sample 1 in the sample table at position 0 with no Idx stream (it acquired nothing), sample 2 two full scans.
"""
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from wiff_riferimento import FULL, LATE, Writer, header, ion_spray, scan_block  # noqa: E402

METHOD = {"Experiment0/ExperimentHeader": header(15, 30.0), "Experiment0/IonSourceParamsTable/Parameter0/ParameterData": ion_spray(5500.0)}
SCANS = [FULL, LATE, [(10 + k, 3 + k) for k in range(12)]]


def three(out: Path, name: str, broken: bool):
    w, starts = Writer(), []
    for n in (1, 2, 3):
        starts.append(len(w.scan) + 44)
        blocks = [("scan", SCANS[(n + k) % 3], 1000.0 * (k + 1), 10.0 * n + k) for k in range(3)]
        w.sample(n, blocks, METHOD)
    if broken:  # every block of sample 1 starts `ffffffff`: the lengths and the other samples stay byte for byte
        at = starts[0]
        for k in range(3):
            w.scan[at:at + 4] = b"\xff\xff\xff\xff"
            at += len(scan_block(SCANS[(1 + k) % 3]))
    w.save(out, name)


def mrm_values(cycles, seed):
    return [[float(seed * 100 + 10 * c + j + 1) for j in range(3)] for c in range(cycles)]


def mrm(out: Path):
    for name, bad in (("mrm", None), ("mrm_nan", math.nan)):
        w = Writer()
        for n, cycles in ((1, 10), (2, 4)):
            vals = mrm_values(cycles, n)
            if n == 1 and bad is not None:
                vals[4][1] = bad
            w.sample(n, [("mrm", v, 500.0 * (c + 1), sum(x for x in v if x == x)) for c, v in enumerate(vals)])
        w.save(out, name)
    w = Writer()
    w.sample(1, [("mrm", v, 500.0 * (c + 1), sum(v)) for c, v in enumerate(mrm_values(10, 1))])
    del w.scan[44 + 12 * 6 + 5:]  # cycles 1-6 whole, cycle 7 cut after 5 of its 12 bytes, 8-10 missing
    w.save(out, "mrm_taglio")


def zeros(out: Path):
    w = Writer()
    w.sample(1, [("scan", FULL, 1000.0, 1.0), ("scan", LATE, 2000.0, 2.0)], METHOD)
    w.sample(2, [("scan", FULL, 1000.0, 1.0), ("scan", LATE, 2000.0, 2.0), ("scan", FULL, 3000.0, 3.0)], METHOD)
    w.streams["SampleSubtree/Sample2/Idx"] = bytes(32 + 3 * 54)
    w.save(out, "zeri")


def empty_at_zero(out: Path):
    w = Writer()
    w.sample(2, [("scan", FULL, 1000.0, 1.0), ("scan", LATE, 2000.0, 2.0)], METHOD)
    w.table.insert(0, (1, 0))  # the row of an injection that acquired nothing: position 0, no Idx stream
    w.streams["MethodSubtree/Method1/DeviceMethod0/Period0/Experiment0/ExperimentHeader"] = header(15, 30.0)
    w.save(out, "vuoto0")


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    three(out, "sano", False)
    three(out, "guasto", True)
    mrm(out)
    zeros(out)
    empty_at_zero(out)


if __name__ == "__main__":
    main()
