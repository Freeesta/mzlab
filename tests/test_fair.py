"""Open-data exports (mzlab/web/fair.js, TP_Mine/js/30_untarget_fair.js): MassBank 2.x record checked with the rules of the format (an independent validator written here), SIRIUS 6 `.ms`, GNPS2 FBMN files read back by the readers of the untargeted tables.
Hand-written examples in tests/data/fair/."""
import json
import re
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
WEB, DATA = ROOT / "mzlab" / "web", ROOT / "tests" / "data" / "fair"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

PRE = f"""const F = require({str(WEB / 'fair.js')!r}); const fs = require('fs');
const spec = (o = {{}}) => ({{ mz: [138.0662, 110.0713, 195.0877, 123.0553, 0], y: [100000, 12000, 30000, 8000, 0], prec: 195.0877, rt: 5.2, sid: 12, file: 'x.mzML', polarity: 1, ce: 35, nce: true, act: 'hcd', ...o }});
const cmp = (o = {{}}) => ({{ name: 'Caffeine', smiles: 'Cn1cnc2c1c(=O)n(C)c(=O)n2C', inchikey: 'RYYVLZVUVIJVGH-UHFFFAOYSA-N', formula: 'C8H10N4O2', mass: 194.080376, ...o }});
const opt = (o = {{}}) => ({{ contributor: 'Esempio_Lab', id: 'EX000001', authors: 'A. Author, B. Author, Example Lab', license: 'CC BY', instrument: 'Q Exactive', itype: 'LC-ESI-QFT', precType: '[M+H]+', date: new Date(2026, 0, 15), ...o }});
"""


