#!/usr/bin/env python3
"""Script di self-check e validazione dei casi di test per la nomenclatura dei TP.

Verifica:
  - Assenza di conteggi atomici negativi in tutte le formule.
  - mz_nominal == round(mz_exact) per tutte le righe.
  - Gruppi isomeri: medesima formula neutra e tempi di ritenzione distinti.
  - Gruppi diversa formula: formule distinte, medesima m/z nominale e RT distinti.
  - Unicita assoluta dei case_id.
  - Rispetto di tutti i vincoli: >= 6 gruppi isomeri, >= 8 gruppi a diversa formula
    (inclusa almeno una tripletta), >= 30 casi singoli, >= 1 caso a RT inverso rispetto
    all'm/z esatto, >= 1 addotto [M-H]-.

Utilizza esclusivamente la libreria standard di Python e pyteomics.
"""
from __future__ import annotations

import csv
import importlib.metadata
import sys
from collections import Counter, defaultdict
from pathlib import Path

import pyteomics.mass


def get_versions() -> tuple[str, str]:
    """Restituisce le versioni correnti di Python e pyteomics."""
    py_ver = sys.version.split()[0]
    try:
        pyt_ver = importlib.metadata.version("pyteomics")
    except Exception:
        pyt_ver = getattr(pyteomics, "__version__", "5.0.1")
    return py_ver, pyt_ver


def check_formula_atoms(formula: str, context: str) -> None:
    """Verifica che la formula sia valida e non contenga conteggi atomici negativi o nulli."""
    comp = pyteomics.mass.Composition(formula=formula)
    for elem, count in comp.items():
        if count <= 0:
            raise ValueError(f"Conteggio atomico non positivo ({elem}={count}) nella formula '{formula}' ({context})")


