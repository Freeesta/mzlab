"""The step/1 descriptors of the Tabui family (package tabui: crates/mzlab-feat/src/steps/tabui*.rs and crates/mzlab-chem/src/step.rs), exported to
docs/schemi/steps-tabui-1.json by the Rust test `steps_tabui`. Each one must validate against the draft contract of the workflow study."""
import json
import re
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = json.loads((ROOT / "docs/studi/workflow/contract.schema.json").read_text())
STEPS = json.loads((ROOT / "docs/schemi/steps-tabui-1.json").read_text())
IDS = ["tp.generate", "tp.filter", "tp.link", "tp.position", "structure.reactivity"]


def refs(step):
    return step.get("references", []) + [r for a in step["algorithms"] for r in a.get("references", [])]


@pytest.mark.parametrize("step", STEPS, ids=IDS)
def test_descriptor_validates_against_the_contract(step):
    v = Draft202012Validator({**CONTRACT, "$ref": "#/$defs/step"})
    errors = sorted(v.iter_errors(step), key=lambda e: list(e.path))
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]


def test_the_five_steps_in_order_and_in_the_tabui_package():
    assert [s["id"] for s in STEPS] == IDS
    assert all(s["package"] == "tabui" and s["schema"] == "step/1" for s in STEPS)


def test_the_default_algorithm_exists_and_every_algorithm_is_written():
    for s in STEPS:
        assert s["default_algorithm"] in [a["id"] for a in s["algorithms"]], s["id"]
        assert all(a["status"] == "available" for a in s["algorithms"]), s["id"]


def test_references_have_the_shape_of_the_study():
    for s in STEPS:
        for r in refs(s):
            assert re.fullmatch(r"10\.\d{4,9}/\S+", r["doi"]), r
            assert re.fullmatch(r"[a-z]+\d{4}[a-z]+", r["citation_key"]), r
            assert r["note"]
    by_key = {}
    for s in STEPS:
        for r in refs(s):
            assert by_key.setdefault(r["citation_key"], r["doi"]) == r["doi"], r


def test_citation_keys_do_not_clash_with_the_identification_chain():
    other = json.loads((ROOT / "docs/schemi/steps-attributio-1.json").read_text())
    known = {r["citation_key"]: r["doi"] for s in other for r in refs(s)}
    for s in STEPS:
        for r in refs(s):
            assert known.get(r["citation_key"], r["doi"]) == r["doi"], r


def test_every_error_key_is_a_string_of_the_code():
    sources = "\n".join(p.read_text(errors="ignore") for p in (ROOT / "crates").rglob("*.rs"))
    for s in STEPS:
        for key in s["errors"]:
            assert f'"{key}"' in sources, (s["id"], key)
