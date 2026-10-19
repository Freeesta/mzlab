"""The measures of `tools/valida/tp_timololo.py` over invented rows (no data repository needed)."""
import importlib.util
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("tp_timololo", Path(__file__).resolve().parents[1] / "tools/valida/tp_timololo.py")
M = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(M)

ROWS = [
    {"id": 1, "mz": 300.1000, "rt": 5.0, "delta": "+O", "confidence": "region", "region": "ring A"},
    {"id": 2, "mz": 400.0000, "rt": 3.0, "delta": "+2O", "confidence": "cannot_say", "region": ""},
]


def test_agreement_disagreement_and_new_candidates():
    truth = {"tps": [{"id": "a", "mz": 300.1002, "rt_obs": [5.1], "region": "ring A"}, {"id": "b", "mz": 300.1002, "rt": 5.0, "region": "chain A"}]}
    m = M.measure({"rows": ROWS, "decoys": {"rules": 20, "survived": 1}}, truth)
    assert (m["tp_in_view"], m["region_agree"], m["region_checked"]) == (2, 1, 2)
    assert [d[0] for d in m["site_disagreements"]] == ["b"]
    assert [r["id"] for r in m["new_candidates"]] == [2]
    assert m["fdr_decoys"] == 0.05
    assert "not the truth" in M.render(m)


def test_a_row_outside_the_ppm_window_is_not_found():
    assert M.find(ROWS, {"mz": 300.2, "rt": 5.0}) == []


def test_cannot_say_is_no_evidence_not_disagreement():
    rows = [{"id": 2, "mz": 400.0, "rt": 3.0, "delta": "+2O", "confidence": "cannot_say", "region": "", "reasons": ["few_fragments"]}]
    m = M.measure({"rows": rows}, {"tps": [{"id": "c", "mz": 400.0, "rt": 3.0, "region": "ring A"}]})
    assert (m["region_checked"], m["site_disagreements"]) == (0, [])
    assert m["no_evidence"] == [("c", "ring A", ["few_fragments"])]


def test_a_row_without_change_is_no_evidence():
    rows = [{"id": 3, "mz": 500.0, "rt": 2.0, "delta": "", "confidence": "", "region": ""}]
    m = M.measure({"rows": rows}, {"tps": [{"id": "d", "mz": 500.0, "region": "chain A"}]})
    assert (m["region_checked"], len(m["no_evidence"])) == (0, 1)
    assert m["no_evidence"][0][2] == ["no_change_proposal"]
