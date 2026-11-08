"""The detection estimate of the `.wasm` (crates/mzlab-feat-wasm, nodejs target) on the fixture tests/dati/stima_detect.json: `estimate_detect` answers the
expected `{mb, via}` at each budget, as the Rust route, `flow run` and the app do (their tests read the same fixture), and `detect_file_to` refuses the
detection with `error.memoryBudget` (phase `detect`) where the fixture says `refused` and writes the features where it says `memory`. The example
`stima_detect --fixture` writes the index. Skipped without cargo, the wasm32 target, wasm-bindgen or node."""
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
const fd = fs.openSync(file, "r"), size = fs.fstatSync(fd).size;
const read = (_k, off, len) => { const b = new Uint8Array(len); fs.readSync(fd, b, 0, len, off); return b; };
const out = [];
for (const c of JSON.parse(cases)) {
  const est = JSON.parse(m.estimate_detect(read, size, "", c.budget_mb));
  let n = 0, err = null;
  try { m.detect_file_to(read, size, "stima", "", (_o, b) => { n += b.length; }, undefined, c.budget_mb); } catch (e) { err = JSON.parse(String(e && e.message || e)); }
  out.push({ est, n, err });
}
console.log(JSON.stringify(out));
"""


def run(*a):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout


def test_the_wasm_estimate_and_refusal_follow_the_fixture(tmp_path):
    spec = json.loads((ROOT / "tests/dati/stima_detect.json").read_text(encoding="utf-8"))
    idx = tmp_path / "stima.mzidx"
    run("cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-feat-wasm", "--example", "stima_detect", "--", "--fixture", str(idx))
    run("cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-wasm")
    pkg = tmp_path / "pkg"
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(pkg), "target/wasm32-unknown-unknown/release/mzlab_feat_wasm.wasm")
    got = json.loads(run("node", "-e", NODE, str(pkg), str(idx), json.dumps(spec["cases"])))
    for case, g in zip(spec["cases"], got):
        assert g["est"] == {"mb": spec["mb"], "via": case["via"]}, (case, g)
        if case["via"] == "memory":
            assert g["err"] is None and g["n"] > 0, (case, g)
        else:
            assert g["err"]["error_key"] == "error.memoryBudget" and g["n"] == 0, (case, g)
            p = g["err"]["params"]
            assert (p["fase"], int(p["stima_mb"]), int(p["budget_mb"])) == ("detect", spec["mb"], case["budget_mb"]), p
