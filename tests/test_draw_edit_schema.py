"""The patches of the shared fixture (tests/dati/draw_d3.json, the ones that apply) validate against the hand-written docs/schemi/draw_edit-1.json,
and the fixture patches the schema can see are wrong (an unknown op or field, too many operations or ids) do not."""
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / "docs/schemi/draw_edit-1.json").read_text())
CASES = json.loads((ROOT / "tests/dati/draw_d3.json").read_text())["cases"]
# the failures the schema states (the rest, such as an unknown atom or mixed units, depend on the drawing)
SEEN_BY_SCHEMA = {"unknown op", "unknown field", "too many operations", "too many ids in a list"}


def patch(case):
    try:
        return json.loads(case["request"]["edit"])
    except ValueError:
        return None


def test_the_schema_is_a_valid_schema():
    Draft202012Validator.check_schema(SCHEMA)


@pytest.mark.parametrize("case", [c for c in CASES if c["exit"] == 0], ids=lambda c: c["name"])
def test_an_applied_patch_validates(case):
    errors = list(Draft202012Validator(SCHEMA).iter_errors(patch(case)))
    assert not errors, [e.message[:200] for e in errors[:3]]


@pytest.mark.parametrize("case", [c for c in CASES if c["name"] in SEEN_BY_SCHEMA], ids=lambda c: c["name"])
def test_a_patch_the_schema_can_see_is_wrong_does_not_validate(case):
    assert not Draft202012Validator(SCHEMA).is_valid(patch(case))


def test_the_fixture_has_each_of_them():
    assert {c["name"] for c in CASES} >= SEEN_BY_SCHEMA