def js(code):
    p = subprocess.run(["node", "-e", PRE + f"console.log(JSON.stringify((() => {{ {code} }})()))"], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


# ---------------------------------------------------------------- an independent validator of MassBank 2.x records (required fields, formats, consistency)
MASS = {"C": 12.0, "H": 1.00782503207, "N": 14.0030740048, "O": 15.99491461956, "S": 31.97207100, "P": 30.97376163, "F": 18.99840322, "Cl": 34.96885268, "Br": 78.9183376}
SHIFT = {"[M+H]+": 1.007276, "[M+Na]+": 22.989218, "[M+NH4]+": 18.033823, "[M+K]+": 38.963158, "[M-H2O+H]+": -17.00274, "[M-H]-": -1.007276, "[M+Cl]-": 34.969402, "[M+HCOO]-": 44.998201, "[M+CH3COO]-": 59.013851}
REQUIRED = ["ACCESSION", "RECORD_TITLE", "DATE", "AUTHORS", "LICENSE", "CH$NAME", "CH$COMPOUND_CLASS", "CH$FORMULA", "CH$EXACT_MASS", "CH$SMILES", "CH$IUPAC", "AC$INSTRUMENT", "AC$INSTRUMENT_TYPE",
            "AC$MASS_SPECTROMETRY", "MS$FOCUSED_ION", "PK$NUM_PEAK", "PK$PEAK"]
LICENSES = {"CC0", "CC BY", "CC BY-SA", "CC BY-NC", "CC BY-NC-SA", "CC BY-NC-ND"}


def formula_mass(f):
    return sum(MASS[e] * (int(n) if n else 1) for e, n in re.findall(r"([A-Z][a-z]?)(\d*)", f))


def validate_massbank(text):
    """Returns the list of problems (empty = valid)."""
    bad, lines = [], text.rstrip("\n").split("\n")
    if lines[-1] != "//":
        bad.append("does not end with //")
    if not lines[0].startswith("ACCESSION: "):
        bad.append("ACCESSION must be the first field")
    kv, peaks, in_pk = {}, [], False
    for ln in lines[:-1]:
        if in_pk:
            if not ln.startswith("  "):
                bad.append("peak line not indented"); continue
            peaks.append(ln.split())
        elif ln.startswith("PK$PEAK:"):
            in_pk = True; kv.setdefault("PK$PEAK", []).append(ln.split(":", 1)[1].strip())
        else:
            k, _, v = ln.partition(": ")
            if not v:
                bad.append(f"field without value: {ln}")
            kv.setdefault(k, []).append(v)
    for k in REQUIRED:
        if k not in kv:
            bad.append(f"missing {k}")
    if bad:
        return bad
    one = lambda k: kv[k][0]
    if not re.fullmatch(r"MSBNK-[A-Za-z0-9_]{1,32}-[A-Za-z0-9_]{1,64}", one("ACCESSION")):
        bad.append("ACCESSION format")
    if not re.fullmatch(r"\d{4}\.\d{2}\.\d{2}", one("DATE")):
        bad.append("DATE format")
    if one("LICENSE") not in LICENSES:
        bad.append("LICENSE")
    if not re.fullmatch(r"INCHIKEY [A-Z]{14}-[A-Z]{10}-[A-Z]", kv.get("CH$LINK", ["INCHIKEY X"])[0]):
        bad.append("InChIKey")
    ms = " ".join(kv["AC$MASS_SPECTROMETRY"])
    if "MS_TYPE MS2" not in ms or not re.search(r"ION_MODE (POSITIVE|NEGATIVE)", ms):
        bad.append("MS_TYPE / ION_MODE")
    fi = " ".join(kv["MS$FOCUSED_ION"])
    m = re.search(r"PRECURSOR_M/Z ([\d.]+)", fi); t = re.search(r"PRECURSOR_TYPE (\S+)", fi)
    if not m or not t or t.group(1) not in SHIFT:
        bad.append("MS$FOCUSED_ION")
    else:
        if abs(float(one("CH$EXACT_MASS")) - formula_mass(one("CH$FORMULA"))) > 0.001:
            bad.append("EXACT_MASS does not match the formula")
        if abs(float(one("CH$EXACT_MASS")) + SHIFT[t.group(1)] - float(m.group(1))) > 0.01:
            bad.append("precursor does not match mass + adduct")
        if (t.group(1).endswith("-")) != ("NEGATIVE" in ms):
            bad.append("precursor type against ion mode")
    if int(one("PK$NUM_PEAK")) != len(peaks):
        bad.append("NUM_PEAK")
    if one("PK$PEAK") != "m/z int. rel.int.":
        bad.append("PEAK header")
    mzs = [float(p[0]) for p in peaks]
    if mzs != sorted(mzs) or any(len(p) != 3 or not (1 <= int(p[2]) <= 999) for p in peaks):
        bad.append("peak lines")
    return bad


def test_hand_written_example_is_valid_for_the_validator():
    assert validate_massbank((DATA / "massbank_esempio.txt").read_text()) == []


def test_validator_catches_problems():
    ex = (DATA / "massbank_esempio.txt").read_text()
    assert validate_massbank(ex.replace("PK$NUM_PEAK: 4", "PK$NUM_PEAK: 5"))
    assert validate_massbank(ex.replace("LICENSE: CC BY", "LICENSE: free"))
    assert validate_massbank(ex.replace("CH$FORMULA: C8H10N4O2", "CH$FORMULA: C8H10N4O3"))
    assert validate_massbank(ex.replace("\n//\n", "\n"))
    assert validate_massbank(ex.replace("CH$SMILES: Cn1cnc2c1c(=O)n(C)c(=O)n2C\n", ""))


def test_record_from_spectrum_follows_the_format_and_the_hand_written_example():
    txt = js("return F.record(spec(), cmp(), opt())")
    assert validate_massbank(txt) == []
    ex = (DATA / "massbank_esempio.txt").read_text()
    keys = lambda t: [ln.split(":")[0] for ln in t.split("\n") if ln and not ln.startswith("  ")]
    got, want = keys(txt), keys(ex)
    assert [k for k in want if k not in got] == []            # every field of the example is written
    for k in ("ACCESSION", "DATE", "AUTHORS", "LICENSE", "CH$NAME", "CH$FORMULA", "CH$SMILES", "AC$INSTRUMENT_TYPE", "PK$NUM_PEAK"):
        a = [ln for ln in txt.split("\n") if ln.startswith(k + ":")]
        b = [ln for ln in ex.split("\n") if ln.startswith(k + ":")]
        assert a == b, k
    assert "  138.0662 100000.0 999" in txt and "  195.0877 30000.0 300" in txt          # the zero peak is dropped, the base peak is 999
    assert "COLLISION_ENERGY 35 % (nominal)" in txt and "FRAGMENTATION_MODE HCD" in txt and "RETENTION_TIME 5.200 min" in txt


def test_negative_and_ev_and_adduct():
    txt = js("return F.record(spec({polarity: -1, prec: 193.0731, nce: false, ce: 20, act: ''}), cmp(), opt({precType: '[M-H]-'}))")
    assert validate_massbank(txt) == [] and "ION_MODE NEGATIVE" in txt and "COLLISION_ENERGY 20 eV" in txt and "FRAGMENTATION_MODE" not in txt


def test_check_blocks_missing_fields_and_lr():
    r = js("return { lr: F.check(spec({lr: true}), cmp(), opt()).errors.map(e => e.key), none: F.check(spec(), cmp({name: '', inchikey: 'bad', smiles: ''}), opt({authors: '', license: 'x', contributor: 'a b'})).errors.map(e => e.key),"
           " warn: F.check(spec({prec: 250}), cmp(), opt()).warnings.map(e => e.key), pol: F.check(spec({polarity: -1}), cmp(), opt()).errors.map(e => e.key), thrown: (() => { try { F.record(spec(), cmp({name: ''}), opt()); return false; } catch (e) { return e.errors.map(x => x.key); } })() }")
    assert r["lr"] == ["lr"]
    assert set(r["none"]) >= {"name", "smiles", "inchikey", "authors", "license", "contributor"}
    assert "precMismatch" in r["warn"] and r["pol"] == ["polarity"] and r["thrown"] == ["name"]


def test_instrument_type():
    r = js("return [F.instrumentType(['orbitrap']), F.instrumentType(['quadrupole','orbitrap']), F.instrumentType(['quadrupole','time-of-flight']), F.instrumentType([])]")
    assert r == ["LC-ESI-ITFT", "LC-ESI-QFT", "LC-ESI-QTOF", "LC-ESI-"]


# ---------------------------------------------------------------- SIRIUS .ms
def parse_ms(text):
    out, cur = [], None
    for ln in text.splitlines():
        if not ln.strip() or ln.startswith("#"):
            continue
        if ln.startswith(">"):
            k, _, v = ln[1:].partition(" ")
            if k == "compound":
                cur = {"name": v, "ms1": [], "ms2": [], "_at": None}; out.append(cur)
            elif k in ("ms1", "ms2", "collision"):
                cur["_at"] = cur["ms1"] if k == "ms1" else []
                if k != "ms1":
                    cur["ms2"].append(cur["_at"]); cur.setdefault("ce", []).append(v)
            else:
                cur[k] = v
        else:
            a, b = ln.split(); cur["_at"].append((float(a), float(b)))
    return out


def test_sirius_ms_roundtrip_with_hand_written_example():
    ex = parse_ms((DATA / "sirius_esempio.ms").read_text())[0]
    txt = js("return F.siriusMs([{name: 'example feature', mass: 195.0877, ion: '[M+H]+', formula: 'C8H10N4O2', rt: 5.2, ms1: {mz: [195.0877, 196.091], it: [100000, 9000]}, ms2: [{mz: [110.0713, 138.0662, 0], it: [12000, 100000, 0], ce: 35}]}, {name: 'no ms2', mass: 100, ms2: []}])")
    got = parse_ms(txt)
    assert len(got) == 1
    g = got[0]
    for k in ("parentmass", "ionization", "formula", "rt", "ce"):
        assert g[k] == ex[k], k
    assert g["ms1"] == ex["ms1"] and g["ms2"] == ex["ms2"] and g["name"] == "example_feature"


def test_sirius_without_formula_and_without_energy():
    g = parse_ms(js("return F.siriusMs([{name: 'a', mass: 301.1, pol: -1, ms2: [{mz: [100], it: [5], ce: null}]}])"))[0]
    assert "formula" not in g and g["ionization"] == "[M-H]-" and "ce" in g



# ---------------------------------------------------------------- GNPS2 FBMN / IIMN (TP_Mine/js/30_untarget_fair.js), read back by the readers of MATCH_UNTARGETED
UD = ROOT / "tests" / "data" / "untargeted"
PRE2 = f"""globalThis.window = globalThis; globalThis.FAIR = require({str(WEB / 'fair.js')!r});
const U = require({str(ROOT / 'TP_Mine' / 'js' / '25_untarget.js')!r}); const X = require({str(ROOT / 'TP_Mine' / 'js' / '28_untarget_export.js')!r}); const N = require({str(ROOT / 'TP_Mine' / 'js' / '30_untarget_fair.js')!r});
const fs = require('fs'); const rd = p => fs.readFileSync({str(UD)!r} + '/' + p, 'utf8');
const T = () => U.build([{{ name: 'fbmn_quant.csv', text: rd('mzmine/fbmn_quant.csv') }}, {{ name: 'fbmn.mgf', text: rd('mzmine/fbmn.mgf') }}, {{ name: 'fbmn_edges.csv', text: rd('mzmine/fbmn_edges.csv') }}], {{ pol: 1 }});
"""


def js2(code):
    p = subprocess.run(["node", "-e", PRE2 + f"console.log(JSON.stringify((() => {{ {code} }})()))"], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


def test_gnps_package_is_read_back_by_the_untargeted_readers():
    r = js2("""const t = T(), fs_ = t.features; t.edges.push({ a: '1', b: '5', type: 'MS1 annotation', score: 0.9, annotation: '[M+H]+ [M+Na]+ dm/z=21.98' }); const g = N.gnps(t, fs_); const names = g.map(x => x[0]);
      const t2 = U.build(g.map(([n, text]) => ({ name: n === 'quant.csv' ? 'fbmn_quant.csv' : n === 'ms2.mgf' ? 'fbmn.mgf' : 'fbmn_edges.csv', text })), { pol: 1 });
      const brief = x => x.features.map(f => [f.id, f.mz, f.rt, f.areas, f.group || null, f.iin ? f.iin.id : null, f.ms2 ? [f.ms2.mz, f.ms2.it] : null]);
      return { names, a: brief(t), b: brief(t2), ea: t.edges.map(e => [e.a, e.b, e.type, e.score]), eb: t2.edges.map(e => [e.a, e.b, e.type, e.score]), samples: [t.samples, t2.samples], mgf: g[1][1] };""")
    assert r["names"] == ["quant.csv", "ms2.mgf", "iin_edges.csv"]
    assert r["a"] == r["b"] and r["samples"][0] == r["samples"][1]
    assert r["eb"] == [["1", "5", "MS1 annotation", 0.9]]          # only IIN edges between exported features travel: cosine edges are the work of GNPS2, the edge to 6 has no row
    assert not any(ln.startswith(("NAME=", "LEVEL=", "LIBRARY")) for ln in r["mgf"].split("\n")) and r["mgf"].count("FEATURE_ID=") == r["mgf"].count("BEGIN IONS")


def test_quant_header_and_rows_follow_the_mzmine_format():
    q = js2("return N.quant(T(), T().features)")
    lines = q.strip().split("\n")
    assert lines[0].split(",")[:7] == ["row ID", "row m/z", "row retention time", "correlation group ID", "annotation network number", "best ion", "neutral M mass"]
    assert lines[0].endswith("Campione_A_pos.mzML Peak area,Campione_B_pos.mzML Peak area") and lines[1].startswith("1,184.1332,5.2,3,7,[M+H]+,183.1259,100000,0")


def test_subset_edges_and_no_ms2():
    r = js2("const t = T(); const sub = t.features.filter(f => f.id === '1'); return { e: N.edges(t, sub), m: N.mgf(t, t.features.filter(f => !f.ms2)), s: N.sirius(t, sub) };")
    assert r["e"] == "" and r["m"] == ""                                   # an edge needs both ends; no MS2 = no block
    assert ">compound feature_" in r["s"] and ">parentmass 184.1332" in r["s"]
