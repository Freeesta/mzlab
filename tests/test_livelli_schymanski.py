"""One fixed fixture per evidence class of Schymanski et al. 2014 (standard, library, diagnostic fragments, formula only, mass only): `assess` must return
the level AND name the evidence (`why`, `decisive`). Diagnostic fragments give 2b only through the chemist's decision. Synthetic inputs, no lab data."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRE = f"""globalThis.window = globalThis; const store = {{}};
globalThis.localStorage = {{ getItem: k => (k in store ? store[k] : null), setItem: (k, v) => {{ store[k] = String(v); }} }};
require({str(WEB / 'livelli-1.js')!r}); const L = require({str(WEB / 'livelli.js')!r});
const lib = {{ ent: .9, cos: .85, shared: 8, dprec: 2, name: 'A', ik: 'AAAAAAAAAAAAAA-X', lib: 'LIB', nlib: 30 }};
"""

# evidence of the 2014 paper -> (input, profile, level, why, words the decisive sentence must contain (interface language))
CASES = {
    "reference standard": ("{cands: [lib], nScans: 3, std: {name: 'S', drt: .05, ent: .85, dprec: 2}}", "mzlab", "1", "standard", "standard S"),
    "library spectrum": ("{cands: [lib], nScans: 3}", "mzlab", "2a", "2a", "libreria LIB"),
    "weak library match": ("{cands: [lib], nScans: 1}", "mzlab", "3a", "confirm", "conferma"),
    "analogue only": ("{cands: [], analogs: [{name: 'An', mcos: .8, matched: 6, delta: 15.9949}]}", "mzlab", "3b", "analogo", "solo un analogo"),
    "unique formula": ("{cands: [], formulaUnique: true}", "mzlab", "4", "formula", "una sola formula"),
    "exact mass only": ("{cands: []}", "mzlab", "5", "empty", "solo la massa esatta"),
    "unit resolution": ("{cands: [lib], nScans: 3}", "lr", "4", "tetto", "risoluzione unitaria"),
}


def run(code):
    p = subprocess.run(["node", "-e", PRE + code], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


@pytest.mark.parametrize("evidence", sorted(CASES))
def test_the_level_and_its_evidence(evidence):
    inp, profile, level, why, key = CASES[evidence]
    r = run(f"const r = L.assess({inp}, {profile!r}); console.log(JSON.stringify({{p: r.proposed, w: r.why, d: r.decisive, x: r.decided}}))")
    assert (r["p"], r["w"], r["x"]) == (level, why, None), r
    assert key in r["d"], r


def test_diagnostic_fragments_reach_2b_only_by_the_chemist():
    r = run("""const r = L.assess({cands: [lib], nScans: 3}); const a = L.decide(r, '2b');
      const none = L.assess({cands: []}); console.log(JSON.stringify([r.proposed, a.ok, L.shown(r), L.decide(none, '2b').ok, L.decide(none, '1').ok]))""")
    assert r == ["2a", True, "2b", False, False]
