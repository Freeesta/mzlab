"""Build TP_Mine/data/composti.csv from tools/composti_nomi.txt and the PubChemLite for Exposomics CSV.

Run by hand (never at use time):  python3 tools/genera_composti.py [path/to/PubChemLite.csv]
Without a path the Zenodo file is downloaded to a temporary folder and checked against the pinned SHA-256.
Licence of the source: CC-BY-4.0 (see LICENZE-TERZI.md).
"""
import csv
import hashlib
import sys
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NAMES = ROOT / "tools" / "composti_nomi.txt"
OUT = ROOT / "TP_Mine" / "data" / "composti.csv"
URL = "https://zenodo.org/api/records/22953038/files/PubChemLite_exposomics_20260925.csv/content"
SHA256 = "6a258459e1ccf29a5d08e82fcf5240a2fc8afbabe5ba9c678e39aa0801dd89cd"   # PubChemLite for Exposomics 3.3.0, 2026-09-25
COLS = ["names", "neutral_formula", "mass", "smiles", "inchikey"]

csv.field_size_limit(1 << 24)


def wanted():
    """[(names, formula or None)]; a line is "name;synonym" or "name;synonym|FORMULA" (formula = fallback when PubChemLite spells the name in IUPAC form)."""
    out = []
    for line in NAMES.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            line, _, formula = line.partition("|")
            out.append(([n.strip() for n in line.split(";") if n.strip()], formula.strip() or None))
    return out


def fetch() -> Path:
    dest = Path(tempfile.mkdtemp(prefix="pubchemlite_")) / "pcl.csv"
    urllib.request.urlretrieve(URL, dest)
    return dest


def main() -> None:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else fetch()
    sha = hashlib.sha256(path.read_bytes()).hexdigest()
    if sha != SHA256:
        sys.exit(f"SHA-256 {sha} differs from the pinned one")
    groups = wanted()
    key = {}
    for gi, (g, _) in enumerate(groups):
        for n in g:
            key.setdefault(n.lower(), gi)
    byformula = {f: gi for gi, (_, f) in enumerate(groups) if f}
    best, fallback = {}, {}     # group index -> (score, row)
    with open(path, encoding="utf-8", newline="") as fh:
        for row in csv.DictReader(fh):
            gi = key.get(row["CompoundName"].lower())
            if gi is None:
                gi = key.get(row["Synonym"].lower())
            if gi is None and row["MolecularFormula"] in byformula and row["SMILES"] and row["InChIKey"]:      # candidates by formula, used only if no name matches
                gj = [k for k, (_, f) in enumerate(groups) if f == row["MolecularFormula"]]
                sc = int(row["PubMed_Count"] or 0) + int(row["Patent_Count"] or 0)
                for k in gj:
                    if k not in fallback or sc > fallback[k][0]:
                        fallback[k] = (sc, row)
            if gi is None or not row["SMILES"] or not row["MolecularFormula"] or not row["InChIKey"]:
                continue
            score = int(row["PubMed_Count"] or 0) + int(row["Patent_Count"] or 0)
            if gi not in best or score > best[gi][0]:
                best[gi] = (score, row)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    rows = []
    for gi, (g, _) in enumerate(groups):
        if gi not in best and gi in fallback:
            best[gi] = fallback[gi]
            print("by formula:", g[0], "->", fallback[gi][1]["CompoundName"])
        if gi not in best:
            print("not found:", g[0])
            continue
        r = best[gi][1]
        names = list(g)
        for extra in (r["Synonym"], r["CompoundName"]):
            if extra and extra.lower() not in {n.lower() for n in names}:
                names.append(extra)
        rows.append([";".join(names), r["MolecularFormula"], f'{float(r["MonoisotopicMass"]):.5f}', r["SMILES"], r["InChIKey"]])
    seen, uniq = set(), []
    for r in rows:
        if r[4] not in seen:
            seen.add(r[4])
            uniq.append(r)
    with open(OUT, "w", encoding="utf-8", newline="") as fh:
        w = csv.writer(fh, lineterminator="\n")
        w.writerow(COLS)
        w.writerows(uniq)
    print(len(uniq), "compounds,", OUT.stat().st_size, "bytes")


if __name__ == "__main__":
    main()
