"""quant.curve@1: the flowcal .wasm (`flow_curve`, through Node) gives the bytes of `mzlab-cli flow run` on the synthetic series of
tools/synthetic_data_calibration.py, which are the golden files of crates/mzlab-cli/tests/data/calibration; the generator writes the committed inputs.
Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import sys
from pathlib import Path

import pytest
from cargo_util import build, cli, run

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates" / "mzlab-cli" / "tests" / "data" / "calibration"
PARAMS = {"istd": "IS", "exclude": "B|std8|saturated detector"}   # PARAMS of flow_curve_tests.rs
sys.path.insert(0, str(ROOT / "tools"))
from synthetic_data_calibration import calibration_series  # noqa: E402

needs = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def test_the_generator_writes_the_committed_inputs(tmp_path):
    for p in calibration_series(tmp_path):
        assert p.read_bytes() == (DATA / p.name).read_bytes(), p.name


@pytest.fixture(scope="module")
def pkg(tmp_path_factory):
    d = tmp_path_factory.mktemp("flowcal")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-flowcal-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_flowcal_wasm.wasm")
    return d


def wasm(pkg, params, resp=DATA / "responses.csv", stds=DATA / "standards.csv"):
    return json.loads(run("node", str(ROOT / "tests" / "flowcal_wasm_node.js"), str(pkg / "pkg"), json.dumps(params), str(resp), str(stds)).stdout)


def native(tmp_path, params):
    for f in ("responses.csv", "standards.csv"):
        shutil.copy2(DATA / f, tmp_path / f)
    plan = {"format": "mzflow/2", "inputs": [{"name": "responses.csv", "type": "report"}, {"name": "standards.csv", "type": "report"}],
            "steps": [{"id": "c", "step": "quant.curve@1", "in": {"responses": "file:responses.csv", "standards": "file:standards.csv"}, "params": params}]}
    (tmp_path / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = tmp_path / "out"
    cli("flow", "run", str(tmp_path / "plan.json"), "--in", str(tmp_path), "--out", str(out))
    return (out / "c.json").read_text(encoding="utf-8"), (out / "c.csv").read_text(encoding="utf-8")


@needs
@pytest.mark.parametrize("params", [PARAMS, {}, {"model": "quadratic", "weight": "1/x2", "istd": "IS"}, {"zero": True, "weight": "1", "signal": "height"}])
def test_wasm_and_cli_give_the_same_bytes(pkg, tmp_path, params):
    got = wasm(pkg, params)
    j, c = native(tmp_path, params)
    assert got["calibration"] == j and got["concentrations"] == c
    assert j.startswith('{"format":"taratura/1"')
    if params == PARAMS:
        assert j == (DATA / "golden.json").read_text(encoding="utf-8") and c == (DATA / "golden.csv").read_text(encoding="utf-8")
        assert "warn.flow.curve.excluded" in got["warnings"]


@needs
def test_an_error_is_the_same_key(pkg):
    with pytest.raises(Exception) as e:
        wasm(pkg, {"weight": "1/x3"})
    assert "err.flow.curve.param" in str(e.value)
