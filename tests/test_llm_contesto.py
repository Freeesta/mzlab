"""The golden `contesto/1` of `mzlab-llm` validates against `docs/schemi/contesto-1.json`, and a context with a path or a spectrum key does not."""
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "docs/schemi/contesto-1.json").read_text())
GOLDEN = json.loads((ROOT / "crates/mzlab-llm/tests/data/contesto_12_file.json").read_text())


def test_golden_validates():
    assert not list(Draft202012Validator(SCHEMA).iter_errors(GOLDEN))
    assert len(GOLDEN["files"]) == 12 and GOLDEN["features"]["rows"] == 800


def test_unknown_sections_are_rejected():
    v = Draft202012Validator(SCHEMA)
    assert list(v.iter_errors({**GOLDEN, "spectra": []}))
    assert list(v.iter_errors({**GOLDEN, "profile": "corso"}))
