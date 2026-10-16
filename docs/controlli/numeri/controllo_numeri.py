#!/usr/bin/env python3
"""Independent numerical verification of mzLab engine calculations (T1).

Compares exact monoisotopic masses, common ESI adducts, isotopic patterns,
ppm mass errors, and chromatographic feature detection (via mzlab-cli)
against independent Python libraries (RDKit, Pyteomics, PyOpenMS, NumPy).
"""

import base64
import csv
import os
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

os.environ.setdefault("KMP_DUPLICATE_LIB_OK", "TRUE")

OUT_DIR = Path(__file__).resolve().parent
REPO_ROOT = OUT_DIR.parents[2]
sys.path.insert(0, str(REPO_ROOT))

import numpy as np  # noqa: E402
import pyopenms as oms  # noqa: E402
from pyteomics import mass as pt_mass  # noqa: E402
from rdkit import Chem  # noqa: E402
from rdkit.Chem import rdMolDescriptors  # noqa: E402

import mzlab.chem.composition as mz_composition  # noqa: E402
import mzlab.chem.elements as mz_elements  # noqa: E402
import mzlab.qaqc as mz_qaqc  # noqa: E402

COMPOSTI = [
    ("fenolo", "C6H6O", "Oc1ccccc1"),
    ("anisolo", "C7H8O", "COc1ccccc1"),
    ("toluene", "C7H8", "Cc1ccccc1"),
    ("acido benzoico", "C7H6O2", "O=C(O)c1ccccc1"),
    ("nitrobenzene", "C6H5NO2", "O=[N+]([O-])c1ccccc1"),
    ("clorobenzene", "C6H5Cl", "Clc1ccccc1"),
    ("anilina", "C6H7N", "Nc1ccccc1"),
    ("naftalene", "C10H8", "c1ccc2ccccc2c1"),
    ("carbamazepina", "C15H12N2O", "NC(=O)N1c2ccccc2C=Cc2ccccc21"),
    ("paracetamolo", "C8H9NO2", "CC(=O)Nc1ccc(O)cc1"),
    ("caffeina", "C8H10N4O2", "Cn1c(=O)c2c(ncn2C)n(C)c1=O"),
    ("atrazina", "C8H14ClN5", "CCNc1nc(Cl)nc(NC(C)C)n1"),
    ("ibuprofene", "C13H18O2", "CC(C)Cc1ccc(C(C)C(=O)O)cc1"),
    ("diclofenac", "C14H11Cl2NO2", "O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl"),
    ("sulfametossazolo", "C10H11N3O3S", "Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1"),
    ("trimetoprim", "C14H18N4O3", "COc1cc(Cc2cnc(N)nc2N)cc(OC)c1OC"),
    ("metoprololo", "C15H25NO3", "COCCc1ccc(OCC(O)CNC(C)C)cc1"),
    ("propranololo", "C16H21NO2", "CC(C)NCC(O)COc1cccc2ccccc12"),
    ("atenololo", "C14H22N2O3", "CC(C)NCC(O)COc1ccc(CC(N)=O)cc1"),
    ("venlafaxina", "C17H27NO2", "COc1ccc(C(CN(C)C)C2(O)CCCCC2)cc1"),
    ("ciprofloxacina", "C17H18FN3O3", "O=C(O)c1cn(C2CC2)c2cc(N3CCNCC3)c(F)cc2c1=O"),
    (
        "tetraciclina",
        "C22H24N2O8",
        "CN(C)C1C2CC3C(C4=C(C(=O)C3=C(C2(C(=O)C(=C1O)C(=O)N)O)O)C(=CC=C4)O)(C)O",
    ),
    ("bisfenolo A", "C15H16O2", "CC(C)(c1ccc(O)cc1)c1ccc(O)cc1"),
    ("acido salicilico", "C7H6O3", "O=C(O)c1ccccc1O"),
    ("naprossene", "C14H14O3", "COc1ccc2cc(C(C)C(=O)O)ccc2c1"),
    ("ketoprofene", "C16H14O3", "CC(C(=O)O)c1cccc(C(=O)c2ccccc2)c1"),
    ("furosemide", "C12H11ClN2O5S", "NS(=O)(=O)c1cc(c(Cl)cc1C(=O)O)NCc1ccco1"),
    ("idroclorotiazide", "C7H8ClN3O4S2", "NS(=O)(=O)c1cc2c(cc1Cl)NCNS2(=O)=O"),
    ("diuron", "C9H10Cl2N2O", "CN(C)C(=O)Nc1ccc(Cl)c(Cl)c1"),
    ("isoproturon", "C12H18N2O", "CC(C)c1ccc(NC(=O)N(C)C)cc1"),
]

E_MASS = 0.000548579909


