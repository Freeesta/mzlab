"""The catalogue of the molecules database (mzlab/web/molecole/catalogo.csv) loses nothing of the two lists it replaces: TP_Mine/data/composti.csv and
tests/data/liste/sostanze_hrms.csv. Every row has its record (names as a set, formula, SMILES, InChIKey, CAS, CID, classes, notes; the mass within 0.0005 Da);
the 43 molecules of both lists are one record, found by the whole InChIKey. The adducts are not copied (derived): the hrms records say which they prefer.
"""
import csv
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from mzlab.chem import elements  # noqa: E402

csv.field_size_limit(1 << 24)
CLASS_KEYS = [("effect_class", "effect"), ("chemical_class", "chemical"), ("role", "role"), ("psychoactive", "psychoactive")]


def read(path, delimiter=","):
    with open(ROOT / path, encoding="utf-8", newline="") as fh:
        return list(csv.DictReader(fh, delimiter=delimiter))


def catalogue():
    rows = read("mzlab/web/molecole/catalogo.csv", ";")
    return rows, {r["inchikey"]: r for r in rows}


def names(r):
    return {n.strip().lower() for n in [r["name"]] + r["synonyms"].split("|") if n.strip()}


def hill(f):
    return elements.fmt(elements.parse_formula(f))


def test_counts_and_order():
    rows, by_key = catalogue()
    tabui, hrms = read("TP_Mine/data/composti.csv"), read("tests/data/liste/sostanze_hrms.csv")
    # the last record (timolol) was added after the two old lists
    assert (len(tabui), len(hrms), len(rows)) == (625, 645, 625 + 645 - 43 + 1)
    assert len(by_key) == len(rows)
    rows = rows[:-1]
    # the rows of composti.csv come first, in their order
    assert [r["inchikey"] for r in rows[:625]] == [r["inchikey"] for r in tabui]
    assert all(r["catalog"].split("|")[0] == "tabui" for r in rows[:625])
    assert sum(r["catalog"] == "tabui|hrms" for r in rows) == 43
    assert all(r["catalog"] == "hrms" for r in rows[625:])


def test_composti_rows_are_all_there():
    _, by_key = catalogue()
    for t in read("TP_Mine/data/composti.csv"):
        r = by_key[t["inchikey"]]
        assert {n.strip().lower() for n in t["names"].split(";") if n.strip()} <= names(r), t["names"]
        assert r["name"] == t["names"].split(";")[0].strip()
        assert r["molecular_formula"] == hill(t["neutral_formula"])
        assert r["smiles"] == t["smiles"]
        assert abs(float(r["monoisotopic_mass"]) - float(t["mass"])) <= 0.0005, t["names"]


def test_hrms_rows_are_all_there():
    _, by_key = catalogue()
    for h in read("tests/data/liste/sostanze_hrms.csv"):
        r = by_key[h["inchikey"]]
        assert {n.strip().lower() for n in [h["name_en"], h["name_it"]] + h["synonyms"].split("|") if n.strip()} <= names(r), h["id"]
        assert r["molecular_formula"] == hill(h["neutral_formula"])
        assert r["smiles"] == h["smiles"]
        assert (r["cas"], r["pubchem_cid"], r["notes"]) == (h["cas"].strip(), h["pubchem_cid"].strip(), h["check"].strip())
        assert set(filter(None, r["classes"].split("|"))) == {f"{k}:{h[c].strip()}" for c, k in CLASS_KEYS if h[c].strip()}
        assert abs(float(r["monoisotopic_mass"]) - float(h["monoisotopic_mass"])) <= 0.0005, h["id"]
        assert r["adducts"] == "[M+H]+|[M+Na]+|[M+NH4]+"
        assert "hrms" in r["catalog"].split("|")
