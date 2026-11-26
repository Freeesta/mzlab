"""`spectrum compare` in the site: `spectrum_compare` of the library .wasm (mzlab-lib-wasm, through Node: tests/speccmp_wasm_node.js) gives the bytes of the app's rpc op and of
`mzlab-cli spectrum compare` on two synthetic product-ion files, for the defaults and for `tol` and `ce_tol`, and the same error key for a parameter the function refuses
(crates/mzlab-desktop/tests/parity_spectrum_compare.rs runs the three and takes the .wasm leg from MZLAB_SPECCMP_WASM_PKG).
Skipped on a machine without cargo, wasm-bindgen or node; in CI (GITHUB_ACTIONS) those are required and a missing one fails. Run by the `rest` shards of test.yml."""
import os
import shutil
import subprocess
from pathlib import Path

import pytest
from cargo_util import run, wasm_module

ROOT = Path(__file__).resolve().parents[1]
TOOLS = all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node"))
CI = bool(os.environ.get("GITHUB_ACTIONS"))
# skipped on a machine without the tools, but never silently in CI: there it must run
pytestmark = pytest.mark.skipif(not TOOLS and not CI, reason="cargo, wasm-bindgen or node not available")


def test_the_tools_are_there_in_ci():
    assert TOOLS or not CI, "cargo, wasm-bindgen and node must be installed in CI: this test would not run"


def test_the_site_the_app_and_the_cli_give_the_same_comparison(tmp_path):
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(tmp_path / "pkg"), str(wasm_module("--target", "wasm32-unknown-unknown", "-p", "mzlab-lib-wasm")))
    r = subprocess.run(["cargo", "test", "--locked", "-p", "mzlab-desktop", "--test", "parity_spectrum_compare"], cwd=ROOT, capture_output=True, encoding="utf-8", env={**os.environ, "MZLAB_SPECCMP_WASM_PKG": str(tmp_path / "pkg")})
    assert r.returncode == 0, f"cargo test exited {r.returncode}:\n{r.stdout[-3000:]}\n{r.stderr[-6000:]}"
    assert "2 passed" in r.stdout, r.stdout[-2000:]
