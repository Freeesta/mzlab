"""Integration and unit tests for the HRMS target screening module (mzlab/web/liste-target.js).

Tests:
- Parsing of psychoactive substances list (sostanze_hrms.csv).
- Target screening engine (XIC extraction, peak detection, S/N threshold, M+1 isotope pattern scoring, MS2 precursor matching).
- Identification proposal assignment: Prev. 1 (MS1 + isotope + MS2), Prev. 2 (MS1 + isotope), Prev. 3 (accurate mass).
- Sorting by proposal rank (Prev. 1 > Prev. 2 > Prev. 3) and descending S/N.
- Decoy verification (zero false positives on noise-only spectra).
- Provenance CSV export format (# mzLab Target screening export...).
- Connection with LISTE engine (LISTE.openTargetScreening, LISTE.target).
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

NODE = shutil.which("node")
TARGET_JS = ROOT / "mzlab" / "web" / "liste-target.js"
LISTE_JS = ROOT / "mzlab" / "web" / "liste.js"
CSV_PATH = ROOT / "tests" / "data" / "liste" / "sostanze_hrms.csv"


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_parse_target_csv():
    code = r"""
const fs = require('fs');
const LT = require(process.argv[1]);
const csvText = fs.readFileSync(process.argv[2], 'utf8');

const entries = LT.parseTargetCsv(csvText);
const out = {
  count: entries.length,
  dmaa: entries.find(x => x.id === '1-3-dimethylamylamine'),
  lsd: entries.find(x => x.id === '1p-lsd'),
  pseudo: entries.find(x => x.id === '1s-2s-pseudoephedrine')
};
console.log(JSON.stringify(out));
"""
    res = subprocess.run([NODE, "-e", code, str(TARGET_JS), str(CSV_PATH)], capture_output=True, text=True, encoding="utf-8", check=True)
    data = json.loads(res.stdout)

    assert data["count"] == 645
    assert data["dmaa"]["name_en"] == "DMAA"
    assert data["dmaa"]["neutral_formula"] == "C7H17N"
    assert abs(data["dmaa"]["monoisotopic_mass"] - 115.1361) < 1e-4
    assert abs(data["dmaa"]["mz_mh"] - 116.143376) < 1e-4
    assert abs(data["dmaa"]["mz_mna"] - 138.12532) < 1e-4

    assert data["lsd"]["name_en"] == "1P-LSD"
    assert data["lsd"]["neutral_formula"] == "C23H29N3O2"

    assert data["pseudo"]["name_en"] == "Pseudoephedrine"
    assert data["pseudo"]["neutral_formula"] == "C10H15NO"


def _run_screening_js():
    code = r"""
const LT = require(process.argv[1]);
const targets = [
  { id: 'pseudo', name_en: 'Pseudoephedrine', neutral_formula: 'C10H15NO', monoisotopic_mass: 165.115364, mz_mh: 166.122641 },
  { id: 'dmaa', name_en: 'DMAA', neutral_formula: 'C7H17N', monoisotopic_mass: 115.1361, mz_mh: 116.143376 },
  { id: 'test', name_en: 'TestComp', neutral_formula: 'C8H10', monoisotopic_mass: 106.07825, mz_mh: 107.085526 }
];
const spectra = [];
for (let i = 0; i < 61; i++) {
  const rt = 1.0 + i * 0.1;
  const mz = [115.05, 350.20, 580.45], intensity = [10.0, 8.0, 12.0];
  const i1 = 1000.0 * Math.exp(-0.5 * Math.pow((rt - 2.5) / 0.1, 2));
  if (i1 >= 5.0) { mz.push(166.122641, 166.122641 + 1.003355); intensity.push(i1, i1 * 0.1117); }
  const i2 = 800.0 * Math.exp(-0.5 * Math.pow((rt - 4.0) / 0.1, 2));
  if (i2 >= 5.0) { mz.push(116.143376, 116.143376 + 1.003355); intensity.push(i2, i2 * 0.0793); }
  const i3 = 300.0 * Math.exp(-0.5 * Math.pow((rt - 5.2) / 0.1, 2));
  if (i3 >= 5.0) { mz.push(107.085526); intensity.push(i3); }
  spectra.push({ level: 1, rt, mz, intensity });
}
spectra.push({ level: 2, rt: 2.51, precursor: 166.122641, mz: [148.11, 133.09], intensity: [500.0, 200.0] });
const matches = LT.cercaTargets(targets, spectra, { tolerance_ppm: 5.0, min_sn: 3.0, min_area: 10.0, adducts: ['[M+H]+'] });
console.log(JSON.stringify(matches));
"""
    res = subprocess.run([NODE, "-e", code, str(TARGET_JS)], capture_output=True, text=True, encoding="utf-8", check=True)
    return json.loads(res.stdout)


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_screening_engine_prev1_prev2():
    matches = _run_screening_js()
    assert len(matches) == 3
    assert matches[0]["proposal"] == "Prev. 1"
    assert matches[0]["id"] == "pseudo"
    assert matches[0]["ms2_matched"] is True
    assert matches[0]["isotope_score"] is not None and matches[0]["isotope_score"] >= 0.7
    assert "MS2 precursor matched" in matches[0]["motif"]
    assert "isotope pattern confirmed" in matches[0]["motif"]
    assert matches[0]["neutral_formula"] == "C10H15NO"

    assert matches[1]["proposal"] == "Prev. 2"
    assert matches[1]["id"] == "dmaa"
    assert matches[1]["ms2_matched"] is False
    assert matches[1]["isotope_score"] is not None and matches[1]["isotope_score"] >= 0.7
    assert "no MS2 precursor matched" in matches[1]["motif"]
    assert "isotope pattern confirmed" in matches[1]["motif"]


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_screening_engine_prev3_and_sort():
    matches = _run_screening_js()
    assert matches[2]["proposal"] == "Prev. 3"
    assert matches[2]["id"] == "test"
    assert matches[2]["ms2_matched"] is False
    assert "isotope pattern unconfirmed" in matches[2]["motif"]


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_decoy_verification_zero_false_positives():
    code = r"""
