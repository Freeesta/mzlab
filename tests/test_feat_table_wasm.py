"""The table bridge of mzFinder in WebAssembly (crates/mzlab-feat-table-wasm) gives the same bytes as the native example `table`: the view, the report, the CSV
full (with the words of mzmine too), the FBMN quantification table, the MGF and the SIRIUS `.ms` have the same SHA-256 through Node as natively. The series is the
synthetic one of tools/feat_mirato.py (profile `piccolo`), aligned by the native example `native`. Skipped on a machine without cargo, the wasm32 target, wasm-bindgen, node or numpy; in CI (GITHUB_ACTIONS) those are
required and a missing one fails. Run by the `solo311` shard of test.yml (tools/pytest_solo_311.txt)."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import build

ROOT = Path(__file__).resolve().parents[1]
TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"
SPEC = {"normalize": {"method": "sum"}, "filter": [{"rule": "min_detected", "value": 2}]}


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def samples_json(idx, blank=None):
    rows = [{"blank_kind": None, "compound": "", "concentration": None, "concentration_unit": None, "file": Path(p).stem + ".mzML", "injection_order": i + 1, "istd": False,
             "kind": "blank" if i == blank else "sample", "note": "", "replicate": 1, "series": "", "sha256": "00" * 32, "time": None, "time_confirmed": False, "time_min": None, "time_unit": "min"} for i, p in enumerate(idx)]
    return json.dumps({"schema": "samples/1", "samples": rows})


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    try:
        import numpy  # noqa: F401  (tools/feat_mirato.py needs it)
    except ImportError:
        if CI:
            raise
        pytest.skip("numpy not available")
    root = tmp_path_factory.mktemp("table")
    ser = root / "ser"
    run("python3", "tools/feat_mirato.py", "--seed", "2", "--profile", "piccolo", "--scale", "1.0", "--out", str(ser))
    idx = sorted(str(p) for p in ser.glob("*.mzidx"))
    build("-p", "mzlab-feat-wasm", "--example", "native", runner=run)
    build("-p", "mzlab-feat-table-wasm", "--example", "table", runner=run)
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-table-wasm", runner=run)
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_table_wasm.wasm")
    series = root / "series"
    series.mkdir()
    run("target/release/examples/native", str(series), *idx)
    (root / "samples.json").write_text(samples_json(idx))
    (root / "spec.json").write_text(json.dumps(SPEC))
    return root, idx, pkg, series / "series.mzfeat"


@pytest.mark.parametrize("spec", ["-", "spec.json"])
def test_wasm_and_native_write_the_same_files(built, spec):
    root, idx, pkg, table = built
    args = [str(table), str(root / "samples.json"), "-" if spec == "-" else str(root / spec), "1", *idx]
    outs = {}
    for name, cmd in (("nat", ["target/release/examples/table"]), ("wasm", ["node", str(ROOT / "tests/feat_table_node.js"), str(pkg)])):
        out = root / f"{name}-{spec}"
        out.mkdir()
        got = run(*cmd, str(out), *args)
        outs[name] = (got, out)
    assert outs["nat"][0] == outs["wasm"][0]
    nat, wasm = outs["nat"][1], outs["wasm"][1]
    for f in sorted(p.name for p in nat.iterdir()):
        assert (nat / f).read_bytes() == (wasm / f).read_bytes(), f
    full = (nat / "full.csv").read_text().splitlines()
    assert full[0].startswith("id,mz,rt,") and len(full) > 10
    assert (nat / "quant.csv").read_text().splitlines()[0].startswith("row ID,row m/z,row retention time")
    view = json.loads((nat / "view.json").read_text()) if spec != "-" else None
    if view:
        assert view["counts"]["kept"] == len(view["rows"]) == len(full) - 1
    assert json.loads((nat / "qc.json").read_text())["schema"] == "qc/2"
    check = json.loads((nat / "check.json").read_text())
    assert check["schema"] == "tabcheck/1" and [c["id"] for c in check["checks"]][:3] == ["size", "blank", "contaminants"]
    plaus = json.loads((nat / "plausible.json").read_text())
    assert plaus["schema"] == "plausible/1" and [c["id"] for c in plaus["checks"]][0] == "non_finite"
    assert (nat / "features.csv").read_text().splitlines()[0].startswith("id;mz;rt;")
    assert len(json.loads((nat / "features.rows.json").read_text())["hojas"][0]["filas"]) == len(full)
    assert json.loads((nat / "report.json").read_text())["schema"] == "report/2"


def test_wasm_and_native_give_the_same_blank_evidence_for_the_plain_ratio_filter(built):
    """AU-7 slice 2: the plain `blank` rule (default options) writes `counts.blank` too, the same bytes in the app engine and in the .wasm. The ratio is huge, so every row
    with a peak in the blank is dropped and the evidence cannot be empty."""
    root, idx, pkg, table = built
    ratio = 1e9
    (root / "samples_blank.json").write_text(samples_json(idx, blank=0))
    (root / "spec_blank.json").write_text(json.dumps({"normalize": {"method": "none"}, "filter": [{"rule": "blank", "value": ratio}]}))
    args = [str(table), str(root / "samples_blank.json"), str(root / "spec_blank.json"), "1", *idx]
    views = {}
    for name, cmd in (("nat", ["target/release/examples/table"]), ("wasm", ["node", str(ROOT / "tests/feat_table_node.js"), str(pkg)])):
        out = root / f"{name}-blank"
        out.mkdir()
        run(*cmd, str(out), *args)
        views[name] = (out / "view.json").read_bytes()
    assert views["nat"] == views["wasm"]
    blank = json.loads(views["nat"])["counts"]["blank"]
    assert blank["quantity"] == "area" and blank["fold"] == ratio and blank["groups"] == 1
    assert blank["dropped_total"] > 0 and len(blank["dropped"]) == min(blank["dropped_total"], 200)
    assert blank["truncated"] is (blank["dropped_total"] > len(blank["dropped"]))
    for rid, value, level, fold in blank["dropped"]:
        assert level > 0 and fold == pytest.approx(value / level) and fold < ratio, (rid, value, level, fold)
    assert blank["fallback_all"] == {"total": 0, "files": []} and "ungrouped_all" not in blank
