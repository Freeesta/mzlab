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
from valida import cancello_tp as T  # noqa: E402
from valida import cancello_tp_motore as TM  # noqa: E402
from valida.nativo import Series  # noqa: E402
from valida.tabelle import Table  # noqa: E402

BUILDABLE = shutil.which("cargo") is not None


def identita(i, mz, rt, gen, livello):
    c = [{"tipo": "cinetica", "segno": "a favore", "fonte": "x", "parametri": {"generation": gen, "shape": 0}}] if gen is not None else []
    return {"id": i, "totale": 1, "membri": [{"mz": mz, "rt": rt}], "contributi": c, "livello_max": livello, "ipotesi": None}


def series(ident, sha="a", tp=None):
    mz = np.array([x["membri"][0]["mz"] for x in ident]); rt = np.array([x["membri"][0]["rt"] for x in ident])
    tab = Table("nativo", ["f1", "f2"], mz, rt, np.full((len(ident), 2), 1e6))
    return Series(tabella=tab, stato=np.zeros((len(ident), 2), dtype=np.uint8), identita=ident, candidati=np.arange(len(ident)), per_file=[], sha={"s": sha}, versione_crate="9.9.9", tp=tp)


def tp(i, mz, rt, gen, level=4, **kw):
    return {"id": i, "mz": mz, "rt_obs": [rt], "gen_attesa": gen, "level": level, "rilevabile": True, **kw}


TRUTH = {"tps": [tp("A", 300.1, 2.0, 1), tp("B", 320.2, 3.0, 2), tp("C", 340.3, 4.0, 3)]}
GOOD = [identita(0, 300.1, 2.0, 0, "4"), identita(1, 320.2, 3.0, 1, "5"), identita(2, 340.3, 4.0, 2, "4")]       # the last one: undecidable generation, accepted


def test_generation_passes_when_right_or_undecidable_and_fails_when_wrong():
    m = TM.engine_measures(TRUTH, series(GOOD))
    assert m["generazione"] == 1.0 and m["generazione_n"] == 3
    sbagliata = [identita(0, 300.1, 2.0, 1, "4"), identita(1, 320.2, 3.0, 0, "4"), identita(2, 340.3, 4.0, None, "4")]       # the first product read as later, the second as first, no kinetics
    assert TM.engine_measures(TRUTH, series(sbagliata))["generazione"] == 0.0
    assert TM.engine_measures(TRUTH, series(GOOD[:2]))["generazione"] == 1.0                      # a product without a row is not counted here (tp_trovati does)


def test_level_never_better_than_the_true_one():
    assert TM.engine_measures(TRUTH, series(GOOD))["livelli"] == 1.0
    troppo = [identita(0, 300.1, 2.0, 0, "3"), identita(1, 320.2, 3.0, 1, "2a"), identita(2, 340.3, 4.0, 2, "4")]
    assert TM.engine_measures(TRUTH, series(troppo))["livelli"] == pytest.approx(1 / 3)


def test_region_is_not_measured_because_the_engine_writes_none():
    v = {"tps": [tp("A", 300.1, 2.0, 1, region="N-alkyl", loc_test=True), tp("B", 320.2, 3.0, 2)]}
    m = TM.engine_measures(v, series(GOOD))
    assert m["regione"] is None and m["regione_n"] == 1
    assert "regione" not in TM.engine_measures(TRUTH, series(GOOD))                               # no localization test in the truth


def test_determinism_compares_the_containers_and_the_tp_block():
    block = {"stato": np.array([0, 3, 0], dtype=np.uint8), "parentela": np.array([3, 0, 2], dtype=np.uint8)}
    equal = TM.engine_measures(TRUTH, series(GOOD, "a", dict(block)), series(GOOD, "a", {k: v.copy() for k, v in block.items()}))
    assert equal["determinismo_tp"] == 1
    assert TM.engine_measures(TRUTH, series(GOOD, "a", dict(block)), series(GOOD, "b", dict(block)))["determinismo_tp"] == 0
    other = {**block, "stato": np.array([0, 0, 0], dtype=np.uint8)}
    assert TM.engine_measures(TRUTH, series(GOOD, "a", dict(block)), series(GOOD, "a", other))["determinismo_tp"] == 0
    assert TM.engine_measures(TRUTH, series(GOOD, "a", None), series(GOOD, "a", None))["determinismo_tp"] == 0          # no tp/1 block: nothing to compare
    assert "determinismo_tp" not in TM.engine_measures(TRUTH, series(GOOD))


def test_weak_and_undetectable_products_are_not_counted():
    v = {"tps": [tp("A", 300.1, 2.0, 1), tp("D", 500.0, 5.0, 1, rilevabile=False), tp("E", 510.0, 5.0, 1, debole=True)]}
    assert TM.engine_measures(v, series(GOOD))["generazione_n"] == 1


def test_criteria_take_the_measures_and_stay_unmeasured_on_a_historical_truth():
    crit = {c["id"]: c for c in T.criteria_tp({"generazione": 0.92, "livelli": 1.0, "determinismo_tp": 1})}
    assert crit["generazione"]["ok"] is True and crit["livelli"]["ok"] is True and crit["determinismo_tp"]["ok"] is True and crit["regione"]["ok"] is None
    assert T.criteria_tp({"generazione": 0.5})[7]["ok"] is False
    doc, det = T.misura({"tps": [{"id": "A", "mz": 300.1, "rt_obs": [2.0]}]}, series(GOOD), [0, 1], "D-X", "2026-10-13")          # historical keys: no fields
    ok = {c["id"]: c["ok"] for c in doc["criteri"]}
    assert ok["generazione"] is None and ok["livelli"] is None and ok["regione"] is None and ok["determinismo_tp"] is None and doc["vista_tp"] is False
    assert det["motore"] == {}


@pytest.mark.skipif(not BUILDABLE, reason="needs cargo")
def test_native_run_on_the_synthetic_v1x_series(tmp_path):
    from valida import cancello
    from valida import sintetici_esteso_verita as V
    c, run, _, _ = cancello.run_synthetic(7, "V1X-piccolo", tmp_path / "a")
    _, run2, _, _ = cancello.run_synthetic(7, "V1X-piccolo", tmp_path / "b")
    verita = V.document(c, 7, "V1X-piccolo", 0)
    handled = [k for k, (_, tipo, _) in enumerate(c.file) if tipo == "trat"]
    doc, det = T.misura(verita, run, handled, "D-V1X", "2026-10-13", run2)
    ok = {k["id"]: k["ok"] for k in doc["criteri"]}
    assert det["motore"]["generazione_n"] > 20 and ok["generazione"] is True and ok["livelli"] is True and ok["determinismo_tp"] is True
    assert ok["regione"] is None and ok["esche"] is None and doc["vista_tp"] is False            # region, decoys and weak recall stay unmeasured: the view stays closed
