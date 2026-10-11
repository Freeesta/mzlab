"""Reference answers of Item.info() (mzlab/explore.py) for crates/mzlab-core/tests/iteminfo_parity.rs.

    python3 tools/info_riferimento.py OUT [FILE.mzML ...]

Writes the synthetic mzML files of tools/mzidx_riferimento.py in OUT/, their `.mzidx` and, for each file, OUT/<stem>.info.json =
[{meta, part, facts, info}] (one entry for a plain file, one per part of a mixed file; `facts` are the few things only the mzML header holds,
handed to Rust as they are). Prints the "<mzML path>" lines. No compound names.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import mzidx_riferimento as ref  # noqa: E402
from mzlab.explore import Item, file_parts  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def facts(it: Item) -> dict:
    r = it.run
    md = r.metadata()
    rng = it._srm_rt_range()
    return {"instrument": md.get("instrument", ""), "n_chromatograms": r.n_chromatograms, "srm": sum(1 for c in r.chromatograms() if c["kind"] == "srm"),
            "pda": it.has_pda(), "header_polarity": it._header_polarity(), "srm_rt": list(rng) if rng else None, "analog": it.analog_list()}


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    files = ref.synthetic(out) + [Path(a) for a in argv[1:]]
    for f in files:
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        run = Run(f)
        parts = file_parts(run)
        rows = []
        for p in (parts or [None]):
            name = f.name if p is None else f"{f.name}#{p['tag']}"
            it = Item(name, "my sample", 12.5, "standard", f, 3.0, "ppm", part=p, run=run)
            it.rep = "A" if p is None else None
            rows.append({"meta": {"file": name, "label": it.label, "sha256": it.sha256, "time": it.time, "type": it.type, "conc": it.conc, "cunit": it.cunit, "rep": it.rep},
                         "part": p, "facts": facts(it), "info": json.loads(json.dumps(it.info()))})
        (out / (f.stem + ".info.json")).write_text(json.dumps(rows), encoding="utf-8")
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
