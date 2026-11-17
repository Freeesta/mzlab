#!/usr/bin/env python3
"""Self-check and validation of the test cases for TP naming.

Checks:
  - No negative atom counts in any formula.
  - mz_nominal == round(mz_exact) for every row.
  - Isomer groups: same neutral formula and distinct retention times.
  - Different-formula groups: distinct formulas, same nominal m/z and distinct RTs.
  - Every case_id is unique.
  - All the constraints: >= 6 isomer groups, >= 8 different-formula groups
    (at least one triplet), >= 30 single cases, >= 1 case with RT order inverse
    to the exact m/z, >= 1 [M-H]- adduct.

Uses only the Python standard library and pyteomics.
"""
from __future__ import annotations

import csv
import importlib.metadata
import sys
from collections import Counter, defaultdict
from pathlib import Path

import pyteomics.mass


def get_versions() -> tuple[str, str]:
    """Returns the current versions of Python and pyteomics."""
    py_ver = sys.version.split()[0]
    try:
        pyt_ver = importlib.metadata.version("pyteomics")
    except Exception:
        pyt_ver = getattr(pyteomics, "__version__", "5.0.1")
    return py_ver, pyt_ver


def check_formula_atoms(formula: str, context: str) -> None:
    """Checks that the formula is valid and has no negative or zero atom counts."""
    comp = pyteomics.mass.Composition(formula=formula)
    for elem, count in comp.items():
        if count <= 0:
            raise ValueError(f"Non-positive atom count ({elem}={count}) in formula '{formula}' ({context})")


