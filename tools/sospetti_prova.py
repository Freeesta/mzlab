"""Synthetic mini-set for the tests of the suspect index (50 invented rows; no pytest here: the Rust parity test runs it through tools/sospetti_riferimento.py)."""
import csv
import hashlib
import random
from pathlib import Path

A = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"


def make_sources(d: Path):
    """25 keys in SusDat, 25 in PubChemLite (5 shared), 10 compounds in two reactions files; masses and keys invented."""
    rnd = random.Random(7)
    keys = ["".join(rnd.choice(A) for _ in range(14)) for _ in range(45)]
    mass = {k: round(rnd.uniform(100, 600), 6) for k in keys}
    form = {k: f"C{rnd.randint(1, 30)}H{rnd.randint(1, 40)}O{rnd.randint(0, 6)}" for k in keys}
    su = keys[:25] + ["short", "X" * 14]
    pl = keys[20:45]

    def w(name, header, rows):
        with open(d / name, "w", newline="", encoding="utf-8") as f:
            c = csv.writer(f)
            c.writerow(header)
            c.writerows(rows)
        return name, hashlib.sha256((d / name).read_bytes()).hexdigest(), "http://invalid/" + name

    f1 = w("su.csv", ["Norman_SusDat_ID", "StdInChIKey", "Molecular_Formula", "Monoiso_Mass"],
           [[i, k + "-UHFFFAOYSA-N", form.get(k, "CH4"), mass.get(k, "") if k != "short" else ""] for i, k in enumerate(su)])
    f2 = w("pl.csv", ["FirstBlock", "PubMed_Count", "Patent_Count", "MolecularFormula", "MonoisotopicMass"],
           [[k, 100000 if i == 0 else i, 3 * i, form[k], mass[k]] for i, k in enumerate(pl)])
    f3 = w("re.csv", ["predecessor", "predecessorcid", "successor", "successorcid"],
           [["a", 1, "b", 2], ["b", 2, "c", 3], ["c", 3, "d", 99]])
    f4 = w("ci.csv", ["CID", "MolecularFormula", "ExactMass", "IKFB"],
           [[1, form[keys[40]], mass[keys[40]], keys[40]], [2, form[keys[41]], mass[keys[41]], keys[41]], [3, form[keys[42]], mass[keys[42]], keys[42]]])
    base = {"licenza": "CC-BY-4.0", "doi": "10.0000/x", "url": "http://x", "istituzione": "Inventata"}
    fonti = [dict(base, id="susdat", nome="S", versione="1", files=[f1]), dict(base, id="pubchemlite", nome="P", versione="1", files=[f2]),
             dict(base, id="transpubchem", nome="T", versione="1", files=[f3, f4])]
    return fonti, keys, mass, form
