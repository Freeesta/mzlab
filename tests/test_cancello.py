"""The gate of quality (tools/valida/cancello.py): the document `cancello/2`, the rule that opens the candidate view, the validity bound to the version of the crate,
the determinism of the file, a forced «does not pass», the agreement with mzmine, and a run of the real pipeline on the small synthetic series (profile `piccolo`).
Skipped without cargo."""
import hashlib
import json
import shutil
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
import valida_feat as VF  # noqa: E402
from valida import cancello as C  # noqa: E402
from valida import cancello_misure as M  # noqa: E402
from valida import rapporto  # noqa: E402
from valida.tabelle import Tabella  # noqa: E402

COSTRUIBILE = shutil.which("cargo") is not None
DATA = "2026-10-12"
FORMAT_KEYS = {"concordanza_mzmine", "crate_version", "criteri", "data", "format", "vista_candidati", "vista_tabella"}


def tutti_ok() -> list:
    """Every criterion passing: the document a perfect run would write (mzmine passes too, speed and memory are not measured)."""
    return ([C.crit(i, 1, 1, "==") for i in C.ID_RICHIESTI] + C.criteri_mzmine({"richiamo": 0.95, "precisione": 0.9, "spearman": 0.95, "log2_mediano": 0.1}) + C.criteri_prestazioni())


def senza(crit: list, *ids) -> list:
    return [k for k in crit if k["id"] not in ids]


def test_crit_and_open_rule():
    assert C.crit("a", 3, 2, ">=")["ok"] is True and C.crit("a", 1, 2, ">=")["ok"] is False and C.crit("a", None, 2, ">=")["ok"] is None
    assert C.crit("a", 0.5, 0.5, "<=")["ok"] is True and C.crit("a", 1, 1, "==")["ok"] is True
    c = C.cancello(tutti_ok(), DATA, "1.2.3")
    assert set(c) == FORMAT_KEYS and c["format"] == "cancello/2" and c["vista_candidati"] is True and c["vista_tabella"] is True and c["concordanza_mzmine"] == "passa"
    assert all(set(k) == {"id", "ok", "soglia", "valore"} for k in c["criteri"])


def test_forced_failure_closes_the_view_and_names_the_criterion():
    crit = tutti_ok()
    crit[C.ID_RICHIESTI.index("sintetici_errore_area_mediano")] = C.crit("sintetici_errore_area_mediano", 0.2, 0.05, "<=")
    c = C.cancello(crit, DATA, "1.2.3")
    assert c["vista_candidati"] is False
    assert [k["id"] for k in c["criteri"] if k["ok"] is False] == ["sintetici_errore_area_mediano"]
    assert "FAIL" in rapporto.cancello_markdown(c, {})


def test_missing_required_measure_keeps_it_closed_but_optional_ones_do_not():
    crit = tutti_ok()
    assert C.cancello(crit, DATA, "1")["vista_candidati"] is True
    sem = senza(crit, "mzmine_richiamo", "mzmine_precisione", "mzmine_spearman", "mzmine_log2_mediano", "prestazioni_13_file") + C.criteri_mzmine(None)
    c = C.cancello(sem, DATA, "1")
    assert c["vista_candidati"] is True and c["concordanza_mzmine"] == "non_misurata"          # mzmine and speed/memory are optional
    assert C.cancello(senza(crit, "determinismo_nativo_wasm") + C.criteri_determinismo(True, None)[1:], DATA, "1")["vista_candidati"] is False      # not measured
    assert C.cancello(senza(crit, "determinismo_nativo_wasm"), DATA, "1")["vista_candidati"] is False                                               # missing
    assert C.cancello(senza(crit, "timololo_tp_trovati", "timololo_gruppi") + C.criteri_timololo(None)[:1], DATA, "1")["vista_candidati"] is False


def test_valid_only_for_the_crate_that_was_run():
    c = C.cancello(tutti_ok(), DATA, C.versione_crate())
    assert C.valido(c, C.versione_crate())
    assert not C.valido(c, "9.9.9")
    assert not C.valido(C.cancello(senza(tutti_ok(), "timololo_gruppi"), DATA, C.versione_crate()), C.versione_crate())
    assert C.valido(dict(c, format="cancello/1"), C.versione_crate()) and not C.valido(dict(c, format="cancello/3"), C.versione_crate()) and not C.valido(None, "1")


def test_table_view_is_wider_than_the_candidate_view():
    assert set(C.ID_TABELLA) < set(C.ID_RICHIESTI)
    crit = tutti_ok()
    c = C.cancello(senza(crit, "sintetici_cinetica_forma") + [C.crit("sintetici_cinetica_forma", 0, 1, ">=")], DATA, "1")
    assert c["vista_tabella"] is True and c["vista_candidati"] is False               # a kinetic criterion closes only the candidates
    assert C.valido_tabella(c, "1") and not C.valido(c, "1") and not C.valido_tabella(c, "2")
    c = C.cancello(senza(crit, "timololo_gruppi") + [C.crit("timololo_gruppi", 9, 1, "<=")], DATA, "1")
    assert c["vista_tabella"] is False and c["vista_candidati"] is False
    old = {"format": "cancello/1", "crate_version": "1", "vista_candidati": True}
    assert C.valido_tabella(old, "1") and not C.valido_tabella(dict(old, vista_candidati=False), "1")


