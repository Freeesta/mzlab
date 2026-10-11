"""Reference answers of the chromatogram list, the chromatogram arrays and the metadata of mzlab/reader/mzml.py for
crates/mzlab-core/tests/mzmlhead_parity.rs.

    python3 tools/mzmlhead_riferimento.py OUT [FILE.mzML ...]

Writes the synthetic files of tools/mzidx_riferimento.py in OUT/, their `.mzidx` and OUT/<stem>.head.json; prints the "<mzML path>" lines.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import mzidx_riferimento as ref  # noqa: E402
from mzlab.explore import Item  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    files = ref.synthetic(out) + [Path(a) for a in argv[1:]]
    for f in files:
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        run = Run(f)
        it = Item(f.name, None, None, "x", f, run=run)
        chroms = []
        for c in run.chromatograms():
            t, y = run.chromatogram(c["index"])
            chroms.append({**c, "t": t.tolist(), "y": y.tolist(), "unit": run.chromatogram_unit(c["index"])})
        md = run.metadata()
        (out / (f.stem + ".head.json")).write_text(json.dumps({"chroms": chroms, "metadata": md, "header_polarity": it._header_polarity(), "nce": bool(run.nce),
                                                                "analyzers": sorted(run._ic.items())}), encoding="utf-8")
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
