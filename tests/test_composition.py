"""Elemental composition: the validation of the Banco HR specification, the limits, the rules, the sub-formula, the API and the speed."""
import time
import pytest
from mzlab.api import dispatch
from mzlab.chem.composition import compose, ion_mass, parse_elements, isotope_ratios, format_formula
from mzlab.chem.elements import parse_formula

ELS = "C:0-13,H:0-25,N:0-4,O:0-3,S:0-1"                            # the presets of the infusion file


def test_progenitor_of_the_infusion_file():
    r = compose(317.1639, elements=parse_elements(ELS), ion="[M+H]+", tol=5)
    top = r["results"][0]
    assert top["formula"] == "C13H25N4O3S" and top["neutral"] == "C13H24N4O3S" and abs(top["delta_ppm"]) < 1.5 and top["rdb"] == 4.0
    assert abs(top["iso"]["m1"] - 0.167) < 0.002                   # as ionfamily.isotope_pattern


@pytest.mark.parametrize("f", ["C9H17N4O3S", "C9H14N3O3S", "C6H10N3O2S", "C6H10N3OS", "C5H9N2OS"])
def test_nodes_of_the_msn_tree_with_the_subformula_of_the_precursor(f):
    m = ion_mass(parse_formula(f), 1)
    r = compose(m, elements=parse_elements(ELS), ion="[M]+", tol=3, parent="C13H25N4O3S", nitrogen="even", rules=["LEWIS"])
    assert r["results"][0]["formula"] == f


def test_subformula_excludes_what_the_parent_does_not_contain():
    m = ion_mass(parse_formula("C9H17N4O3S"), 1)
    free = compose(m, elements=parse_elements("C:0-13,H:0-30,N:0-6,O:0-6,S:0-2"), ion="[M]+", tol=10)
    sub = compose(m, elements=parse_elements("C:0-13,H:0-30,N:0-6,O:0-6,S:0-2"), ion="[M]+", tol=10, parent="C13H25N4O3S")
    assert sub["n_candidates"] <= free["n_candidates"] and all(parse_formula(x["formula"]).get("N", 0) <= 4 for x in sub["results"])


def test_ranges_and_tolerance_are_respected():
    r = compose(317.1639, elements=parse_elements("C:0-13,H:0-25,N:0-4,O:0-3,S:0-1"), tol=5)
    assert all(abs(x["delta_ppm"]) <= 5.01 for x in r["results"])
    none = compose(317.1639, elements=parse_elements("C:0-5,H:0-10"), tol=5)
    assert none["n_candidates"] == 0
    mda = compose(317.1639, elements=parse_elements(ELS), tol=0.1, unit="mda")           # 0.1 mDa: the 0.29 mDa error is out
    assert mda["n_candidates"] == 0


def test_isotopes_as_elements_and_adducts():
    m = ion_mass({"C": 12, "13C": 1, "H": 13, "N": 3, "O": 2}, 1)
    r = compose(m, elements=parse_elements("C:0-13,13C:0-2,H:0-20,N:0-4,O:0-4"), ion="[M]+", tol=2)
    assert any(x["formula"] == "C12H13N3O2" + "13C" for x in r["results"]) or any("13C" in x["formula"] for x in r["results"])
    na = compose(ion_mass({"C": 6, "H": 12, "O": 6, "Na": 1}, 1), elements=parse_elements("C:0-10,H:0-20,O:0-8"), ion="[M+Na]+", tol=3)
    assert na["results"][0]["formula"] == "C6H12NaO6" and na["results"][0]["neutral"] == "C6H12O6"


def test_rules_and_nitrogen_rule():
    big = parse_elements("C:0-12,H:0-30,N:0-6,O:0-6")
    m = ion_mass(parse_formula("C10H15N5O"), 1)
    allr = compose(m, elements=big, ion="[M+H]+", tol=10, max_results=200)
    ruled = compose(m, elements=big, ion="[M+H]+", tol=10, rules=["HC", "NC", "OC", "LEWIS", "SENIOR"], max_results=200)
    assert ruled["n_candidates"] <= allr["n_candidates"]
    assert all(not ({"HC", "NC", "OC", "LEWIS", "SENIOR"} & set(x["rules_failed"])) for x in ruled["results"])
    ev = compose(m, elements=big, ion="[M]+", tol=10, nitrogen="even", max_results=200)
    od = compose(m, elements=big, ion="[M]+", tol=10, nitrogen="odd", max_results=200)
    ne = lambda f, z: sum({"C": 6, "H": 1, "N": 7, "O": 8}[k] * v for k, v in parse_formula(f).items()) - z
    assert all(ne(x["formula"], 1) % 2 == 0 for x in ev["results"]) and all(ne(x["formula"], 1) % 2 == 1 for x in od["results"])


def test_isotope_check_and_helpers():
    t1, t2 = isotope_ratios(parse_formula("C13H25N4O3S"))
    r = compose(317.1639, elements=parse_elements(ELS), m1=t1 * 0.93, m2=t2)          # -7%: the usual Orbitrap offset
    assert r["results"][0]["iso"]["ok"] is True
    r = compose(317.1639, elements=parse_elements(ELS), m1=t1 * 1.6, m2=t2)
    assert r["results"][0]["iso"]["ok"] is False
    assert format_formula({"C": 2, "H": 6, "O": 1}) == "C2H6O" and format_formula({"H": 2, "C": 1, "Cl": 1, "13C": 1}) == "CH2Cl13C"


def test_errors():
    with pytest.raises(ValueError):
        parse_elements("C:5-2")
    with pytest.raises(ValueError):
        parse_elements("Xx:0-3")
    with pytest.raises(ValueError):
        compose(-1)
    with pytest.raises(ValueError):
        compose(300, ion="[M+Zz]+")


def _ratio_to_reference(work, ref, n=7):
    """work/ref in seconds: the two are timed alternately and each keeps its minimum (load only adds time), so a loaded runner slows both alike."""
    w = r = float("inf")
    for _ in range(n):
        t = time.perf_counter(); work(); w = min(w, time.perf_counter() - t)
        t = time.perf_counter(); ref(); r = min(r, time.perf_counter() - t)
    return w / r


def test_speed_with_chnosp_and_three_halogens():
    els = parse_elements("C:0-60,H:0-120,N:0-10,O:0-12,S:0-3,P:0-3,F:0-6,Cl:0-3,Br:0-2")

    def ref():                                                      # fixed pure-Python work that measures the speed of this machine
        s, d = 0.0, {}
        for i in range(300000):
            s += (i * 0.5) % 7; d[i % 97] = s

    # compose/ref is ~4.5 (0.11 s / 0.023 s); the limit is 10, so a 3x slowdown of compose (ratio ~13) fails and a loaded runner does not
    assert _ratio_to_reference(lambda: compose(799.5, elements=els, tol=5), ref) < 10


class _App:                                                         # the API does not touch the app for this route
    pass


def test_api_route():
    code, ct, body, _ = dispatch(_App(), "GET", "/api/composition", {"mz": "317.1639", "elements": ELS, "ion": "[M+H]+", "tol": "5", "unit": "ppm"})
    import json
    j = json.loads(body)
    assert code == 200 and j["results"][0]["formula"] == "C13H25N4O3S"
    code, _, body, _ = dispatch(_App(), "GET", "/api/composition", {"mz": "abc"})
    assert code == 400
    code, _, body, _ = dispatch(_App(), "GET", "/api/composition", {"mz": "300", "elements": "C:9-2"})
    assert code == 400 and "error" in json.loads(body)
