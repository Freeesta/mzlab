# TPMINE-PRIVATE
"""Identities on the real HR files of the private data repository (skipped without it): the features of the time series are detected and aligned like in the engine, and the
protonated parent, its sodium adduct, its isotopologues and its in-source fragments (the m/z are read from the truth file of the data repository, `scratchpad/*_truth.json`)
are ONE identity with the main hypothesis [M+H]+; no compound, formula or m/z of the test set is in this public file."""
import json
import os
import re
from pathlib import Path

import pytest

from mzlab.reader.mzml import Run
from tpmine.hr import features, identity

ROOT = Path(__file__).resolve().parent.parent.parent


def _dati():
    for c in (os.environ.get("MZLAB_DATI"), os.environ.get("QQQ_DATI"), str(ROOT.parent / "mzlab-dati"), str(ROOT.parent / "QqQ-lab-dati")):
        if c and (Path(c) / "scratchpad").is_dir():
            return Path(c)
    return None


DATI = _dati()
TRUTH = None
if DATI:
    for f in sorted((DATI / "scratchpad").glob("*_truth.json")):
        try:
            if "tps" in json.loads(f.read_text(encoding="utf-8")):
                TRUTH = f
        except ValueError:
            pass
need = pytest.mark.skipif(TRUTH is None or not (DATI / "HRMS").is_dir(), reason="MZLAB_DATI with the truth file and HRMS/ not available")
NSERIES = 8                    # the first files of the time series are enough to compare the shapes


@pytest.fixture(scope="module")
def table():
    truth = json.loads(TRUTH.read_text(encoding="utf-8"))
    files = sorted(p for p in (DATI / "HRMS").rglob("*.mzML") if re.search(r"t\d{3}min", p.name))[:NSERIES]
    assert len(files) >= 4
    al = features.align([features.detect_file(Run(f), 1) for f in files])
    rows = [{"k": f"POS:{i}", "id": str(i), "mz": float(al.mz[i]), "rt": float(al.rt[i]), "a": [[j, float(v)] for j, v in enumerate(al.area[i]) if v > 0]}
            for i in range(len(al)) if (al.area[i] > 0).sum() >= 3]
    res = identity.build({"pol": 1, "samples": [f.stem for f in files], "features": rows, "opt": {"cycle_s": 0.6}})
    return truth, rows, res


def _near(rows, mz, rt, ppm=5.0, drt=0.15):
    return [r["k"] for r in rows if abs(r["mz"] - mz) <= mz * ppm * 1e-6 and abs(r["rt"] - rt) <= drt]


@need
def test_parent_and_its_adduct_isotope_and_fragments_are_one_identity(table):
    truth, rows, res = table
    p = truth["parent"]
    notes = [float(x) for x in re.findall(r"\b\d{3}\.\d{4}\b", p["notes"])]
    na = [m for m in notes if abs(m - (p["mz"] + 21.98194)) < 0.001]
    isf = [m for m in notes if m < p["mz"] - 1]
    parent = _near(rows, p["mz"], p["rt_obs"])
    assert parent, "parent not found in the table"
    idn = next(x for x in res["identities"] if parent[0] in x["members"])
    mzof = {r["k"]: r["mz"] for r in rows}
    got = [mzof[k] for k in idn["members"]]
    assert idn["neutral"]["form"] == "[M+H]+" and abs(idn["neutral"]["mass"] - (p["mz"] - 1.007276)) < 0.001
    assert na and any(abs(g - na[0]) < na[0] * 5e-6 for g in got), "sodium adduct not in the identity"
    # the fragments whose loss is a known neutral (the two first of the list: alkene loss, then alkene + NH3) join; the one whose loss is not in the table of losses does not
    assert all(any(abs(g - m) < m * 10e-6 for g in got) for m in isf[:2]), "in-source fragments not in the identity"
    assert any(abs(g - (p["mz"] + 1.003355)) < p["mz"] * 5e-6 for g in got), "13C isotopologue not in the identity"
    assert any(e["source"] == "computed" and e["r"] >= 0.85 for e in res["edges"])
