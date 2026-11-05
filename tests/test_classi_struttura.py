"""The `structure:` classes of tools/classi_struttura.py: fixed rules on synthetic SMILES, and the catalogue (catalogo.csv) carrying exactly what they give,
with every class tag from a closed vocabulary."""
import csv
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
from classi_struttura import VOCABULARY, classes, parse  # noqa: E402

CATALOGUE = ROOT / "mzlab" / "web" / "molecole" / "catalogo.csv"


@pytest.mark.parametrize("smiles,want", [
    ("CC(=O)O", ["carboxylic_acid"]),
    ("OC(=O)c1ccccc1", ["carboxylic_acid"]),
    ("CC(=O)[O-].[Na+]", []),                                      # a carboxylate salt is not the acid as written
    ("OC(=O)O", []),                                               # carbonic acid: the carbonyl carbon is bound to two O
    ("CC(=O)OC", ["ester"]),
    ("O=C1OCCC1", ["ester"]),                                      # a lactone is a cyclic ester
    ("CCOC(=O)N", []),                                             # a carbamate is not an ester here
    ("NC1=CC=C(C=C1)S(N)(=O)=O", ["sulfonamide"]),
    ("NS(=O)(=O)OC", []),                                          # a sulfamate (O-SO2-N)
    ("C1=CC=C(C=C1)[N+](=O)[O-]", ["nitro"]),
    ("O=N(=O)c1ccccc1", ["nitro"]),
    ("CN(C)[N+](=O)[O-]", ["nitro"]),                              # N-nitro
    ("C(CO[N+](=O)[O-])O[N+](=O)[O-]", []),                        # a nitrate ester
    ("C[N+](C)(C)CCO", ["quaternary_ammonium"]),
    ("C[NH+](C)C", []),                                            # a protonated amine
    ("FC(F)(F)c1ccc(Cl)cc1Br", ["brominated", "chlorinated", "fluorinated"]),
    ("[2H]C([2H])([2H])C(=O)O", ["carboxylic_acid"]),
    ("", []),
])
def test_rules(smiles, want):
    assert classes(smiles) == ["structure:" + w for w in want]


def test_a_smiles_it_cannot_read_is_an_error():
    for bad in ("C1CC", "C(C", "C?C"):
        with pytest.raises(ValueError):
            parse(bad)


def test_the_catalogue_carries_the_rules_and_a_closed_vocabulary():
    rows = list(csv.DictReader(CATALOGUE.open(encoding="utf-8"), delimiter=";"))
    closed = {"chemical", "effect", "psychoactive", "role", "structure"}
    for r in rows:
        tags = [t for t in r["classes"].split("|") if t]
        assert {t.split(":")[0] for t in tags} <= closed, r["id"]
        assert [t for t in tags if t.startswith("structure:")] == classes(r["smiles"]), r["id"]
        assert all(t[len("structure:"):] in VOCABULARY for t in tags if t.startswith("structure:"))
    tabui = [r for r in rows if "tabui" in r["catalog"].split("|")]
    assert sum(1 for r in tabui if r["classes"]) >= 390
