"""The curated list of known compounds (TP_Mine/data/composti.csv) is well formed and in the size budget."""
import csv
import re
from pathlib import Path

from mzlab.chem import elements as E

CSV = Path(__file__).resolve().parents[1] / "TP_Mine" / "data" / "composti.csv"
ROWS = list(csv.DictReader(open(CSV, encoding="utf-8", newline="")))


def test_columns_and_size():
    assert list(ROWS[0]) == ["names", "neutral_formula", "mass", "smiles", "inchikey"]
    assert CSV.stat().st_size < 300_000
    assert 300 <= len(ROWS) <= 2500


def test_every_row_is_complete_and_unique():
    keys = [r["inchikey"] for r in ROWS]
    assert len(set(keys)) == len(keys)
    for r in ROWS:
        assert r["names"].split(";")[0].strip(), r
        assert r["smiles"] and re.fullmatch(r"[A-Z]{14}-[A-Z]{10}-[A-Z]", r["inchikey"]), r


def test_mass_matches_the_formula():
    bad = [(r["names"], r["neutral_formula"], r["mass"]) for r in ROWS if abs(E.mass(E.parse_formula(r["neutral_formula"])) - float(r["mass"])) > 1e-4]
    assert not bad, bad[:5]


def test_known_compounds_are_there():
    names = {n.strip().lower(): r for r in ROWS for n in r["names"].split(";")}
    assert names["caffeine"]["neutral_formula"] == "C8H10N4O2"
    assert names["ibuprofen"]["neutral_formula"] == "C13H18O2"


def test_forbidden_names_are_not_in_the_list():
    import os
    nomi = []
    for c in (os.environ.get("MZLAB_DATI"), os.environ.get("QQQ_DATI"), str(Path(__file__).resolve().parents[2] / "mzlab-dati")):
        f = Path(c) / "nomi_vietati.txt" if c else None
        if f and f.is_file():
            nomi += [n.strip().lower() for n in f.read_text(encoding="utf-8").splitlines() if n.strip() and not n.startswith("#")]
    text = CSV.read_text(encoding="utf-8").lower()
    assert not [n for n in nomi if n in text]
