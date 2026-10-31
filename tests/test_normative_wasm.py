"""The reader of the regulatory package in WebAssembly (`normative_listings` of crates/mzlab-feat-wasm, feature `finder`) gives the same `listings` JSON, byte for
byte, as the native `mzlab-cli normative listings`, on the package that `normative pack` writes from the committed lists. A file without the extension is refused by both.
Skipped without cargo, the wasm32 target, wasm-bindgen or node."""
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

NODE = """
const fs = require('fs');
const w = require(process.argv[2]);
const raw = fs.readFileSync(process.argv[3]);
try { process.stdout.write(w.normative_listings(raw)); } catch (e) { process.stderr.write('ERR ' + (e && e.message)); process.exit(3); }
"""


def run(*a, check=True):
    return subprocess.run(a, cwd=ROOT, check=check, capture_output=True)


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    root = tmp_path_factory.mktemp("normwasm")
    run("cargo", "build", "--locked", "--release", "-q", "-p", "mzlab-cli")
    run("cargo", "build", "--locked", "--release", "-q", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm", "--no-default-features", "--features", "finder")
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    (root / "run.js").write_text(NODE)
    out = root / "package"
    run("target/release/mzlab-cli", "normative", "pack", "normative", str(out))
    return root, pkg / "mzlab_feat_wasm.js", out


def test_listings_json_is_the_same_in_wasm_and_native(built):
    root, wasm, out = built
    native = run("target/release/mzlab-cli", "normative", "listings", str(out)).stdout
    got = run("node", str(root / "run.js"), str(wasm), str(out / "normative.mzsus")).stdout
    assert len(native) > 100_000 and got == native


def test_file_without_listings_is_refused_by_both(built, tmp_path):
    root, wasm, out = built
    v1 = bytearray((out / "normative.mzsus").read_bytes())
    v1[24:28] = bytes(4)             # flags: no listings
    f = tmp_path / "v1.mzsus"
    f.write_bytes(v1)
    n = run("target/release/mzlab-cli", "normative", "listings", str(f), check=False)
    w = run("node", str(root / "run.js"), str(wasm), str(f), check=False)
    assert n.returncode != 0 and w.returncode == 3 and b"no listings" in w.stderr and b"no listings" in n.stderr
