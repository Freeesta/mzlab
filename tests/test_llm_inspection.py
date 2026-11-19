"""The golden `inspection/1` files of `mzlab-llm` validate against `docs/schemi/inspection-1.json`, and a private field does not."""
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "docs/schemi/inspection-1.json").read_text())
LINES = (ROOT / "crates/mzlab-llm/tests/data/inspection_synthetic.json").read_text().splitlines()
FILES = [json.loads(l.rstrip(",")) for l in LINES if l.strip().startswith("{")]


def doc(files):
    return {"schema": "inspection/1", "profile": "research", "budget_tokens": 8000, "truncated": [], "files": files}


def test_golden_validates():
    assert len(FILES) >= 10
    assert not list(Draft202012Validator(SCHEMA).iter_errors(doc(FILES)))


def test_private_fields_are_rejected():
    v = Draft202012Validator(SCHEMA)
    for key in ("path", "operator", "serial", "computer"):
        assert list(v.iter_errors(doc([{**FILES[0], key: "x"}])))
