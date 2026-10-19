"""Recipe of the pipeline of mzFinder in the .mzflow (TP_Mine/js/35_pipeline_recipe.js, mzlab/web/workflow.js) with node: the record of the operation «feature_pipeline» has the fields of
docs/schemi/feature_pipeline-1.json, the check refuses incomplete records, the comparison of a re-application tells what differs, a reader that does not know the operation ignores it with a warning."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
JS = ROOT / "TP_Mine" / "js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")


def run_node(body):
    js = f"""globalThis.window = globalThis; globalThis.UNT = require({json.dumps(str(JS / '25_untarget.js'))});
require('vm').runInThisContext(require('fs').readFileSync({json.dumps(str(ROOT / 'mzlab/web/workflow.js'))}, 'utf8'));
const R = require({json.dumps(str(JS / '35_pipeline_recipe.js'))}); const W = globalThis.WORKFLOW;
const A = {{ recipe: JSON.stringify({{ ppm: 5, floor: null }}), proofs: [1], parent: {{ formula: 'C9H9', mz: 200.1, rt: NaN, minuti: [0, 5, NaN] }}, crate: '0.1.0',
  out: {{ evid: new TextEncoder().encode('evid'), tried: 4, found: 2, registry: '{{"a":1}}' }}, data: {{ meta: {{ inputs: [{{ name: 'a.mzidx', sha256: 'ab' }}] }} }},
  rules: '{{"version": 3}}', levels: '{{"versione": 2}}', suspects: null }};
(async () => {{ {body} }})();"""
    out = subprocess.run(["node", "-e", js], capture_output=True, text=True, encoding="utf-8", timeout=60)
    assert out.returncode == 0, out.stderr
    return json.loads(out.stdout)


def test_record_has_the_fields_of_the_schema():
    schema = json.loads((ROOT / "docs/schemi/feature_pipeline-1.json").read_text())
    r = run_node("const o = await R.record(A); console.log(JSON.stringify({o, bad: W.checkFeatureOp(o)}));")
    assert r["bad"] == []
    o = r["o"]
    assert o["schema"] == "feature_pipeline/1" and o["giri"] == 1 and o["sospetti"] is None
    assert o["genitore"] == {"formula": "C9H9", "mz": 200.1, "rt": None, "minuti": [0, 5, None]}
    assert o["regole"]["versione"] == 3 and o["livelli"]["versione"] == 2 and len(o["tabella_sha256"]) == 64 and len(o["ipotesi"]["registro_sha256"]) == 64
    assert o["ingressi"] == [{"nome": "a.mzidx", "sha256": "ab"}] and o["parametri"] == {"ppm": 5, "floor": None}
    assert set(o) <= set(schema["properties"])


def test_check_refuses_incomplete_records():
    r = run_node("""const o = await R.record(A); const bad = Object.assign({}, o, { recipe: 'no json', tabella_sha256: 'x' }); delete bad.regole;
console.log(JSON.stringify({ bad: W.checkFeatureOp(bad), other: W.checkFeatureOp({ op: 'export_report' }), last: R.lastOp([bad, o, { op: 'x' }], W.checkFeatureOp) === o, none: R.lastOp([bad], W.checkFeatureOp) }));""")
    assert sorted(r["bad"]) == ["recipe", "regole", "tabella_sha256"] and r["other"] == ["op"] and r["last"] is True and r["none"] is None


def test_compare_tells_what_differs():
    r = run_node("""const o = await R.record(A);
const same = R.compare(o, o), files = R.compare(o, Object.assign({}, o, { ingressi: [{ nome: 'a.mzidx', sha256: 'cd' }] })), eng = R.compare(o, Object.assign({}, o, { crate_version: '0.2.0', tabella_sha256: 'f'.repeat(64) }));
console.log(JSON.stringify({ same: same.same, sl: same.lines, files: [files.filesSame, files.lines.length], eng: [eng.same, eng.crateSame, eng.lines.length] }));""")
    assert r["same"] is True and r["sl"] == [] and r["files"] == [False, 1] and r["eng"] == [False, False, 2]


def test_unknown_operation_is_ignored_with_warning():
    r = run_node("""const o = await R.record(A); const s = W.splitOps([o, { op: 'dal_futuro' }]); console.log(JSON.stringify({ keep: s.keep.length, unknown: s.unknown }));""")
    assert r == {"keep": 1, "unknown": ["dal_futuro"]}
