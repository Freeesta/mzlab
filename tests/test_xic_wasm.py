"""`api/xic` with `smooth` of the engine .wasm (mzlab-wasm, through Node, the generic `route` the site calls) gives the same numbers, token for token, as
`mzlab-cli trace xic` (single ion, --smooth, a list, the wide CSV) on a public synthetic full-scan file. Skipped without cargo, wasm-bindgen or node."""
import json
import re
import shutil
import sys
from pathlib import Path

import numpy as np
import pytest
from cargo_util import build, build_example, cli, example, run

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds

pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

ION, TOL = "241.3", "0.4"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("xic")
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "pkg"), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm")
    build("-p", "mzlab-cli")
    build_example("mzlab-core", "mzidx")
    ds.full_scan(d / "scan.mzML", 10.0, np.random.default_rng(7))
    return d


def wasm_xic(work, mz, smooth):
    mzml = work / "scan.mzML"
    idx = work / "scan.mzidx"
    if not idx.exists():
        example("mzlab-core", "mzidx", str(mzml), str(idx))
    query = f"mz={mz}&tol={TOL}&level=1&smooth={smooth}"
    plan = [{"op": "idxopen", "slot": 0, "name": mzml.name}, {"op": "route", "slot": 0, "path": "/api/xic", "query": query}]
    (work / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = json.loads(run("node", str(ROOT / "tests" / "rotte_wasm_node.js"), str(work / "pkg"), str(idx), str(mzml), str(work / "plan.json")).stdout)
    assert "s" in out[1], out[1]
    return out[1]["s"]


def tokens(text, key):
    body = re.search(rf'"{key}":\[(.*?)\]', text).group(1)
    return body.split(",") if body else []


def cli_points(work, *args):
    doc = cli("trace", "xic", str(work / "scan.mzML"), "--da", TOL, *args).stdout
    return [m.groups() for m in re.finditer(r"\[([-0-9.e]+),([-0-9.e]+)\]", doc.split('"points":')[1])]


@pytest.mark.parametrize("smooth", [3, 5])
def test_wasm_route_and_cli_give_the_same_trace(work, smooth):
    got = wasm_xic(work, ION, smooth)
    want = cli_points(work, "--mz", ION, "--smooth", str(smooth))
    assert list(zip(tokens(got, "rt"), tokens(got, "y"))) == want
    assert len(want) > 10


def test_wasm_route_and_cli_give_the_same_wide_csv(work):
    ions = ["241.3", "300"]
    csv = cli("trace", "xic", str(work / "scan.mzML"), "--da", TOL, "--mz", ",".join(ions), "--smooth", "5", "--format", "csv").stdout.splitlines()
    assert csv[0] == "rt_min,xic:241.3,xic:300"
    rows = [r.split(",") for r in csv[1:]]
    for col, mz in enumerate(ions, 1):
        got = wasm_xic(work, mz, 5)
        assert [(r[0], r[col]) for r in rows] == list(zip(tokens(got, "rt"), tokens(got, "y")))


def wasm_sweep(work, spec, smooth):
    mzml = work / "scan.mzML"
    idx = work / "scan.mzidx"
    if not idx.exists():
        example("mzlab-core", "mzidx", str(mzml), str(idx))
    query = f"sweep={spec}&tol={TOL}&level=1&smooth={smooth}"
    plan = [{"op": "idxopen", "slot": 0, "name": mzml.name}, {"op": "route", "slot": 0, "path": "/api/xic", "query": query}]
    (work / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    out = json.loads(run("node", str(ROOT / "tests" / "rotte_wasm_node.js"), str(work / "pkg"), str(idx), str(mzml), str(work / "plan.json")).stdout)
    assert "s" in out[1], out[1]
    return out[1]["s"]


def test_wasm_sweep_route_and_cli_give_the_same_wide_csv(work):
    spec = "240:243:1"
    csv = cli("trace", "xic", str(work / "scan.mzML"), "--da", TOL, "--sweep", spec, "--smooth", "5", "--format", "csv").stdout.splitlines()
    assert csv[0] == "rt_min,xic:240,xic:241,xic:242,xic:243"
    rows = [r.split(",") for r in csv[1:]]
    got = wasm_sweep(work, spec, 5)
    assert got.startswith('{"schema":"trace-set/1","n_traces":4,')
    members = got.split('{"id":')[1:]
    assert len(members) == 4
    for col, m in enumerate(members, 1):
        assert [(r[0], r[col]) for r in rows] == list(zip(tokens(m, "rt"), tokens(m, "y")))
    # and the sweep is the list: the CLI with the expanded --mz gives the same table
    assert cli("trace", "xic", str(work / "scan.mzML"), "--da", TOL, "--mz", "240,241,242,243", "--smooth", "5", "--format", "csv").stdout.splitlines() == csv
