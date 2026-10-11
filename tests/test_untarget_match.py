"""Untargeted match (TP_Mine/js/26_untarget_match.js) with a fake search engine: the level of each feature comes from the library candidate (profile «mzlab»), a feature without MS2 stays at 4 or 5,
the external annotations (mzMine, GNPS2) are evidence and never a level, 2b only by the chemist's decision, the queries go in batches and keep their indices."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB, JS, DATA = ROOT / "mzlab" / "web", ROOT / "TP_Mine" / "js", ROOT / "tests" / "data" / "untargeted"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRE = f"""globalThis.window = globalThis; const store = {{}};
globalThis.localStorage = {{ getItem: k => (k in store ? store[k] : null), setItem: (k, v) => {{ store[k] = String(v); }} }};
require({str(WEB / 'livelli-1.js')!r}); require({str(WEB / 'livelli.js')!r}); const U = require({str(JS / '25_untarget.js')!r}); const M = require({str(JS / '26_untarget_match.js')!r}); const fs = require('fs');
const rd = p => fs.readFileSync({str(DATA)!r} + '/' + p, 'utf8');
const cand = (o = {{}}) => ({{ name: 'Alpha', ik: 'AAAAAAAAAAAAAA-X', ent: 0.9, cos: 0.85, shared: 8, dprec: 1.2, nlib: 30, lib: 'LIB', ...o }});
const feat = (o = {{}}) => ({{ key: 'POS:1', id: '1', pol: 1, mz: 301.1547, rt: 5, areas: {{ a: 1000 }}, ms2: {{ mz: [100, 200], it: [1, 2], prec: 0 }}, ann: [], nScans: 3, ...o }});
"""


def js(code):
    p = subprocess.run(["node", "-e", PRE + f"(async () => {{ console.log(JSON.stringify(await (async () => {{ {code} }})())); }})();"], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


def test_levels_from_the_library_candidate():
    r = js("""const a = M.assessRow(feat(), cand()), b = M.assessRow(feat({ nScans: 1 }), cand()), c = M.assessRow(feat(), cand({ ent: 0.65, shared: 3 })), d = M.assessRow(feat(), cand({ ent: 0.4, shared: 2 }));
      return { a: [a.level, a.name, a.ent, a.dppm], b: b.level, c: c.level, d: d.level, ev: a.evidence }""")
    assert r["a"] == ["2a", "Alpha", 0.9, 1.2] and r["b"] == "3" and r["c"] == "3" and r["d"] == "5"
    assert len(r["ev"]) >= 4          # the gates are listed as evidence


def test_a_second_structure_close_behind_makes_it_3_not_distinct():
    r = js("""const best = cand({ alt: cand({ name: 'Beta', ik: 'BBBBBBBBBBBBBB-X', ent: 0.88 }) }); const a = M.assessRow(feat(), best); return [a.level, a.alt, a.lv.why]""")
    assert r == ["3", "Beta", "confirm"]


def test_feature_without_ms2_is_4_with_a_unique_formula_else_5():
    r = js("""const noms = { ms2: null };
      const f5 = feat({ ...noms }), f4 = feat({ ...noms, ann: [{ source: 'mzMine library', name: 'X', formula: 'C17H20N2O3', adduct: '[M+H]+', structure: 'ok', analog: false, propagated: false }] }),
        f2 = feat({ ...noms, ann: [{ source: 'a', formula: 'C17H20N2O3', adduct: '[M+H]+' }, { source: 'b', formula: 'C16H16N4O3', adduct: '[M+H]+' }] }), fbad = feat({ ...noms, ann: [{ source: 'a', formula: 'C10H17NO2', adduct: '[M+H]+' }] });
      return [f5, f4, f2, fbad].map(f => { const x = M.assessRow(f, null); return [x.level, x.hasMs2, x.formula]; })""")
    assert r[0] == ["5", False, ""] and r[1][0] == "4" and r[1][2] == "C17H20N2O3" and r[2][0] == "4" and r[3][0] == "5"          # a formula that does not explain the m/z is ignored; none that does → 5


def test_external_annotations_are_evidence_never_a_level():
    r = js("""const t = U.build([{ name: 'tabella_pos.csv', text: rd('mzmine/tabella_pos.csv') }, { name: 'biblioteca.tsv', text: rd('gnps/biblioteca.tsv') }], { pol: 1 });
      const f = t.features[2]; const x = M.assessRow(f, null); return { level: x.level, ext: x.external, ml: U.assessExternal(f).level }""")
    assert r["level"] == "5" and r["ml"] == "2a"          # the profile attributio reads it as 2a; the profile mzlab never does
    assert r["ext"] == ["GNPS2: Caffeine MQScore 0.85, 8 shared"]


def test_2b_only_by_decision_and_refused_on_4_5_0():
    r = js("""const a = M.assessRow(feat(), cand()), five = M.assessRow(feat({ ms2: null }), null);
      const ok = M.decide(a, '2b'), bad = M.decide(five, '2b'), std = M.decide(a, '1');
      const unset = M.decide(a, ''); return { auto: a.level, ok: ok.ok, shown: M.shown(a), bad: bad.ok, std: std.ok, back: [unset.ok, M.shown(a)], dec: a.decided }""")
    assert r["auto"] == "2a" and r["ok"] is True and r["bad"] is False and r["std"] is False
    assert r["back"] == [True, "2a"]


def test_run_batches_the_queries_and_keeps_the_indices():
    r = js("""const feats = []; for (let i = 0; i < 2500; i++) feats.push(feat({ key: 'POS:' + i, id: String(i), ms2: i % 5 === 0 ? null : { mz: [100 + i], it: [1], prec: 0 } }));
      const calls = []; globalThis.LIB = { call: async (cmd, p, prog) => { calls.push(p.queries.length); prog && prog({ read: p.queries.length }); return { rows: p.queries.map(q => ({ key: q.key, ...(q.key % 2 ? cand({ name: 'N' + q.key }) : { name: '' }) })) }; } };
      const seen = []; const out = await M.run({ features: feats, pol: 1 }, { tolPpm: 10, onProgress: (i, n) => seen.push([i, n]) });
      const f1 = out.get('POS:1'), f2 = out.get('POS:2'), f5 = out.get('POS:5');
      return { calls, n: out.size, f1: [f1.name, f1.level], f2: [f2.name, f2.level], f5: [f5.hasMs2, f5.level], last: seen[seen.length - 1], q: out.stats.queries }""")
    assert r["calls"] == [1000, 1000, 1000][:len(r["calls"])] or r["calls"] == [1000, 1000, 0]
    assert r["q"] == 2000 and r["n"] == 2500 and r["f1"] == ["N1", "2a"] and r["f2"] == ["", "5"] and r["f5"] == [False, "5"] and r["last"] == [2000, 2000]


def test_run_stops_when_cancelled_and_uses_the_pseudo_panel_shape():
    r = js("""const feats = []; for (let i = 0; i < 3000; i++) feats.push(feat({ key: 'POS:' + i, id: String(i) }));
      let n = 0; globalThis.LIB = { call: async (c, p) => { n++; return { rows: p.queries.map(q => ({ key: q.key, ...cand() })) }; } };
      const out = await M.run({ features: feats, pol: 1 }, { isCancelled: () => n >= 1 });
      const p = M.panelOf(feats[0], { pol: 1, file: { name: 't.csv' } });
      return { calls: n, got: [...out.values()].filter(x => x.name).length, p: [p._a.data[0].d.mz, p._a.data[0].d.y, p._a.data[0].d.prec, p._a.data[0].f.polarity, p.r0, p.avg.length] }""")
    assert r["calls"] == 1 and r["got"] == 1000
    assert r["p"] == [[100, 200], [0.5, 1], 301.1547, "positive", 5, 3]
