"""Fill the PubChem columns (pubchem_cid, formula, smiles, inchikey) of mzlab/chem/env_compounds.csv from the curated columns (name or query).

Run by hand, with network:  python3 tools/download_env_compounds.py
Only descriptors PubChem computes itself are taken (formula, SMILES, InChIKey): NCBI places no restriction on their use. Rows that already have a
`pubchem_cid` are kept as they are. A name that PubChem does not resolve, resolves to more than one compound, to a charged or a multi-component one
is listed on stderr (exit 1) and keeps empty PubChem columns: name it exactly in `query` or remove the row.
"""
import csv
import io
import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSV = ROOT / "mzlab" / "chem" / "env_compounds.csv"
COLUMNS = ["name", "group", "use", "synonyms", "query", "pubchem_cid", "formula", "smiles", "inchikey"]
URL = "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/{}/property/MolecularFormula,SMILES,InChIKey,Charge/JSON"


def fetch(q):
    for attempt in range(4):
        try:
            with urllib.request.urlopen(URL.format(urllib.parse.quote(q, safe="")), timeout=30) as r:
                props = json.load(r)["PropertyTable"]["Properties"]
            break
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None, "not found"
            time.sleep(2 * (attempt + 1))
    else:
        return None, "PubChem did not answer"
    if len(props) != 1:
        return None, f"{len(props)} compounds"
    p = props[0]
    if p.get("Charge", 0) != 0 or "." in p["SMILES"]:
        return None, "charged or multi-component"
    return p, ""


def write(rows):
    out = io.StringIO()
    w = csv.DictWriter(out, COLUMNS, delimiter=";", lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
    CSV.write_text(out.getvalue(), encoding="utf-8")


if __name__ == "__main__":
    rows = list(csv.DictReader(CSV.open(encoding="utf-8"), delimiter=";"))
    bad = []
    for r in rows:
        if r["pubchem_cid"]:
            continue
        p, why = fetch(r["query"] or r["name"])
        if p is None:
            bad.append(f"{r['name']}: {why}")
        else:
            r.update(pubchem_cid=str(p["CID"]), formula=p["MolecularFormula"], smiles=p["SMILES"], inchikey=p["InChIKey"])
        time.sleep(0.25)                                         # PubChem asks for at most 5 requests a second
    write(rows)
    print(f"{CSV}: {len(rows)} rows, {len(bad)} unresolved", file=sys.stderr)
    for b in bad:
        print(b, file=sys.stderr)
    sys.exit(1 if bad else 0)
