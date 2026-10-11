"""Identities of an untargeted table (TP_Mine/js/29_untarget_identity.js) with the real Python engine (tpmine/hr/identity.py) between two node runs: the payload sent by the page, the
singletons, the level of an identity = the one of its best member (the 2b only if the chemist decided it), the structure key, the words of the evidence and the small network."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB, JS = ROOT / "mzlab" / "web", ROOT / "TP_Mine" / "js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")
sys.path.insert(0, str(ROOT / "TP_Mine" / "py"))
from tpmine.hr import identity  # noqa: E402

P, NA, NH4 = 1.00727646688, 22.98976928 - 0.00054858, 18.03437413 - 0.0
M0 = 300.1000
RAMP = [1.0, 2.5, 4.0, 3.0, 6.0]
OTHER = [5.0, 1.0, 4.0, 6.0, 0.5]


def csv_table():
    samples = [f"S{i}" for i in range(1, 6)]
    head = "id,mz,rt," + ",".join(f"datafile:{s}.mzML:area" for s in samples)
    def row(i, mz, rt, prof, k=1e5):
        return f"{i},{mz:.5f},{rt},{','.join(str(int(k * v)) for v in prof)}"
    rows = [row(1, M0 + P, 5.20, RAMP), row(2, M0 + 22.98921, 5.21, [v * 0.3 for v in RAMP]), row(3, M0 + 18.03382, 5.20, [v * 0.1 + (0.05 if i % 2 else 0) for i, v in enumerate(RAMP)]),
            row(4, M0 + P + 1.003355, 5.20, [v * 0.2 for v in RAMP]),
            row(5, 410.2000, 7.00, OTHER), row(6, 426.1950, 7.00, [v * 0.7 for v in OTHER]),             # co-eluted, same shape, difference 15.995: not a relation
            row(7, 505.3000, 3.00, RAMP[:2] + [0, 0, 0])]                                                # two samples only: a singleton
    return head + "\n" + "\n".join(rows) + "\n"


PRE = f"""globalThis.window = globalThis; const store = {{}};
globalThis.localStorage = {{ getItem: k => (k in store ? store[k] : null), setItem: (k, v) => {{ store[k] = String(v); }} }};
require({str(WEB / 'livelli-1.js')!r}); require({str(WEB / 'livelli.js')!r}); const U = require({str(JS / '25_untarget.js')!r}); const M = require({str(JS / '26_untarget_match.js')!r}); const ID = require({str(JS / '29_untarget_identity.js')!r});
const TXT = {json.dumps(csv_table())}; const mk = () => U.build([{{ name: 'tabella_pos.csv', text: TXT }}], {{ pol: 1 }});
"""


def js(code, extra=""):
    p = subprocess.run(["node", "-e", PRE + extra + f"(async () => {{ console.log(JSON.stringify(await (async () => {{ {code} }})())); }})();"], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


@pytest.fixture(scope="module")
def graph():
    """the payload the page sends → the Python engine → the graph the page receives"""
    body = js("let b; await ID.compute(mk(), async (fn, body) => { b = body; return { identities: [], edges: [], stats: {} }; }); return b")
    return body, identity.build(body)


def with_graph(g, code):
    return js(f"const G = {json.dumps(g)}; const t = mk(); const gr = await ID.compute(t, async () => G); {code}")


def test_payload_sends_only_features_that_can_be_linked(graph):
    body, _ = graph
    assert body["pol"] == 1 and body["samples"] == ["S1", "S2", "S3", "S4", "S5"] and [f["id"] for f in body["features"]] == ["1", "2", "3", "4", "5", "6"]    # 7 has two samples: a singleton
    assert body["features"][0]["a"][0] == [0, 100000]


def test_adducts_and_isotope_one_identity_unexplained_difference_not_grouped(graph):
    _, g = graph
    r = with_graph(g, "return { ids: gr.identities.map(x => [x.id, x.members.join(' ')]), by7: gr.byKey.get('POS:7').members, st: gr.stats }")
    groups = {tuple(sorted(x[1].split())) for x in r["ids"]}
    assert groups == {("POS:1", "POS:2", "POS:3", "POS:4"), ("POS:5",), ("POS:6",), ("POS:7",)} and r["by7"] == ["POS:7"]
    assert r["st"]["identities"] == 4 and r["st"]["grouped"] == 1 and [x[0] for x in r["ids"]] == ["ID1", "ID2", "ID3", "ID4"]


def test_the_singleton_is_read_as_the_protonated_ion(graph):
    _, g = graph
    r = with_graph(g, "const x = gr.byKey.get('POS:7'); return x.neutral")
    assert r["form"] == "[M+H]+" and abs(r["mass"] - (505.3 - P)) < 1e-4 and r["explained"] == 1


def test_level_of_an_identity_is_the_best_member_and_2b_only_if_decided(graph):
    _, g = graph
    r = with_graph(g, """const cand = { name: 'Alpha', ik: 'AAAAAAAAAAAAAA-UHFFFAOYSA-N', ent: 0.9, cos: 0.85, shared: 8, dprec: 1.2, nlib: 30, lib: 'LIB' };
      t.features.forEach(f => { f.ms2 = { mz: [100, 200], it: [1, 2], prec: 0 }; f.nScans = 3; });
      const res = new Map(t.features.map(f => [f.key, M.assessRow(f, f.id === '3' ? cand : null)]));
      ID.annotate(gr, t, res); const x = gr.byKey.get('POS:1');
      const lv = [x.level, x.via, x.best.name, x.structKeys, x.structConflict, gr.byKey.get('POS:5').level];
      const o = M.decide(res.get('POS:3'), '2b'); ID.annotate(gr, t, res); const after = [o.ok, gr.byKey.get('POS:1').level, gr.byKey.get('POS:1').decided];
      const o2 = M.decide(res.get('POS:5'), '2b'); return { lv, after, refused: o2.ok }""")
    assert r["lv"] == ["2a", "POS:3", "Alpha", ["AAAAAAAAAAAAAA"], False, "5"]            # the level comes from feature 3, the member with the library match
    assert r["after"] == [True, "2b", True] and r["refused"] is False                    # the 2b appears only after the chemist decided it, and not from a level 5
    # (a refused decision leaves the level as it was)


def test_structure_key_and_two_structures_in_one_identity(graph):
    _, g = graph
    r = with_graph(g, """const mkc = ik => ({ name: 'N' + ik[0], ik, ent: 0.9, cos: 0.85, shared: 8, dprec: 1, nlib: 30, lib: 'L' });
      t.features.forEach(f => { f.ms2 = { mz: [1], it: [1], prec: 0 }; f.nScans = 3; });
      const res = new Map(t.features.map(f => [f.key, M.assessRow(f, f.id === '1' ? mkc('AAAAAAAAAAAAAA-UHFFFAOYSA-N') : f.id === '2' ? mkc('BBBBBBBBBBBBBB-UHFFFAOYSA-N') : null)]));
      ID.annotate(gr, t, res); const x = gr.byKey.get('POS:1');
      return { k: [ID.structKey('abcdefghijklmn-XXXXXXXXXX-N'), ID.structKey('ABC'), ID.structKey(null)], c: x.structConflict, keys: x.structKeys }""")
    assert r["k"] == ["ABCDEFGHIJKLMN", "", ""] and r["c"] is True and r["keys"] == ["AAAAAAAAAAAAAA", "BBBBBBBBBBBBBB"]


def test_evidence_words_and_network(graph):
    _, g = graph
    r = with_graph(g, """const x = gr.byKey.get('POS:1'); const es = x.edges.map(i => gr.edges[i]);
      return { h: ID.hypothesisText(x.neutral, t), e: es.map(ID.edgeText), s: es.map(ID.edgeShort), svg: ID.networkSvg(x, gr, t), none: ID.networkSvg(gr.byKey.get('POS:5'), gr, t).includes('aria-label') }""")
    assert r["h"].startswith("Neutral mass 300.1000 · [M+H]+ of feature 1 · explains 4 members")
    na = next(e for e in r["e"] if "[M+Na]+" in e)
    assert "r 1.00 over 5 samples" in na and "ΔRT" in na and "scans" in na and "ppm" in na
    assert "[M+Na]+" in r["s"] and any("isotope" in s for s in r["s"])
    assert r["svg"].count("<line") == len(r["e"]) and "Network of identity" in r["svg"] and "stroke=" not in r["svg"]
    assert r["none"]
    words = " ".join(r["e"] + [r["h"]]).lower()
    assert not any(w in words for w in ("identified", "confirmed", "is the compound", "proves"))


def test_the_label_of_the_link_carries_its_evidence(graph):
    _, g = graph
    r = with_graph(g, "const x = gr.byKey.get('POS:1'); return x.edges.map(i => gr.edges[i]).map(e => [e.kind, ID.edgeText(e)])")
    by = {k: t for k, t in r}
    na = next(t for k, t in r if "[M+Na]+" in t)
    assert na.startswith("adduct (evidence: Δm/z 21.9819 = adduct [M+Na]+") and "ΔRT" in na                                # the step is in the table of adducts: a signal, not a decision
    nh4 = next(t for k, t in r if "[M+NH4]+" in t)
    assert nh4.startswith("not distinguishable") and "NH3" in nh4                                                         # 17.0265 is an adduct and also the loss of NH3
    assert any(k == "isotope" and not t.startswith(("adduct", "in-source", "not distinguishable")) for k, t in r)         # an isotope gets no adduct/fragment label


def test_polarity_must_be_known():
    r = js("const t = mk(); t.pol = 0; try { await ID.compute(t, async () => ({})); return 'no'; } catch (e) { return e.key; }")
    assert r == "pol"


def test_pairs_of_the_two_polarities_use_the_engine_payload():
    r = js("const f = (k, mz, rt) => ({ key: k, mz, rt }); let b; await ID.pairPolarity({ features: [f('POS:1', 301.1, 5)] }, { features: [f('NEG:2', 299.0, 5.1)] }, async (fn, body) => { b = [fn, body]; return []; }); return b")
    assert r[0] == "polarity_pairs" and r[1]["pos"] == [{"k": "POS:1", "mz": 301.1, "rt": 5}] and r[1]["ppm"] == 5 and r[1]["rt_min"] == 0.15
