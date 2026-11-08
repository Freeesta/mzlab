"""The low-resolution criteria (tools/valida/lr*.py): each criterion on data built here, once passing and once failing, the tolerances, and a run of the
native pipeline on the synthetic series (lr_synth full scan, six-level MRM). The run needs cargo; real data are never read."""
import shutil
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from valida import lr  # noqa: E402
from valida import lr_criteri as C  # noqa: E402

BUILDABLE = shutil.which("cargo") is not None
D, F, A = C.CELL_DETECTED, C.CELL_FILLED, C.CELL_ABSENT


def mrm(stato, area, tipo=None, names=None):
    """An Mrm of 2 transitions x (6 standards + 2 treated): `stato`/`area` are the two rows of the standards (6) followed by the treated ones (2)."""
    n = len(stato[0])
    tipo = tipo or ["std"] * 6 + ["campione"] * (n - 6)
    return C.Mrm(names or [f"f{i}" for i in range(n)], np.array([263.1, 263.1]), np.array([181.1, 152.0]), np.array([5.0, 5.02]), np.array(stato), np.array(area, float), tipo,
                 [1.0, 2.0, 5.0, 10.0, 20.0, 50.0] + [None] * (n - 6), True)


RIGHT = ([[F, D, D, D, D, D, D, D], [D] * 8], [[40, 100, 250, 500, 1000, 2500, 300, 700], [10, 20, 50, 100, 200, 500, 60, 150]])


def values(crit):
    return {c["id"]: c["ok"] for c in crit}


def test_mrm_a_good_series_passes_and_the_log_log_r2_is_only_reported():
    crit, det = C.criteria_mrm(mrm(*RIGHT))
    assert all(c["ok"] is True for c in crit), crit
    assert det["r2_loglog"]["mediano"] > 0.99 and not any("r2" in c["id"] for c in crit)


def test_mrm_a_transition_missing_in_a_level_above_the_lowest_fails():
    stato = [list(r) for r in RIGHT[0]]
    stato[1][3] = A
    area = [list(r) for r in RIGHT[1]]
    area[1][3] = 0
    v = values(C.criteria_mrm(mrm(stato, area))[0])
    assert v["lr_mrm_rilevate_livelli"] is False and v["lr_mrm_stati_coerenti"] is True


def test_mrm_the_area_must_grow_with_the_concentration():
    area = [list(r) for r in RIGHT[1]]
    area[0][3], area[0][4] = 1500, 400                               # not monotone: Spearman below 1 for this transition
    crit, _ = C.criteria_mrm(mrm(RIGHT[0], area))
    assert crit[1]["valore"] == 0.5 and crit[1]["ok"] is False
    assert C.criteria_mrm(mrm(RIGHT[0], RIGHT[1]))[0][1]["ok"] is True


def test_mrm_incoherent_states_of_the_treated_files_fail():
    area = [list(r) for r in RIGHT[1]]
    area[0][6] = 0.0                                                 # detected with no area
    assert values(C.criteria_mrm(mrm(RIGHT[0], area))[0])["lr_mrm_stati_coerenti"] is False
    stato = [list(r) for r in RIGHT[0]]
    stato[0][7] = C.CELL_TARGETED                                    # a targeted cell does not exist in an MRM table
    assert values(C.criteria_mrm(mrm(stato, RIGHT[1]))[0])["lr_mrm_stati_coerenti"] is False


def test_mrm_without_the_table_is_not_measured_and_without_levels_too():
    assert all(c["ok"] is None for c in C.criteria_mrm(None)[0])
    assert all(c["ok"] is None for c in C.criteria_mrm(mrm(*RIGHT, tipo=["campione"] * 8))[0][:2])


def test_names_kinds_levels_and_statistics():
    assert [C.classifica(x) for x in ("B_MRM-STD_3", "B_MRM-t30", "B_FullMass-blank", "B_MS2-xx")] == ["std", "campione", "bianco", "altro"]
    assert C.level_from_name("B_MRM-STD_12ppb") == (pytest.approx(0.012), True) and C.level_from_name("B_MRM-STD_3") == (3.0, False) and C.level_from_name("x") == (None, False)
    assert C.spearman([1, 2, 3, 4], [1, 5, 9, 20]) == pytest.approx(1.0) and C.spearman([1, 2, 3], [5, 5, 5]) is None and C.spearman([1, 2, 3, 4], [1, 2, 2, 5]) < 1.0
    assert C.r2_loglog([1, 10, 100], [2, 20, 200]) == pytest.approx(1.0) and C.r2_loglog([1, 2], [1, 2]) is None


RIF = [{"mz": 257.37, "rt": 7.4}, {"mz": 257.37, "rt": 8.3}, {"mz": 333.49, "rt": 6.8}]


def test_fullscan_every_ion_of_the_reference_is_a_row_within_the_tolerances():
    mz, rt = [257.4, 257.4, 333.6], [7.5, 8.2, 6.7]                   # +0.23 Da at most, +0.1 min at most
    assert C.coverage(RIF, mz, rt)[:2] == (3, 3)
    n, tot, miss = C.coverage(RIF, [257.4, 333.6], [7.5, 6.7])      # the second isomer has no row
    assert (n, tot) == (2, 3) and miss[0]["rt"] == 8.3
    assert C.coverage(RIF, [257.4, 257.4, 333.9], [7.5, 8.2, 6.8])[0] == 2          # 0.41 Da away is another ion
    assert C.coverage(RIF, [257.4, 257.4, 333.6], [7.5, 8.2, 7.2])[0] == 2          # 0.4 min away too


