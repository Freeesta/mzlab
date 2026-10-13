"""Contract of the low-resolution Dig of mzFinder (`crates/mzlab-feat/src/legacy/lr/`, test crates/mzlab-finder-lr-wasm/tests/lr_dig.rs): the keys
and JSON types of `api.run` and `api.detail` of the Python engine on the synthetic series of TP_Mine/tests/lr_synth.py, and its rows that pass the
criteria (m/z, RT, kind, label). No parity of the numbers: the Rust Dig runs on the pipeline, not on this engine.

    python3 tools/finder_lr_riferimento.py [--golden]

Writes tests/golden/finder_lr_contratto.json with --golden, else prints it. Synthetic inputs only (fixed seed), no compound names.
"""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path[:0] = [str(AQUI.parent), str(AQUI.parent / "TP_Mine" / "py"), str(AQUI.parent / "TP_Mine" / "tests")]

import lr_synth  # noqa: E402
from tpmine.engine import Experiment  # noqa: E402

PARENT = {"name": "Parent", "neutral": "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"}
# keys whose value can be null even when this series gives a value
NULLABLE = {"offset", "decay", "mrm", "score", "level", "ref_rt", "tmax", "isotope_of", "generation", "isf", "mz_observed", "peak", "apex_rt",
            "time"}


def kind(v) -> str:
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "bool"
    if isinstance(v, (int, float)):
        return "number"
    return {str: "string", list: "array", dict: "object"}[type(v)]


def shape(items: list[dict]) -> dict:
    """Every key of the dicts with the sorted list of the JSON types it takes (null added for the nullable keys)."""
    out: dict[str, set] = {}
    for d in items:
        for k, v in d.items():
            out.setdefault(k, set()).add(kind(v))
    return {k: sorted(t | ({"null"} if k in NULLABLE else set())) for k, t in sorted(out.items())}


def contract() -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        ex = Experiment(lr_synth.write_series(tmp), PARENT, {})
        s = json.loads(json.dumps(ex.run()))
        passing = [r for r in s["rows"] if r["label"] in ("progenitore", "forte", "possibile")]
        details = [json.loads(json.dumps(ex.detail(r["id"]))) for r in passing]
    return {
        "summary": shape([s]),
        "row": shape(s["rows"]),
        "kinetics": shape([k for r in s["rows"] for k in r["kinetics"]]),
        "detail": shape(details),
        "trace": shape([t for d in details for t in d["traces"]]),
        "python_rows": [{"mz": r["mz"], "ref_rt": r["ref_rt"], "kind": r["kind"], "label": r["label"]} for r in passing],
    }


if __name__ == "__main__":
    text = json.dumps(contract(), indent=1, sort_keys=True) + "\n"
    if "--golden" in sys.argv:
        (AQUI.parent / "tests" / "golden" / "finder_lr_contratto.json").write_text(text, encoding="utf-8")
    else:
        print(text)
