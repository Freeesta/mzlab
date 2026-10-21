"""Build mzlab/web/molecole/catalogo.csv (the catalogue of the molecules database, `molecules/1`) from the two lists of before.

Run by hand, once:  python3 tools/genera_molecole.py
Reads TP_Mine/data/composti.csv (catalog `tabui`, in its order) and tests/data/liste/sostanze_hrms.csv (catalog `hrms`; the 43 molecules in both are one record,
joined by the whole InChIKey, with the name of composti.csv and every other name as a synonym). The file is written in the canonical form of the Rust writer
(`molecules::to_csv`): the test `crates/mzlab-core/tests/molecules.rs` checks that reading it and writing it back gives the same bytes.
"""
import csv
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from mzlab.chem import elements  # noqa: E402  (the reference of the Rust masses)

COMPOSTI = ROOT / "TP_Mine" / "data" / "composti.csv"
HRMS = ROOT / "tests" / "data" / "liste" / "sostanze_hrms.csv"
OUT = ROOT / "mzlab" / "web" / "molecole" / "catalogo.csv"
COLUMNS = ["id", "name", "synonyms", "molecular_formula", "charge", "monoisotopic_mass", "smiles", "inchikey", "adducts", "collections", "classes", "cas",
           "pubchem_cid", "notes", "origin", "catalog"]
HRMS_ADDUCTS = "[M+H]+|[M+Na]+|[M+NH4]+"
CLASS_KEYS = [("effect_class", "effect"), ("chemical_class", "chemical"), ("role", "role"), ("psychoactive", "psychoactive")]


def tidy(names, skip):
    """Without repeats (case ignored, the first spelling stays) and without `skip`; sorted like the Rust writer."""
    out, seen = [], {skip.lower()}
    for n in (x.strip() for x in names):
        if n and n.lower() not in seen:
            seen.add(n.lower())
            out.append(n)
    return sorted(out, key=lambda s: (s.lower(), s))


def quote(s):
    return '"' + s.replace('"', '""') + '"' if any(c in s for c in ';"\n\r') or s.startswith("#") else s


def record(name, synonyms, formula, smiles, inchikey, catalogs, extra):
    f = elements.parse_formula(formula)
    return {
        "id": "ik:" + inchikey, "name": name.strip(), "synonyms": "|".join(tidy(synonyms, name)), "molecular_formula": elements.fmt(f), "charge": "",
        "monoisotopic_mass": f"{round(elements.mass(f), 6):.6f}", "smiles": smiles.strip(), "inchikey": inchikey.strip(),
        "adducts": HRMS_ADDUCTS if "hrms" in catalogs else "", "collections": "", "classes": "|".join(sorted(extra.get("classes", []))),
        "cas": extra.get("cas", ""), "pubchem_cid": extra.get("cid", ""), "notes": extra.get("notes", ""), "origin": "catalogo", "catalog": "|".join(catalogs),
    }


# Records added to the catalogue after the two old lists (name, synonyms, formula, SMILES, InChIKey, catalogs).
ADDED = [("timolol", [], "C13H24N4O3S", "CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1", "BLJRIMJGRPQVNF-JTQLQIEISA-N", ["tabui", "hrms"])]


def hrms_extra(r):
    classes = [f"{k}:{r[c].strip()}" for c, k in CLASS_KEYS if r[c].strip()]
    return {"classes": classes, "cas": r["cas"].strip(), "cid": r["pubchem_cid"].strip(), "notes": r["check"].strip()}


def hrms_names(r):
    return [r["name_en"], r["name_it"]] + r["synonyms"].split("|")


def main():
    csv.field_size_limit(1 << 24)
    tabui = list(csv.DictReader(open(COMPOSTI, encoding="utf-8", newline="")))
    hrms = list(csv.DictReader(open(HRMS, encoding="utf-8", newline="")))
    by_key = {r["inchikey"].strip(): r for r in hrms}
    in_tabui = {r["inchikey"].strip() for r in tabui}
    rows = []
    for r in tabui:
        names = [n.strip() for n in r["names"].split(";") if n.strip()]
        h = by_key.get(r["inchikey"].strip())
        rows.append(record(names[0], names[1:] + (hrms_names(h) if h else []), r["neutral_formula"], r["smiles"], r["inchikey"],
                           ["tabui", "hrms"] if h else ["tabui"], hrms_extra(h) if h else {}))
    for r in hrms:
        if r["inchikey"].strip() not in in_tabui:
            rows.append(record(r["name_en"], hrms_names(r)[1:], r["neutral_formula"], r["smiles"], r["inchikey"], ["hrms"], hrms_extra(r)))
    rows += [record(n, sy, f, sm, ik, cat, {}) for n, sy, f, sm, ik, cat in ADDED]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="") as fh:
        fh.write(";".join(COLUMNS) + "\n")
        for r in rows:
            fh.write(";".join(quote(r[c]) for c in COLUMNS) + "\n")
    print(len(rows), "records ->", OUT.relative_to(ROOT))


if __name__ == "__main__":
    main()
