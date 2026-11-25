"""The label check of the unit-resolution recipe (a file labelled profile whose points are sparse ion counts is not centroided again, `lrscreen::label_check`) acts in
the mzFinder `.wasm` (through Node) exactly as in the native engine: every `.mzfeatf` and the `series.mzfeat` of a mislabelled series have the same bytes, `auto` equals
`centroid` off there (the label did not cost a centroiding pass) and `centroid` on, the user's word, still centroids. Skipped without cargo, wasm-bindgen or node only outside
CI: in CI a missing tool fails the test (this is the wasm leg of the parity rule for the label check)."""
import json
import os
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import pytest
from cargo_util import build, cli, wasm_module

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")) and not os.environ.get("CI"), reason="cargo, wasm-bindgen or node not available")


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    pytest.importorskip("numpy")
    root = tmp_path_factory.mktemp("label")
    with ThreadPoolExecutor(2) as pool:
        made = pool.submit(run, "python3", "tools/lr_synthetic.py", str(root / "ser"), "--drift-series", "mislabelled")
        build("-p", "mzlab-feat-wasm", "--example", "native", runner=run)
        module = wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", runner=run)
        made.result()
    files = sorted(str(p) for p in (root / "ser").glob("drift-s*.mzML"))
    listed = json.loads(cli("file", "index", "--cache", str(root / "cache"), *files).stdout)["files"]
    idx = [str(root / "cache" / f["key"]) for f in listed]
    assert len(idx) == 8
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), str(module))
    return root, idx, pkg


def both(root, idx, pkg, name, recipe):
    """(native dir, wasm dir) of the same recipe, the printed SHA-256 lines already compared."""
    nat_dir, wasm_dir = root / f"{name}_nat", root / f"{name}_wasm"
    nat_dir.mkdir(exist_ok=True)
    wasm_dir.mkdir(exist_ok=True)
    nat = run("target/release/examples/native", str(nat_dir), *idx, "--recipe", recipe)
    got = run("node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(wasm_dir), *idx, "--recipe", recipe)
    assert got == nat
    return nat_dir, wasm_dir


def test_a_profile_label_on_sparse_counts_is_left_alone_the_same_in_node_and_native(built):
    root, idx, pkg = built
    auto_n, auto_w = both(root, idx, pkg, "auto", "")
    off_n, _ = both(root, idx, pkg, "off", '{"centroid": "off"}')
    on_n, on_w = both(root, idx, pkg, "on", '{"centroid": "on"}')
    names = sorted(p.name for p in auto_n.iterdir())
    assert "series.mzfeat" in names and len(names) == 9
    for f in names:
        assert (auto_n / f).read_bytes() == (auto_w / f).read_bytes(), f
        assert (on_n / f).read_bytes() == (on_w / f).read_bytes(), f
    # the label alone did not trigger the centroiding: `auto` writes the bytes of `centroid` off, and `on` (the user's word) writes others
    assert (auto_n / "series.mzfeat").read_bytes() == (off_n / "series.mzfeat").read_bytes()
    assert (auto_n / "series.mzfeat").read_bytes() != (on_n / "series.mzfeat").read_bytes()
