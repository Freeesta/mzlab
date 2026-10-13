"""D2: writes a synthetic series (mzML + `verita.json`) and the mzFinder table on it, so that the other tools can run on the same mzML (`corri_<tool>.py`).

    python3 tools/bench/genera_d2.py OUT_DIR --seme 7 --profilo completo
OUT_DIR gets `mzml/` (the series), `verita.json` (truth of `sintetici_identita`) and `mzlab_D2_predefinita.json` (`bench/1` of the native run with the parent)."""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import adatta  # noqa: E402
from valida import cancello  # noqa: E402
from valida import nativo  # noqa: E402
from valida import sintetici_identita as si  # noqa: E402


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("out")
    ap.add_argument("--seme", type=int, default=7)
    ap.add_argument("--profilo", default="completo")
    a = ap.parse_args()
    out = Path(a.out)
    c, idx = cancello.genera_sintetico(a.seme, a.profilo, out / "lavoro")
    mzml = out / "mzml"
    mzml.mkdir(parents=True, exist_ok=True)
    for p in sorted((out / "lavoro").glob("SIN_*.mzML")):
        p.replace(mzml / p.name)
    si.scrivi_verita(out, c, a.seme, a.profilo)
    padre = next(o for o in c.ioni if o.ruolo == "progenitore")
    serie = nativo.esegui(idx, nativo.neutra_da_ione(padre.ion), padre.mz, padre.rt, out / "nativo")
    t = adatta.da_serie(serie, f"D2-{a.profilo}-{a.seme}")
    (out / f"mzlab_D2_predefinita.json").write_text(adatta.a_json(dict(t, versione=serie.versione_crate)), encoding="utf-8")
    print(out, len(t["righe"]), "rows,", len(list(mzml.glob('*.mzML'))), "files")


if __name__ == "__main__":
    main()
