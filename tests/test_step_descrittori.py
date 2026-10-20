"""The step/1 descriptors of the flagship chain (crates/mzlab-core/src/step, crates/mzlab-feat/src/steps), exported to docs/schemi/steps-1.json by
the Rust test `steps`. The external truth is the draft contract of the workflow study, not our code: every descriptor must validate against it."""
import copy
import json
import re
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = json.loads((ROOT / "docs/studi/workflow/contract.schema.json").read_text())
STEPS = json.loads((ROOT / "docs/schemi/steps-1.json").read_text())
SKETCH = json.loads((ROOT / "docs/studi/workflow/step.features.detect.json").read_text())
IDS = ["import.raw", "convert.mzml", "index.mzidx", "features.estimate", "features.detect", "features.align",
       "features.gapfill", "table.filter", "features.ms2_link", "export.table_csv"]


def validator(definition):
    """A validator of one definition of the contract ($defs/step, $defs/algorithm...)."""
    return Draft202012Validator({**CONTRACT, "$ref": f"#/$defs/{definition}"})


def test_the_contract_is_a_valid_schema():
    Draft202012Validator.check_schema(CONTRACT)


@pytest.mark.parametrize("step", STEPS, ids=[s["id"] for s in STEPS])
def test_descriptor_validates_against_the_contract(step):
    errors = sorted(validator("step").iter_errors(step), key=lambda e: list(e.path))
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]


def test_the_validation_is_not_vacuous():
    broken = copy.deepcopy(STEPS[0])
    del broken["out"]
    assert list(validator("step").iter_errors(broken))
    broken = copy.deepcopy(STEPS[4])
    broken["algorithms"][0]["status"] = "ready"
    assert list(validator("step").iter_errors(broken))


def test_the_ten_steps_of_the_chain_in_order():
    assert [s["id"] for s in STEPS] == IDS
    assert all(s["package"] == "core" and s["version"] == 1 and s["schema"] == "step/1" for s in STEPS)


def test_references_have_the_shape_of_the_study():
    keys = [r["citation_key"] for s in STEPS for r in s.get("references", []) + [x for a in s.get("algorithms", []) for x in a.get("references", [])]]
    assert keys == ["tautenhahn2008highly"]
    for s in STEPS:
        for a in s.get("algorithms", []):
            for r in a.get("references", []):
                assert re.fullmatch(r"10\.\d{4,9}/\S+", r["doi"])


def test_default_algorithm_and_preset_exist():
    for s in STEPS:
        if "default_algorithm" in s:
            assert s["default_algorithm"] in [a["id"] for a in s["algorithms"]]
        if "default_preset" in s:
            assert s["default_preset"] in s["presets"]
        for p in s.get("presets", {}).values():
            assert p.get("algorithm", s.get("default_algorithm")) in [a["id"] for a in s["algorithms"]]


def test_features_detect_agrees_with_the_study_sketch():
    detect = next(s for s in STEPS if s["id"] == "features.detect")
    assert [(a["id"], a["status"]) for a in detect["algorithms"]] == [(a["id"], a["status"]) for a in SKETCH["algorithms"]]
    for name in ("floor", "min_height", "polarity", "axis"):
        assert detect["params"]["properties"][name]["default"] == SKETCH["params"]["properties"][name]["default"]
    assert detect["presets"]["hr"]["params"] == SKETCH["presets"]["hr"]["params"]
    assert detect["presets"]["lr"]["params"] == SKETCH["presets"]["lr"]["params"]
    mine = {a["id"]: a["params"]["properties"] for a in detect["algorithms"]}
    for a in SKETCH["algorithms"]:
        for name, p in a["params"]["properties"].items():
            assert mine[a["id"]][name]["default"] == p["default"], (a["id"], name)


def test_every_error_key_is_a_string_of_the_code():
    sources = "\n".join(p.read_text(errors="ignore") for p in (ROOT / "crates").rglob("*.rs"))
    for s in STEPS:
        for key in s["errors"]:
            assert f'"{key}"' in sources, (s["id"], key)
