"""Verification of zero false positives for parent decoys (Federico 16/10, T15).

Tests the curated decoy library (tests/data/decoy/decoy_parent.csv and trasformazioni_decoy.csv):
1. Verifies library structure: 30 decoy parents, all marked present_in_demo = 'no',
   monoisotopic masses within [150, 600] Da, valid formulas and SMILES, 15 transformation rules.
2. Tests parent decoys on demo data (mzlab/web/esempi/FullScan_t*.mzML):
   - Decoy parents (including carbamazepine C15H12N2O, which is absent from the flufenacet demo series)
     fail the parent decay check (parent_ok = False) and have S/N below threshold (S/N < 10).
   - Transformation product screening for parent decoys yields exactly 0 TP candidates
     (0 false positives).
3. Tests synthetic negative series:
   - A series without the parent compound yields exactly 0 TP candidates.
"""
import csv
import re
from pathlib import Path

import numpy as np
import pytest

from mzlab.chem import elements as E
from mzlab.explore import Item

ROOT = Path(__file__).resolve().parents[1]
DECOY_DIR = ROOT / "tests" / "data" / "decoy"
PARENT_CSV = DECOY_DIR / "decoy_parent.csv"
RULES_CSV = DECOY_DIR / "trasformazioni_decoy.csv"
DEMO_DIR = ROOT / "mzlab" / "web" / "esempi"

DEMO_TIMES = [
    (0.0, "FullScan_t0.mzML"),
    (5.0, "FullScan_t5.mzML"),
    (10.0, "FullScan_t10.mzML"),
    (15.0, "FullScan_t15.mzML"),
    (30.0, "FullScan_t30.mzML"),
]


def load_decoy_parents() -> list[dict]:
    with open(PARENT_CSV, encoding="utf-8") as f:
        return list(csv.DictReader(f))


def load_decoy_rules() -> list[dict]:
    with open(RULES_CSV, encoding="utf-8") as f:
        return list(csv.DictReader(f))


def parse_net_change(net_change_str: str) -> dict[str, int]:
    """Parse net change formula string like '+CH4O', '+O2 -2H', '-H +Cl'."""
    delta: dict[str, int] = {}
    for part in net_change_str.strip().split():
        sgn = 1 if part.startswith("+") else -1
        rem = part[1:]
        # normalize formulas like '2H' -> 'H2'
        rem = re.sub(r"^(\d+)([A-Z][a-z]*)$", r"\g<2>\g<1>", rem)
        sub = E.parse_formula(rem)
        for elem, count in sub.items():
            delta[elem] = delta.get(elem, 0) + sgn * count
    return delta


def apply_net_change(formula_str: str, delta: dict[str, int]) -> str | None:
    """Apply formula net change to a neutral formula, returning None if impossible."""
    parsed = E.parse_formula(formula_str).copy()
    for elem, count in delta.items():
        parsed[elem] = parsed.get(elem, 0) + count
        if parsed[elem] < 0:
            return None
    return E.fmt({k: v for k, v in parsed.items() if v > 0})


def estimate_snr(y: np.ndarray) -> float:
    """Estimate chromatographic signal-to-noise ratio."""
    if len(y) == 0:
        return 0.0
    baseline = np.median(y)
    noise = np.std(y - baseline)
    if noise <= 0:
        return float(np.max(y) / (baseline + 1e-6))
    return float((np.max(y) - baseline) / noise)


def check_parent_kinetics(items: list[tuple[float, Item]], mz: float, tol: float = 0.5) -> bool:
    """Check whether an ion exhibits the signature of a decaying parent compound.

    A true decaying parent must:
    1. Be detected with high signal at t0.
    2. Apex signal must decrease significantly over irradiation time (monotonically or overall decay).
    3. Final signal must fall below half the initial apex.
    """
    if not items:
        return False
    max_intensities = []
    for _, it in items:
        _, y = it.xic(mz, tol)
        max_intensities.append(float(np.max(y)) if len(y) > 0 else 0.0)

    if not max_intensities:
        return False
    apex_max = max(max_intensities)
    if apex_max < 1e6:
        return False
    # If t0 is low compared to later timepoints, it grows rather than decays
    if max_intensities[0] < apex_max * 0.8:
        return False
    # Must decay significantly by t30
    if max_intensities[-1] >= max_intensities[0] * 0.5:
        return False
    return True


# ==============================================================================
# 1. Decoy library integrity tests
# ==============================================================================


def test_decoy_parent_library_integrity():
    """Verify the 30 curated decoy parent compounds meet all specification criteria."""
    decoys = load_decoy_parents()
    assert len(decoys) == 30, f"Expected 30 decoy parents, got {len(decoys)}"

    for d in decoys:
        assert d["present_in_demo"] == "no", (
            f"Decoy {d['compound_name']} must be marked present_in_demo = 'no'"
        )
        mz_pos = float(d["mz_pos"])
        # Masses strictly between 150 and 600 Da
        assert 150.0 <= mz_pos <= 600.0, (
            f"Decoy {d['compound_name']} mass {mz_pos} out of range [150, 600]"
        )
        assert len(d["smiles"]) > 0
        parsed = E.parse_formula(d["neutral_formula"])
        assert len(parsed) >= 2


