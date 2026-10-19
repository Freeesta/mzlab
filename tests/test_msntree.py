"""WP-H4: tree of the fragmentation paths of a direct-infusion MSn file (private validation: the file is not in this repository; skipped without it).
No compound is named here: the chain of the fragments is given as formulas."""
import sys
from pathlib import Path
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from dati_percorsi import percorso  # noqa: E402


def _file():
    hrms = percorso("dati/HRMS")
    return next(iter(hrms.glob("*/*direct-infusion_MSn.mzML")), None) if hrms else None


pytestmark = pytest.mark.skipif(_file() is None, reason="infusion MSn file of the data repository not available")


@pytest.fixture(scope="module")
def tree():
    sys.path.insert(0, str(ROOT))
    from mzlab.reader.mzml import Run
    from mzlab.chem.msntree import msn_tree
    return msn_tree(Run(str(_file())), "C13H25N4O3S")


def test_chain_to_ms8(tree):
    assert max(n["level"] for n in tree["nodes"]) == 8
    assert tree["formula_given"] and tree["n_nodes"] >= 19
    by = {(n["path"][-1][0], n["level"]): n for n in tree["nodes"]}
    for nominal, level, f in ((317, 2, "C13H25N4O3S"), (261, 3, "C9H17N4O3S"), (244, 5, "C9H14N3O3S"), (200, 6, "C7H10N3O2S"), (172, 7, "C6H10N3OS"), (145, 8, "C5H9N2OS")):
        n = next((x for x in tree["nodes"] if x["path"][-1][0] == nominal and x["formula"] == f), None)
        assert n is not None, (nominal, f)


def test_sub_formulas_and_exact_precursors(tree):
    from mzlab.chem.msntree import _parse
    for n in tree["nodes"]:
        if n["parent"] is not None and n["formula"] and tree["nodes"][n["parent"]]["formula"]:
            top = _parse(tree["nodes"][n["parent"]]["formula"])
            assert all(top.get(e, 0) >= c for e, c in _parse(n["formula"]).items()), n["label"]
        if n["prec"] and n["ppm"] is not None:
            assert abs(n["ppm"]) < 5, n["label"]


def test_without_formula_offers_candidates():
    sys.path.insert(0, str(ROOT))
    from mzlab.reader.mzml import Run
    from mzlab.chem.msntree import msn_tree
    j = msn_tree(Run(str(_file())))
    assert not j["formula_given"] and len(j["root_candidates"]) >= 2


def test_dda_file_has_no_tree():
    from mzlab.reader.mzml import Run
    from mzlab.chem.msntree import msn_tree
    f = _file().parent.parent / "Orbitrap_Exploris120_DDApos_10-15min.mzML"
    if not f.is_file():
        pytest.skip("DDA file not available")
    with pytest.raises(ValueError):
        msn_tree(Run(str(f)))
