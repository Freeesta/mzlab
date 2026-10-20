"""§7 test 3 of the workflow study: a small synthetic `mzflow/2` (files of tools/dati_sintetici.py, never lab data) run twice natively and once as WASM (wasm32-wasip1
under node's WASI) gives the same SHA-256 for every file it makes, and a `prov.json` that validates `prov1` of the draft contract."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici  # noqa: E402

CONTRACT = json.loads((ROOT / "docs/studi/workflow/contract.schema.json").read_text())
PLAN = {
    "format": "mzflow/2",
    "program": {"name": "mzLab", "version": "test"},
    "inputs": [{"name": f"{n}.mzML", "type": "mzml"} for n in "abc"],
    "steps": [
        {"id": "low", "step": "features.table@1", "in": {"source": [f"file:{n}.mzML" for n in "abc"]},
         "params": {"polarity": 1, "min_height": 100000}},
        # the same chain with other numbers, the keys in another order and a value spelled differently
        {"id": "high", "step": "features.table@1", "in": {"source": [f"file:{n}.mzML" for n in "abc"]},
         "params": {"min_height": 1e6, "polarity": 1}},
    ],
}
# SHA-256 of the three inputs and of the two tables: written here, so a drift of the engine or of the synthetic files is a visible change
INPUTS = {
    "file:a.mzML": "5671f6315a51881f3fd28080be6e2ecd8882f9e686ab81ae799cea1ebcbd686b",
    "file:b.mzML": "2dc833c170ee71037d1a3b87732261d3c304bbaba102804d483d0c4f893a17bf",
    "file:c.mzML": "b97698d8b27df1180ea2488b3a2c73cfa8e89f7e40e8a382923b6099917ec296",
}
GENERATED = {
    "low.report": "e85d52a65f6cbe85f70d8e1afac642144e55821924542012b771cd6adb08a8e0",
    "high.report": "bcc9ff769b8a186663b012e83babd185479671686ee8f28925a4c7d1e356a98b",
}


@pytest.fixture(scope="module")
def synthetic(tmp_path_factory):
    d = tmp_path_factory.mktemp("flow_in")
    rng = np.random.default_rng(7)
    for n, t in (("a", 0), ("b", 15), ("c", 60)):
        dati_sintetici.full_scan(d / f"{n}.mzML", t, rng)
    (d / "plan.json").write_text(json.dumps(PLAN))
    return d


def cargo(*args):
    r = subprocess.run(["cargo", *args], cwd=ROOT, capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]


def run_native(synthetic, work, workers):
    exe = ROOT / "target/release/examples/flow"
    work.mkdir()
    r = subprocess.run([str(exe), str(synthetic / "plan.json"), str(work), str(workers)], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    return json.loads((work / "prov.json").read_text())


@pytest.fixture(scope="module")
def native(synthetic, tmp_path_factory):
    cargo("build", "--release", "-p", "mzlab-cli", "--example", "flow")
    base = tmp_path_factory.mktemp("flow_native")
    return [run_native(synthetic, base / "one", 1), run_native(synthetic, base / "two", 2)]


def made(prov):
    return {k: v["sha256"] for k, v in prov["entity"].items() if not k.startswith("file:")}


def without_clock(prov):
    out = json.loads(json.dumps(prov))
    for a in out["activity"].values():
        a.pop("ms", None)
        a.pop("heap_peak_bytes", None)
    return out


def test_native_twice_gives_the_same_files(native):
    assert {k: v["sha256"] for k, v in native[0]["entity"].items() if k.startswith("file:")} == INPUTS
    assert made(native[0]) == GENERATED
    assert made(native[1]) == GENERATED
    # one worker or two: the same record but for the clock
    assert without_clock(native[0]) == without_clock(native[1])


def test_prov_validates_prov1(native):
    v = Draft202012Validator({**CONTRACT, "$ref": "#/$defs/prov1"})
    assert not list(v.iter_errors(native[0]))
    low = native[0]["activity"]["low"]
    assert low["step"] == "features.table@1" and low["used"] == list(INPUTS) and low["generated"] == ["low.report"]
    # canonical: sorted keys, shortest numbers, the same fingerprint however the file was written
    assert low["params"] == {"min_height": 100000, "polarity": 1}
    assert native[0]["activity"]["high"]["params"] == {"min_height": 1000000, "polarity": 1}
    broken = json.loads(json.dumps(native[0]))
    del broken["activity"]["low"]["params_sha256"]
    assert list(v.iter_errors(broken))


def test_wasm_gives_the_same_files(native, synthetic, tmp_path):
    if shutil.which("node") is None:
        pytest.skip("node is not installed")
    have = subprocess.run(["rustup", "target", "list", "--installed"], capture_output=True, text=True).stdout
    if "wasm32-wasip1" not in have:
        pytest.skip("rustup target add wasm32-wasip1")
    cargo("build", "--release", "--target", "wasm32-wasip1", "-p", "mzlab-cli", "--example", "flow")
    out = tmp_path / "out"
    out.mkdir()
    module = ROOT / "target/wasm32-wasip1/release/examples/flow.wasm"
    r = subprocess.run(["node", str(ROOT / "tools/wasi_run.mjs"), str(module), str(synthetic), str(out), "/in/plan.json", "/out"],
                       capture_output=True, text=True)
    assert r.returncode == 0, r.stderr[-2000:]
    prov = json.loads((out / "prov.json").read_text())
    assert prov["agent"]["platform"] == "wasm"
    assert made(prov) == GENERATED
    a, b = without_clock(prov), without_clock(native[0])
    a["agent"], b["agent"] = {}, {}
    # the plan hash is the one of the same file; everything else is equal
    assert a == b
