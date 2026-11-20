"""features-notes/1 (`mzlab-cli features --notes`) and the `notices` of estimates/1 respect their schemas: the documents the Rust test
`the_notes_of_a_series_hold_one_entry_per_file_in_the_given_order` produces on synthetic unit-resolution data (left in a temporary folder by
MZLAB_JSON_SAMPLES) are validated, and a document that breaks a rule is refused."""
import json
import os
import subprocess
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
FILES = {"features-notes": "features-notes.json", "estimates": "estimates.json"}


def schema(name):
    return json.loads((ROOT / f"docs/schemi/{name}-1.json").read_text())


@pytest.mark.parametrize("name", FILES)
def test_the_schema_is_valid_json_schema(name):
    s = schema(name)
    Draft202012Validator.check_schema(s)
    assert s["$id"] == f"{name}/1"


@pytest.fixture(scope="module")
def samples(tmp_path_factory):
    folder = tmp_path_factory.mktemp("notes_samples")
    r = subprocess.run(
        ["cargo", "test", "-p", "mzlab-cli", "--lib", "the_notes_of_a_series"],
        cwd=ROOT, capture_output=True, text=True, env={**os.environ, "MZLAB_JSON_SAMPLES": str(folder)},
    )
    assert r.returncode == 0, (r.stdout + r.stderr)[-2000:]
    return folder


@pytest.mark.parametrize("name", FILES)
def test_a_document_of_the_cli_respects_its_schema(samples, name):
    doc = json.loads((samples / FILES[name]).read_text())
    errors = sorted(Draft202012Validator(schema(name)).iter_errors(doc), key=lambda e: list(e.path))
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]
    assert doc["schema"] == f"{name}/1"


def test_the_notices_of_the_notes_and_of_the_estimate_are_the_same_items(samples):
    notes = json.loads((samples / "features-notes.json").read_text())
    estimate = json.loads((samples / "estimates.json").read_text())
    # the first file of the series is all profile: its notes and its estimate say the same notice
    assert notes["files"][0]["notices"] == estimate["notices"]
    assert notes["files"][0]["notices"][0]["key"] == "warn.features.profileUnit"


def test_a_notice_without_its_parameters_or_a_second_series_member_is_refused():
    v = Draft202012Validator(schema("features-notes"))
    bad = {"schema": "features-notes/1", "files": [{"file": "a", "axis": "Da", "defect": 0, "floor": 1, "min_height": 2, "profile_scans": 1,
                                                      "ms1_scans": 1, "notices": [{"key": "warn.features.profileUnit"}]}]}
    assert list(v.iter_errors(bad))
    assert list(v.iter_errors({"schema": "features-notes/1", "files": [], "extra": 1}))
