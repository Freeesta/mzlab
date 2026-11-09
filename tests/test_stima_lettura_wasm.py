"""The read estimate of the `.wasm` (crates/mzlab-feat-wasm, nodejs target) on the fixture tests/dati/stima_lettura.json: `flow_ms2` refuses `spectra.ms2`
with `error.memoryBudget` (phase `read`) and the expected estimate where the fixture says `refused` and writes the MGF where it says `memory`, as the Rust
route and `flow run` do (their tests read the same fixture). The example `stima_lettura --fixture` writes the mzML. Skipped without cargo, the wasm32
target, wasm-bindgen or node."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

NODE = r"""
const fs = require("fs");
const [pkg, file, cases] = process.argv.slice(1);
const m = require(pkg + "/mzlab_feat_wasm.js");
const bytes = new Uint8Array(fs.readFileSync(file));
const out = [];
for (const c of JSON.parse(cases)) {
  let n = 0, err = null;
  try { n = m.flow_ms2("{}", "stima.mzML", bytes, undefined, c.budget_mb).length; } catch (e) { err = JSON.parse(String(e && e.message || e)); }
  out.push({ n, err });
}
console.log(JSON.stringify(out));
"""


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def test_the_wasm_read_refusal_follows_the_fixture(tmp_path):
    spec = json.loads((ROOT / "tests/dati/stima_lettura.json").read_text(encoding="utf-8"))
    mzml = tmp_path / "stima.mzML"
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "stima_lettura", "--", "--fixture", str(mzml))
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
    pkg = tmp_path / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    got = json.loads(run("node", "-e", NODE, str(pkg), str(mzml), json.dumps(spec["cases"])))
    for case, g in zip(spec["cases"], got):
        if case["via"] == "memory":
            assert g["err"] is None and g["n"] > 0, (case, g)
        else:
            assert g["err"]["error_key"] == "error.memoryBudget" and g["n"] == 0, (case, g)
            p = g["err"]["params"]
            assert (p["fase"], int(p["stima_mb"]), int(p["budget_mb"])) == ("read", spec["mb"], case["budget_mb"]), p
