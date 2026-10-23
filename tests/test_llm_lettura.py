"""`lettura/1`, `param-scelte/1` and `assist-ruoli/1` of `mzlab-llm`: the schemas accept the examples of docs/studi/assistente-dati and reject an unknown action, a ninth finding and a role file with a made-up field."""
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


def validator(name):
    return Draft202012Validator(json.loads((ROOT / f"docs/schemi/{name}.json").read_text()))


FINDING = {"id": "F1", "topic": "blank", "statement": "23 of 412 rows", "evidence": ["tabcheck.blank.value"], "confidence": "high",
           "action": {"type": "set_param", "step": "table.filter@1", "name": "blank_ratio", "value": 3}}
READING = {"schema": "lettura/1", "kind": "table", "findings": [FINDING],
           "questions": [{"id": "Q1", "text": "Known additive?", "options": ["yes", "no", "not sure"]}], "numbers": ["23", "412"]}


def test_reading_valid_and_defects():
    v = validator("lettura-1")
    assert not list(v.iter_errors(READING))
    assert list(v.iter_errors({**READING, "findings": [{**FINDING, "action": {"type": "delete_everything"}}]}))
    assert list(v.iter_errors({**READING, "findings": [FINDING] * 9}))
    assert list(v.iter_errors({**READING, "kind": "verdict"}))
    assert list(v.iter_errors({**READING, "questions": [{"id": "Q1", "text": "x", "options": ["only"]}]}))


def test_choices_valid_and_defects():
    v = validator("param-scelte-1")
    choice = {"step": "tp.dig_lr@1", "name": "tol_da", "value": 0.4, "source": "rule:R-da-1", "reason": "Half of the width."}
    doc = {"schema": "param-scelte/1", "choices": [choice], "questions": [], "skipped": [], "numbers": ["0.4"]}
    assert not list(v.iter_errors(doc))
    assert list(v.iter_errors({**doc, "choices": [{**choice, "source": "guess"}]}))
    assert list(v.iter_errors({**doc, "choices": [{k: x for k, x in choice.items() if k != "reason"}]}))


def test_roles_file_is_valid_and_strict():
    v = validator("assist-ruoli-1")
    roles = json.loads((ROOT / "crates/mzlab-llm/data/assist/assist-ruoli-1.json").read_text())
    assert not list(v.iter_errors(roles))
    roles["roles"][0]["temperature"] = 0
    assert list(v.iter_errors(roles))
