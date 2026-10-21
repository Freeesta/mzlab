"""The JSON answers of `mzlab-cli` for agents: the four schemas (flowcheck/1, flowdry/1, flowrun/1, errore/1) are valid JSON Schema, and the documents
the Rust test `flow_json` produces on its synthetic plan (left in a temporary folder by MZLAB_JSON_SAMPLES) respect them."""
import json
import os
import subprocess
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
NAMES = {"flowcheck": "flowcheck.json", "flowdry": "flowdry.json", "flowrun": "flowrun.json", "errore": "errore.json"}


def schema(name):
    return json.loads((ROOT / f"docs/schemi/{name}-1.json").read_text())


@pytest.mark.parametrize("name", NAMES)
def test_the_schema_is_valid_json_schema(name):
    s = schema(name)
    Draft202012Validator.check_schema(s)
    assert s["$id"] == f"{name}/1"


@pytest.fixture(scope="module")
def samples(tmp_path_factory):
    folder = tmp_path_factory.mktemp("json_samples")
    r = subprocess.run(
        ["cargo", "test", "-p", "mzlab-cli", "--lib", "flow_json"],
        cwd=ROOT, capture_output=True, text=True, env={**os.environ, "MZLAB_JSON_SAMPLES": str(folder)},
    )
    assert r.returncode == 0, (r.stdout + r.stderr)[-2000:]
    return folder


@pytest.mark.parametrize("name", NAMES)
def test_a_document_of_the_cli_respects_its_schema(samples, name):
    doc = json.loads((samples / NAMES[name]).read_text())
    errors = sorted(Draft202012Validator(schema(name)).iter_errors(doc), key=lambda e: list(e.path))
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]
    assert doc["schema"] == f"{name}/1"


def test_flowrun_lists_the_files_with_their_hashes(samples):
    doc = json.loads((samples / "flowrun.json").read_text())
    files = [o for s in doc["steps"] for o in s["outputs"]]
    assert files and all(len(o["sha256"]) == 64 for o in files)
