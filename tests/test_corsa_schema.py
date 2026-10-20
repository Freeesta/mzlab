"""`mzlab-cli flow report` on a real run of a synthetic plan (files of tools/dati_sintetici.py, never lab data): the `corsa/1` text validates against
docs/schemi/corsa-1.json, comes out with the same bytes twice, carries the `report/1` of the table, and the SHA-256 of an input follows its bytes."""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici  # noqa: E402

SCHEMA = json.loads((ROOT / "docs/schemi/corsa-1.json").read_text())
PLAN = {
    "format": "mzflow/2",
    "inputs": [{"name": f"{n}.mzML", "type": "mzml"} for n in "ab"],
    "steps": [{"id": "t", "step": "features.table@1", "in": {"source": [f"file:{n}.mzML" for n in "ab"]}, "params": {"polarity": 1}}],
}


def cargo(*args):
    r = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]


def run(folder, tmp, seed):
    folder.mkdir()
    rng = np.random.default_rng(seed)
    for n, t in (("a", 0), ("b", 15)):
        dati_sintetici.full_scan(folder / f"{n}.mzML", t, rng)
    (folder / "plan.json").write_text(json.dumps(PLAN))
    work = tmp / (folder.name + "_work")
    work.mkdir()
    r = subprocess.run([str(ROOT / "target/release/examples/flow"), str(folder / "plan.json"), str(work), "1"], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    # the CSV step does not write the table report: a report/1 left beside prov.json is included as it is
    (work / "report.json").write_text('{"schema":"report/1","rows":3}\n')
    cli = [str(ROOT / "target/release/mzlab-cli"), "flow", "report", str(work)]
    a = subprocess.run(cli, capture_output=True, text=True)
    assert a.returncode == 0, a.stderr[-2000:]
    return a.stdout, subprocess.run(cli, capture_output=True, text=True).stdout


@pytest.fixture(scope="module")
def corsa(tmp_path_factory):
    cargo("build", "--release", "-p", "mzlab-cli", "--example", "flow", "--bin", "mzlab-cli")
    tmp = tmp_path_factory.mktemp("corsa")
    return run(tmp / "one", tmp, 7), run(tmp / "two", tmp, 8)


def test_validates_and_is_stable(corsa):
    first, again = corsa[0]
    assert first == again
    c = json.loads(first)
    assert not list(Draft202012Validator(SCHEMA).iter_errors(c))
    assert c["steps"][0]["status"] == "ok" and [f["name"] for f in c["inputs"]] == ["a.mzML", "b.mzML"]


def test_includes_the_table_report(corsa):
    assert json.loads(corsa[0][0])["report"] == {"schema": "report/1", "rows": 3}


def test_an_input_that_changes_changes_its_sha256(corsa):
    one, two = (json.loads(c[0]) for c in corsa)
    assert one["inputs"][0]["sha256"] != two["inputs"][0]["sha256"]
    assert one["plan_sha256"] == two["plan_sha256"]
