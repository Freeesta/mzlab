"""The TP criteria that read what the engine says (tools/valida/cancello_tp_motore.py): generation, level, region and determinism, each passing and failing on a
small hand-made series, left not measured when the truth has no field for them (the historical keys of the real series), and a run of the native pipeline on
the synthetic V1X series with its own truth `verita_tp/1`. No real data."""
import shutil
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
if not (ROOT / "TP_Mine" / "py").exists():
    pytest.skip("TP_Mine/py not available", allow_module_level=True)
from valida import cancello_tp as T  # noqa: E402
from valida import cancello_tp_motore as TM  # noqa: E402
from valida.nativo import Serie  # noqa: E402
from valida.tabelle import Tabella  # noqa: E402

COSTRUIBILE = shutil.which("cargo") is not None


def identita(i, mz, rt, gen, livello):
    c = [{"tipo": "cinetica", "segno": "a favore", "fonte": "x", "parametri": {"generation": gen, "shape": 0}}] if gen is not None else []
    return {"id": i, "totale": 1, "membri": [{"mz": mz, "rt": rt}], "contributi": c, "livello_max": livello, "ipotesi": None}


def serie(ident, sha="a", tp=None):
    mz = np.array([x["membri"][0]["mz"] for x in ident]); rt = np.array([x["membri"][0]["rt"] for x in ident])
    tab = Tabella("nativo", ["f1", "f2"], mz, rt, np.full((len(ident), 2), 1e6))
    return Serie(tabella=tab, stato=np.zeros((len(ident), 2), dtype=np.uint8), identita=ident, candidati=np.arange(len(ident)), per_file=[], sha={"s": sha}, versione_crate="9.9.9", tp=tp)


def tp(i, mz, rt, gen, level=4, **kw):
    return {"id": i, "mz": mz, "rt_obs": [rt], "gen_attesa": gen, "level": level, "rilevabile": True, **kw}


VERITA = {"tps": [tp("A", 300.1, 2.0, 1), tp("B", 320.2, 3.0, 2), tp("C", 340.3, 4.0, 3)]}
BUONA = [identita(0, 300.1, 2.0, 0, "4"), identita(1, 320.2, 3.0, 1, "5"), identita(2, 340.3, 4.0, 2, "4")]       # the last one: undecidable generation, accepted


def test_generation_passes_when_right_or_undecidable_and_fails_when_wrong():
    m = TM.misure_motore(VERITA, serie(BUONA))
    assert m["generazione"] == 1.0 and m["generazione_n"] == 3
    sbagliata = [identita(0, 300.1, 2.0, 1, "4"), identita(1, 320.2, 3.0, 0, "4"), identita(2, 340.3, 4.0, None, "4")]       # the first product read as later, the second as first, no kinetics
    assert TM.misure_motore(VERITA, serie(sbagliata))["generazione"] == 0.0
    assert TM.misure_motore(VERITA, serie(BUONA[:2]))["generazione"] == 1.0                      # a product without a row is not counted here (tp_trovati does)


def test_level_never_better_than_the_true_one():
    assert TM.misure_motore(VERITA, serie(BUONA))["livelli"] == 1.0
    troppo = [identita(0, 300.1, 2.0, 0, "3"), identita(1, 320.2, 3.0, 1, "2a"), identita(2, 340.3, 4.0, 2, "4")]
    assert TM.misure_motore(VERITA, serie(troppo))["livelli"] == pytest.approx(1 / 3)


def test_region_is_not_measured_because_the_engine_writes_none():
    v = {"tps": [tp("A", 300.1, 2.0, 1, region="N-alkyl", loc_test=True), tp("B", 320.2, 3.0, 2)]}
    m = TM.misure_motore(v, serie(BUONA))
    assert m["regione"] is None and m["regione_n"] == 1
    assert "regione" not in TM.misure_motore(VERITA, serie(BUONA))                               # no localization test in the truth


def test_determinism_compares_the_containers_and_the_tp_block():
    blocco = {"stato": np.array([0, 3, 0], dtype=np.uint8), "parentela": np.array([3, 0, 2], dtype=np.uint8)}
    uguale = TM.misure_motore(VERITA, serie(BUONA, "a", dict(blocco)), serie(BUONA, "a", {k: v.copy() for k, v in blocco.items()}))
    assert uguale["determinismo_tp"] == 1
    assert TM.misure_motore(VERITA, serie(BUONA, "a", dict(blocco)), serie(BUONA, "b", dict(blocco)))["determinismo_tp"] == 0
    altro = {**blocco, "stato": np.array([0, 0, 0], dtype=np.uint8)}
    assert TM.misure_motore(VERITA, serie(BUONA, "a", dict(blocco)), serie(BUONA, "a", altro))["determinismo_tp"] == 0
    assert TM.misure_motore(VERITA, serie(BUONA, "a", None), serie(BUONA, "a", None))["determinismo_tp"] == 0          # no tp/1 block: nothing to compare
    assert "determinismo_tp" not in TM.misure_motore(VERITA, serie(BUONA))


def test_weak_and_undetectable_products_are_not_counted():
    v = {"tps": [tp("A", 300.1, 2.0, 1), tp("D", 500.0, 5.0, 1, rilevabile=False), tp("E", 510.0, 5.0, 1, debole=True)]}
    assert TM.misure_motore(v, serie(BUONA))["generazione_n"] == 1


def test_criteria_take_the_measures_and_stay_unmeasured_on_a_historical_truth():
    crit = {c["id"]: c for c in T.criteri_tp({"generazione": 0.92, "livelli": 1.0, "determinismo_tp": 1})}
    assert crit["generazione"]["ok"] is True and crit["livelli"]["ok"] is True and crit["determinismo_tp"]["ok"] is True and crit["regione"]["ok"] is None
    assert T.criteri_tp({"generazione": 0.5})[7]["ok"] is False
    doc, det = T.misura({"tps": [{"id": "A", "mz": 300.1, "rt_obs": [2.0]}]}, serie(BUONA), [0, 1], "D-X", "2026-10-13")          # historical keys: no fields
    ok = {c["id"]: c["ok"] for c in doc["criteri"]}
    assert ok["generazione"] is None and ok["livelli"] is None and ok["regione"] is None and ok["determinismo_tp"] is None and doc["vista_tp"] is False
    assert det["motore"] == {}


@pytest.mark.skipif(not COSTRUIBILE, reason="needs cargo")
def test_native_run_on_the_synthetic_v1x_series(tmp_path):
    from valida import cancello
    from valida import sintetici_esteso_verita as V
    c, run, _, _ = cancello.esegui_sintetico(7, "V1X-piccolo", tmp_path / "a")
    _, run2, _, _ = cancello.esegui_sintetico(7, "V1X-piccolo", tmp_path / "b")
    verita = V.documento(c, 7, "V1X-piccolo", 0)
    trattate = [k for k, (_, tipo, _) in enumerate(c.file) if tipo == "trat"]
    doc, det = T.misura(verita, run, trattate, "D-V1X", "2026-10-13", run2)
    ok = {k["id"]: k["ok"] for k in doc["criteri"]}
    assert det["motore"]["generazione_n"] > 20 and ok["generazione"] is True and ok["livelli"] is True and ok["determinismo_tp"] is True
    assert ok["regione"] is None and ok["esche"] is None and doc["vista_tp"] is False            # region, decoys and weak recall stay unmeasured: the view stays closed
