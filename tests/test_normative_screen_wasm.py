"""Both regulatory screening paths give the same bytes in WebAssembly as in a native `flow run`: `screen.suspects` (`flow_screen_suspects` of the `table`
.wasm) and `annot.regulatory` (`flow_regulatory` of the `attributio-all` .wasm), on the invented 6-row package of `crates/mzlab-cli/tests/data/normative_screen/`.
The native run and its inputs come from the Rust test `both_paths_run_in_one_plan_for_the_wasm_parity`. Skipped without cargo, the wasm32 target, wasm-bindgen or node."""
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not all(shutil.which(x) for x in ("cargo", "wasm-bindgen", "node")), reason="cargo, wasm-bindgen or node not available")

NODE = """
const fs = require('fs');
const [table, attr, dir] = process.argv.slice(2);
const t = require(table), a = require(attr);
const prov = JSON.parse(fs.readFileSync(dir + '/prov.json'));
const read = f => fs.readFileSync(dir + '/' + f);
const pkg = read('normative.mzsus');
const imp = read('imp.mzsus');
const pack = fs_ => Buffer.concat(fs_.flatMap(f => { const h = Buffer.alloc(8); h.writeBigUInt64LE(BigInt(f.length), 0); return [h, f]; }));
const names = JSON.stringify({ suspects: ['normative.mzsus'], imported: ['imp.mzsus'] });
const s = t.flow_screen_suspects(JSON.stringify(prov.activity.s.params), read('table.mzfeat'), names, pack([pkg, imp]), new Uint8Array(0), new Uint8Array(0));
const r = a.flow_regulatory(JSON.stringify(prov.activity.r.params), read('annot.json'), pkg, JSON.stringify(['imp.mzsus']), pack([imp]));
fs.writeFileSync(dir + '/s.wasm.json', s); fs.writeFileSync(dir + '/r.wasm.json', r);
"""


def run(*a, env=None):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, env=env)


@pytest.fixture(scope="module")
def made(tmp_path_factory):
    root = tmp_path_factory.mktemp("normscreen")
    out = root / "run"
    run("cargo", "test", "--locked", "-q", "-p", "mzlab-cli", "--lib", "both_paths_run_in_one_plan_for_the_wasm_parity",
        env={**os.environ, "MZLAB_PARITA_NORMATIVE": str(out)})
    wasm = "target/wasm32-unknown-unknown/release/"
    run("cargo", "build", "--locked", "--release", "-q", "--target", "wasm32-unknown-unknown", "-p", "mzlab-feat-table-wasm")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(root / "t"), wasm + "mzlab_feat_table_wasm.wasm")
    run("cargo", "build", "--locked", "--release", "-q", "--target", "wasm32-unknown-unknown", "-p", "mzlab-attributio-wasm", "--features", "all")
    run("wasm-bindgen", "--target", "nodejs", "--no-typescript", "--out-dir", str(root / "a"), wasm + "mzlab_attributio_wasm.wasm")
    (root / "run.js").write_text(NODE)
    run("node", str(root / "run.js"), str(root / "t" / "mzlab_feat_table_wasm.js"), str(root / "a" / "mzlab_attributio_wasm.js"), str(out))
    return out


def test_mass_path_has_the_bytes_of_flow_run(made):
    native = (made / "s.json").read_bytes()
    assert b'"memberships":[{"list"' in native and (made / "s.wasm.json").read_bytes() == native


def test_structure_path_has_the_bytes_of_flow_run(made):
    native = (made / "r.json").read_bytes()
    assert b'"match":"InChIKey"' in native and (made / "r.wasm.json").read_bytes() == native
    assert {l["id"] for l in json.loads(native)["regulatory"]["lists"]} == {"T_LIST_A", "T_LIST_C"}