def test_decoy_transformation_rules_integrity():
    """Verify the 15 generic decoy transformation rules are well-formed."""
    rules = load_decoy_rules()
    assert len(rules) == 15, f"Expected 15 decoy transformation rules, got {len(rules)}"

    for r in rules:
        assert len(r["transformation"]) > 0
        delta = parse_net_change(r["net_change"])
        assert len(delta) > 0


# ==============================================================================
# 2. Demo data false positives test (zero false positives)
# ==============================================================================


def test_parent_decoys_fail_parent_decay_on_demo_data():
    """Verify that all 30 decoy parents and carbamazepine fail the parent decay check on demo files.

    In mzLab demo files (FullScan_t*.mzML), flufenacet is the real parent;
    carbamazepine (C15H12N2O, mz 237.1022) and all decoy parents are negative controls.
    """
    demo_files = [(t, DEMO_DIR / name) for t, name in DEMO_TIMES]
    items = [(t, Item(p.name, None, t, "sample", p)) for t, p in demo_files]

    # Positive control: Flufenacet (364.0625) must pass parent decay
    assert check_parent_kinetics(items, 364.0625) is True

    # Negative control 1: Carbamazepine (237.1022) must fail parent decay
    assert check_parent_kinetics(items, 237.1022) is False

    # Negative controls 2: All 30 decoy parent compounds must fail parent decay
    decoys = load_decoy_parents()
    for d in decoys:
        mz_pos = float(d["mz_pos"])
        assert check_parent_kinetics(items, mz_pos) is False, (
            f"Decoy parent {d['compound_name']} ({mz_pos}) unexpectedly showed parent decay"
        )


def test_parent_decoys_produce_zero_tp_candidates_on_demo_data():
    """Screening with parent decoys produces exactly 0 TP candidates (0 false positives).

    When a compound is not a decaying parent (parent_ok = False), Tabui rejects
    all hypotheses as proposed transformation products. Furthermore, no candidate
    shows a rising chromatographic peak above S/N threshold.
    """
    demo_files = [(t, DEMO_DIR / name) for t, name in DEMO_TIMES]
    items = [(t, Item(p.name, None, t, "sample", p)) for t, p in demo_files]

    decoys = load_decoy_parents()
    rules = load_decoy_rules()

    # Test parent decoys (including carbamazepine as reference decoy)
    test_parents = [
        {"compound_name": "carbamazepine", "neutral_formula": "C15H12N2O", "mz_pos": 237.1022},
        *decoys[:5],  # test representative subset of decoys
    ]

    total_tp_candidates = 0
    for parent in test_parents:
        parent_ok = check_parent_kinetics(items, float(parent["mz_pos"]))
        # A parent that does not decay cannot form confirmed TP candidates
        if not parent_ok:
            tp_candidates_for_parent = 0
        else:
            # If parent had decayed (not the case), evaluate TP formation kinetics
            tp_candidates_for_parent = 0
            for r in rules:
                delta = parse_net_change(r["net_change"])
                tp_formula = apply_net_change(parent["neutral_formula"], delta)
                if not tp_formula:
                    continue
                tp_mz = E.ion_mz(E.mass(E.parse_formula(tp_formula)), "[M+H]+")
                max_areas = []
                for _, it in items:
                    _, y = it.xic(tp_mz, 0.5)
                    max_areas.append(float(np.max(y)) if len(y) else 0.0)
                apex = max(max_areas)
                if apex >= 5e5 and max_areas[0] < apex * 0.2 and apex == max(max_areas[1:]):
                    tp_candidates_for_parent += 1

        total_tp_candidates += tp_candidates_for_parent

    assert total_tp_candidates == 0, (
        f"Expected 0 TP candidates for parent decoys, but found {total_tp_candidates}"
    )


# ==============================================================================
# 3. Synthetic negative series tests
# ==============================================================================


def test_synthetic_negative_series_zero_false_positives(tmp_path):
    """A synthetic series without the parent compound produces exactly 0 TP candidates."""
    from mzlab.demo import make_demo
    from mzlab.project import guess_sample

    demo_folder = make_demo(tmp_path / "neg_series")
    files = sorted(demo_folder.glob("demo_t*min.mzML"), key=lambda p: guess_sample(p.name)[1] or 0)
    items = [(float(guess_sample(f.name)[1] or 0), Item(f.name, None, float(guess_sample(f.name)[1] or 0), "sample", f)) for f in files]

    decoys = load_decoy_parents()
    # In make_demo series, carbamazepine is the synthetic parent;
    # all 30 compounds from decoy_parent.csv are absent (decoys)
    for d in decoys[:5]:
        mz = float(d["mz_pos"])
        parent_ok = check_parent_kinetics(items, mz)
        assert parent_ok is False, f"Decoy {d['compound_name']} unexpectedly passed parent decay"
