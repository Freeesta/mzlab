"""Validation harness (tools/valida, tools/valida_feat.py) on synthetic HR series with a truth built here: every number and formula is invented."""
import json
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
if not (ROOT / "TP_Mine" / "py").exists():
    pytest.skip("TP_Mine/py not available", allow_module_level=True)
import dati_sintetici as ds  # noqa: E402
import valida_feat as VF  # noqa: E402
from valida import abbina as AB  # noqa: E402
from valida import isotopologhi as ISO  # noqa: E402
from valida import metriche as M  # noqa: E402
from valida import rapporto as RP  # noqa: E402
from valida import tabelle as T  # noqa: E402
from tpmine.hr import filters as FL  # noqa: E402

PARENT, PROD_A, PROD_B = "C15H22N3O3", "C15H22N3O4", "C15H20N3O3"        # invented ions (the [M+H]+ formulas)
RT_P, RT_A, RT_B = 5.4, 4.6, 6.3


def _ion(formula, rt, amp, sg=0.05):
    mz, rel = ISO.componenti(formula)
    return [(mz[k], rt, sg, amp * rel[k] / 100.0) for k in range(3)]


@pytest.fixture(scope="module")
def serie(tmp_path_factory):
    d = tmp_path_factory.mktemp("hrserie")
    rng = np.random.default_rng(3)
    plan = {"S_dark": (1e7, 0.0, 0.0), "S_t05min": (8e6, 2e6, 0.0), "S_t10min": (5e6, 5e6, 2e6), "S_t20min": (2e6, 8e6, 6e6)}
    for name, (p, a, b) in plan.items():
        feats = _ion(PARENT, RT_P, p) + _ion(PROD_A, RT_A, a) + _ion(PROD_B, RT_B, b)
        feats += [(210.1234, 3.1, 0.06, 4e6), (480.5, 7.0, 0.06, 3e6)]               # constant ions of the matrix
        ds.hr_series(d / f"{name}.mzML", rng, feats, rt_end=9.0)
    return d


@pytest.fixture(scope="module")
def verita():
    return [{"id": "par", "mz": ISO.componenti(PARENT)[0][0], "ion": PARENT, "rt_obs": [RT_P], "parent": True},
            {"id": "A", "mz": ISO.componenti(PROD_A)[0][0], "ion": PROD_A, "rt_obs": [RT_A]},
            {"id": "B", "mz": ISO.componenti(PROD_B)[0][0], "ion": PROD_B, "rt_obs": [RT_B]},
            {"id": "C", "mz": 400.1, "ion": "C20H20N3O4", "rt_obs": []}]


@pytest.fixture(scope="module")
def rep(serie, verita):
    return VF.valida(T.trova_serie(serie), verita, sensibilita=True, nome="sintetica")


def test_abbina_one_to_one_and_deterministic():
    mz_a, rt_a = [100.0, 100.0003, 200.0], [1.0, 1.0, 2.0]
    mz_b, rt_b = [100.00002, 200.1], [1.0, 2.0]
    ia, ib = AB.abbina(mz_a, rt_a, mz_b, rt_b, 5.0, 0.15)
    assert ia.tolist() == [0] and ib.tolist() == [0]                  # the closest wins, b is used once; 200 vs 200.1 is far in ppm
    assert (ia, ib) == (ia, ib) and all(np.array_equal(x, y) for x, y in zip(AB.abbina(mz_a, rt_a, mz_b, rt_b), (ia, ib)))
    ia, ib = AB.abbina([100.0, 100.0], [1.0, 1.0], [100.0], [1.0])      # exact tie: the lower index of the first list
    assert ia.tolist() == [0]
    assert AB.abbina([], [], [1.0], [1.0])[0].size == 0


def test_series_is_found_and_ordered(serie):
    s = T.trova_serie(serie)
    assert [x["nome"] for x in s] == ["S_dark", "S_t05min", "S_t10min", "S_t20min"]
    assert [x["time"] for x in s] == [0.0, 5.0, 10.0, 20.0]


def test_reference_peaks_follow_the_formula(rep):
    comp = rep["tabelle"]["mzFinder riempita"]["completezza"]
    assert comp["M"]["rilevabili"] == 3 and comp["M"]["frazione"] == 1.0
    assert comp["M+1"]["rilevabili"] >= 2 and comp["M+1"]["frazione"] == 1.0
    r = rep["tabelle"]["mzFinder riempita"]["rapporto_isotopico"]
    assert r["riferimento"]["n"] >= 2 and abs(r["riferimento"]["mediana"]) < 0.1
    assert abs(r["tabella"]["mediana"]) < 0.15


