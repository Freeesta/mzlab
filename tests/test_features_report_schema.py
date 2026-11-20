"""features-report/1 (`mzlab-cli features --report`) and features-plan/1 (`--dry-run`) respect their schemas: the documents the Rust tests
`the_report_says_what_each_file_went_through...` and `a_dry_run_prints_the_plan...` produce on a synthetic unit-resolution series (left in a temporary
folder by MZLAB_JSON_SAMPLES) are validated, and a document that breaks a rule is refused."""
import json
import os
import subprocess
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
FILES = {"features-report": "features-report.json", "features-plan": "features-plan.json"}


def schema(name):
    return json.loads((ROOT / f"docs/schemi/{name}-1.json").read_text())


@pytest.mark.parametrize("name", FILES)
def test_the_schema_is_valid_json_schema(name):
    s = schema(name)
    Draft202012Validator.check_schema(s)
    assert s["$id"] == f"{name}/1"


@pytest.fixture(scope="module")
def samples(tmp_path_factory):
    folder = tmp_path_factory.mktemp("report_samples")
    r = subprocess.run(
        ["cargo", "test", "-p", "mzlab-cli", "--lib", "features_lr_tests"],
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


def test_the_report_and_the_plan_name_the_same_members(samples):
    report = json.loads((samples / "features-report.json").read_text())
    plan = json.loads((samples / "features-plan.json").read_text())
    # the two tests run in folders of their own: the members are the same files by name
    name = lambda m: Path(m).name  # noqa: E731
    assert [name(f["member"]) for f in report["files"]] == [name(m["member"]) for m in plan["members"]]
    assert all(f["centroided"] for f in report["files"]) and all(m["centroid"] for m in plan["members"])


def test_a_report_without_its_files_or_with_an_extra_member_is_refused():
    v = Draft202012Validator(schema("features-report"))
    assert list(v.iter_errors({"schema": "features-report/1"}))
    assert list(v.iter_errors({"schema": "features-report/1", "recipe": "x", "files": [], "merged": [], "choices": [], "extra": 1}))
    assert list(Draft202012Validator(schema("features-plan")).iter_errors({"schema": "features-plan/1", "members": [{"member": "a"}]}))