def test_fullscan_the_threshold_is_95_percent():
    rif = [{"mz": 200.0 + i, "rt": 5.0} for i in range(20)]
    rows = [x["mz"] for x in rif]
    crit, _ = C.criteria_fullscan(rif, rows, [5.0] * 20, [], [])
    assert crit[0]["ok"] is True
    crit, _ = C.criteria_fullscan(rif, rows[1:], [5.0] * 19, [], [])
    assert crit[0]["valore"] == 0.95 and crit[0]["ok"] is True
    crit, _ = C.criteria_fullscan(rif, rows[2:], [5.0] * 18, [], [])
    assert crit[0]["valore"] == 0.9 and crit[0]["ok"] is False


def test_fullscan_a_candidate_needs_its_proofs():
    bene = {"formula": "C1", "contributi": [{"tipo": "RT", "segno": "neutra", "fonte": "x"}, {"tipo": "frequenza", "segno": "a favore", "fonte": "x"}]}
    neutral_only = {"formula": "C2", "contributi": [{"tipo": "RT", "segno": "neutra", "fonte": "x"}]}
    anonima = {"formula": "C3", "contributi": [{"tipo": "frequenza", "segno": "a favore", "fonte": ""}]}
    no_formula = {"formula": None, "contributi": []}                # an unexpected ion is not a candidate
    assert C.lr_candidates([bene, neutral_only, anonima, no_formula]) == [bene, neutral_only, anonima]
    ok = C.criteria_fullscan(RIF, [257.4, 257.4, 333.6], [7.4, 8.3, 6.8], [bene, no_formula], [])[0]
    assert ok[1]["valore"] == 1.0 and ok[1]["ok"] is True
    ko = C.criteria_fullscan(RIF, [257.4, 257.4, 333.6], [7.4, 8.3, 6.8], [bene, neutral_only, anonima], [])[0]
    assert ko[1]["ok"] is False and ko[1]["valore"] == pytest.approx(1 / 3, abs=1e-3)
    assert C.criteria_fullscan(RIF, [257.4], [7.4], [no_formula], [])[0][1]["ok"] is None


def test_fullscan_separated_isomers_are_two_rows():
    pairs = [((257.37, 7.4), (257.37, 8.3))]
    assert C.separated_isomers(pairs, [257.4, 257.4], [7.4, 8.3]) == (1, 1)
    assert C.separated_isomers(pairs, [257.4], [7.8]) == (0, 1)       # a fused row at 7.8 is outside both windows
    assert C.separated_isomers(pairs, [257.4], [7.9]) == (0, 1)       # one row near both ends is still one row
    assert C.isomer_pairs(RIF) == [((257.37, 7.4), (257.37, 8.3))] and C.isomer_pairs([{"mz": 257.0, "rt": 7.4}, {"mz": 257.2, "rt": 7.5}]) == []


def test_valley_below_half_separates():
    assert C.valley(7.4, 0.07, 1e6, 8.3, 0.07, 1e6) < C.VALLEY_MAX
    assert C.valley(7.4, 0.12, 1e6, 7.58, 0.12, 1e6) > C.VALLEY_MAX


def test_the_view_opens_only_if_every_criterion_was_measured_and_passes():
    ok = [{"id": "a", "ok": True}]
    assert lr.lr_table_view(ok) is True
    assert lr.lr_table_view(ok + [{"id": "b", "ok": None}]) is False and lr.lr_table_view(ok + [{"id": "b", "ok": False}]) is False and lr.lr_table_view([]) is False
    d = lr.document(ok, "2026-10-13", "1.2.3")
    assert d["format"] == "cancello_lr/1" and set(d) == {"criteri", "crate_version", "data", "format", "vista_tabella_lr"}


@pytest.mark.skipif(not BUILDABLE, reason="needs cargo")
def test_native_run_on_the_synthetic_series(tmp_path):
    crit, det = lr.sintetici(tmp_path)
    v = {c["id"]: c for c in crit}
    for i in ("mrm_rilevate_livelli", "mrm_area_cresce", "mrm_stati_coerenti", "ioni_python_in_tabella", "isomeri_due_righe", "determinismo"):
        assert v["lr_sint_" + i]["ok"] is True, v["lr_sint_" + i]
    assert v["lr_sint_candidati_con_prove"]["ok"] in (True, False) and det["fullscan"]["ioni_riferimento"] == 5 and det["mrm"]["standard"]["livelli"] == 6
    c, det2 = lr.misura(str(tmp_path / "nessuna"), None, tmp_path / "lavoro", "2026-10-13")
    assert c["vista_tabella_lr"] is False and "saltato" in det2["dati_reali"]
    assert all(k["ok"] is None for k in c["criteri"] if not k["id"].startswith("lr_sint_"))          # no data: skipped, never passed
    assert lr.a_json(c) == lr.a_json(lr.misura(str(tmp_path / "nessuna"), None, tmp_path / "lavoro2", "2026-10-13")[0])


@pytest.mark.skipif(not BUILDABLE, reason="needs cargo")
def test_native_run_a_broken_growth_and_a_flat_transition_fail_the_growth_criterion(tmp_path):
    from valida import lr_sintetici as S
    for name, kw, atteso in (("guasto", {"guasto": True}, 0.5), ("piatta", {"piatta": True}, 2 / 3)):
        crit, _, _ = lr.mrm(S.mrm_series(tmp_path / name, **kw), tmp_path / ("run_" + name))
        assert crit[1]["ok"] is False and crit[1]["valore"] == pytest.approx(atteso, abs=1e-3), (name, crit[1])