def test_true_products_found_and_ranked(rep):
    tp = rep["tabelle"]["mzFinder riempita"]["tp_veri"]
    assert tp["riassunto"]["valutabili"] == 2 and tp["riassunto"]["trovati"] == 2 and tp["riassunto"]["senza_rt_osservato"] == ["C"]
    assert tp["riassunto"]["nei_candidati"] == 2
    d = {x["id"]: x for x in tp["dettaglio"]}
    assert d["A"]["rango_candidati"] in (1, 2, 3, 4) and abs(d["A"]["ppm"]) < 3 and abs(d["B"]["drt"]) < 0.1
    assert rep["funnel"][0]["n"] > rep["funnel"][-1]["n"] > 0


def test_stages_and_sensitivity(rep):
    t = rep["tabelle"]
    assert t["mzFinder rilevate"]["righe"]["righe"] >= t["mzFinder allineata"]["righe"]["righe"] > 5
    assert "tp_veri" not in t["mzFinder rilevate"]
    assert set(rep["sensibilita"]) == {"0.8", "1.2"} and rep["sensibilita"]["0.8"]["righe"] >= rep["sensibilita"]["1.2"]["righe"]
    assert rep["prestazioni"]["memoria_massima_mb"] > 0 and rep["prestazioni"]["totale_s"] > 0


def test_two_runs_give_the_same_json(serie, verita):
    s = T.trova_serie(serie)
    a = RP.a_json(VF.valida(s, verita, sensibilita=False), prestazioni=False)
    b = RP.a_json(VF.valida(s, verita, sensibilita=False), prestazioni=False)
    assert a == b and "prestazioni" not in a


def test_cli_writes_report_and_is_byte_identical(serie, tmp_path):
    truth = tmp_path / "truth.json"
    truth.write_text(json.dumps({"parent": {"name": "par", "mz": 1.0, "ion": PARENT, "rt_obs": RT_P},
                                 "tps": [{"id": "A", "mz": ISO.componenti(PROD_A)[0][0], "ion": PROD_A, "rt_obs": [RT_A]}]}), encoding="utf-8")
    outs = []
    for i in (1, 2):
        out = tmp_path / f"o{i}"
        assert VF.main(["--serie", str(serie), "--verita", str(truth), "--uscita", str(out), "--senza-sensibilita", "--deterministico"]) == 0
        outs.append(out)
    assert (outs[0] / "rapporto.json").read_bytes() == (outs[1] / "rapporto.json").read_bytes()
    assert (outs[0] / "rapporto.md").read_bytes() == (outs[1] / "rapporto.md").read_bytes()
    assert "True products" in (outs[0] / "rapporto.md").read_text(encoding="utf-8")


def test_mzmine_csv_and_jaccard(serie, verita, tmp_path):
    s = T.trova_serie(serie)
    es = T.esegui_mzfinder(s, None)
    al = es.allineata
    names = [x["nome"] for x in s]
    head = ["row ID", "row m/z", "row retention time"] + [f"datafile:{n}.mzML:area" for n in names]
    lines = [";".join(head)]
    for i in range(len(al)):
        lines.append(";".join([str(i + 1), repr(float(al.mz[i])), repr(float(al.rt[i]))] + [repr(float(v)) for v in al.area[i]]))
    csv_ok = tmp_path / "mzmine.csv"
    csv_ok.write_text("\n".join(lines), encoding="utf-8")
    mm = T.leggi_mzmine(csv_ok, s)
    assert len(mm) == len(al) and mm.campioni == names
    _, tr = FL.split_columns(*T.tipi_e_tempi(s))
    same = M.jaccard(al, mm, tr, 6e4)
    assert same["totale"]["jaccard"] == 1.0 and same["totale"]["solo_a"] == 0
    half = T.Tabella("x", names, mm.mz[::2], mm.rt[::2], mm.area[::2])
    j = M.jaccard(al, half, tr, 6e4)
    assert 0.3 < j["totale"]["jaccard"] < 0.7 and j["totale"]["solo_b"] == 0
    assert sum(c["comuni"] for c in j["classi"].values()) == j["totale"]["comuni"]
    rep = VF.valida(s, verita, str(csv_ok), sensibilita=False, nome="sintetica")
    assert rep["jaccard"]["totale"]["jaccard"] > 0.9 and "mzmine" in rep["tabelle"]
    with pytest.raises(ValueError):
        csv_bad = tmp_path / "bad.csv"
        csv_bad.write_text("row m/z;row retention time;other\n1;1;1\n", encoding="utf-8")
        T.leggi_mzmine(csv_bad, s)


def test_blank_rows(serie, tmp_path):
    s = T.trova_serie(serie)
    al = T.esegui_mzfinder(s, None).allineata
    b = M.righe_nel_bianco(al, [0])
    assert b["righe_nel_bianco"] > 0 and b["righe_con_segnale"] >= b["righe_nel_bianco"]
    assert M.righe_nel_bianco(al, []) is None