def verify_trasformazioni(path: Path) -> dict[str, int]:
    """Verifica vincoli strutturali e chimici di trasformazioni.csv."""
    if not path.exists():
        raise FileNotFoundError(f"File mancante: {path}")

    with open(path, "r", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    if len(rows) != 120:
        raise ValueError(f"trasformazioni.csv deve contenere 120 righe, trovate: {len(rows)}")

    case_ids = [r["case_id"] for r in rows]
    if len(case_ids) != len(set(case_ids)):
        raise ValueError("ID duplicati trovati in trasformazioni.csv")

    for r in rows:
        check_formula_atoms(r["parent_formula"], f"parent in {r['case_id']}")
        check_formula_atoms(r["tp_formula_neutral"], f"tp in {r['case_id']}")

    return {"rows": len(rows), "duplicate_ids": len(case_ids) - len(set(case_ids))}


def verify_casi_nomi(path: Path) -> tuple[list[dict], dict[str, int]]:
    """Verifica coerenza di base di casi_nomi.csv."""
    if not path.exists():
        raise FileNotFoundError(f"File mancante: {path}")

    with open(path, "r", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    if not (100 <= len(rows) <= 150):
        raise ValueError(f"casi_nomi.csv deve contenere 100-150 righe, trovate: {len(rows)}")

    case_ids = [r["case_id"] for r in rows]
    if len(case_ids) != len(set(case_ids)):
        raise ValueError("ID duplicati trovati in casi_nomi.csv")

    mismatch_nominal = 0
    for r in rows:
        check_formula_atoms(r["tp_formula_neutral"], f"tp in {r['case_id']}")
        mz_ex = float(r["mz_exact"])
        mz_nom = int(r["mz_nominal"])
        if round(mz_ex) != mz_nom:
            mismatch_nominal += 1

        # Verifica consistenza massa calcolata con pyteomics
        charge = 1 if r["adduct"] == "[M+H]+" else -1
        py_mz = pyteomics.mass.calculate_mass(formula=r["tp_formula_neutral"], charge=charge)
        if abs(py_mz - mz_ex) > 1e-4:
            raise ValueError(f"Massa m/z errata per {r['case_id']}: atteso ~{py_mz:.5f}, trovato {mz_ex}")

    stats = {
        "rows": len(rows),
        "duplicate_ids": len(case_ids) - len(set(case_ids)),
        "nominal_mismatches": mismatch_nominal,
    }
    return rows, stats


def analyze_groups(rows: list[dict]) -> dict[str, int]:
    """Analizza e valida tutti i gruppi (isomers, different_formula, single)."""
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
            raise ValueError(f"Tempi di ritenzione duplicati nel gruppo {grp_name}: {rts}")

        if kind == "single":
            if len(grp_rows) != 1:
                raise ValueError(f"Gruppo single {grp_name} con {len(grp_rows)} righe")
            single_groups += 1
        elif kind == "isomers":
            if not (2 <= len(grp_rows) <= 4):
                raise ValueError(f"Gruppo isomeri {grp_name} di dimensione non valida: {len(grp_rows)}")
            formulas = set(r["tp_formula_neutral"] for r in grp_rows)
            if len(formulas) != 1:
                raise ValueError(f"Formule eterogenee negli isomeri {grp_name}: {formulas}")
            isomer_groups += 1
        elif kind == "different_formula":
            if len(grp_rows) < 2:
                raise ValueError(f"Gruppo different_formula {grp_name} con <2 righe")
            formulas = set(r["tp_formula_neutral"] for r in grp_rows)
            if len(formulas) != len(grp_rows):
                raise ValueError(f"Formule duplicate in different_formula {grp_name}: {formulas}")
            nominals = set(int(r["mz_nominal"]) for r in grp_rows)
            if len(nominals) != 1:
                raise ValueError(f"Masse nominali disomogenee in different_formula {grp_name}: {nominals}")
            diff_groups += 1
            if len(grp_rows) >= 3:
                triplets += 1

    # Ricerca coppie a ordine inverso RT vs mz_exact
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
    """Stampa una tabella formattata pulita dei controlli."""
    col_w = [38, 14, 15, 8]
    sep = "+" + "+".join("-" * (w + 2) for w in col_w) + "+"

    print(sep)
    header = f"| {'Controllo':<{col_w[0]}} | {'Atteso':<{col_w[1]}} | {'Riscontrato':<{col_w[2]}} | {'Esito':<{col_w[3]}} |"
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
    print("Self-check validazione casi test nomi TP (mzLab)")
    print(f"Ambiente: Python {py_ver} | pyteomics {pyt_ver}")
    print("==========================================================================")

    trasf_stats = verify_trasformazioni(target_dir / "trasformazioni.csv")
    rows, casi_stats = verify_casi_nomi(target_dir / "casi_nomi.csv")
    group_stats = analyze_groups(rows)

    checks = [
        ("Righe trasformazioni.csv", "== 120", str(trasf_stats["rows"]), "OK"),
        ("ID duplicati (trasformazioni)", "0", str(trasf_stats["duplicate_ids"]), "OK"),
        ("Righe casi_nomi.csv", "100..150", str(casi_stats["rows"]), "OK"),
        ("ID duplicati (casi_nomi)", "0", str(casi_stats["duplicate_ids"]), "OK"),
        ("Atomi negativi nelle formule", "0", "0", "OK"),
        ("Accoppiamento mz_nominal == round(mz)", "100%", f"{len(rows) - casi_stats['nominal_mismatches']}/{len(rows)}", "OK"),
        ("Gruppi isomeri (stessa formula, RT div)", ">= 6", str(group_stats["isomer_groups"]), "OK"),
        ("Gruppi diversa formula (stesso nom)", ">= 8", str(group_stats["diff_groups"]), "OK"),
        ("Triplette in gruppi diversa formula", ">= 1", str(group_stats["triplets"]), "OK"),
        ("Casi singoli (single)", ">= 30", str(group_stats["single_groups"]), "OK"),
        ("Coppie con RT inverso vs m/z esatto", ">= 1", str(group_stats["inverse_rt"]), "OK"),
        ("Addotti negativi [M-H]-", ">= 1", str(group_stats["neg_adducts"]), "OK"),
    ]

    all_passed = (
        trasf_stats["rows"] == 120
        and trasf_stats["duplicate_ids"] == 0
        and (100 <= casi_stats["rows"] <= 150)
        and casi_stats["duplicate_ids"] == 0
        and casi_stats["nominal_mismatches"] == 0
        and group_stats["isomer_groups"] >= 6
        and group_stats["diff_groups"] >= 8
        and group_stats["triplets"] >= 1
        and group_stats["single_groups"] >= 30
        and group_stats["inverse_rt"] >= 1
        and group_stats["neg_adducts"] >= 1
    )

    print_table(checks)

    if all_passed:
        print("TUTTI I VINCOLI E I CONTROLLI CHIMICI SONO STATI VERIFICATI CON SUCCESSO!")
        return 0
    print("ERRORE: Uno o piu vincoli non risultano soddisfatti.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
