"""The step/1 descriptors of the identification chain (package attributio, crates/mzlab-feat/src/steps/ident*.rs), exported to
docs/schemi/steps-attributio-1.json by the Rust test `steps_attributio`. Each one must validate against the draft contract of the workflow study."""
import json
import re
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = json.loads((ROOT / "docs/studi/workflow/contract.schema.json").read_text())
STEPS = json.loads((ROOT / "docs/schemi/steps-attributio-1.json").read_text())
RECORDS = json.loads((ROOT / "docs/schemi/records-1.json").read_text())
IDS = ["spectra.chimeric", "formula.ms1", "formula.ms2", "spectra.match", "spectra.insilico", "rt.predict",
       "annot.level", "annot.regulatory", "annot.hazard"]


def refs(step):
    return step.get("references", []) + [r for a in step["algorithms"] for r in a.get("references", [])]


@pytest.mark.parametrize("step", STEPS, ids=IDS)
def test_descriptor_validates_against_the_contract(step):
    v = Draft202012Validator({**CONTRACT, "$ref": "#/$defs/step"})
    errors = sorted(v.iter_errors(step), key=lambda e: list(e.path))
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]


def test_the_nine_steps_in_order_and_all_need_high_resolution():
    assert [s["id"] for s in STEPS] == IDS
    for s in STEPS:
        assert s["package"] == "attributio" and s["schema"] == "step/1"
        assert any(p.get("requires", {}).get("resolution") == ["hr"] for p in s["in"].values()), s["id"]


def test_the_default_algorithm_exists():
    for s in STEPS:
        assert s["default_algorithm"] in [a["id"] for a in s["algorithms"]], s["id"]


def test_references_have_the_shape_of_the_study():
    keys = [r["citation_key"] for s in STEPS for r in refs(s)]
    assert len(keys) == len(set(keys)) and keys
    for s in STEPS:
        for r in refs(s):
            assert re.fullmatch(r"10\.\d{4,9}/\S+", r["doi"]), r
            assert re.fullmatch(r"[a-z]+\d{4}[a-z]+", r["citation_key"]), r
            assert r["note"]


def test_what_is_not_written_yet_is_planned():
    status = {a["id"]: a["status"] for s in STEPS for a in s["algorithms"]}
    assert status["library_deconvolution"] == "planned"
    assert [k for k, v in status.items() if v == "planned"] == ["library_deconvolution"]


def test_the_shared_records_are_the_data_types_of_the_steps():
    assert [r["dataType"] for r in RECORDS] == ["candidates", "annot"]
    used = {p["type"] for s in STEPS for p in [*s["in"].values(), *s["out"].values()]}
    assert {r["dataType"] for r in RECORDS} <= used
    for r in RECORDS:
        names = [f["name"] for f in r["fields"]]
        assert len(names) == len(set(names)) and r["fields"]


def test_every_error_key_is_a_string_of_the_code():
    sources = "\n".join(p.read_text(errors="ignore") for p in (ROOT / "crates").rglob("*.rs"))
    for s in STEPS:
        for key in s["errors"]:
            assert f'"{key}"' in sources, (s["id"], key)