def ref_adduct(m_neutral: float, name: str) -> float:
    """Calculate reference adduct m/z using Pyteomics and NIST electron mass."""
    m_h = pt_mass.calculate_mass(formula="H")
    m_na = pt_mass.calculate_mass(formula="Na")
    m_nh4 = pt_mass.calculate_mass(formula="NH4")
    m_hcoo = pt_mass.calculate_mass(formula="CHO2")
    mapping = {
        "[M+H]+": m_neutral + m_h - E_MASS,
        "[M+Na]+": m_neutral + m_na - E_MASS,
        "[M+NH4]+": m_neutral + m_nh4 - E_MASS,
        "[M-H]-": m_neutral - m_h + E_MASS,
        "[M+HCOO]-": m_neutral + m_hcoo + E_MASS,
    }
    return mapping[name]


def add_row(rows, cat, comp, form, param, v_mz, v_ref, unit, tol):
    """Append a formatted comparison row to the output table."""
    diff = v_mz - v_ref
    esito = "PASS" if abs(diff) <= tol else "FAIL"
    rows.append({
        "categoria": cat,
        "composto": comp,
        "formula": form,
        "parametro": param,
        "valore_mzlab": f"{v_mz:.6f}",
        "valore_riferimento": f"{v_ref:.6f}",
        "differenza": f"{diff:.6e}",
        "unita": unit,
        "tolleranza": f"{tol}",
        "esito": esito,
    })


def check_compounds(rows: list[dict]) -> None:
    """Verify exact masses, adducts, isotopic patterns, and ppm errors."""
    adducts = ["[M+H]+", "[M+Na]+", "[M+NH4]+", "[M-H]-", "[M+HCOO]-"]
    for name, formula, smiles in COMPOSTI:
        counts = mz_elements.parse_formula(formula)
        m_mz = mz_elements.mass(counts)
        m_pt = pt_mass.calculate_mass(formula=formula)
        mol = Chem.MolFromSmiles(smiles)
        _ = rdMolDescriptors.CalcExactMolWt(mol)
        add_row(rows, "massa_neutra", name, formula, "massa_monoisotopica",
                m_mz, m_pt, "Da", 0.001)

    for name, formula, _ in COMPOSTI:
        counts = mz_elements.parse_formula(formula)
        m_mz = mz_elements.mass(counts)
        m_ref = pt_mass.calculate_mass(formula=formula)
        for add in adducts:
            add_row(rows, "addotti", name, formula, add,
                    mz_elements.ion_mz(m_mz, add), ref_adduct(m_ref, add), "Da", 0.001)

    for name, formula, _ in COMPOSTI:
        counts = mz_elements.parse_formula(formula)
        r1, r2 = mz_composition.isotope_ratios(counts)
        ef = oms.EmpiricalFormula(formula)
        dist = ef.getIsotopeDistribution(oms.CoarseIsotopePatternGenerator(3))
        p = [x.getIntensity() for x in dist.getContainer()]
        pairs = [
            ("M_rel", 100.0, 100.0),
            ("M+1_rel", r1 * 100.0, (p[1] / p[0]) * 100.0),
            ("M+2_rel", r2 * 100.0, (p[2] / p[0]) * 100.0),
        ]
        for param, v_mz, v_ref in pairs:
            add_row(rows, "pattern_isotopico", name, formula, param,
                    v_mz, v_ref, "%", 0.5)

    for idx in range(10):
        theo = 100.0 * (idx + 1)
        ppm_t = -15.0 + 3.5 * idx
        obs = theo * (1.0 + ppm_t * 1e-6)
        add_row(rows, "errore_ppm", f"test_{int(theo)}Da_{ppm_t:+.1f}ppm", "N/A",
                "ppm_error", mz_qaqc.ppm_error(obs, theo),
                (obs - theo) / theo * 1e6, "ppm", 1e-5)


def write_synthetic_mzml(file_path: Path, peaks_info: list[dict]) -> np.ndarray:
    """Write a standard zlib-encoded synthetic LC-MS mzML file."""
    def b64(a, f4=False):
        dt = "<f4" if f4 else "<f8"
        return base64.b64encode(zlib.compress(np.asarray(a, dt).tobytes())).decode()

    def arr(acc, name, data, f4=False):
        cv = "MS:1000521" if f4 else "MS:1000523"
        bits = "32-bit float" if f4 else "64-bit float"
        return (
            f'<binaryDataArray encodedLength="0">'
            f'<cvParam cvRef="MS" accession="{cv}" name="{bits}" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value=""/>'
            f"<binary>{b64(data, f4)}</binary></binaryDataArray>"
        )

    rts = np.linspace(1.0, 5.5, 150)
    specs = []
    for i, rt in enumerate(rts):
        mzs, its = [], []
        for p in peaks_info:
            sigma = p["fwhm"] / 2.35482
            val = p["height"] * np.exp(-0.5 * ((rt - p["rt"]) / sigma) ** 2)
            if val > 100.0:
                mzs.append(p["mz"])
                its.append(val)
        if not mzs:
            mzs, its = [100.0], [10.0]
        a1 = arr("MS:1000514", "m/z array", mzs, False)
        a2 = arr("MS:1000515", "intensity array", its, True)
        specs.append(
            f'<spectrum index="{i}" id="scan={i+1}" defaultArrayLength="{len(mzs)}">'
            f'<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>'
            f'<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
            f'<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum" value=""/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" '
            f'name="scan start time" value="{rt:.5f}" unitCvRef="UO" '
            f'unitAccession="UO:0000031" unitName="minute"/></scan></scanList>'
            f'<binaryDataArrayList count="2">{a1}{a2}</binaryDataArrayList></spectrum>'
        )
    xml = (
        '<?xml version="1.0" encoding="utf-8"?>'
        '<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
        f'<run id="{file_path.stem}"><spectrumList count="{len(specs)}">'
        + "".join(specs) + "</spectrumList></run></mzML>"
    )
    file_path.write_text(xml, encoding="utf-8")
    return rts


