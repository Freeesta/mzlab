"""The schemas of the tutor (docs/schemi/tutor-ask-1.json, tutorstat-1.json): valid, and the samples the Rust tests write (tests/data/tutor) fit them."""
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent.parent


def schema(name: str) -> Draft202012Validator:
    s = json.loads((ROOT / f"docs/schemi/{name}-1.json").read_text())
    Draft202012Validator.check_schema(s)
    return Draft202012Validator(s)


ASK = {"course": "LAB3", "client": "ab" * 16, "lang": "it", "question": "Che cos'è un addotto?", "where": "20-esi#addotti", "history": [{"q": "a", "a": "b [1]"}], "share": False}


def test_ask_sample_and_limits():
    v = schema("tutor-ask")
    assert not list(v.iter_errors(ASK))
    for change in ({"question": "x" * 601}, {"lang": "fr"}, {"client": "short"}, {"extra": 1}, {"history": [{"q": "a", "a": "b"}] * 3}):
        assert list(v.iter_errors({**ASK, **change})), change


def test_stat_sample_from_the_rust_test():
    v = schema("tutorstat")
    sample = json.loads((ROOT / "tests/data/tutor/tutorstat-esempio.json").read_text())
    assert not list(v.iter_errors(sample))
    assert sample["schema"] == "tutorstat/1"
    assert list(v.iter_errors({**sample, "email": "x"}))


@pytest.mark.parametrize("name", ["req-pick.json", "req-answer.json"])
def test_expected_bodies_are_json_and_carry_no_key(name):
    body = json.loads((ROOT / "tests/data/tutor" / name).read_text())
    text = json.dumps(body)
    assert "sk-ant" not in text and body["stream"] is True
