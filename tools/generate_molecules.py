"""Build mzlab/web/molecules/catalogue.csv (the built-in catalogue of the molecules database, `molecules/1`): environmental compounds only.

Run by hand:  python3 tools/generate_molecules.py
Reads mzlab/chem/pesticides.csv and mzlab/chem/env_compounds.csv (curated names, PubChem formula/SMILES/InChIKey filled by tools/download_pesticides.py and
tools/download_env_compounds.py). A compound in both files, or twice under one InChIKey, is one record (the first name stays, the others become synonyms).
Records are written sorted by name; every record is in the screening list (`catalog = hrms`). Every record with a SMILES also gets the `structure:` classes
of tools/classes_structure.py. The file is written in the canonical form of the Rust writer
(`molecules::to_csv`): the test `crates/mzlab-core/tests/molecules.rs` checks that reading it and writing it back gives the same bytes.
"""
import csv
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from mzlab.chem import elements  # noqa: E402  (the reference of the Rust masses)
from classes_structure import classes as structure_classes  # noqa: E402  (structure:* classes read from the SMILES by fixed rules)

PESTICIDES = ROOT / "mzlab" / "chem" / "pesticides.csv"
ENV = ROOT / "mzlab" / "chem" / "env_compounds.csv"
OUT = ROOT / "mzlab" / "web" / "molecules" / "catalogue.csv"
COLUMNS = ["id", "name", "synonyms", "molecular_formula", "charge", "monoisotopic_mass", "smiles", "inchikey", "adducts", "collections", "classes", "cas",
           "pubchem_cid", "notes", "origin", "catalog"]
HRMS_ADDUCTS = "[M+H]+|[M+Na]+|[M+NH4]+"
# Full names of the compounds that pesticides.csv lists under an acronym, so that a search by the full name finds them.
LONG_NAMES = {
    "PFOA": "perfluorooctanoic acid", "PFOS": "perfluorooctanesulfonic acid|perfluorooctane sulfonic acid", "PFHxS": "perfluorohexanesulfonic acid",
    "PFNA": "perfluorononanoic acid", "PFHxA": "perfluorohexanoic acid", "PFBS": "perfluorobutanesulfonic acid", "PFBA": "perfluorobutanoic acid",
    "HFPO-DA (GenX)": "GenX|hexafluoropropylene oxide dimer acid", "AMPA": "aminomethylphosphonic acid", "TCEP": "tris(2-chloroethyl) phosphate",
    "TCPP": "tris(1-chloro-2-propyl) phosphate", "TDCPP": "tris(1,3-dichloro-2-propyl) phosphate", "TPHP": "triphenyl phosphate",
    "TBEP": "tris(2-butoxyethyl) phosphate", "TnBP": "tri-n-butyl phosphate", "DEET": "N,N-diethyl-meta-toluamide", "2,4-D": "2,4-dichlorophenoxyacetic acid",
    "MCPA": "2-methyl-4-chlorophenoxyacetic acid", "MCPB": "4-(4-chloro-2-methylphenoxy)butanoic acid", "2,4,5-T": "2,4,5-trichlorophenoxyacetic acid",
    "6PPD-quinone": "6PPD quinone", "bisphenol A": "BPA|4,4'-isopropylidenediphenol", "bisphenol S": "BPS", "bisphenol F": "BPF",
}


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


def record(name, synonyms, formula, smiles, inchikey, cid, classes):
    f = elements.parse_formula(formula)
    return {
        "id": "ik:" + inchikey, "name": name.strip(), "synonyms": "|".join(tidy(synonyms, name)), "molecular_formula": elements.fmt(f), "charge": "",
        "monoisotopic_mass": f"{round(elements.mass(f), 6):.6f}", "smiles": smiles.strip(), "inchikey": inchikey.strip(), "adducts": HRMS_ADDUCTS,
        "collections": "", "classes": "|".join(sorted(set(classes + structure_classes(smiles)))), "cas": "", "pubchem_cid": cid, "notes": "", "origin": "catalog",
        "catalog": "hrms",
    }


def sources():
    """(name, synonyms, formula, smiles, inchikey, cid, classes) of every curated row, pesticides.csv first."""
    for r in csv.DictReader(PESTICIDES.open(encoding="utf-8", newline=""), delimiter=";"):
        cl = ["group:pesticide", "use:" + r["use"], "chemical:" + r["class"]]
        yield r["name"], LONG_NAMES.get(r["name"], "").split("|"), r["formula"], r["smiles"], r["inchikey"], r["pubchem_cid"], cl
    for r in csv.DictReader(ENV.open(encoding="utf-8", newline=""), delimiter=";"):
        if not r["pubchem_cid"]:
            sys.exit(f"{r['name']}: no PubChem data; run tools/download_env_compounds.py or remove the row")
        cl = ["group:" + r["group"]]
        if r["group"] == "transformation_product":
            cl.append("parent:" + r["use"])
        elif r["use"]:
            cl.append("use:" + r["use"])
        yield r["name"], r["synonyms"].split("|"), r["formula"], r["smiles"], r["inchikey"], r["pubchem_cid"], cl


def main():
    csv.field_size_limit(1 << 24)
    by_key = {}
    for name, syn, formula, smiles, key, cid, cl in sources():
        if key in by_key:                                                  # the same compound twice: keep the first, the other name is a synonym
            old = by_key[key]
            old["synonyms"] = "|".join(tidy(old["synonyms"].split("|") + [name] + syn, old["name"]))
            old["classes"] = "|".join(sorted(set(old["classes"].split("|")) | set(cl)))
            continue
        by_key[key] = record(name, syn, formula, smiles, key, cid, cl)
    rows = sorted(by_key.values(), key=lambda r: (r["name"].lower(), r["name"]))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="") as fh:
        fh.write(";".join(COLUMNS) + "\n")
        for r in rows:
            fh.write(";".join(quote(r[c]) for c in COLUMNS) + "\n")
    print(len(rows), "records ->", OUT.relative_to(ROOT))


if __name__ == "__main__":
    main()