const LT = require(process.argv[1]);

// 10 targets with diverse masses
const targets = [
  { id: 't1', name_en: 'Target 1', neutral_formula: 'C10H15N', monoisotopic_mass: 149.1204, mz_mh: 150.1277 },
  { id: 't2', name_en: 'Target 2', neutral_formula: 'C12H18O2', monoisotopic_mass: 194.1307, mz_mh: 195.1380 },
  { id: 't3', name_en: 'Target 3', neutral_formula: 'C15H21NO', monoisotopic_mass: 231.1623, mz_mh: 232.1696 }
];

// Pure background spectra with random noise ions distinct from targets
const spectra = [];
for (let i = 0; i < 30; i++) {
  spectra.push({
    level: 1,
    rt: 1.0 + i * 0.1,
    mz: [78.05, 112.08, 175.05, 301.20, 420.15],
    intensity: [15.0, 22.0, 18.0, 30.0, 25.0]
  });
}

const matches = LT.cercaTargets(targets, spectra, {
  tolerance_ppm: 5.0,
  min_sn: 3.0,
  min_area: 100.0,
  adducts: ['[M+H]+']
});

console.log(JSON.stringify(matches));
"""
    res = subprocess.run([NODE, "-e", code, str(TARGET_JS)], capture_output=True, text=True, encoding="utf-8", check=True)
    matches = json.loads(res.stdout)
    assert len(matches) == 0


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_csv_export_format():
    code = r"""
const LT = require(process.argv[1]);

const sampleMatches = [
  {
    id: 'dmaa',
    name_en: 'DMAA',
    neutral_formula: 'C7H17N',
    adduct: '[M+H]+',
    target_mz: 116.143376,
    observed_mz: 116.143250,
    ppm_error: -1.08,
    rt_apex: 3.2500,
    rt_start: 3.1000,
    rt_end: 3.4000,
    height: 1250.0,
    area: 4500.0,
    sn_ratio: 18.5,
    isotope_score: 0.95,
    ms2_matched: true,
    proposal: 'Prev. 1',
    motif: 'Peak at RT 3.25 min, S/N 18.5, mass error -1.1 ppm, isotope pattern confirmed (score 0.95), MS2 precursor matched'
  }
];

const csv = LT.csvExport(sampleMatches);
console.log(csv);
"""
    res = subprocess.run([NODE, "-e", code, str(TARGET_JS)], capture_output=True, text=True, encoding="utf-8", check=True)
    csv_text = res.stdout.strip()

    assert csv_text.startswith("# mzLab Target screening export")
    lines = csv_text.splitlines()
    header_cols = lines[1].split(",")
    assert "neutral_formula" in header_cols
    assert "formula" not in header_cols  # Rule: never bare "formula"
    assert "proposal" in header_cols
    assert "motif" in header_cols

    row = lines[2]
    assert "DMAA" in row
    assert "Prev. 1" in row
    assert "C7H17N" in row


@pytest.mark.skipif(not NODE, reason="node non disponibile")
def test_liste_target_integration():
    code = r"""
const L = require(process.argv[1]);
const out = {
  hasOpenTarget: typeof L.openTargetScreening === 'function',
  hasTargetMethod: typeof L.target === 'function'
};
console.log(JSON.stringify(out));
"""
    res = subprocess.run([NODE, "-e", code, str(LISTE_JS)], capture_output=True, text=True, encoding="utf-8", check=True)
    out = json.loads(res.stdout)
    assert out["hasOpenTarget"] is True
    assert out["hasTargetMethod"] is True
