"""Unit tests for the reference lists engine (mzlab/web/liste.js and chem/contaminants.py):
- Nominal search (unit resolution, ±0.5 Da, ordered by proximity) vs HR search (ppm).
- CSV parsing (comma, semicolon, Italian decimal comma, NORMAN Suspect List format).
- SMILES to molecular formula via OpenChemLib.
- ESI adduct differences (Huang et al. 1999).
- Guo et al. 2006 solvent clusters.
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from mzlab.chem.contaminants import builtin, expand  # noqa: E402
from mzlab.chem.elements import ADDUCT_SHIFT, ELECTRON, mass, parse_formula  # noqa: E402

NODE = shutil.which("node")


def test_cluster_solventi_keller():
    b = builtin()
    items = b["items"]
    # the built-in list comes from Keller 2008 only
    assert "Keller" in b["source"] and "Guo" not in b["source"]
    assert all("Keller 2008" in x.get("source", "") for x in items)

    # Verify methanol clusters: [MeOH+H]+ ~33.0335, [2MeOH+H]+ ~65.0597, [MeOH+Na]+ ~55.0154
    meoh = [x for x in items if x["id"] == "metanolo"]
    assert any(x["adduct"] == "[M+H]+" and abs(x["mz"] - 33.0335) < 1e-3 for x in meoh)
    assert any(x["adduct"] == "[M+H]+" and abs(x["mz"] - 65.0597) < 1e-3 for x in meoh)
    assert any(x["adduct"] == "[M+Na]+" and abs(x["mz"] - 55.0154) < 1e-3 for x in meoh)

    # Verify TFA clusters in negative mode: [2TFA-H]- ~226.9785, [2TFA+Na-2H]- ~248.9604
    tfa_neg = [x for x in items if x["id"] == "tfa_cluster_neg"]
    assert any(x["adduct"] == "[M-H]-" and abs(x["mz"] - 226.9785) < 1e-3 for x in tfa_neg)
    assert any(x["adduct"] == "[M+Na-2H]-" and abs(x["mz"] - 248.9604) < 1e-3 for x in tfa_neg)


def test_differenze_addotti_huang1999():
    # Exact shifts calculated from elements.py
    na_h = ADDUCT_SHIFT["[M+Na]+"] - ADDUCT_SHIFT["[M+H]+"]
    nh4_h = ADDUCT_SHIFT["[M+NH4]+"] - ADDUCT_SHIFT["[M+H]+"]
    k_h = ADDUCT_SHIFT["[M+K]+"] - ADDUCT_SHIFT["[M+H]+"]
    k_na = ADDUCT_SHIFT["[M+K]+"] - ADDUCT_SHIFT["[M+Na]+"]
    acn_h = mass(parse_formula("C2H3N"))
    meoh_h = mass(parse_formula("CH4O"))

    assert na_h == pytest.approx(21.98194, abs=5e-5)
    assert nh4_h == pytest.approx(17.02655, abs=5e-5)
    assert k_h == pytest.approx(37.95588, abs=5e-5)
    assert k_na == pytest.approx(15.97394, abs=5e-5)
    assert acn_h == pytest.approx(41.02655, abs=5e-5)
    assert meoh_h == pytest.approx(32.02621, abs=5e-5)

    # ESI- differences:
    # [M+HCOO]- vs [M-H]-: diff = HCOOH = 46.00548
    # [M+CH3COO]- vs [M-H]-: diff = CH3COOH = 60.02113
    # [M+Cl]- vs [M-H]-: diff = HCl = 35.97668
    # [M+Na-2H]- vs [M-H]-: diff = Na - H = 21.98194
    diff_hcoo = ADDUCT_SHIFT["[M+HCOO]-"] - ADDUCT_SHIFT["[M-H]-"]
    diff_cl = ADDUCT_SHIFT["[M+Cl]-"] - ADDUCT_SHIFT["[M-H]-"]
    assert diff_hcoo == pytest.approx(46.00548, abs=5e-5)
    assert diff_cl == pytest.approx(35.97668, abs=5e-5)


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_list_engine_nominal_vs_ppm_and_csv(tmp_path):
    code = r"""
const L = require(process.argv[1]);
const items = JSON.parse(require('fs').readFileSync(process.argv[2], 'utf8'));
const idx = L.buildIndex([{ id: "keller", name: "Keller/Guo", items }]);

