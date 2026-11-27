"""The built-in molecule catalogue (mzlab/web/molecules/catalogue.csv) is environmental only: built from pesticides.csv and env_compounds.csv (PubChem
descriptors), every record with a real common name, formula agreeing with the SMILES and the mass, the must-have names there, the removed codes gone."""
import collections
import csv
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tools"))
from mzlab.chem import elements  # noqa: E402
from classes_structure import classes as structure_classes  # noqa: E402

csv.field_size_limit(1 << 24)
GROUPS = {"pesticide", "pharmaceutical", "contaminant", "transformation_product", "probe"}
KEYS = {"group", "use", "chemical", "parent", "structure"}


def read(path):
    with open(ROOT / path, encoding="utf-8", newline="") as fh:
        return list(csv.DictReader(fh, delimiter=";"))


CAT = read("mzlab/web/molecules/catalogue.csv")
ENV = read("mzlab/chem/env_compounds.csv")


def lines(name):
    return [x.strip() for x in (ROOT / "tests/data" / name).read_text(encoding="utf-8").splitlines() if x.strip() and not x.startswith("#")]


def names(r):
    return {n.strip().lower() for n in [r["name"]] + r["synonyms"].split("|") if n.strip()}


def heavy_atoms(smiles):
    out = collections.Counter()
    for bracket, plain in re.findall(r"\[([^\]]+)\]|(Cl|Br|[BCNOPSFI]|[bcnops])", smiles):
        el = re.match(r"\d*([A-Z][a-z]?|[a-z]{1,2})", bracket).group(1) if bracket else plain
        if el != "H":
            out[el.capitalize()] += 1
    return out


def test_size_and_uniqueness():
    assert 800 <= len(CAT) <= 1228, len(CAT)                       # fewer than the 1228 records of the old catalogue
    for k in ("id", "inchikey", "name"):
        vals = [r[k].lower() for r in CAT]
        assert len(set(vals)) == len(vals), (k, [v for v, n in collections.Counter(vals).items() if n > 1])
    assert all(r["origin"] == "catalog" and r["catalog"] == "hrms" for r in CAT)


def test_the_source_rows_have_their_pubchem_columns():
    assert all(r["pubchem_cid"].isdigit() and r["formula"] and r["smiles"] and r["inchikey"] for r in ENV)
    assert {r["group"] for r in ENV} <= GROUPS


@pytest.mark.parametrize("r", CAT, ids=lambda r: r["name"])
def test_every_record_is_consistent(r):
    f = elements.parse_formula(r["molecular_formula"])
    assert elements.fmt(f) == r["molecular_formula"]
    assert abs(elements.mass(f) - float(r["monoisotopic_mass"])) < 5e-7
    assert heavy_atoms(r["smiles"]) == collections.Counter({k: v for k, v in f.items() if k != "H"})
    tags = [t for t in r["classes"].split("|") if t]
    assert {t.split(":")[0] for t in tags} <= KEYS
    assert [t for t in tags if t.startswith("structure:")] == structure_classes(r["smiles"])
    assert any(t.startswith("group:") and t[6:] in GROUPS for t in tags)
    # a real name: words of lower-case letters, or an acronym that carries its full name as a synonym (PFOA, 2,4,5-T)
    assert any(re.search(r"[a-z]{4}", n) for n in [r["name"], *r["synonyms"].split("|")]), "a code-only entry"


def test_the_must_have_names_are_there_and_the_removed_codes_are_gone():
    have = set().union(*(names(r) for r in CAT))
    assert [n for n in lines("catalogue_must_have.txt") if n.lower() not in have] == []
    assert [n for n in lines("catalogue_removed.txt") if n.lower() in have] == []


def test_every_pesticide_of_the_contaminant_list_is_in_the_catalogue():
    keys = {r["inchikey"] for r in CAT}
    assert [r["name"] for r in read("mzlab/chem/pesticides.csv") if r["inchikey"] not in keys] == []


def test_the_generator_gives_the_file_in_the_repo():
    before = (ROOT / "mzlab/web/molecules/catalogue.csv").read_bytes()
    subprocess.run([sys.executable, str(ROOT / "tools/generate_molecules.py")], check=True, capture_output=True)
    assert (ROOT / "mzlab/web/molecules/catalogue.csv").read_bytes() == before
