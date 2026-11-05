"""Fill the PubChem columns (pubchem_cid, formula, smiles, inchikey) of mzlab/chem/pesticides.csv from the curated columns (name_en or query).

Run by hand, with network:  python3 tools/scarica_pesticidi.py
Only descriptors PubChem computes itself are taken (formula, SMILES, InChIKey): NCBI places no restriction on their use. A name that PubChem resolves to
more than one compound, or to a charged one, stops the run: the curated row must name the compound exactly (column `query`).
"""
import csv
import io
import json
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSV = ROOT / "mzlab" / "chem" / "pesticides.csv"
COLUMNS = ["id", "name_en", "name", "class", "use", "ions", "query", "pubchem_cid", "formula", "smiles", "inchikey"]
URL = "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/{}/property/MolecularFormula,SMILES,InChIKey,Charge/JSON"


def fetch(q):
    with urllib.request.urlopen(URL.format(urllib.parse.quote(q, safe="")), timeout=30) as r:
        props = json.load(r)["PropertyTable"]["Properties"]
    if len(props) != 1 or props[0].get("Charge", 0) != 0:
        sys.exit(f"{q}: {len(props)} compounds or a charged one; name it exactly in the column query")
    return props[0]


def write(rows):
    out = io.StringIO()
    w = csv.DictWriter(out, COLUMNS, delimiter=";", lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
    CSV.write_text(out.getvalue(), encoding="utf-8")


if __name__ == "__main__":
    rows = list(csv.DictReader(CSV.open(encoding="utf-8"), delimiter=";"))
    for r in rows:
        p = fetch(r["query"] or r["name_en"])
        r.update(pubchem_cid=str(p["CID"]), formula=p["MolecularFormula"], smiles=p["SMILES"], inchikey=p["InChIKey"])
        time.sleep(0.25)                                         # PubChem asks for at most 5 requests a second
    write(rows)
    print(f"{CSV}: {len(rows)} rows")
