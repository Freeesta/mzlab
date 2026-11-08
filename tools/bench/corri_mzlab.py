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


def table_simple(folder: Path, names: list[str], dataset: str, config: str) -> dict:
    """`series.mzfeat` (no identities) -> `bench/1`."""
    meta, a = nativo.open_data((folder / "series.mzfeat").read_bytes())
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k)
    area = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["area"].reshape(n, k)))
    righe = [{"mz": float(a["mz"][i]), "rt": float(a["rt"][i]), "aree": [float(x) for x in area[i]]} for i in range(n)]
    return adatta.tabella("mzlab", dataset, config, names, righe)


def main() -> None:
    a = corri.argomenti(__doc__, extra)
    par = corri.parameters("mzlab", a.config)
    fs = corri.file_mzml(a.mzml_dir, a.solo)
    nat = corri.native_folder(a.out_dir, "mzlab", a.config)
    nativo.build_example()
    idx = nativo.indexes(fs, nat / "indici")
    names = [Path(x).stem for x in idx]
    if a.engine == "untargeted":
        from valida import cancello_untargeted as U
        output = nat / "uscita"
        U.esegui(idx, output, a.params)
        t = table_simple(output, names, a.dataset, a.config)
        version = nativo.open_data((output / "series.mzfeat").read_bytes())[0]["crate"]
    elif a.genitore:
        series = nativo.esegui(idx, a.genitore, a.mz_genitore, a.rt_genitore, nat / "uscita", a.smiles)
        t = adatta.from_series(series, a.dataset, a.config)
        version = series.versione_crate
    else:
        output = nat / "uscita"
        output.mkdir(exist_ok=True)
        cmd = [str(nativo.EXAMPLE), str(output), *idx] + (["--recipe", par["recipe"]] if par["recipe"] else [])
        subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True)
        t = table_simple(output, names, a.dataset, a.config)
        version = nativo.open_data((output / "series.mzfeat").read_bytes())[0]["crate"]
    print(corri.salva(t, a.out_dir, version), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
