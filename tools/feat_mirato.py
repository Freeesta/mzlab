"""Synthetic series with weak products for the targeted pass (`crates/mzlab-feat/src/target.rs`): the series of `sintetici_identita.py` with the
height of the products (tp1, tp2, isomer) multiplied by a factor, so that some peaks fall under the detection (minimum height 2e5, ten times the noise floor).

    python3 tools/feat_mirato.py --seme N --profilo piccolo|completo|debole --scala F --out DIR

Writes in DIR the mzML files, `verita.json` / `verita.csv` (same format as the series; the ions are invented) and one `.mzidx` per file (the reader of the
Rust crate). The molecules, the numbers and the seed are those of the series: only the heights of the products change.
The profile `debole` is `piccolo` plus `DEBOLI` matrix features in every file with a height between `WEAK_RATIO` (0.3, `crates/mzlab-feat/src/recipe.rs`) and 1 of the
minimum height (2e5): the weak tier (`series::weak_tier`) makes tens of thousands of rows of them, enough for the weak tables to leave the memory of the module
(`spill::Weak::add`) under a budget of a few MB."""
import argparse
import sys
from pathlib import Path

import numpy as np

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI))
from valida import sintetici_identita as si           # noqa: E402
from valida.sintetici_mzml import scrivi_file          # noqa: E402
from mzlab.reader import mzidx                         # noqa: E402
from mzlab.reader.mzml import Run                      # noqa: E402

DEBOLI = 5000
ALTEZZE_DEBOLI = (7e4, 1.8e5)
si.PROFILI["debole"] = {**si.PROFILI["piccolo"], "debole": DEBOLI}
PRODOTTI = ("tp1", "tp2", "isomero")


def genera(seme: int, profilo: str, scala: float, out: Path) -> list[float]:
    """Heights (maximum of the most abundant component over the files) of the products after the scaling."""
    out.mkdir(parents=True, exist_ok=True)
    c = si.costruisci(seme, profilo)
    if c.P.get("debole"):
        rng = np.random.default_rng([seme, 77])
        n = c.P["debole"]
        rm, rr, rs, rh = c.rumore
        c.rumore = (np.concatenate([rm, rng.uniform(*si.MZ_RUMORE, n)]), np.concatenate([rr, rng.uniform(1.0, c.rt_fine - 0.3, n)]),
                    np.concatenate([rs, rng.uniform(0.04, 0.08, n)]), np.concatenate([rh, rng.uniform(*ALTEZZE_DEBOLI, n)]))
    alte = []
    for o in c.ioni:
        if o.ruolo in PRODOTTI and o.amp is not None:
            o.amp = o.amp * scala
            alte.append(float(o.amp.max()))
    for k, (stem, _, _) in enumerate(c.file):
        scrivi_file(out / f"SIN_{stem}.mzML", k, c, seme)
        run = Run(out / f"SIN_{stem}.mzML", index_path=out / f"SIN_{stem}.mzidx")
        mzidx.build(run, out / f"SIN_{stem}.mzidx")
    si.scrivi_verita(out, c, seme, profilo)
    return sorted(alte)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--seme", type=int, default=1)
    ap.add_argument("--profilo", choices=sorted(si.PROFILI), default="piccolo")
    ap.add_argument("--scala", type=float, default=1.0)
    ap.add_argument("--out", required=True)
    a = ap.parse_args(argv)
    h = genera(a.seme, a.profilo, a.scala, Path(a.out))
    print(f"{len(h)} products, heights {h[0]:.3g} .. {h[-1]:.3g}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