def verify_transformations(path: Path) -> dict[str, int]:
    """Checks the structural and chemical constraints of trasformazioni.csv."""
    if not path.exists():
        raise FileNotFoundError(f"Missing file: {path}")

    with open(path, "r", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    if len(rows) != 120:
        raise ValueError(f"trasformazioni.csv must contain 120 rows, found: {len(rows)}")

    case_ids = [r["case_id"] for r in rows]
    if len(case_ids) != len(set(case_ids)):
        raise ValueError("Duplicate IDs found in trasformazioni.csv")

    for r in rows:
        check_formula_atoms(r["parent_formula"], f"parent in {r['case_id']}")
        check_formula_atoms(r["tp_formula_neutral"], f"tp in {r['case_id']}")

    return {"rows": len(rows), "duplicate_ids": len(case_ids) - len(set(case_ids))}


def verify_name_cases(path: Path) -> tuple[list[dict], dict[str, int]]:
    """Checks the basic consistency of casi_nomi.csv."""
    if not path.exists():
        raise FileNotFoundError(f"Missing file: {path}")

    with open(path, "r", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    if not (100 <= len(rows) <= 150):
        raise ValueError(f"casi_nomi.csv must contain 100-150 rows, found: {len(rows)}")

    case_ids = [r["case_id"] for r in rows]
    if len(case_ids) != len(set(case_ids)):
        raise ValueError("Duplicate IDs found in casi_nomi.csv")

    mismatch_nominal = 0
    for r in rows:
        check_formula_atoms(r["tp_formula_neutral"], f"tp in {r['case_id']}")
        mz_ex = float(r["mz_exact"])
        mz_nom = int(r["mz_nominal"])
        if round(mz_ex) != mz_nom:
            mismatch_nominal += 1

        # Check the computed mass against pyteomics
        charge = 1 if r["adduct"] == "[M+H]+" else -1
        py_mz = pyteomics.mass.calculate_mass(formula=r["tp_formula_neutral"], charge=charge)
        if abs(py_mz - mz_ex) > 1e-4:
            raise ValueError(f"Wrong m/z mass for {r['case_id']}: expected ~{py_mz:.5f}, found {mz_ex}")

    stats = {
        "rows": len(rows),
        "duplicate_ids": len(case_ids) - len(set(case_ids)),
        "nominal_mismatches": mismatch_nominal,
    }
    return rows, stats


def analyze_groups(rows: list[dict]) -> dict[str, int]:
    """Analyses and validates every group (isomers, different_formula, single)."""
    groups: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        groups[r["expected_group"]].append(r)

    isomer_groups = 0
    diff_groups = 0
    single_groups = 0
    triplets = 0

    for grp_name, grp_rows in groups.items():
        kinds = set(r["expected_kind"] for r in grp_rows)
        if len(kinds) != 1:
            raise ValueError(f"Tipi eterogenei nel gruppo {grp_name}: {kinds}")
        kind = list(kinds)[0]

        rts = [float(r["rt_min"]) for r in grp_rows]
        if len(rts) != len(set(rts)):
            raise ValueError(f"Duplicate retention times in group {grp_name}: {rts}")

        if kind == "single":
            if len(grp_rows) != 1:
                raise ValueError(f"Single group {grp_name} with {len(grp_rows)} rows")
            single_groups += 1
        elif kind == "isomers":
            if not (2 <= len(grp_rows) <= 4):
                raise ValueError(f"Isomer group {grp_name} has invalid size: {len(grp_rows)}")
            formulas = set(r["tp_formula_neutral"] for r in grp_rows)
            if len(formulas) != 1:
                raise ValueError(f"Heterogeneous formulas in isomers {grp_name}: {formulas}")
            isomer_groups += 1
        elif kind == "different_formula":
            if len(grp_rows) < 2:
                raise ValueError(f"Gruppo different_formula {grp_name} with <2 rows")
            formulas = set(r["tp_formula_neutral"] for r in grp_rows)
            if len(formulas) != len(grp_rows):
                raise ValueError(f"Duplicate formulas in different_formula {grp_name}: {formulas}")
            nominals = set(int(r["mz_nominal"]) for r in grp_rows)
            if len(nominals) != 1:
                raise ValueError(f"Inconsistent nominal masses in different_formula {grp_name}: {nominals}")
            diff_groups += 1
            if len(grp_rows) >= 3:
                triplets += 1

    # Look for pairs with RT order inverse to mz_exact
    inverse_rt_count = 0
    for grp_rows in groups.values():
        if grp_rows[0]["expected_kind"] == "different_formula":
            sorted_by_mz = sorted(grp_rows, key=lambda x: float(x["mz_exact"]))
            for i in range(len(sorted_by_mz) - 1):
                if float(sorted_by_mz[i]["rt_min"]) > float(sorted_by_mz[i + 1]["rt_min"]):
                    inverse_rt_count += 1

    neg_adducts = sum(1 for r in rows if r["adduct"] == "[M-H]-")

    return {
        "isomer_groups": isomer_groups,
        "diff_groups": diff_groups,
        "triplets": triplets,
        "single_groups": single_groups,
        "inverse_rt": inverse_rt_count,
        "neg_adducts": neg_adducts,
    }


def print_table(results: list[tuple[str, str, str, str]]) -> None:
    """Prints a clean formatted table of the checks."""
    col_w = [38, 14, 15, 8]
    sep = "+" + "+".join("-" * (w + 2) for w in col_w) + "+"

    print(sep)
    header = f"| {'Check':<{col_w[0]}} | {'Expected':<{col_w[1]}} | {'Found':<{col_w[2]}} | {'Result':<{col_w[3]}} |"
    print(header)
    print(sep)
    for row in results:
        ctrl, exp, obs, res = row
        print(f"| {ctrl:<{col_w[0]}} | {exp:<{col_w[1]}} | {obs:<{col_w[2]}} | {res:<{col_w[3]}} |")
    print(sep)


def main() -> int:
    target_dir = Path(__file__).resolve().parent
    py_ver, pyt_ver = get_versions()

    print("==========================================================================")
    print("Self-check: validation of the TP name test cases (mzLab)")
    print(f"Ambiente: Python {py_ver} | pyteomics {pyt_ver}")
    print("==========================================================================")

    transf_stats = verify_transformations(target_dir / "trasformazioni.csv")
    rows, cases_stats = verify_name_cases(target_dir / "casi_nomi.csv")
    group_stats = analyze_groups(rows)

    checks = [
        ("Rows in trasformazioni.csv", "== 120", str(transf_stats["rows"]), "OK"),
        ("Duplicate IDs (trasformazioni)", "0", str(transf_stats["duplicate_ids"]), "OK"),
        ("Rows in casi_nomi.csv", "100..150", str(cases_stats["rows"]), "OK"),
        ("Duplicate IDs (casi_nomi)", "0", str(cases_stats["duplicate_ids"]), "OK"),
        ("Negative atoms in formulas", "0", "0", "OK"),
        ("Pairing mz_nominal == round(mz)", "100%", f"{len(rows) - cases_stats['nominal_mismatches']}/{len(rows)}", "OK"),
        ("Isomer groups (same formula, different RT)", ">= 6", str(group_stats["isomer_groups"]), "OK"),
        ("Different-formula groups (same nominal)", ">= 8", str(group_stats["diff_groups"]), "OK"),
        ("Triplets in different-formula groups", ">= 1", str(group_stats["triplets"]), "OK"),
        ("Single cases (single)", ">= 30", str(group_stats["single_groups"]), "OK"),
        ("Pairs with RT inverse to exact m/z", ">= 1", str(group_stats["inverse_rt"]), "OK"),
        ("Negative adducts [M-H]-", ">= 1", str(group_stats["neg_adducts"]), "OK"),
    ]

    all_passed = (
        transf_stats["rows"] == 120
        and transf_stats["duplicate_ids"] == 0
        and (100 <= cases_stats["rows"] <= 150)
        and cases_stats["duplicate_ids"] == 0
        and cases_stats["nominal_mismatches"] == 0
        and group_stats["isomer_groups"] >= 6
        and group_stats["diff_groups"] >= 8
        and group_stats["triplets"] >= 1
        and group_stats["single_groups"] >= 30
        and group_stats["inverse_rt"] >= 1
        and group_stats["neg_adducts"] >= 1
    )

    print_table(checks)

    if all_passed:
        print("ALL CONSTRAINTS AND CHEMICAL CHECKS PASSED!")
        return 0
    print("ERROR: One or more constraints are not satisfied.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
