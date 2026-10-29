"""The module `valida` (crates/mzlab-lettori-wasm, feature `valida`) gives through Node the same bytes as `mzlab-cli validate --json`: a valid file, one with an accession
that does not exist and a truncated one (made from the synthetic msconvert file of tools/mzml_msconvert_sintetico.py and the example mzML, as crates/mzlab-cli/src/validate_tests.rs does). Skipped without cargo, the wasm32 target,
wasm-bindgen or node."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")


def run(*a, ok=(0,)):
    r = subprocess.run(a, cwd=ROOT, capture_output=True, encoding="utf-8")
    assert r.returncode in ok, r.stderr
    return r.stdout


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    root = tmp_path_factory.mktemp("valida")
    run("cargo", "build", "--release", "--locked", "-q", "-p", "mzlab-cli")
    run("cargo", "build", "--release", "--locked", "-q", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", "valida")
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_lettori_wasm.wasm")
    pytest.importorskip("numpy")
    files = root / "files"
    files.mkdir()
    run("python3", "tools/mzml_msconvert_sintetico.py", str(files / "good.mzML"), "--scans", "30", "--peaks1", "200", "--peaks2", "40")
    run("python3", "tools/mzml_msconvert_sintetico.py", str(files / "offsets.mzML"), "--scans", "30", "--peaks1", "200", "--peaks2", "40", "--variant", "indice_falso")
    good = (files / "good.mzML").read_bytes()
    (files / "example.mzML").write_bytes((ROOT / "mzlab/web/esempi/FullScan_t10.mzML").read_bytes())
    at = good.index(b"MS:1000127")
    (files / "bad.mzML").write_bytes(good[:at] + b"MS:1999127" + good[at + 10:])
    (files / "cut.mzML").write_bytes(good[: len(good) // 2])
    return pkg, files


@pytest.mark.parametrize("name,codes", [("good.mzML", (0,)), ("offsets.mzML", (1,)), ("bad.mzML", (1,)), ("example.mzML", (0, 1)), ("cut.mzML", (0, 1, 2))])
def test_same_bytes_as_the_command_line(built, name, codes):
    pkg, files = built
    f = files / name
    cli = run("target/release/mzlab-cli", "validate", str(f), "--json", ok=codes)
    if cli:                         # a file the command cannot read has no JSON; the module then fails and the page says so
        assert run("node", "tests/valida_nodo.js", str(pkg / "mzlab_lettori_wasm.js"), str(f)) == cli
        assert ("\"valid\":true" in cli) == (codes == (0,)), cli[:200]
