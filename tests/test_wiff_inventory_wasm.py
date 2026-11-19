"""The module `wiffwrite` (crates/mzlab-lettori-wasm, feature `wiffwrite`) gives through Node the same bytes as `mzlab-cli file info|verify` on the synthetic pairs of
tools/wiff_riferimento.py: a clean file, one with an empty injection, refused ones, an unindexed section, the full inventory and a verify of equal and of different files.
Skipped without cargo, wasm-bindgen or node only outside CI (CI has them all, so there a missing tool fails the test)."""
import os
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import build

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")) and not os.environ.get("CI"), reason="cargo, wasm-bindgen or node not available")


def run(*a, ok=(0,)):
    r = subprocess.run(a, cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode in ok, r.stderr
    return r.stdout


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    root = tmp_path_factory.mktemp("wiffwrite")
    build("-p", "mzlab-cli", runner=run)
    build("--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", "wiffwrite", runner=run)
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm")
    files = root / "files"
    run("python3", "tools/wiff_riferimento.py", str(files))
    return pkg, files


def node(pkg, *a, ok=(0,)):
    return run("node", "tests/wiff_inventory_node.js", str(pkg / "mzlab_lettori_wasm.js"), *a, ok=ok)


@pytest.mark.parametrize("stem,codes", [("one", (0,)), ("two", (0,)), ("gaps", (0, 3)), ("periods", (3,)), ("tof", (3,)), ("unindexed", (3,))])
def test_list_same_bytes_as_the_command_line(built, stem, codes):
    pkg, files = built
    f = str(files / f"{stem}.wiff")
    cli = run("target/release/mzlab-cli", "file", "info", f, "--json", ok=codes)
    assert cli.startswith('{"schema":"wiff.list/1"')
    assert node(pkg, "list", f) == cli


def test_inventory_same_bytes_as_the_command_line(built):
    pkg, files = built
    f = str(files / "two.wiff")
    cli = run("target/release/mzlab-cli", "file", "info", f, "--inventory")
    assert '"directory"' in cli
    assert node(pkg, "list", "--inventory", f) == cli


def test_verify_same_bytes_as_the_command_line(built):
    pkg, files = built
    a, b = str(files / "two.wiff"), str(files / "one.wiff")
    for out, codes in ((a, (0,)), (b, (1,))):
        cli = run("target/release/mzlab-cli", "file", "verify", out, "--against", a, ok=codes)
        assert node(pkg, "verify", out, a) == cli
        assert ('"equal":true' in cli.rsplit(",", 1)[-1]) == (codes == (0,))