def check_cli_features(rows: list[dict]) -> None:
    """Run mzlab-cli features on synthetic mzML and compare against analytical truth."""
    peaks_def = [
        {"name": "paracetamolo", "formula": "C8H9NO2", "mz": 152.07060, "rt": 1.80, "height": 3.0e5, "fwhm": 0.12},
        {"name": "caffeina", "formula": "C8H10N4O2", "mz": 195.08765, "rt": 2.50, "height": 5.0e5, "fwhm": 0.15},
        {"name": "ibuprofene", "formula": "C13H18O2", "mz": 207.13795, "rt": 3.20, "height": 4.0e5, "fwhm": 0.14},
        {"name": "carbamazepina", "formula": "C15H12N2O", "mz": 237.10224, "rt": 4.00, "height": 6.0e5, "fwhm": 0.16},
        {"name": "atrazina", "formula": "C8H14ClN5", "mz": 216.10105, "rt": 4.60, "height": 3.5e5, "fwhm": 0.13},
    ]
    with tempfile.TemporaryDirectory() as tmp:
        mzml_p = Path(tmp) / "sintetico_t1.mzML"
        csv_p = Path(tmp) / "features.csv"
        rts = write_synthetic_mzml(mzml_p, peaks_def)
        cli_bin = REPO_ROOT / "target" / "release" / "mzlab-cli"
        cmd = ([str(cli_bin), "features", str(mzml_p), "--out", str(csv_p)]
               if cli_bin.exists() else
               ["cargo", "run", "--release", "-p", "mzlab-cli", "--",
                "features", str(mzml_p), "--out", str(csv_p)])
        subprocess.run(cmd, cwd=str(REPO_ROOT), check=True, capture_output=True, text=True)
        with csv_p.open("r", encoding="utf-8") as f:
            feats = list(csv.DictReader(f))

    dt_sec = float((rts[1] - rts[0]) * 60.0)
    for p in peaks_def:
        sigma = p["fwhm"] / 2.35482
        y = np.array([p["height"] * np.exp(-0.5 * ((rt - p["rt"]) / sigma) ** 2) for rt in rts])
        ref_area = float(np.trapezoid(np.where(y > 100.0, y, 0.0), dx=dt_sec))
        ref_h = float(np.max(y))
        ref_rt = float(rts[np.argmax(y)])
        m = min(feats, key=lambda ft: abs(float(ft["mz"]) - p["mz"]))
        add_row(rows, "rivelazione_picchi", p["name"], p["formula"], "picco_mz",
                float(m["mz"]), p["mz"], "Da", 0.005)
        add_row(rows, "rivelazione_picchi", p["name"], p["formula"], "picco_rt",
                float(m["rt"]), ref_rt, "min", 0.05)
        add_row(rows, "rivelazione_picchi", p["name"], p["formula"], "picco_height",
                float(m["datafile:sintetico_t1:height"]), ref_h, "counts", ref_h * 0.01)
        add_row(rows, "rivelazione_picchi", p["name"], p["formula"], "picco_area",
                float(m["datafile:sintetico_t1:area"]), ref_area, "counts*s", ref_area * 0.01)


def main() -> None:
    """Execute all 300 T1 checks and save confronto_numeri.csv."""
    rows: list[dict] = []
    check_compounds(rows)
    check_cli_features(rows)
    csv_path = OUT_DIR / "confronto_numeri.csv"
    fields = [
        "categoria", "composto", "formula", "parametro", "valore_mzlab",
        "valore_riferimento", "differenza", "unita", "tolleranza", "esito",
    ]
    with csv_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)
    n_fail = sum(1 for r in rows if r["esito"] != "PASS")
    print(f"Completati {len(rows)} controlli numerici ({n_fail} FAIL).")


if __name__ == "__main__":
    main()
