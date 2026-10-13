"""The reference of the low-resolution full scan: the ions that `engine.py` (TP_Mine/py/tpmine, Python) gives as candidates or unexpected ions on the same parent.

The Python engine is not the engine of the site: the table of the site must contain these ions within the tolerances, never agree to the last digit."""
from __future__ import annotations

from pathlib import Path


def ioni_riferimento(files: list[dict], neutra: str, etichette=("forte", "possibile")) -> list[dict]:
    """Ions {mz, rt, kind, label} of the rows of `Experiment(...).run()` that are not the parent and have a peak, with the label in `etichette`.
    `files`: [{name, path}] (time and type are guessed from the names); `mz` is the extracted m/z (calibrated), the one seen in the table."""
    from tpmine.engine import Experiment
    ex = Experiment([{"name": Path(f["path"]).name, "path": str(f["path"])} for f in files], {"name": "parent", "neutral": neutra}, {})
    out = ex.run()
    mzx = {e["id"]: e["mz_x"] for e in ex.entries}
    return [{"mz": float(mzx.get(r["id"], r["mz"])), "rt": float(r["ref_rt"]), "kind": r["kind"], "label": r["label"]}
            for r in out["rows"] if r["label"] in etichette and r["ref_rt"] and r["kind"] != "parent"]
