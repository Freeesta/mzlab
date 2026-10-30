"""Importing your own official list gives the same bytes in the browser (`mzlab-normative-wasm`, run under node) as in `mzlab-cli normative import-official` (the engine
of the app route too): the `.mzsus`, the summary and the lists read back by `lists_of`. Invented fixtures only. Skipped without cargo, wasm-bindgen or node."""
import json
import shutil
import struct
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "crates/mzlab-feat/tests/data/official"
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

NODE = """
const fs = require('fs');
const w = require(process.argv[2]);
const [reader, csv, side, label, out, lists] = process.argv.slice(3);
try {
  const env = w.import_official(reader, fs.readFileSync(csv), fs.readFileSync(side, 'utf8'), label);
  fs.writeFileSync(out, env);
  const n = new DataView(env.buffer, env.byteOffset).getUint32(0, true);
  fs.writeFileSync(lists, w.lists_of(env.subarray(4 + n)));
} catch (e) { process.stderr.write('ERR ' + (e && e.message ? e.message : e)); process.exit(3); }
"""


def run(*a, check=True):
    return subprocess.run(a, cwd=ROOT, check=check, capture_output=True)


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    root = tmp_path_factory.mktemp("impwasm")
    run("cargo", "build", "--locked", "--release", "-q", "-p", "mzlab-cli")
    run("cargo", "build", "--locked", "--release", "-q", "--target", "wasm32-unknown-unknown", "-p", "mzlab-normative-wasm")
    pkg = root / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_normative_wasm.wasm")
    (root / "run.js").write_text(NODE)
    return root, pkg / "mzlab_normative_wasm.js"


def test_import_is_the_same_in_wasm_and_native(built):
    root, wasm = built
    csv, side = DATA / "candidate-list.csv", DATA / "cas.tsv"
    out = root / "native.mzsus"
    summary = run("target/release/mzlab-cli", "normative", "import-official", "svhc", str(csv), str(side), str(out), "--file", "candidate-list.csv", "--date", "2026-10-30").stdout
    label = json.dumps({"file": "candidate-list.csv", "date": "2026-10-30"})
    env_f, lists_f = root / "wasm.env", root / "wasm.lists"
    run("node", str(root / "run.js"), str(wasm), "svhc", str(csv), str(side), label, str(env_f), str(lists_f))
    env = env_f.read_bytes()
    n = struct.unpack("<I", env[:4])[0]
    assert env[4 + n:] == out.read_bytes()
    assert json.loads(env[4:4 + n]) == json.loads(summary)
    lists = json.loads(lists_f.read_text())
    assert [x["id"] for x in lists] == ["SVHC"]


def test_unknown_reader_is_refused_by_wasm(built, tmp_path):
    root, wasm = built
    r = run("node", str(root / "run.js"), str(wasm), "nope", str(DATA / "candidate-list.csv"), str(DATA / "cas.tsv"), "{}", str(tmp_path / "a"), str(tmp_path / "b"), check=False)
    assert r.returncode == 3 and b"err.normative.parse" in r.stderr
