"""`mzlab-cli schema NAME` serves schemas that validate the real documents: `mzflow/2` the BASE plan, `commands/1` and the compact catalogs the
output of their own commands; and the two contract schemas of docs/schemi are current."""
import json
import subprocess
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
BASE = {"format": "mzflow/2", "inputs": [{"name": "s_t000.mzML", "type": "raw"}, {"name": "s_t010.mzML", "type": "raw"}], "steps": [
    {"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:s_t000.mzML", "file:s_t010.mzML"]}},
    {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
    {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
    {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]}


@pytest.fixture(scope="module")
def cli():
    subprocess.run(["cargo", "build", "-q", "-p", "mzlab-cli"], cwd=ROOT, check=True)
    exe = ROOT / "target/debug/mzlab-cli"

    def run(*args):
        r = subprocess.run([str(exe), *args], capture_output=True, text=True, check=True)
        assert r.stdout.endswith("\n")
        return json.loads(r.stdout)

    return run


def test_mzflow2_validates_the_base_plan_and_refuses_a_broken_one(cli):
    v = Draft202012Validator(cli("schema", "mzflow/2"))
    assert not list(v.iter_errors(BASE))
    assert list(v.iter_errors({**BASE, "format": "mzflow/1"}))
    assert list(v.iter_errors({k: x for k, x in BASE.items() if k != "steps"}))


def test_the_commands_of_the_cli_validate_against_their_schemas(cli):
    for args, name in ((("commands", "--json"), "commands/1"), (("schema", "--list"), "schemas/1"), (("flow", "steps", "--compact"), "steps-compact/1"),
                       (("llm", "tools", "--compact"), "tools-compact/1")):
        doc = cli(*args)
        assert doc["schema"] == name
        assert not list(Draft202012Validator(cli("schema", name)).iter_errors(doc)), name


def test_the_contract_schemas_are_current():
    subprocess.run(["python3", str(ROOT / "tools/schemi_contratto.py"), "--check"], check=True)
