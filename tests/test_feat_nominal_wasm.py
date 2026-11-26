"""The `nominal` preset of `features.detect` (nominal-resolution triple quadrupole: lower floor and height, wider grid merge) acts in the mzFinder `.wasm` (through Node) exactly as in
the native engine: every `.mzfeatf` and the `series.mzfeat` of a synthetic unit-resolution series have the same bytes with the preset's layer as with the default recipe, and the
preset writes other bytes than the default. The preset values are read from the committed step export, the one source the site, the app and the CLI share. Skipped without cargo,
wasm-bindgen or node only outside CI: in CI a missing tool or a missing numpy FAILS the test (this is the wasm leg of the parity rule for the preset)."""
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


def preset_layer(preset):
    """The recipe text of the browser runner for a plan that names `preset` on `features.detect`: the values of the preset as one layer."""
    steps = json.loads((ROOT / "docs/schemi/steps-1.json").read_text(encoding="utf-8"))
    detect = next(s for s in steps if s["id"] == "features.detect")
    return json.dumps({"fill": True, "layers": [{"step": "features.detect", "params": detect["presets"][preset]["params"]}]})


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    try:
        import numpy  # noqa: F401  (tools/lr_synthetic.py needs it)
    except ImportError:
        if os.environ.get("CI"):
            pytest.fail("numpy is missing: the wasm leg of the nominal preset cannot be skipped in CI")
        pytest.skip("numpy not available")
    root = tmp_path_factory.mktemp("nominal")
    with ThreadPoolExecutor(2) as pool:
        made = pool.submit(run, "python3", "tools/lr_synthetic.py", str(root / "ser"), "--drift-series", "centroid", "--jitter", "0.3")
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


def test_the_nominal_preset_gives_the_same_bytes_in_node_and_native_and_not_those_of_the_default(built):
    root, idx, pkg = built
    nom_n, nom_w = both(root, idx, pkg, "nominal", preset_layer("nominal"))
    def_n, def_w = both(root, idx, pkg, "default", "")
    names = sorted(p.name for p in nom_n.iterdir())
    assert "series.mzfeat" in names and len(names) == 9
    for f in names:
        assert (nom_n / f).read_bytes() == (nom_w / f).read_bytes(), f
        assert (def_n / f).read_bytes() == (def_w / f).read_bytes(), f
    assert (nom_n / "series.mzfeat").read_bytes() != (def_n / "series.mzfeat").read_bytes()