def test_json_is_deterministic_and_has_no_names():
    a, b = C.a_json(C.cancello(tutti_ok(), DATA, "1.2.3")), C.a_json(C.cancello(tutti_ok(), DATA, "1.2.3"))
    assert a == b and hashlib.sha256(a.encode()).hexdigest() == hashlib.sha256(b.encode()).hexdigest()
    assert json.loads(a)["data"] == DATA


def test_timolol_criteria_follow_the_baseline_of_v0():
    ok = C.criteri_timololo({"trovati": 21, "nei_candidati": 19, "candidati": 494, "righe": 25606})
    assert all(k["ok"] for k in ok)
    peggio = C.criteri_timololo({"trovati": 20, "nei_candidati": 18, "candidati": 495, "righe": 25607})
    assert not any(k["ok"] for k in peggio)


def test_agreement_with_mzmine_on_an_identical_and_on_a_shuffled_table():
    rng = np.random.default_rng(1)
    n = 300
    mz, rt = np.sort(rng.uniform(150, 800, n)), rng.uniform(1, 20, n)
    area = np.exp(rng.uniform(np.log(1e6), np.log(1e9), (n, 3)))
    a = Tabella("nativa", ["x", "y", "z"], mz, rt, area)
    stessa = M.concordanza_mzmine(a, Tabella("mzmine", ["x", "y", "z"], mz * (1 + 1e-6), rt + 0.01, area * 3.0), [0, 1, 2], 50.0)
    assert stessa["richiamo"] == 1.0 and stessa["precisione"] == 1.0 and stessa["spearman"] > 0.999 and stessa["log2_mediano"] < 1e-9
    altra = M.concordanza_mzmine(a, Tabella("mzmine", ["x", "y", "z"], mz, rt + 5.0, area), [0, 1, 2], 50.0)
    assert altra["richiamo"] < 0.1
    c = C.cancello(tutti_ok()[:3] + C.criteri_mzmine(altra), DATA, "1")
    assert c["concordanza_mzmine"] == "non_passa"


@pytest.fixture(scope="module")
def piccolo(tmp_path_factory):
    if not COSTRUIBILE:
        pytest.skip("cargo not available")
    pytest.importorskip("numpy")
    lavoro = tmp_path_factory.mktemp("cancello")
    c, det = C.misura(None, [], None, "piccolo", 2, None, lavoro, DATA, wasm=False)
    return c, det, lavoro


def test_real_pipeline_on_small_synthetic_series(piccolo):
    c, det, _ = piccolo
    per_id = {k["id"]: k for k in c["criteri"]}
    assert c["format"] == "cancello/2" and c["crate_version"] == C.versione_crate() and c["data"] == DATA
    assert per_id["determinismo_due_esecuzioni"]["ok"] is True                    # two runs, the same SHA-256 of every .mzfeat / .mzfeatf
    assert per_id["sintetici_richiamo_sopra_10x"]["ok"] is True and per_id["sintetici_purezza_errore_mediano"]["ok"] is True
    assert per_id["timololo_tp_trovati"]["ok"] is None and per_id["prestazioni_13_file"]["ok"] is None
    assert c["vista_candidati"] is False                                         # real series and wasm not measured: the gate stays closed
    assert det["sintetici"]["righe"] > 0 and det["sintetici"]["misure"]["richiamo_forte"]["n"] > 0


def test_the_file_of_the_run_is_valid_for_its_crate_only(piccolo):
    c, _, lavoro = piccolo
    forzato = C.cancello([dict(k, ok=True) if k["ok"] is False else k for k in c["criteri"]], DATA, c["crate_version"])
    assert C.valido(dict(forzato, vista_candidati=True), c["crate_version"]) and not C.valido(dict(forzato, vista_candidati=True), "0.0.0-altra")
    p = rapporto.scrivi_cancello(lavoro / "uscita", c, {"sintetici": {"profilo": "piccolo", "seme": 2}}, lavoro / "uscita" / "cancello.json")
    assert json.loads(p[0].read_text(encoding="utf-8")) == c and p[0].read_text(encoding="utf-8") == C.a_json(c)
    assert "| criterion |" in p[2].read_text(encoding="utf-8")


def test_gate_of_the_run_is_reproducible(piccolo, tmp_path):
    c, _, lavoro = piccolo
    c2, _ = C.misura(None, [], None, "piccolo", 2, None, tmp_path, DATA, wasm=False)
    assert C.a_json(c) == C.a_json(c2)
    assert hashlib.sha256(C.a_json(c).encode()).hexdigest() == hashlib.sha256(C.a_json(c2).encode()).hexdigest()


def test_option_in_the_command_line_writes_the_files(tmp_path):
    if not COSTRUIBILE:
        pytest.skip("cargo not available")
    dest = tmp_path / "cancello.json"
    assert VF.main(["--cancello", "--sintetico", "piccolo", "--seme", "2", "--senza-wasm", "--data", DATA, "--lavoro", str(tmp_path / "l"), "--cancello-json", str(dest)]) == 0
    assert json.loads(dest.read_text(encoding="utf-8"))["format"] == "cancello/2" and (tmp_path / "l" / "cancello_rapporto.md").exists()
