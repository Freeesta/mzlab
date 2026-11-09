"""The element ranges cut to the ion mass lose nothing in the Python reference either: every grid that runs without the cut gives the same answer
(the same cases as crates/mzlab-core/tests/composition_cut.rs), and a grid still over the limit names its size and the limit. Synthetic m/z values."""
import json

import pytest

from mzlab.chem import composition as comp
from mzlab.i18n import UserError

SETS = ["C:0-40,H:0-80,N:0-6,O:0-12,F:0-30,Cl:0-6,Br:0-4",
        "C:0-30,H:0-60,13C:0-3,2H:0-5,15N:0-2,O:0-10,S:0-4,Si:0-6,P:0-3",
        "C:0-20,H:0-40,Na:0-2,K:0-2,I:0-5,37Cl:0-4,81Br:0-3,34S:0-3,18O:0-3",
        "C:2-20,H:1-40,N:1-4,O:2-8,F:1-60,Cl:0-20",
        "N:0-50,O:0-50,S:0-50",
        ""]
IONS = ["[M+H]+", "[M+NH4]+", "[M+Na]+", "[M+K]+", "[M-H]-", "[M+Cl]-", "[M+HCOO]-", "M+.", "M-.", "[M]+", "[M]-"]


def _both(monkeypatch, mz, **kw):
    """(answer without the cut or None when that grid was over the limit, answer with the cut)."""
    with monkeypatch.context() as m:
        m.setattr(comp, "_clip_to_mass", lambda els, limit: None)
        try:
            before = comp.compose(mz, **kw)
        except UserError as e:
            assert e.key == "err.comp.many"
            before = None
    return before, comp.compose(mz, **kw)


# every charge with every tolerance (ppm and mDa, narrow and broad), one per m/z and ion in turn
STATES = [(z, tol, unit) for z in (None, 2, 3) for tol, unit in ((2.0, "ppm"), (50.0, "ppm"), (5.0, "mda"), (50.0, "mda"))]


@pytest.mark.parametrize("set_", SETS)
def test_the_cut_to_the_ion_mass_changes_no_answer(monkeypatch, set_):
    els = comp.parse_elements(set_) if set_ else None
    ran = 0
    for i, (mz, ion) in enumerate((mz, ion) for mz in (75.51, 181.0707, 455.1234, 1210.4) for ion in IONS):
        z, tol, unit = STATES[(i * 5 + SETS.index(set_)) % 12]
        if mz * (z or 1) > (600 if not set_ else 1300):          # heavy ions are slow: the default set leaves C and H open up to the ion mass
            continue
        before, after = _both(monkeypatch, mz, elements=els, ion=ion, z=z, tol=tol, unit=unit, max_results=25)
        if before is not None:
            assert json.dumps(before) == json.dumps(after), (set_, mz, ion, z, tol, unit)
            ran += 1
    assert ran > 15


def test_the_cut_keeps_the_scored_answer_and_the_parent(monkeypatch):
    for mz, parent in ((181.070665, None), (181.070665, "C6H12O6"), (300.1, "C12F10Cl2")):
        obs = [(mz, 100.0), (mz + 1.00336, 6.6), (mz + 2.0042, 1.2)]
        before, after = _both(monkeypatch, mz, elements=comp.parse_elements(SETS[0]), tol=20.0, parent=parent, obs=obs)
        assert json.dumps(before) == json.dumps(after), parent


def test_a_grid_wider_than_the_mass_now_runs(monkeypatch):
    before, after = _both(monkeypatch, 300.0, elements=comp.parse_elements("C:0-30,H:0-60,F:0-200,Cl:0-50,Br:0-50,Si:0-6"))
    assert before is None and "results" in after


def test_a_grid_still_over_the_limit_names_its_size_and_the_limit():
    with pytest.raises(UserError) as e:
        comp.compose(3000.0, elements=comp.parse_elements("N:0-150,O:0-150,F:0-150"))
    assert e.value.key == "err.comp.many" and e.value.params == {"n": 3442951, "max": 3000000}
