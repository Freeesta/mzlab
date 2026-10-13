"""Untargeted tables (TP_Mine/js/25_untarget.js): the readers recognise the columns by name (mzMine full table, annotations, FBMN quant + MGF + IIN edges, MS-DIAL alignment, GNPS2
library and network), the rules of Attributio on structures / lipids / propagated names / analogues, and the external evidence read with the profile «attributio» of livelli/1.
The readers are Rust (crates/mzlab-feat/src/import; the .wasm for node is built by tests/conftest.py); whole tables, quoted fields and CRLF are compared with the answers of the old JS readers by crates/mzlab-feat/tests/import_parity.rs.
The fixtures in tests/data/untargeted/ are written by hand in the format of each program (nothing real). The parity with the synthetic campaign of the data repository runs only with MZLAB_DATI."""
import csv
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB, JS, DATA = ROOT / "mzlab" / "web", ROOT / "TP_Mine" / "js", ROOT / "tests" / "data" / "untargeted"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRE = f"""globalThis.window = globalThis; const store = {{}};
globalThis.localStorage = {{ getItem: k => (k in store ? store[k] : null), setItem: (k, v) => {{ store[k] = String(v); }} }};
require({str(WEB / 'livelli-1.js')!r}); require({str(WEB / 'livelli.js')!r}); const U = require({str(JS / '25_untarget.js')!r}); const fs = require('fs');
const rd = p => fs.readFileSync({str(DATA)!r} + '/' + p, 'utf8'), F = (p, name) => ({{ name: name || p.split('/').pop(), text: rd(p) }});
const ALL = () => [F('mzmine/tabella_pos.csv'), F('mzmine/annotazioni_pos.csv'), F('mzmine/fbmn.mgf'), F('gnps/biblioteca.tsv'), F('gnps/rete.tsv')];
"""


