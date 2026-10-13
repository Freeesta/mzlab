"""XCMS (R, Bioconductor): `corri_xcms.R` on the mzML (centWave, obiwarp, peak density, gap filling) -> CSV -> `bench/1`. See `corri.py`."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import adatta, corri  # noqa: E402

R = Path(__file__).with_name("corri_xcms.R")


def main() -> None:
    a = corri.argomenti(__doc__)
    fs = corri.file_mzml(a.mzml_dir, a.solo)
    nat = corri.cartella_nativa(a.out_dir, "xcms", a.config)
    csv_out = nat / "feature_values.csv"
    csv_out.unlink(missing_ok=True)
    r = subprocess.run(["Rscript", str(R), str(corri.PARAMETRI / "xcms" / f"{a.config}.json"), str(csv_out), *map(str, fs)], capture_output=True, text=True)
    (nat / "xcms.log").write_text(r.stdout + r.stderr, encoding="utf-8")
    if r.returncode != 0 or not csv_out.exists():
        raise SystemExit(f"XCMS failed (exit {r.returncode}):\n{r.stderr[-2000:]}")
    versione = next((l.split()[1] for l in r.stdout.splitlines() if l.startswith("xcms ")), "unknown")
    t = adatta.da_csv("xcms", csv_out, [f.stem for f in fs], a.dataset, a.config)
    print(corri.salva(t, a.out_dir, versione), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
