"""Confidence levels (mzlab/web/livelli.js + the data file livelli-1.js): synthetic cases for every level, the evidence lines, the chemist's decision
(2b is never assigned by the program), the thresholds (storage, old format, workflow file) and the hidden «attributio» profile."""
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
const c = (ent, shared, dprec, name = 'A', ik = 'AAAAAAAAAAAAAA-X', extra = {{}}) => ({{ ent, cos: ent - 0.05, shared, dprec, name, ik, lib: 'LIB', nlib: 30, ...extra }});
"""


def run(code):
    p = subprocess.run(["node", "-e", PRE + code], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


def lv(code):
    return run(f"console.log(JSON.stringify((() => {{ {code} }})()))")


def test_data_file_is_versioned_and_has_two_profiles():
    d = run("console.log(JSON.stringify({f: L.data.formato, p: Object.keys(L.data.profili), vis: L.names(), def: L.profileName(), order: L.ORDER}))")
    assert d["f"] == "livelli/1" and d["p"] == ["mzlab", "attributio", "lr"] and d["vis"] == ["mzlab"] and d["def"] == "mzlab"
    assert d["order"] == ["1", "2a", "2b", "3a", "3b", "4", "5", "0"]


def test_attributio_profile_carries_the_draft_values():
    t = run("console.log(JSON.stringify(L.thresholds('attributio')))")
    assert t["coseno_min"] == 0.70 and t["ioni_comuni_min"] == 4 and t["precursore_ppm_max"] == 15.0 and t["margine_conflitto_coseno"] == 0.05
    assert run("console.log(JSON.stringify(L.data.profili.attributio.cascata.map(x => x.base)))")[:3] == ["reference standard", "spectral library", "diagnostic evidence"]


@pytest.mark.parametrize("code,level,why", [
    ("return L.assess({cands: [c(.9, 8, 2)], nScans: 3})", "2a", "2a"),                                           # every gate passes
    ("return L.assess({cands: [c(.9, 8, 2)], nScans: 1})", "3a", "confirm"),                                      # one scan only
    ("return L.assess({cands: [c(.9, 8, 2)]})", "3a", "confirm"),                                                 # the number of scans unknown counts as one
    ("return L.assess({cands: [c(.90, 8, 2), c(.88, 8, 2, 'B', 'BBBBBBBBBBBBBB-X')], nScans: 3})", "3a", "confirm"),  # two close candidates of different structure
    ("return L.assess({cands: [c(.90, 8, 2), c(.89, 8, 2, 'A2', 'AAAAAAAAAAAAAA-Y')], nScans: 3})", "2a", "2a"),     # same first block: stereoisomers are not competitors
    ("return L.assess({cands: [c(.90, 8, 2), c(.70, 8, 2, 'B', 'BBBBBBBBBBBBBB-X')], nScans: 3})", "2a", "2a"),     # clear lead
    ("return L.assess({cands: [c(.65, 8, 2)], nScans: 5})", "3a", "provvisorio"),                                 # entropy between 0.60 and 0.75
    ("return L.assess({cands: [c(.90, 8, 8)], nScans: 5})", "3a", "provvisorio"),                                 # precursor 8 ppm away
    ("return L.assess({cands: [c(.90, 3, 2, 'A', 'AAAAAAAAAAAAAA-X', {nlib: 4})], nScans: 3})", "2a", "2a"),       # a library spectrum of 4 peaks needs only 3
    ("return L.assess({cands: [c(.90, 3, 2)], nScans: 3})", "3a", "provvisorio"),
    ("return L.assess({cands: [c(.50, 8, 2)], nScans: 3})", "5", "none"),                                        # nothing above the gates, no analog
    ("return L.assess({cands: [c(.50, 8, 2)], nScans: 3, formulaUnique: true})", "4", "formula"),
    ("return L.assess({cands: [], nScans: 3})", "5", "empty"),
    ("return L.assess({cands: [], analogs: [{name: 'An', mcos: .8, matched: 6, delta: 15.9949}]})", "3b", "analogo"),   # an analog only
    ("return L.assess({cands: [], analogs: [{name: 'An', mcos: .4, matched: 6, delta: 15.9949}]})", "5", "empty"),
])
def test_proposed_level(code, level, why):
    r = lv(code)
    assert (r["proposed"], r["why"]) == (level, why) and r["decided"] is None, r


def test_level_1_only_with_the_standard_marked_by_the_user():
    ok = lv("return L.assess({cands: [c(.9, 8, 2)], nScans: 3, std: {name: 'S', drt: 0.05, ent: .85, dprec: 2}})")
    assert ok["proposed"] == "1" and ok["why"] == "standard"
    for std in ("{name: 'S', drt: 0.30, ent: .85, dprec: 2}", "{name: 'S', drt: 0.05, ent: .70, dprec: 2}", "{name: 'S', drt: 0.05, ent: .85, dprec: 9}"):
        r = lv(f"return L.assess({{cands: [c(.9, 8, 2)], nScans: 3, std: {std}}})")
        assert r["proposed"] == "2a", (std, r["proposed"])                      # the standard fails a gate: the library decides
    assert lv("return L.assess({cands: [c(.99, 20, 0)], nScans: 9})")["proposed"] == "2a"       # a perfect library match is never 1


def test_the_program_never_proposes_2b():
    r = run("""const out = new Set(); for (const ent of [0, .5, .65, .8, .99]) for (const sh of [1, 3, 8]) for (const pp of [0, 3, 9]) for (const n of [1, 2, 5])
      for (const an of [null, [{name: 'x', mcos: .9, delta: 1}]]) for (const std of [null, {drt: 0, ent: 1, dprec: 0}]) out.add(L.assess({cands: [c(ent, sh, pp)], analogs: an, nScans: n, std}).proposed);
      console.log(JSON.stringify([...out]))""")
    assert "2b" not in r and set(r) <= {"1", "2a", "3a", "3b", "4", "5", "0"}


def test_evidence_has_one_line_per_gate_with_value_threshold_and_outcome():
    r = lv("return L.assess({cands: [c(.62, 8, 2)], nScans: 3})")
    assert len(r["gates"]) == 5 and len(r["lines"]) == 5
    assert "entropia 0.62 vs 0.75 ✗" in r["lines"], r["lines"]
    assert any(x.startswith("|Δ precursore| 2.0 vs 5.0 ppm ✓") for x in r["lines"]), r["lines"]
    assert r["decisive"].startswith("L3a: provvisorio")


def test_decisive_sentence_of_the_gates_that_pass_and_of_no_source():
    assert lv("return L.assess({cands: [c(.9, 8, 2)], nScans: 3})")["decisive"] == "L2a: la libreria LIB passa ogni cancello (A)"
    assert lv("return L.assess({cands: [c(.5, 8, 2)], nScans: 3})")["decisive"].startswith("Nessuna fonte sopra i cancelli; la più vicina: A: entropia 0.50 vs 0.75 ✗")


def test_decision_of_the_chemist():
    r = lv("""const out = []; const r3 = L.assess({cands: [c(.9, 8, 2)], nScans: 1}), r5 = L.assess({cands: []}), r2a = L.assess({cands: [c(.9, 8, 2)], nScans: 3});
      out.push(L.decide(r3, '2b').ok, r3.decided, L.shown(r3), r3.proposed);                       // 2b from 3a: allowed, written only in `decided`
      out.push(L.decide(r2a, '2b').ok);
      const bad = L.decide(r5, '2b'); out.push(bad.ok, bad.msg, r5.decided);                       // refused on 5 (and on 0), with a message
      out.push(L.decide(L.assess({cands: [], formulaUnique: true}), '2b').ok, L.decide(L.assess({cands: [], analogs: []}, 'attributio'), '2b').ok);
      out.push(L.decide(r3, '1').ok, L.decide(r3, 'x').ok);                                        // 1 without a standard, unknown level
      out.push(L.decide(r3, '').ok, r3.decided);                                                   // cleared
      return out""")
    assert r[:4] == [True, "2b", "2b", "3a"] and r[4] is True
    assert r[5] is False and "2b" in r[6] and r[7] is None
    assert r[8] is True and r[9] is False                                                              # 4 may carry a chemical context; 0 never
    assert r[10] is False and r[11] is False and r[12] is True and r[13] is None


def test_thresholds_are_editable_stored_and_exported():
    r = lv("""const out = [L.thresholds().entropia_min]; L.setThreshold('entropia_min', .9); out.push(L.thresholds().entropia_min);
      out.push(L.assess({cands: [c(.85, 8, 2)], nScans: 3}).proposed);                             // with 0.90 an entropy of 0.85 is no longer 2a
      out.push(L.setThreshold('inesistente', 1), L.setThreshold('picchi_min', -1));
      const ex = L.exportState(); out.push(ex.formato, ex.profilo, ex.soglie.entropia_min);
      L.reset(); out.push(L.thresholds().entropia_min); L.importState(ex); out.push(L.thresholds().entropia_min);
      return out""")
    assert r == [0.75, 0.9, "3a", False, False, "livelli/1", "mzlab", 0.9, 0.75, 0.9]


def test_old_threshold_format_is_still_read():
    r = lv("""localStorage.setItem('qqq.lib.soglie', JSON.stringify({ent: .8, shared: 7, ppm: 3, ent2: .5}));
      const t = L.thresholds(); const o = [t.entropia_min, t.picchi_min, t.precursore_ppm_max, t.entropia_provvisorio];
      L.importState({ent: .7, shared: 5, ppm: 4, ent2: .55}); o.push(L.thresholds().entropia_min, L.thresholds().picchi_min); return o""")
    assert r == [0.8, 7, 3, 0.5, 0.7, 5]


def test_attributio_profile_cascade():
    base = "L.assess({cands: [%s], nScans: 1}, 'attributio')"
    assert lv("return " + base % "c(.9, 5, 10, 'A', 'AAAAAAAAAAAAAA-X', {cos: .75})")["proposed"] == "2a"        # cosine 0.75, 5 shared ions, 10 ppm: a valid source
    assert lv("return " + base % "c(.9, 5, 10, 'A', 'AAAAAAAAAAAAAA-X', {cos: .62})")["proposed"] == "0"         # cosine 0.62 < 0.70: not a source, nothing else
    assert lv("return " + base % "c(.9, 5, 20, 'A', 'AAAAAAAAAAAAAA-X', {cos: .75})")["proposed"] == "0"         # 20 ppm: an analog, never 2a
    assert lv("return " + base % "c(.9, 3, 10, 'A', 'AAAAAAAAAAAAAA-X', {cos: .75})")["proposed"] == "0"         # 3 shared ions < 4
    conflict = lv("return L.assess({cands: [c(.9, 5, 10, 'A', 'AAAAAAAAAAAAAA-X', {cos: .75}), c(.9, 5, 10, 'B', 'BBBBBBBBBBBBBB-X', {cos: .73})]}, 'attributio')")
    assert conflict["proposed"] == "3a" and conflict["why"] == "confirm"                                           # a name conflict: the best is ahead by less than 0.05
    near = lv("return L.assess({cands: [c(.9, 5, 10, 'A', 'AAAAAAAAAAAAAA-X', {cos: .62})]}, 'attributio')")
    assert near["decisive"].startswith("Nessuna fonte sopra i cancelli; la più vicina: A: coseno 0.62 vs 0.70 ✗")


def test_lr_profile_never_goes_above_4():
    """Unit resolution: every gate passed gives 4 (never 2a: an MS2 does not raise a level, D6 of Tabui); no source gives 4 with a unique nominal formula, else 5."""
    passed = lv("return L.assess({cands: [c(.9, 8, 100)], nScans: 3}, 'lr')")
    assert passed["proposed"] == "4" and passed["why"] == "tetto"
    assert lv("return L.assess({cands: [c(.3, 1, 500)], nScans: 3, formulaUnique: true}, 'lr')")["proposed"] == "4"
    assert lv("return L.assess({cands: [], nScans: 1}, 'lr')")["proposed"] == "5"
    assert lv("return L.assess({cands: [c(.9, 8, 2)], nScans: 3}, 'mzlab')")["proposed"] == "2a"


def test_colour_class_and_sentence_never_say_it_is():
    r = lv("return [L.cls('2a'), L.cls('0'), L.sentence('2a'), L.sentence('3a'), L.tag('3b'), L.name('5')]")
    assert r[0] == "lv-2a" and r[1] == "lv-0"
    assert r[2] == "Candidato · L2a · Probabile (libreria)" and r[3].startswith("Candidato · L3a") and r[4] == "L3b"


def test_points_section_does_not_change_the_levels():
    """The points of the proofs (section `punti`, read by crates/mzlab-feat) explain the order and never assign a level: the same cases give the same
    proposal, reason and gates with the section and without it."""
    r = run("""const cases = [];
      for (const ent of [.5, .62, .7, .8, .95]) for (const shared of [2, 4, 6, 9]) for (const dprec of [1, 4.9, 7]) for (const nScans of [1, 3]) for (const profile of ['mzlab', 'attributio'])
        cases.push([{ cands: [c(ent, shared, dprec), c(ent - .02, shared, dprec, 'B', 'BBBBBBBBBBBBBB-X')], nScans, formulaUnique: shared > 5 }, profile]);
      cases.push([{ cands: [] }, 'mzlab'], [{ cands: [], formulaUnique: true }, 'mzlab'], [{ cands: [c(.9, 8, 2)], std: { drt: .05, ent: .9, dprec: 1, name: 'S' }, nScans: 3 }, 'mzlab']);
      const run = () => cases.map(([i, p]) => { const x = L.assess(i, p); return [x.proposed, x.why, x.gates.map(g => g.txt), x.decisive]; });
      const withPoints = run(), had = !!L.data.profili.mzlab.punti; delete L.data.profili.mzlab.punti;
      console.log(JSON.stringify({ had, n: cases.length, same: JSON.stringify(withPoints) === JSON.stringify(run()) }))""")
    assert r["had"] is True and r["n"] > 200 and r["same"] is True


def test_points_section_shape():
    p = run("console.log(JSON.stringify(L.data.profili.mzlab.punti))")
    assert p["versione"] == 1 and set(p["prove"]) == {"massa", "isotopi", "MS2", "frammenti", "rete", "cinetica", "controlli", "RT", "frequenza", "esterne"}
    assert [g["punti"] for g in p["prove"]["massa"]["gradini"]] == [2, 1] and p["prove"]["MS2"]["gradini"][0] == {"soglia": "entropia_min", "punti": 3}
    assert p["prove"]["cinetica"]["massimo"] == 0 and p["prove"]["cinetica"]["massimo_se_pesata"] == 1           # K1: informative only
    soglie = run("console.log(JSON.stringify(L.data.profili.mzlab.soglie))")
    keys = [g["soglia"] for r in p["prove"].values() for g in r["gradini"] if isinstance(g["soglia"], str)] + [r["richiede"]["soglia"] for r in p["prove"].values() if "richiede" in r]
    keys += list(p["tolleranze"].values())
    assert keys and all(k in soglie for k in keys)                                                               # every named threshold exists in `soglie`
    for r in p["prove"].values():
        pts = [g["punti"] for g in r["gradini"]]
        assert pts == sorted(pts, reverse=True) and all(x >= 0 for x in pts)                                    # best step first, never negative points


def test_rust_json_is_generated_from_the_data_file():
    out = subprocess.run(["python3", str(WEB.parent.parent / "tools" / "esporta_livelli.py"), "--controlla"], capture_output=True, encoding="utf-8")
    assert out.returncode == 0, out.stderr
