"""Reference answers of App.files() (the loading-table entry of a file) and Session(...).info() for crates/mzlab-core/tests/lr_parity.rs.

    python3 tools/sessione_riferimento.py OUT [FILE.mzML ...]

Writes the synthetic files of tools/mzidx_riferimento.py in OUT/, their `.mzidx` and OUT/<stem>.lr.json = {entry, plain, edited}; prints the
"<mzML path>" lines. No compound names.
"""
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import mzidx_riferimento as ref  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.explore import Session  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

EDIT = {"label": "my sample", "type": "standard", "time": 12.5, "conc": 3.0, "cunit": "ppm", "rep": "A"}


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    os.environ["MZLAB_CACHE"] = str(out / "cache")
    files = ref.synthetic(out) + [Path(a) for a in argv[1:]]
    for f in files:
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        work = out / ("work_" + f.stem)
        work.mkdir(exist_ok=True)
        link = work / f.name
        if not link.exists():
            link.symlink_to(f.resolve())
        entry = next(e for e in App(work).files() if e["name"] == f.name)
        none = {"file": f.name, "label": None, "type": None, "time": None, "conc": None, "cunit": None, "rep": None}
        plain = [i.info() for i in Session([none], work).items]
        edited = [i.info() for i in Session([{**none, **EDIT}], work).items]
        (out / (f.stem + ".lr.json")).write_text(json.dumps({"entry": entry, "plain": plain, "edited": edited}), encoding="utf-8")
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