def run(code):
    p = subprocess.run(["node", "-e", PRE + code], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


def js(code):
    return run(f"console.log(JSON.stringify((() => {{ {code} }})()))")


def test_detect_recognises_every_file_by_its_columns():
    d = js("return Object.fromEntries(['mzmine/tabella_pos.csv','mzmine/annotazioni_pos.csv','mzmine/fbmn_quant.csv','mzmine/fbmn_edges.csv','mzmine/fbmn.mgf','msdial/allineamento.txt','gnps/biblioteca.tsv','gnps/rete.tsv'].map(p => [p, U.detect('x', rd(p))]))")
    assert d == {"mzmine/tabella_pos.csv": "mzmine-full", "mzmine/annotazioni_pos.csv": "mzmine-annotations", "mzmine/fbmn_quant.csv": "mzmine-quant", "mzmine/fbmn_edges.csv": "mzmine-edges",
                 "mzmine/fbmn.mgf": "mgf", "msdial/allineamento.txt": "msdial", "gnps/biblioteca.tsv": "gnps-library", "gnps/rete.tsv": "gnps-network"}


def test_mzmine_full_table_model():
    t = js("const t = U.build([F('mzmine/tabella_pos.csv', 't')], {pol: 1}); return {n: t.features.length, s: t.samples, f1: t.features[0], f4: t.features[3], f6: t.features[5], st: t.stats}")
    assert t["n"] == 6 and t["s"] == ["Campione_A_pos", "Campione_B_pos"]
    f1 = t["f1"]
    assert f1["key"] == "POS:1" and f1["mz"] == 184.1332 and f1["rt"] == 5.2 and f1["areas"] == {"Campione_A_pos": 100000} and f1["nScans"] == 3
    assert f1["detected"] == ["Campione_A_pos"] and f1["iin"]["id"] == "7" and f1["iin"]["neutral"] == 183.1259
    a = f1["ann"][0]
    assert (a["source"], a["type"], a["score"], a["shared"], a["adduct"]) == ("mzMine library", "cosine", 0.91, 8, "[M+H]+") and a["cleanName"] == "Testamine"
    assert abs(a["ppm"]) < 1e-6
    assert t["f4"]["detected"] == []          # ESTIMATED is not DETECTED
    assert t["f6"]["areas"] == {"Campione_B_pos": 35000}          # tolerant number format
    assert t["st"]["withMatch"] == 3


def test_mzmine_annotations_complete_the_same_feature_and_name_and_drop_lipids():
    t = js("const t = U.build([F('mzmine/tabella_pos.csv'), F('mzmine/annotazioni_pos.csv')], {pol: 1}); return {a1: t.features[0].ann, a5: t.features[4].ann, st: t.stats}")
    assert len(t["a1"]) == 1 and t["a1"][0]["ik"] == "AAAAAAAAAAAAAA-UHFFFAOYSA-N" and t["a1"][0]["entry"] == "E-1"          # name matched without case; the other name of the same feature is not used
    assert t["a5"][0]["ik"] == "FFFFFFFFFFFFFF-UHFFFAOYSA-N"
    assert t["st"]["lipidRowsDropped"] == 1 and t["st"]["annotationsMerged"] == 3


def test_library_structure_must_explain_the_precursor():
    t = js("const t = U.build([F('mzmine/tabella_pos.csv'), F('mzmine/annotazioni_pos.csv')], {pol: 1}); const a = i => t.features[i].ann[0]; return {a1: a(0), a2: a(1), a5: a(4), st: t.stats}")
    assert t["a1"]["structure"] == "ok" and t["a5"]["structure"] == "ok"
    bad = t["a2"]
    assert bad["structure"] == "unreliable" and bad["ik"] == "" and bad["smiles"] == "" and bad["formula"] == "" and any("unreliable" in n for n in bad["notes"])
    assert bad["cleanName"] == "Testol" and bad["score"] == 0.88          # the match stays; only its structure is dropped
    assert t["st"]["unreliableStructures"] == 1


def test_check_structure_adducts_and_tolerance():
    d = js("""return {ok: U.checkStructure('C10H17NO2', 184.1332, 'M+H', 1), na: U.checkStructure('C10H17NO2', 206.1151, '[M+H]+', 1), bad: U.checkStructure('C10H17NO2', 301.1547, 'M+H', 1),
      none: U.checkStructure('', 184.1, 'M+H', 1), neg: U.checkStructure('C10H17NO2', 182.1187, '[M-H]-', -1), norm: [U.normAdduct('M+H'), U.normAdduct('M-H'), U.normAdduct('[M+Na]+'), U.normAdduct('M+HCOO')]}""")
    assert d["ok"]["ok"] is True and d["na"]["ok"] is True and d["na"]["adduct"] == "[M+Na]+"          # another common adduct explains it
    assert d["bad"]["ok"] is False and d["none"]["ok"] is None and d["neg"]["ok"] is True
    assert d["norm"] == ["[M+H]+", "[M-H]-", "[M+Na]+", "[M+HCOO]-"]


def test_mgf_ms2_only_and_most_peaks_per_feature():
    d = js("const t = U.build([F('mzmine/tabella_pos.csv'), F('mzmine/fbmn.mgf')], {pol: 1}); return {blocks: t.stats.mgfBlocks, notMs2: t.stats.mgfNotMs2, f: t.features.map(f => f.ms2)}")
    assert d["blocks"] == 4 and d["notMs2"] == 1 and len([m for m in d["f"] if m]) == 2
    s1 = d["f"][0]
    assert s1["mz"] == [56.05, 84.08, 156.1] and s1["it"] == [120.5, 800, 300]          # the block with 3 peaks wins over the one with 1; the MS1 block is out
    t = js("const t = U.build(ALL(), {pol: 1}); return {ms2: t.features.map(f => !!f.ms2), n: t.features.map(f => f.nScans), st: t.stats}")
    assert t["ms2"] == [True, False, False, False, True, False] and t["n"][0] == 3 and t["n"][4] == 3 and t["st"]["ms2"] == 2


def test_fbmn_quant_and_iin_edges():
    d = js("const t = U.build([F('mzmine/fbmn_quant.csv'), F('mzmine/fbmn_edges.csv')], {pol: 1}); return {f: t.features.map(f => [f.id, f.group, f.iin && f.iin.id, f.areas]), s: t.samples, e: t.edges, n: t.stats.edges}")
    assert d["s"] == ["Campione_A_pos", "Campione_B_pos"] and d["n"] == 2
    assert d["f"][0] == ["1", "3", "7", {"Campione_A_pos": 100000}] and d["f"][1][1] is None          # −1 = no group
    assert d["e"][0] == {"a": "1", "b": "6", "type": "MS1 annotation", "score": 0.97, "annotation": "[M+H]+ [M+Na]+ dm/z=21.98"}


def test_msdial_alignment_skips_the_metadata_rows():
    t = js("const t = U.build([F('msdial/allineamento.txt', 'a')], {pol: 1}); return {f: t.features, s: t.samples}")
    assert t["s"] == ["Campione_A", "Campione_B"] and len(t["f"]) == 4
    f0 = t["f"][0]
    assert f0["mz"] == 184.1332 and f0["rt"] == 5.2 and f0["ms2"]["mz"] == [56.05, 84.08, 156.1] and f0["ms2"]["it"] == [120, 800, 300] and f0["areas"] == {"Campione_A": 100000}
    assert f0["ann"][0]["source"] == "MS-DIAL" and f0["ann"][0]["score"] == 912 and f0["ann"][0]["ik"].startswith("AAAA")
    assert t["f"][1]["ann"] == [] and t["f"][2]["ann"] == []          # «Unknown» and «w/o MS2» are not annotations
    assert t["f"][3]["ms2"]["mz"] == [100, 110]


def test_gnps2_library_is_external_evidence_with_analogues_and_no_propagated_names():
    t = js("const t = U.build(ALL(), {pol: 1}); return {a1: t.features[0].ann, a2: t.features[1].ann, a3: t.features[2].ann, comp: t.features.map(f => f.component), st: t.stats}")
    g1 = [a for a in t["a1"] if a["source"] == "GNPS2"]
    assert [a["name"] for a in g1][:1] == ["MassbankEU:Testamine|HCD"] and g1[0]["cleanName"] == "Testamine" and g1[0]["ik"] == "AAAAAAAAAAAAAA"          # InChIKey-Planar preferred
    assert g1[0]["analog"] is False and g1[1]["analog"] is True and g1[1]["ppm"] > 15          # exact first, then the analogue
    g2 = [a["name"] for a in t["a2"] if a["source"] == "GNPS2"]
    assert g2 == ["Testol"]          # «Candidate … (known structural isomers» is never a source
    assert t["st"]["gnpsPropagated"] == 1 and t["st"]["gnpsAnalogs"] == 1 and t["st"]["gnpsUnmatched"] == 1
    assert t["comp"][0] == 7 and t["comp"][1] is None          # component −1 = isolated node


def test_clean_names_follow_the_profile():
    d = js("""const c = U.cleanName; return [c('Massbank:Caffeine'), c('MassbankEU:Foo|HCD 30'), c('Spectral Match to Bar from NIST14'), c('MoNA:Baz - 30.0 eV'), c('MB000123 Qux'), c('Quux_CE35.5'),
      c('  Plain  '), c('ReSpect:Name|x|y'), c('')]""")
    assert d == ["Caffeine", "Foo", "Bar", "Baz", "Qux", "Quux", "Plain", "Name", ""]
    assert js("return [U.isPropagated('Candidate X'), U.isPropagated('Y (known structural isomers)'), U.isPropagated('Z (delta mass: 14)'), U.isPropagated('Candidate')]")[:3] == [True, True, True]


def test_mzfinder_internal_features_enter_the_same_model():
    t = js("const t = U.fromMzfinder({files: ['a', {name: 'b'}], rows: [{id: 3, mz: 301.1, rt: 4.2, series: [0, 5e4], formula: 'C1H1', ms2_scans: 2}]}, {pol: 'positive'}); return {f: t.features[0], p: t.pol, s: t.samples}")
    assert t["p"] == 1 and t["s"] == ["a", "b"] and t["f"]["key"] == "POS:3" and t["f"]["areas"] == {"b": 50000} and t["f"]["program"] == "mzfinder" and t["f"]["ann"][0]["source"] == "mzFinder"


def test_build_rejects_without_a_table_and_keeps_going_with_a_bad_side_file():
    assert js("try { U.build([F('mzmine/fbmn.mgf')]); return 'no'; } catch (e) { return e.key; }") == "noTable"
    t = js("const t = U.build([F('mzmine/tabella_pos.csv'), {name: 'x.tsv', text: 'a\\tb\\n1\\t2\\n'}], {pol: 1}); return {used: t.used, ign: t.ignored}")
    assert t["used"] == ["tabella_pos.csv"] and t["ign"] == ["x.tsv"]


def test_external_levels_with_the_attributio_profile_on_our_fixtures():
    r = js("""const t = U.build(ALL(), {pol: 1}), m = U.assessAllExternal(t); return Object.fromEntries([...m].map(([k, v]) => [k, [v.level, v.base]]))""")
    assert r["POS:1"] == ["2a", "spectral library"]          # mzMine (0.91, 8 ions) and GNPS2 (0.93, 9 ions) name the same structure
    assert r["POS:2"] == ["0", "none"]          # mzMine 3 ions < 4; GNPS2 0.50 < 0.70
    assert r["POS:3"] == ["2a", "spectral library"] and r["POS:5"] == ["2a", "spectral library"]
    assert r["POS:4"] == ["0", "none"] and r["POS:6"] == ["0", "none"]


def test_external_name_conflict_and_isomers_not_discriminated():
    r = js("""const t = U.build(ALL(), {pol: 1}); const f = t.features[2];
      f.ann.push({source: 'mzMine library', type: 'cosine', score: 0.83, shared: 6, ppm: 0, name: 'Other', cleanName: 'Other', ik: 'OTHEROTHEROTHE', analog: false, propagated: false, notes: []});
      const g = t.features[4]; g.ann.push({source: 'GNPS2', type: 'MQScore', score: 0.2, shared: 9, ppm: 0, name: 'Weak', cleanName: 'Weak', ik: 'WEAKWEAKWEAKWE', analog: false, propagated: false, notes: []});
      const a = U.assessExternal(f), b = U.assessExternal(g);
      return {conf: [a.level, a.base, a.best.name], weak: [b.level, b.base, b.valid]}""")
    assert r["conf"] == ["3", "name conflict", "Caffeine"]          # two valid sources, different structures, 0.85 vs 0.83 < 0.05
    assert r["weak"][:2] == ["2a", "spectral library"] and r["weak"][2] == 1          # a source below the gates does not count
    iso = js("""const t = U.build([F('mzmine/tabella_pos.csv'), F('gnps/biblioteca.tsv')], {pol: 1});
      const mk = (id, rt) => { const f = {key: 'POS:' + id, id, pol: 1, rt, mz: 222.1, ann: [{source: 'GNPS2', type: 'MQScore', score: 0.9, shared: 8, ppm: 0, name: 'Fictazine', cleanName: 'Fictazine', ik: 'IIIIIIIIIIIIII', notes: []}]}; return f; };
      const tt = {features: [mk('10', 5.0), mk('11', 8.0), mk('12', 5.1)]}; const m = U.assessAllExternal(tt); return [...m].map(([k, v]) => [k, v.level, v.base])""")
    assert iso == [["POS:10", "3", "isomers not discriminated"], ["POS:11", "3", "isomers not discriminated"], ["POS:12", "3", "isomers not discriminated"]]
    same = js("""const mk = (id, rt) => ({key: 'POS:' + id, id, pol: 1, rt, mz: 1, ann: [{source: 'GNPS2', type: 'MQScore', score: 0.9, shared: 8, ppm: 0, name: 'F', cleanName: 'F', ik: 'IIIIIIIIIIIIII', notes: []}]});
      const m = U.assessAllExternal({features: [mk('1', 5.0), mk('2', 5.15)]}); return [...m.values()].map(v => v.level)""")
    assert same == ["2a", "2a"]          # inside the RT tolerance of the group


# ------------------------------------------------------------ parity with the synthetic campaign of the data repository (only with MZLAB_DATI)
def _campaign():
    for c in (os.environ.get("MZLAB_DATI"), os.environ.get("QQQ_DATI"), str(ROOT.parent / "mzlab-dati"), "/home/user/mzlab-dati"):
        if c:
            p = Path(c) / "scratchpad" / "da_copiare" / "attributio_synthetic_campaign"
            if (p / "livelli_attesi.csv").is_file():
                return p
    return None


@pytest.mark.skipif(_campaign() is None, reason="MZLAB_DATI with scratchpad/da_copiare/attributio_synthetic_campaign not available")
def test_parity_with_the_synthetic_campaign_for_features_decided_by_mzmine_and_gnps2():
    c = _campaign()
    gold = {(r["_pol"], r["_id"]): r for r in csv.DictReader((c / "livelli_attesi.csv").open(encoding="utf-8"), delimiter=";")}
    d = c / "02 Data" / "Data Synth"
    out = {}
    for pol, tab, ann, mgf, gn, net in (("POS", "merged_pos.csv", "merged_pos_annotations.csv", "merged_pos_gnps.mgf", "GNPS2_output_pos", True), ("NEG", "merged_neg.csv", None, None, None, False)):
        m = d / "1 - mzMine" / f"DDA_{pol}" / "merged"
        files = [(tab, m / tab)] + ([(ann, m / ann)] if ann else []) + ([(mgf, m / mgf)] if mgf else [])
        if gn:
            g = d / "3 - GNPS" / gn / "nf_output"
            files += [("merged_results_with_gnps.tsv", g / "library" / "merged_results_with_gnps.tsv"), ("clustersummary_with_network.tsv", g / "networking" / "clustersummary_with_network.tsv")]
        spec = json.dumps([[n, str(p)] for n, p in files])
        out[pol] = js(f"""const files = {spec}.map(([n, p]) => ({{ name: n, text: fs.readFileSync(p, 'utf8') }})); const t = U.build(files, {{ pol: {1 if pol == 'POS' else -1} }});
          const m = U.assessAllExternal(t); return Object.fromEntries([...m].map(([k, v]) => [k.split(':')[1], [v.level, v.base]]))""")
    # features whose gold level rests only on mzMine / GNPS2 library sources (the others need the standards, the chemist's decisions or SIRIUS: another package)
    only = {("POS", "3"), ("POS", "6"), ("POS", "9"), ("POS", "10"), ("POS", "11"), ("POS", "12"), ("NEG", "2")}
    for pol, fid in sorted(only):
        g = gold[(pol, fid)]
        assert out[pol][fid] == [g["level"] + g["level_sub"], g["level_basis"]], (pol, fid, out[pol][fid], g)
