"""The step/1 descriptors that live in crates/mzlab-cli (spectra.ms2), exported to docs/schemi/steps-cli-1.json by the Rust test `flow_ms2`."""
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = json.loads((ROOT / "docs/studi/workflow/contract.schema.json").read_text())
STEPS = json.loads((ROOT / "docs/schemi/steps-cli-1.json").read_text())


def test_descriptors_validate_against_the_contract():
    v = Draft202012Validator({**CONTRACT, "$ref": "#/$defs/step"})
    for step in STEPS:
        errors = sorted(v.iter_errors(step), key=lambda e: list(e.path))
        assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]


def test_ms2_step_turns_mzml_into_spectra():
    (s,) = STEPS
    assert s["id"] == "spectra.ms2" and s["in"]["source"]["type"] == "mzml" and s["out"]["spectra"]["type"] == "spectra"
