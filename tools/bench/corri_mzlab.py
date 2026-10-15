"""mzLab (the native run of the pipeline of the site, example `nativo`) on the mzML of a series -> `bench/1`. See `corri.py`.

Without `--genitore` only the feature table is produced (`series.mzfeat`); with `--genitore FORMULA_NEUTRA [--mz-genitore X --rt-genitore Y --smiles S]` the run also
tests the hypotheses and the table carries the identities (`candidato`, `livello`, `punteggio`). The cells «absent» of the table are zero areas."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import adatta, corri  # noqa: E402
from valida import ROOT, nativo  # noqa: E402


def extra(ap) -> None:
    ap.add_argument("--genitore", default="", help="neutral formula of the parent (enables hypotheses and identities)")
    ap.add_argument("--mz-genitore", type=float, default=float("nan"))
    ap.add_argument("--rt-genitore", type=float, default=float("nan"))
    ap.add_argument("--smiles", default="")
    ap.add_argument("--engine", choices=("mzfinder", "untargeted"), default="mzfinder", help="untargeted = the mzmine-like engine (example `untargeted`, features of mzlab-feat-wasm); --params = its recipe JSON")
    ap.add_argument("--params", default="")


def tabella_semplice(cartella: Path, nomi: list[str], dataset: str, config: str) -> dict:
    """`series.mzfeat` (no identities) -> `bench/1`."""
    meta, a = nativo.apri((cartella / "series.mzfeat").read_bytes())
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k)
    area = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["area"].reshape(n, k)))
    righe = [{"mz": float(a["mz"][i]), "rt": float(a["rt"][i]), "aree": [float(x) for x in area[i]]} for i in range(n)]
    return adatta.tabella("mzlab", dataset, config, nomi, righe)


def main() -> None:
    a = corri.argomenti(__doc__, extra)
    par = corri.parametri("mzlab", a.config)
    fs = corri.file_mzml(a.mzml_dir, a.solo)
    nat = corri.cartella_nativa(a.out_dir, "mzlab", a.config)
    nativo.costruisci_esempio()
    idx = nativo.indici(fs, nat / "indici")
    nomi = [Path(x).stem for x in idx]
    if a.engine == "untargeted":
        from valida import cancello_untargeted as U
        uscita = nat / "uscita"
        U.esegui(idx, uscita, a.params)
        t = tabella_semplice(uscita, nomi, a.dataset, a.config)
        versione = nativo.apri((uscita / "series.mzfeat").read_bytes())[0]["crate"]
    elif a.genitore:
        serie = nativo.esegui(idx, a.genitore, a.mz_genitore, a.rt_genitore, nat / "uscita", a.smiles)
        t = adatta.da_serie(serie, a.dataset, a.config)
        versione = serie.versione_crate
    else:
        uscita = nat / "uscita"
        uscita.mkdir(exist_ok=True)
        cmd = [str(nativo.ESEMPIO), str(uscita), *idx] + (["--recipe", par["recipe"]] if par["recipe"] else [])
        subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True)
        t = tabella_semplice(uscita, nomi, a.dataset, a.config)
        versione = nativo.apri((uscita / "series.mzfeat").read_bytes())[0]["crate"]
    print(corri.salva(t, a.out_dir, versione), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