const out = {};

// 1. Nominal search vs ppm search
// Searching 445.0 at unit resolution (±0.5 Da): should find cyclic siloxane (445.1200) sorted by proximity
const nomHits = L.findNominal(idx, "positive", 445.0, 0.5);
out.nomHitNames = nomHits.map(x => x.name);
out.nomHitDiffs = nomHits.map(x => x.diffDa);

// HR search at 5 ppm around 445.0: siloxane (445.1200) is at ~270 ppm, so it must NOT be found at 5 ppm
out.hrMiss = L.find(idx, "positive", 445.0, 5).length;
// But at 445.1200 with 5 ppm it MUST be found
out.hrHit = L.find(idx, "positive", 445.1200, 5).length;

// 2. CSV parsing with semicolon and Italian decimal comma
const csvIta = "mz;name;formula;polarity;note\n150,5;Composto A;C6H6O4;+;fondo HPLC\n223,02;Acetato cluster;;-;\n";
out.parsedIta = L.csvParse(csvIta);

// 3. NORMAN Suspect List Exchange format parsing
const normanCsv = `Compound_Name,Molecular_Formula,Monoisotopic_Mass,Polarity,SMILES,Note
Atrazine,C8H14ClN5,215.0938,positive,CCNC1=NC(=NC(=N1)Cl)NC(C)C,pesticide
Bisphenol A,C15H16O2,228.1150,both,CC(C)(C1=CC=C(C=C1)O)C2=CC=C(C=C2)O,additive
`;
out.parsedNorman = L.csvParse(normanCsv);

// 4. User list generation
const uList = L.userList("usr1", "Mia Lista", "sospetti.csv", out.parsedNorman);
out.uListCount = uList.items.length;
out.uListMz0 = uList.items[0].mz;
out.uListName0 = uList.items[0].name;

console.log(JSON.stringify(out));
"""
    items = builtin()["items"]
    f = tmp_path / "items.json"
    f.write_text(json.dumps(items), encoding="utf-8")
    r = subprocess.run([NODE, "-e", code, str(ROOT / "mzlab/web/liste.js"), str(f)], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    o = json.loads(r.stdout)

    # Nominal search found siloxane
    assert any("polisilossano" in n for n in o["nomHitNames"])
    # HR search at 445.0 is 0 hits, but at 445.1200 is >= 1 hit
    assert o["hrMiss"] == 0
    assert o["hrHit"] >= 1

    # Italian CSV parsing
    assert len(o["parsedIta"]) == 2
    assert o["parsedIta"][0]["mz"] == 150.5
    assert o["parsedIta"][0]["name"] == "Composto A"
    assert o["parsedIta"][0]["polarity"] == 1
    assert o["parsedIta"][1]["mz"] == 223.02
    assert o["parsedIta"][1]["polarity"] == -1

    # NORMAN CSV parsing
    assert len(o["parsedNorman"]) == 2
    assert o["parsedNorman"][0]["name"] == "Atrazine"
    assert o["parsedNorman"][0]["mz"] == pytest.approx(215.0938, abs=1e-4)
    assert o["parsedNorman"][0]["formula"] == "C8H14ClN5"
    assert o["parsedNorman"][0]["smiles"] == "CCNC1=NC(=NC(=N1)Cl)NC(C)C"

    # User list structure
    assert o["uListCount"] == 2
    assert o["uListMz0"] == pytest.approx(215.0938, abs=1e-4)
    assert o["uListName0"] == "Atrazine"


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_smiles_to_formula_via_openchemlib():
    code = r"""
import('./mzlab/web/vendor/openchemlib.js').then(OCL => {
  const m = OCL.Molecule.fromSmiles('CC(=O)Oc1ccccc1C(=O)O'); // Aspirina
  const mf = m.getMolecularFormula();
  console.log(JSON.stringify({ formula: mf.formula, weight: mf.absoluteWeight }));
}).catch(err => {
  console.error(err);
  process.exit(1);
});
"""
    r = subprocess.run([NODE, "-e", code], cwd=str(ROOT), capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    o = json.loads(r.stdout)
    assert o["formula"] == "C9H8O4"
    assert o["weight"] == pytest.approx(180.04226, abs=1e-4)
