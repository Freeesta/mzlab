"""The label check of the unit-resolution recipe (a file labelled profile whose points are sparse ion counts is not centroided again, `lrscreen::label_check`) acts in
the mzFinder `.wasm` (through Node) exactly as in the native engine: every `.mzfeatf` and the `series.mzfeat` of a mislabelled series have the same bytes, `auto` equals
`centroid` off there (the label did not cost a centroiding pass), `centroid` on, the user's word, still centroids, and the notice (`warn.features.profileLabel` with its numbers
and cut-offs) is the same JSON in the `.wasm` and in the native engine. (The CLI report's text, built from the same check, is pinned in `features_lr_tests`.) Skipped without cargo,
wasm-bindgen or node only outside CI: in CI a missing tool or a missing numpy FAILS the test (this is the wasm leg of the parity rule for the label check)."""
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
    try:
        import numpy  # noqa: F401  (tools/lr_synthetic.py needs it)
    except ImportError:
        if os.environ.get("CI"):
            pytest.fail("numpy is missing: the wasm leg of the label check cannot be skipped in CI")
        pytest.skip("numpy not available")
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
    nat = run("target/release/examples/native", str(nat_dir), *idx, "--recipe", recipe, "--notes")
    got = run("node", str(ROOT / "tests" / "feat_wasm_node.js"), str(pkg), str(wasm_dir), *idx, "--recipe", recipe, "--notes", "1")
    assert got == nat
    return nat_dir, wasm_dir, nat


def test_a_profile_label_on_sparse_counts_is_left_alone_the_same_in_node_and_native(built):
    root, idx, pkg = built
    auto_n, auto_w, auto_out = both(root, idx, pkg, "auto", "")
    off_n, _, off_out = both(root, idx, pkg, "off", '{"centroid": "off"}')
    on_n, on_w, on_out = both(root, idx, pkg, "on", '{"centroid": "on"}')
    names = sorted(p.name for p in auto_n.iterdir())
    assert "series.mzfeat" in names and len(names) == 9
    for f in names:
        assert (auto_n / f).read_bytes() == (auto_w / f).read_bytes(), f
        assert (on_n / f).read_bytes() == (on_w / f).read_bytes(), f
    # the label alone did not trigger the centroiding: `auto` writes the bytes of `centroid` off, and `on` (the user's word) writes others
    assert (auto_n / "series.mzfeat").read_bytes() == (off_n / "series.mzfeat").read_bytes()
    assert (auto_n / "series.mzfeat").read_bytes() != (on_n / "series.mzfeat").read_bytes()
    # the notice: the same text from the `.wasm` and from native (`both` compared every printed line), one per file, with the numbers and the cut-offs of the check
    notes = [json.loads(line.split(" ", 1)[1]) for line in auto_out.splitlines() if ".notes " in line]
    assert len(notes) == 8
    for n in notes:
        assert [x["key"] for x in n] == ["warn.features.profileLabel"], n
        p = n[0]["params"]
        assert p["profile"] == p["scans"] and 0 < p["probed"] <= p["scans"] and p["points"] > 0 and p["runs"] == 0 and p["fine"] == 0, p
        assert (p["runs_cut"], p["fine_cut"], p["run_points"], p["run_step"], p["fine_step"]) == (50, 2, 4, 600, 50), p
    # `centroid` off and `on` are the user's word: the label check does not speak for `off`, and `on` keeps the ordinary note
    assert all("warn.features.profileLabel" not in line for line in off_out.splitlines())
    assert all("warn.features.profileLabel" not in line and "note.features.centroided" in line for line in on_out.splitlines() if ".notes " in line)
