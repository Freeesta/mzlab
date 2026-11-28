"""The per-file recipe of a unit-resolution series acts in the mzFinder `.wasm` (through Node) exactly as in the native engine: files whose noise, and so whose estimated floor, differ are detected at
their own floor and gap-filled at it (`Screen::file_floors`, set by `series_recipe` of the `.wasm` and by the example `native`, the rule of `flow run` and of the desktop `run_plan`: the Rust test
`flow_recipe_tests.rs` compares the three and the first-file rule). Every `.mzfeatf` and the `series.mzfeat` of the synthetic series have the same bytes in both, and the profile points kept (`centroid`
off, so each file keeps its own floor) write other bytes than the default (centroids, one floor). Skipped without cargo, wasm-bindgen, node or numpy only outside CI: in CI a missing tool FAILS."""
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
PROFILE_POINTS = json.dumps({"layers": [{"step": "features.detect", "params": {"centroid": "off"}}]})


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    try:
        import numpy  # noqa: F401  (tools/lr_synthetic.py needs it)
    except ImportError:
        if os.environ.get("CI"):
            pytest.fail("numpy is missing: the wasm leg of the per-file recipe cannot be skipped in CI")
        pytest.skip("numpy not available")
    root = tmp_path_factory.mktemp("recipe")
    with ThreadPoolExecutor(2) as pool:
        made = pool.submit(run, "python3", "tools/lr_synthetic.py", str(root / "ser"), "--floor-series")
        build("-p", "mzlab-feat-wasm", "--example", "native", runner=run)
        module = wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", runner=run)
        made.result()
    files = sorted(str(p) for p in (root / "ser").glob("floor-s*.mzML"))
    listed = json.loads(cli("file", "index", "--cache", str(root / "cache"), *files).stdout)["files"]
    idx = [str(root / "cache" / f["key"]) for f in listed]
    assert len(idx) == 3
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


def test_files_with_other_floors_give_the_same_bytes_in_node_and_native_and_not_those_of_one_floor(built):
    root, idx, pkg = built
    own_n, own_w = both(root, idx, pkg, "own", PROFILE_POINTS)
    def_n, def_w = both(root, idx, pkg, "default", "")
    names = sorted(p.name for p in own_n.iterdir())
    assert "series.mzfeat" in names and len(names) == 4
    for f in names:
        assert (own_n / f).read_bytes() == (own_w / f).read_bytes(), f
        assert (def_n / f).read_bytes() == (def_w / f).read_bytes(), f
    assert (own_n / "series.mzfeat").read_bytes() != (def_n / "series.mzfeat").read_bytes()
