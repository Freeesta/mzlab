"""Untargeted export (TP_Mine/js/28_untarget_export.js): CSV with «;» (UTF-8 with BOM, quoting), MGF of the features with level and the licences of the cited library records, the sheets of the xlsx
(features, evidence, records, parameters), key = (polarity, id) plus the SHA-256 of the table."""
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
require({str(WEB / 'livelli-1.js')!r}); require({str(WEB / 'livelli.js')!r}); const U = require({str(JS / '25_untarget.js')!r}); const M = require({str(JS / '26_untarget_match.js')!r}); const X = require({str(JS / '28_untarget_export.js')!r});
const fs = require('fs'); const rd = p => fs.readFileSync({str(DATA)!r} + '/' + p, 'utf8');
const cand = (o = {{}}) => ({{ name: 'Alpha; "beta"', ik: 'AAAAAAAAAAAAAA-X', ent: 0.912345, cos: 0.85, shared: 8, dprec: 1.234, nlib: 30, lib: 'LIB', accession: 'MSBNK-1', authors: 'A. Author', license: 'CC BY 4.0', smiles: 'CCO', ...o }});
const T = () => {{ const t = U.build([{{ name: 'tabella_pos.csv', text: rd('mzmine/tabella_pos.csv') }}, {{ name: 'fbmn.mgf', text: rd('mzmine/fbmn.mgf') }}, {{ name: 'biblioteca.tsv', text: rd('gnps/biblioteca.tsv') }}], {{ pol: 1 }}); t.file.sha256 = 'ab'.repeat(32);
  const res = new Map(); t.features.forEach((f, i) => res.set(f.key, M.assessRow(f, f.ms2 ? cand({{ name: i === 0 ? 'Alpha; "beta"' : 'Gamma', accession: 'MSBNK-' + (i + 1), license: i === 0 ? 'CC BY 4.0' : '' }}) : null))); return {{ t, res }}; }};
"""


def js(code):
    p = subprocess.run(["node", "-e", PRE + f"console.log(JSON.stringify((() => {{ {code} }})()))"], check=True, capture_output=True, encoding="utf-8")
    return json.loads(p.stdout)


def test_csv_semicolon_bom_quoting_key_and_areas():
    r = js("const { t, res } = T(); const c = X.csv(t, res, t.features); return { c, n: t.features.length }")
    c = r["c"]
    assert c.startswith("﻿") and c.endswith("\r\n")
    lines = c.lstrip("﻿").split("\r\n")[:-1]
    assert len(lines) == r["n"] + 1
    head = lines[0].split(";")
    assert head[:6] == ["key", "polarity", "feature_id", "mz", "rt_min", "level"] and head[-2:] == ["Campione_A_pos", "Campione_B_pos"] and "table_sha256" in head
    row1 = lines[1]
    assert row1.startswith("POS:1;POS;1;184.1332;5.2;") and '"Alpha; ""beta"""' in row1          # quoted: the name has «;» and quotes
    assert row1.count(";") >= len(head) - 1
    assert ("ab" * 32) in row1 and "CC BY 4.0" in row1 and row1.endswith(";100000;")
    row5 = next(x for x in lines if x.startswith("POS:5;"))
    assert "MSBNK-5" in row5 and "not stated" in row5          # a record without licence says so


def test_mgf_has_levels_ms2_only_and_the_licences_in_the_comments():
    m = js("const { t, res } = T(); return X.mgf(t, res, t.features)")
    assert m.startswith("# Tabui untargeted export") and "SHA-256 " + "ab" * 32 in m
    assert "# Library records cited (" in m and "CC BY 4.0: 1 record" in m and "not stated: " in m
    blocks = m.count("BEGIN IONS")
    assert blocks == 2 and m.count("END IONS") == 2          # features 1 and 5 have an MS2
    b1 = m.split("BEGIN IONS")[1]
    assert "FEATURE_ID=1\n" in b1 and "PEPMASS=184.1332" in b1 and "CHARGE=1+" in b1 and "RTINSECONDS=312.0" in b1 and "LEVEL=" in b1 and "LIBRARY_LICENSE=CC BY 4.0" in b1 and "56.05 120.5" in b1


def test_sources_are_unique_and_counted():
    s = js("const { t, res } = T(); const f2 = t.features.filter(f => f.ms2); res.get(f2[1].key).accession = 'MSBNK-1'; res.get(f2[1].key).name = 'Alpha; \"beta\"'; return X.sources(res, t.features)")
    assert [x["accession"] for x in s][0] == "MSBNK-1" and s[0]["features"] == 2 and s[0]["license"] == "CC BY 4.0"


def test_sheets_and_a_valid_xlsx():
    r = js("""const { t, res } = T(); const sh = X.sheets(t, res, t.features, { tol: 10 }); const vm = require('vm'); vm.runInThisContext(fs.readFileSync(%r, 'utf8') + '; globalThis.xlsxBuild = xlsxBuild;');
      const u8 = xlsxBuild(sh); return { names: sh.map(s => s.name), n: sh[0].rows.length, w: sh[0].head.length, rw: sh[0].rows[0].length, ev: sh[1].rows.length, par: Object.fromEntries(sh[3].rows), pk: [u8[0], u8[1]], size: u8.length,
        txt: Buffer.from(u8).toString('latin1').includes('Library records') }""" % str(WEB / "xlsx.js"))
    assert r["names"] == ["Features", "Evidence", "Library records", "Parameters"] and r["n"] == 6 and r["w"] == r["rw"]
    assert r["ev"] > 6 and r["par"]["table SHA-256"] == "ab" * 32 and r["par"]["precursor tolerance (ppm)"] == 10 and "never identifications" in r["par"]["note"]
    assert r["pk"] == [0x50, 0x4B] and r["size"] > 500 and r["txt"]


def test_base_name_has_no_path_or_odd_characters():
    b = js("return X.base({ file: { name: 'Mia tabella (pos) è.csv' } })")
    assert b.startswith("untargeted_Mia_tabella_pos_") and "/" not in b and "." not in b.split("_pos_")[1][:3]
