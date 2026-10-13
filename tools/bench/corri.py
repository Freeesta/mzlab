"""Shared by the `corri_<tool>.py` runners: find the mzML of a series, write the native output and the common table `bench/1`.

A runner is `python3 tools/bench/corri_<tool>.py MZML_DIR OUT_DIR --dataset D1 [--config predefinita]`: it runs the tool on ALL the mzML of the directory (same input
for every tool), keeps the native output in OUT_DIR/<tool>/<config>/ and writes OUT_DIR/<tool>_<dataset>_<config>.json (`bench/1`). Only the quality of the result
matters: no timing is taken. Parameters live in `parametri/<tool>/<config>.json` (versioned; the default is the documented one for Orbitrap)."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from . import adatta

PARAMETRI = Path(__file__).resolve().parent / "parametri"


def argomenti(descrizione: str, extra=None) -> argparse.Namespace:
    ap = argparse.ArgumentParser(description=descrizione)
    ap.add_argument("mzml_dir")
    ap.add_argument("out_dir")
    ap.add_argument("--dataset", default="D1")
    ap.add_argument("--config", default="predefinita")
    ap.add_argument("--solo", nargs="*", help="only these mzML stems (smoke tests)")
    if extra:
        extra(ap)
    return ap.parse_args()


def file_mzml(cartella, solo=None) -> list[Path]:
    fs = sorted(Path(cartella).glob("*.mzML"))
    if solo:
        fs = [f for f in fs if f.stem in set(solo)]
    if not fs:
        raise SystemExit("no mzML files")
    return fs


def parametri(strumento: str, config: str) -> dict:
    return json.loads((PARAMETRI / strumento / f"{config}.json").read_text(encoding="utf-8"))


def cartella_nativa(out_dir, strumento: str, config: str) -> Path:
    p = Path(out_dir) / strumento / config
    p.mkdir(parents=True, exist_ok=True)
    return p


def salva(t: dict, out_dir, versione: str) -> Path:
    t = dict(t, versione=versione)
    p = Path(out_dir) / f"{t['strumento']}_{t['dataset']}_{t['config']}.json"
    p.write_text(adatta.a_json(t), encoding="utf-8")
    return p
