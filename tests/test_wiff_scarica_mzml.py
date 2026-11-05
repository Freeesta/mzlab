"""«Download .mzML» of a .wiff sample on the site (rust-nativo.js `toMzml`: `to_mzml_come` of the `sciex` module, no optical matrix) is byte for byte the file of
`mzlab-cli convert` with the same `--compression`, for a Full Scan, a product-ion and a dense MRM pair (the engine alone leaves out what `estesi` writes: SRM dwell and
energy, isolation offsets, the TIC and BPC of a Full Scan). Synthetic pairs of tools/wiff_riferimento.py only. Skipped without cargo, wasm-bindgen or node."""
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from cargo_util import run

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")
WASM = ROOT / "target/wasm32-unknown-unknown/release"


@pytest.fixture(scope="module")
def work(tmp_path_factory):
    d = tmp_path_factory.mktemp("scarica")
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lettori-wasm", "--no-default-features", "--features", "sciex")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(d / "mod"), str(WASM / "mzlab_lettori_wasm.wasm"))
    run("cargo", "build", "--release", "--locked", "-p", "mzlab-cli")
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(d / "ref")], check=True, capture_output=True)
    return d


@pytest.mark.parametrize(("stem", "comp"), [("one", "standard"), ("totale", "standard"), ("prodotto", "standard"), ("transizioni", "standard"), ("transizioni", "smallest")])
def test_the_download_is_the_file_of_the_cli(work, stem, comp):
    pair = [work / "ref" / f"{stem}.wiff", work / "ref" / f"{stem}.wiff.scan"]
    site, out = work / f"{stem}.{comp}.mzML", work / f"{stem}.{comp}"
    run("node", str(ROOT / "tests" / "mzml_scarica_node.js"), str(work / "mod"), str(site), "0", comp, *map(str, pair))
    run(str(ROOT / "target/release/mzlab-cli"), "convert", str(pair[0]), "--out", str(out), "--fresh", "--workers", "1", "--compression", comp)
    cli = sorted(out.glob("*.mzML"))      # NAME.mzML, or NAME-ID.mzML per sample of a pair with several: sample 0 is the first
    assert cli and site.read_bytes() == cli[0].read_bytes(), cli
