"""Gate of the engine `untargeted` (mzmine-like, `crates/mzlab-feat/src/untargeted/pipeline.rs`): the example `untargeted` is run on the mzML of a series, its table is compared with the
final table of mzmine 3.9.0 (the same series) and the section `untargeted` of `TP_Mine/data/cancello.json` is written. The data of the laboratory are never in the repository: Federico runs

    python3 tools/valida/cancello_untargeted.py --mzml-dir <folder of the mzML> --mzmine <final CSV of mzmine> [--altezza-min 1e5] [--json TP_Mine/data/cancello.json]

(the CSV is the export of the batch `tools/bench/corri_mzmine.py`, columns `datafile:<name>:area|height`). The section is
`{ok, crate_version, recall, precision, spearman, log2, protocol: "ion/height-floor/counts-s", old_protocol: {...}, files: N, rows_mzmine, rows_native, floor}`; `ok` is true only if the new protocol
passes recall >= 0.95, precision >= 0.95, Spearman >= 0.95 and |log2| <= 0.10 AND the old one passes today's thresholds (0.90 / 0.80 / 0.90 / 0.25). Without `--mzmine` the section keeps
`ok: null` (not measured). Nothing else of the document is touched; `crate_version` is the one of the engine that was run."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from valida import ROOT, cancello_misure as M, nativo  # noqa: E402
from valida.cancello import a_json, versione_crate  # noqa: E402

PROTOCOLLO = "ion/height-floor/counts-s"
NEW_THRESHOLDS = {"richiamo": (">=", 0.95), "precisione": (">=", 0.95), "spearman": (">=", 0.95), "log2_mediano": ("<=", 0.10)}
OLD_THRESHOLDS = {"richiamo": (">=", 0.90), "precisione": (">=", 0.80), "spearman": (">=", 0.90), "log2_mediano": ("<=", 0.25)}
ESEMPIO = ROOT / "target" / "release" / "examples" / "untargeted"


def costruisci_esempio() -> None:
    subprocess.run(["cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--features", "untargeted", "--example", "untargeted"], cwd=ROOT, check=True)


def esegui(idx_paths: list[str], output: Path, parametri: str = "") -> None:
    """Runs the example on the indexes (in this order); writes `series.mzfeat` and the `.mzfeatf` in `output`."""
    costruisci_esempio()
    output.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(ESEMPIO), str(output), *idx_paths] + (["--params", parametri] if parametri else []), cwd=ROOT, check=True, capture_output=True)


def tabella(output: Path, nomi: list[str]):
    """`series.mzfeat` as a table of the validation (areas in counts x s, the cells absent at zero)."""
    import numpy as np
    from valida.tabelle import Tabella
    meta, a = nativo.apri((output / "series.mzfeat").read_bytes())
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k)
    area = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["area"].reshape(n, k)))
    alt = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["height"].reshape(n, k).astype(float)))
    return Tabella("untargeted", nomi, a["mz"].copy(), a["rt"].copy(), area, altezza=alt), meta["crate"]


def _pass(c: dict, thresholds: dict) -> bool:
    return all(k in c and (c[k] >= v if op == ">=" else c[k] <= v) for k, (op, v) in thresholds.items())


def sezione(native, mzmine, columns: list[int], min_height: float, crate: str) -> dict:
    """The section `untargeted` from the native table and the one of mzmine (the columns are the treated samples)."""
    new = M.concordanza_mzmine_ioni(native, mzmine, columns, min_height)
    old = M.concordanza_mzmine(native, mzmine, columns, 50.0)
    ok = _pass(new, NEW_THRESHOLDS) and _pass(old, OLD_THRESHOLDS)
    return {"ok": ok, "crate_version": crate, "recall": new.get("richiamo"), "precision": new.get("precisione"), "spearman": new.get("spearman"), "log2": new.get("log2_mediano"),
            "protocol": PROTOCOLLO, "old_protocol": {k: old.get(k) for k in ("richiamo", "precisione", "spearman", "log2_mediano")}, "floor": min_height,
            "rows_mzmine": new.get("righe_mzmine"), "rows_native": new.get("righe_native"), "files": len(columns)}


def vuota() -> dict:
    """Not measured: nothing is claimed."""
    return {"ok": None, "crate_version": versione_crate(), "recall": None, "precision": None, "spearman": None, "log2": None, "protocol": PROTOCOLLO}


def scrivi(sez: dict, destino: Path) -> None:
    """Puts the section in the document of the gate (created if missing), leaving every other key as it is."""
    doc = json.loads(destino.read_text(encoding="utf-8")) if destino.exists() else {}
    doc["untargeted"] = sez
    destino.write_text(a_json(doc), encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--mzml-dir", default="")
    ap.add_argument("--mzmine", default="")
    ap.add_argument("--solo", nargs="*", default=None)
    ap.add_argument("--altezza-min", type=float, default=1e5)
    ap.add_argument("--params", default="", help="recipe JSON of the engine (default: the batch of mzmine)")
    ap.add_argument("--lavoro", default=str(ROOT / ".verifica" / "cancello_untargeted"))
    ap.add_argument("--json", default=str(ROOT / "TP_Mine" / "data" / "cancello.json"))
    a = ap.parse_args(argv)
    if not (a.mzml_dir and a.mzmine):
        scrivi(vuota(), Path(a.json))
        print("section untargeted: not measured (no --mzml-dir and --mzmine)")
        return 0
    from bench import corri
    from valida.tabelle import leggi_mzmine
    fs = corri.file_mzml(Path(a.mzml_dir), a.solo)
    work = Path(a.lavoro)
    idx = nativo.indici(fs, work / "indici")
    nomi = [Path(x).stem for x in idx]
    esegui(idx, work / "uscita", a.params)
    native, crate = tabella(work / "uscita", nomi)
    mm = leggi_mzmine(a.mzmine, [{"nome": f.stem} for f in fs])      # mzmine names the columns after the mzML; the indexes may carry `min` after the time
    sez = sezione(native, mm, list(range(len(nomi))), a.altezza_min, crate)
    scrivi(sez, Path(a.json))
    print(json.dumps(sez, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
