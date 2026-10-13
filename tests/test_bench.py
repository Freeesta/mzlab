"""Benchmark core (tools/bench): statistics on cases with a known result, metrics on a hand-made table, the adapters, the truths, and mzFinder on the small synthetic
series (D2, profile `piccolo`) written as `bench/1`, with a deterministic report. Every compound and number is invented. The run of mzFinder is skipped without cargo."""
import json
import re
import shutil
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
if not (ROOT / "TP_Mine" / "py").exists():
    pytest.skip("TP_Mine/py not available", allow_module_level=True)
from bench import adatta as A  # noqa: E402
from bench import metriche as M  # noqa: E402
from bench import rapporto as R  # noqa: E402
from bench import statistica as S  # noqa: E402
from bench import verita as V  # noqa: E402

COSTRUIBILE = shutil.which("cargo") is not None


# ---------------------------------------------------------------------------------------------------------------- statistics with a known result
def test_holm_textbook():
    h = S.holm([0.01, 0.04, 0.03, 0.005])
    assert [round(x, 6) for x in h["p_corretto"]] == [0.03, 0.06, 0.06, 0.02]
    assert h["rifiutata"] == [True, False, False, True]
    assert S.holm([0.5, 0.9])["p_corretto"] == [1.0, 1.0]                                # capped at 1 and monotone
    assert S.holm([]) == {"p_corretto": [], "rifiutata": []}


def test_mcnemar_exact_known_values():
    assert S.mcnemar_esatto([1] * 5 + [0] * 3, [0] * 5 + [0] * 3)["p"] == pytest.approx(2 * 0.5 ** 5)
    r = S.mcnemar_esatto([1] * 9 + [0] + [1] * 4, [0] * 9 + [1] + [1] * 4)             # 9 vs 1 discordant
    assert (r["solo_a"], r["solo_b"]) == (9, 1) and r["p"] == pytest.approx(2 * 11 / 1024)
    assert S.mcnemar_esatto([1, 0, 1], [1, 0, 1])["p"] == 1.0                           # no discordant pair


def test_bootstrap_known_cases():
    assert S.intervallo([1.0] * 30)["ic_basso"] == S.intervallo([1.0] * 30)["ic_alto"] == 1.0
    x = [1] * 50 + [0] * 50                                                              # mean 0.5, standard error 0.05: the interval is about +-0.098
    i = S.intervallo(x)
    assert i["valore"] == 0.5 and 0.39 < i["ic_basso"] < 0.43 and 0.57 < i["ic_alto"] < 0.61
    st = [0] * 10 + [1] * 10                                                             # each stratum is constant: a stratified mean cannot move
    v = [float(s) for s in st]
    assert S.intervallo(v, strati=st)["ic_basso"] == S.intervallo(v, strati=st)["ic_alto"] == 0.5
    assert S.intervallo(v)["ic_basso"] < 0.5 < S.intervallo(v)["ic_alto"]
    d = S.differenza([1] * 40, [0] * 40)                                                 # a wins everywhere
    assert (d["valore"], d["ic_basso"], d["ic_alto"]) == (1.0, 1.0, 1.0) and d["p"] < 0.01
    d = S.differenza(x, x)
    assert (d["valore"], d["ic_basso"], d["ic_alto"], d["p"]) == (0.0, 0.0, 0.0, 1.0)  # same tool: paired, so no spread at all
    m = S.differenza([0.1, 0.2, 0.9], [0.1, 0.2, 0.3], "mediana")
    assert m["valore"] == pytest.approx(0.0)                                             # the medians are equal even if one value differs
    with pytest.raises(ValueError):
        S.differenza([1, 0], [1])


def test_bootstrap_deterministic_and_seed_sensitive():
    x = [0.3, 1.7, 2.2, 0.9, 4.1, 3.3, 0.1, 2.8, 1.4, 5.0, 0.6, 2.0]
    assert S.intervallo(x) == S.intervallo(x)
    assert S.intervallo(x, seme=1) != S.intervallo(x, seme=2)
    assert S.indici_bootstrap(12, n_boot=5).shape == (5, 12)


def test_noninferiority_and_superiority_rules():
    d = {"valore": -0.01, "ic_basso": -0.015, "ic_alto": 0.0}
    assert S.non_inferiore(d, "alto", -0.02) is True and S.non_inferiore(dict(d, ic_basso=-0.03), "alto", -0.02) is False
    assert S.non_inferiore({"valore": 0.0, "ic_basso": -0.2, "ic_alto": 0.04}, "basso", 0.05) is True
    assert S.non_inferiore({"valore": None}, "alto", -0.02) is None
    assert S.superiore({"valore": 0.2, "ic_basso": 0.1, "ic_alto": 0.3}, "alto", True) is True
    assert S.superiore({"valore": 0.2, "ic_basso": 0.1, "ic_alto": 0.3}, "alto", False) is False      # not significant after Holm
    assert S.superiore({"valore": -0.2, "ic_basso": -0.3, "ic_alto": -0.1}, "basso", True) is True


# ---------------------------------------------------------------------------------------------------------------- metrics on a hand-made table
def comp(i, mz, rt, ruolo="tp1", **kw):
    return {"id": i, "mz": mz, "rt": rt, "ruolo": ruolo, "fascia": kw.get("fascia"), "atteso": kw.get("atteso"), "area_attesa": kw.get("area_attesa")}


def riga(mz, rt, aree, **kw):
    return {"mz": mz, "rt": rt, "aree": aree, **kw}


VERITA = [comp("par", 300.1000, 5.0, "progenitore"), comp("a", 316.1000, 4.0, "tp1"), comp("b", 330.2000, 6.0, "tp1"), comp("c", 350.3000, 7.0, "tp2"),
          comp("i1", 400.4000, 3.00, "isomero"), comp("i2", 400.4000, 3.10, "isomero"),               # two isomers that one row merges
          comp("e1", 500.5000, None, "esca"), comp("e2", 600.6000, None, "esca"), comp("f1", 450.4500, 8.0, "isf_finto")]
TAB = A.tabella("x", "T", "predefinita", ["f1", "f2"], [
    riga(300.1000, 5.00, [4e6, 4e6], candidato=True, livello="3", punteggio=3.0),                       # parent
    riga(316.1000, 4.02, [1e6, 2e6], candidato=True, livello="3", punteggio=2.0),                       # a: found, candidate, rank 2 (points)
    riga(330.2000, 6.00, [1e5, 9e5], candidato=False, livello="4", punteggio=0.0),                      # b: found, not a candidate
    riga(330.2000, 6.05, [1e5, 9e4], candidato=False, livello="4", punteggio=0.0),                      # b again: a double row
    riga(400.4000, 3.05, [7e6, 7e6], candidato=True, livello="2b", punteggio=3.0),                      # i1 and i2 merged
    riga(500.5000, 2.00, [1e6, 1e6], candidato=True, livello="3", punteggio=1.0),                       # a candidate on the m/z of the decoy e1
    riga(600.6000, 2.00, [1e6, 1e6], candidato=True, livello="4", punteggio=1.0),                       # on the decoy e2, but only level 4: not a false product
    riga(700.7000, 9.00, [5e5, 5e5], candidato=False, livello="4", punteggio=0.0),                      # unexplained row
])


def test_p1_p3_by_hand():
    p1 = M.p1(VERITA, TAB)
    ids = [c["id"] for c in V.feature_vere(VERITA)]
    assert ids == ["par", "a", "b", "c", "i1", "i2"]
    assert p1["per_composto"] == [1, 1, 1, 0, 1, 0]                                              # c is missing, i2 loses the row to i1
    assert p1["richiamo"] == pytest.approx(4 / 6)
    p3 = M.p3(VERITA, TAB)
    assert p3["per_composto"] == [1, 1, 0, 0, 0, 0]                                              # b has two rows, c none, the isomers share one
    assert p3["integrita"] == pytest.approx(2 / 6)


def test_p4_p5_by_hand():
    p4 = M.p4(VERITA, TAB)
    assert [c["id"] for c in V.prodotti(VERITA)] == ["a", "b", "c", "i1", "i2"]
    assert p4["per_composto"] == [1, 0, 0, 1, 0] and p4["trovati"] == 3
    assert p4["richiamo_candidati"] == pytest.approx(2 / 5)
    assert p4["ranghi"] == [1, 3] and p4["candidati"] == 5                                       # merged isomer row (7e6) first, a third
    assert p4["per_90"] is None                                                                  # 4.5 -> 5 products needed, only 2 reachable
    p5 = M.p5(VERITA, TAB)
    assert [c["id"] for c in V.esche(VERITA)] == ["e1", "e2", "f1"]
    assert p5["per_composto"] == [1, 0, 0]                                                       # e1 hit by a level-3 candidate, e2 only by level 4, f1 not at all
    assert p5["precisione_candidati"] == pytest.approx(3 / 5)                                    # 5 flagged rows; parent, a and the merged isomer row match a true compound


def test_p4_rank_curve_and_ties():
    v = [comp("t1", 100.0, 1.0), comp("t2", 200.0, 2.0)]
    t = A.tabella("x", "T", "c", ["f"], [riga(100.0, 1.0, [1.0]), riga(200.0, 2.0, [9.0]), riga(300.0, 3.0, [5.0])])           # no flags: all rows, by area
    p = M.p4(v, t)
    assert p["ranghi"] == [1, 3] and p["per_90"] == 3 and p["candidati"] == 3
    t2 = A.tabella("x", "T", "c", ["f"], [riga(100.0, 1.0, [1.0], candidato=True, punteggio=1.0), riga(200.0, 2.0, [9.0], candidato=False, punteggio=0.0)])
    assert M.p4(v, t2)["per_composto"] == [1, 0] and M.p4(v, t2)["candidati"] == 1


def test_p2_ratio_and_area():
    v = [comp("m1", 100.0, 1.0, "progenitore", atteso=2.0), comp("m2", 200.0, 2.0, "progenitore", atteso=4.0), comp("m3", 300.0, 3.0, "progenitore", atteso=1.0)]
    t = A.tabella("x", "T", "c", ["a1", "a2", "b1", "b2"], [riga(100.0, 1.0, [8, 8, 4, 4]),          # ratio 2: error 0
                                                            riga(200.0, 2.0, [8, 8, 4, 4])])         # ratio 2 instead of 4: error 1 (log2)
    p = M.p2(v, t, {"A": [0, 1], "B": [2, 3]})
    assert p["per_composto"] == [0.0, 1.0, None] and p["mediana"] == 0.5 and p["n_misurati"] == 2
    w = [comp("m1", 100.0, 1.0, "progenitore", area_attesa=[10.0, 10.0])]
    q = M.p2(w, A.tabella("x", "T", "c", ["a", "b"], [riga(100.0, 1.0, [8.0, 8.0])]))
    assert q["per_composto"] == [pytest.approx(0.2)]


def test_secondary_and_empty_table():
    s = M.secondarie(VERITA, TAB)
    assert s["righe"] == 8 and s["righe_non_spiegate"] == 3 and s["isomeri_separati"] == 1 and s["isomeri"] == 2
    vuota = A.tabella("x", "T", "c", ["f"], [])
    r = M.tutte(VERITA, vuota)
    assert r["P1"]["richiamo"] == 0.0 and r["P3"]["integrita"] == 0.0 and r["P5"]["falsi"] == 0.0 and r["P4"]["candidati"] == 0


# ---------------------------------------------------------------------------------------------------------------- truths and adapters
def test_truth_roles_and_multiple_rt():
    righe = [{"id": "p", "ruolo": "progenitore", "mz": 300.0, "rt_obs": [5.0], "presente": 1}, {"id": "t", "mz": 316.0, "rt_obs": [4.0, 6.0]},
             {"id": "e", "ruolo": "esca", "mz": 500.0, "rt_obs": [], "presente": 0}, {"id": "n", "ruolo": "isotopo", "mz": 301.0, "rt_obs": [5.0], "presente": 1, "note": "fascia=alta"}]
    c = V.da_righe(righe)
    assert [x["id"] for x in c] == ["p", "n", "t", "t#2", "e"] and {x["id"]: x["ruolo"] for x in c}["t#2"] == "tp1"
    assert [x["id"] for x in V.feature_vere(c)] == ["p", "t", "t#2"] and [x["id"] for x in V.esche(c)] == ["e"]
    assert [x["fascia"] for x in c if x["id"] == "n"] == ["alta"]


def test_truth_files(tmp_path):
    d3 = {"parent": {"name": "P", "mz": 300.0, "rt_obs": 5.0}, "tps": [{"id": "T1", "mz": 316.0, "rt_obs": [4.0]}]}
    (tmp_path / "t.json").write_text(json.dumps(d3))
    assert {c["id"]: c["ruolo"] for c in V.da_json(tmp_path / "t.json")} == {"P": "progenitore", "T1": "tp1"}
    (tmp_path / "m.tsv").write_text("metabolite_identification\tmass_to_charge\tretention_time\texpected_ratio\nA\t150.05\t120\t2.5\nB\t200.1\t60\t\nC\t210.1\t30\t0.5\n")
    maf = V.da_maf(tmp_path / "m.tsv", rt_secondi=True)
    assert [(c["mz"], c["rt"], c["atteso"]) for c in maf] == [(150.05, 2.0, 2.5), (210.1, 0.5, 0.5)]
    assert [V.rapporto_atteso(x) for x in ("1\\2", "16\\1", "4/1", "1,5", 2)] == [0.5, 16.0, 4.0, 1.5, 2.0]


def test_table_is_sorted_numbered_and_roundtrips(tmp_path):
    t = A.tabella("x", "T", "c", ["f"], [riga(200.0, 1.0, [1.0]), riga(100.0, 2.0, [3.0]), riga(100.0, 1.0, [2.0])])
    assert [(r["id"], r["mz"], r["rt"]) for r in t["righe"]] == [(0, 100.0, 1.0), (1, 100.0, 2.0), (2, 200.0, 1.0)] and t["formato"] == "bench/1"
    (tmp_path / "t.json").write_text(A.a_json(t))
    assert A.leggi(tmp_path / "t.json") == t
    with pytest.raises(ValueError):
        A.tabella("x", "T", "c", ["f", "g"], [riga(1.0, 1.0, [1.0])])
    (tmp_path / "u.json").write_text('{"formato": "altro"}')
    with pytest.raises(ValueError):
        A.leggi(tmp_path / "u.json")


def test_csv_adapters(tmp_path):
    (tmp_path / "mm.csv").write_text("id,mz,rt,datafile:S1.mzML:area,datafile:S2.mzML:area\n1,100.1,2.5,10,\n2,200.2,3.5,20,30\n")
    t = A.da_csv("mzmine", tmp_path / "mm.csv", ["S1.mzML", "S2.mzML", "S3.mzML"], "T")
    assert [(r["mz"], r["rt"], r["aree"]) for r in t["righe"]] == [(100.1, 2.5, [10.0, 0.0, 0.0]), (200.2, 3.5, [20.0, 30.0, 0.0])] and t["file"] == ["S1", "S2", "S3"]
    (tmp_path / "x.csv").write_text("mzmed,rtmed,npeaks,S1,S2\n100.1,150,2,5,6\n")
    t = A.da_csv("xcms", tmp_path / "x.csv", ["S1", "S2"], "T")
    assert t["righe"][0]["rt"] == 2.5 and t["righe"][0]["aree"] == [5.0, 6.0]                    # seconds -> minutes
    (tmp_path / "o.csv").write_text("mz,rt,intensity_S1,intensity_S2\n100.1,60,5,\n")
    assert A.da_csv("openms", tmp_path / "o.csv", ["S1", "S2"], "T")["righe"][0]["aree"] == [5.0, 0.0]
    (tmp_path / "a.tsv").write_text("id_number\tmz\trtime\tsnr\tS1\tS2\nF1\t100.1\t60\t9\t5\t6\n")
    assert A.da_csv("asari", tmp_path / "a.tsv", ["S1", "S2"], "T")["righe"][0]["aree"] == [5.0, 6.0]


# ---------------------------------------------------------------------------------------------------------------- comparison on known outcomes
def test_confronto_known_outcome():
    v = [comp(f"t{i}", 100.0 + i, 1.0 + 0.5 * i, "tp1") for i in range(40)]
    buono = A.tabella("buono", "T", "c", ["f"], [riga(c["mz"], c["rt"], [1e6], candidato=True, livello="3", punteggio=3.0) for c in v])
    cattivo = A.tabella("cattivo", "T", "c", ["f"], [])
    r = R.rapporto("T", R.misura(v, buono), [R.misura(v, cattivo)])
    c = {x["endpoint"]: x for x in r["confronti"]}
    assert c["P1"]["differenza"]["valore"] == 1.0 and c["P1"]["rifiutata_holm"] is True and c["P1"]["superiore"] is True
    assert c["P1"]["mcnemar"]["solo_a"] == 40 and c["P1"]["non_inferiore"] is True
    assert c["P4"]["superiore"] is True and c["P2"]["differenza"]["valore"] is None and c["P2"]["superiore"] is None       # nothing measurable for P2
    assert len(r["confronti"]) == 5 and all(x["p_holm"] >= x["differenza"]["p"] for x in r["confronti"] if x["differenza"]["p"] is not None)
    worse = R.rapporto("T", R.misura(v, cattivo), [R.misura(v, buono)])
    assert {x["endpoint"]: x for x in worse["confronti"]}["P1"]["non_inferiore"] is False
    assert "| buono/c | P1 | 40 | 1.000 |" in R.a_markdown(r) and "| cattivo/c | P1 |" in R.a_markdown(r)


def test_unisci_stratifies_by_series():
    v = [comp(f"t{i}", 100.0 + i, 1.0, "tp1") for i in range(5)]
    t = A.tabella("x", "T", "c", ["f"], [riga(100.0, 1.0, [1.0], candidato=True)])
    m = R.misura(v, t)
    u = R.unisci([m, m, m])
    assert u["P1"]["n"] == 15 and u["strati_p"]["P1"] == [0] * 5 + [1] * 5 + [2] * 5
    assert R.riassunto(u)["P1"]["valore"] == pytest.approx(0.2)


def test_json_is_deterministic_without_dates():
    v = [comp("t", 100.0, 1.0)]
    t = A.tabella("x", "T", "c", ["f"], [riga(100.0, 1.0, [1.0])])
    a, b = (R.a_json(R.rapporto("T", R.misura(v, t), [R.misura(v, t)])) for _ in range(2))
    assert a == b and a.endswith("\n") and json.loads(a)["formato"] == "bench-rapporto/1"
    assert not re.search(r"\d{4}-\d{2}-\d{2}|\d{2}:\d{2}:\d{2}", a)                       # no date, no time of day


# ---------------------------------------------------------------------------------------------------------------- mzFinder on D2 (small)
@pytest.mark.skipif(not COSTRUIBILE, reason="cargo not available")
def test_mzfinder_on_small_d2_in_bench1(tmp_path):
    t, verita = A.corri_mzfinder_sintetico(7, "piccolo", tmp_path / "a")
    assert t["formato"] == "bench/1" and t["strumento"] == "mzfinder" and len(t["file"]) == 6 and len(t["righe"]) > 100
    assert any(r["candidato"] for r in t["righe"]) and all(len(r["aree"]) == 6 for r in t["righe"])
    assert {c["ruolo"] for c in verita} >= {"progenitore", "tp1", "isomero", "esca"}
    m = R.misura(verita, t)
    assert 0.3 < m["P1"]["richiamo"] <= 1.0 and m["P5"]["falsi"] is not None and m["P4"]["n"] > 0
    r1 = R.a_json(R.rapporto("D2/piccolo/7", m))
    t2, verita2 = A.corri_mzfinder_sintetico(7, "piccolo", tmp_path / "b")
    assert A.a_json(t2) == A.a_json(t)                                                           # same series, same bytes
    assert R.a_json(R.rapporto("D2/piccolo/7", R.misura(verita2, t2))) == r1
    assert "mzfinder/predefinita" in r1


@pytest.mark.skipif(not COSTRUIBILE, reason="cargo not available")
def test_cli_writes_the_files(tmp_path):
    assert R.main(["--d2", "--seme", "7", "--profilo", "piccolo", "--out", str(tmp_path)]) == 0
    assert A.leggi(tmp_path / "bench1.json")["strumento"] == "mzfinder"
    assert json.loads((tmp_path / "rapporto.json").read_text())["dataset"] == "D2/piccolo/7" and (tmp_path / "rapporto.md").read_text().startswith("# Benchmark")


# ---------------------------------------------------------------------------------------------------------------- B3: competitors and report from tables
def test_mzmine_batch_has_the_seven_steps_and_the_parameter_names(tmp_path):
    from bench import corri, corri_mzmine
    x = corri_mzmine.batch_xml([tmp_path / "a.mzML", tmp_path / "b.mzML"], tmp_path / "out.csv", corri.parametri("mzmine", "predefinita"))
    import xml.dom.minidom
    xml.dom.minidom.parseString(x)                                                              # well-formed
    pas = re.findall(r'method="[^"]*\.(\w+)"', x)
    assert pas == ["MSDKmzMLImportModule", "MassDetectionModule", "ModularADAPChromatogramBuilderModule", "MinimumSearchFeatureResolverModule", "JoinAlignerModule",
                   "MultiThreadPeakFinderModule", "CSVExportModularModule"]
    assert x.count('name="m/z tolerance (sample-to-sample)"') == 2 and 'unit="MINUTES"' in x                  # the names the headless batch accepts


def test_report_from_tables_with_groups(tmp_path):
    """Two tables of the same three files and a truth with expected ratios: the CLI of the tables writes the report; the group names become column indices."""
    verita = {"composti": [{"id": f"c{i}", "mz": 100.0 + i, "rt": 2.0, "atteso": 2.0, "visto": True} for i in range(6)] + [{"id": "x", "mz": 500.0, "rt": 3.0, "atteso": 1.0, "visto": False}]}
    (tmp_path / "v.json").write_text(json.dumps(verita))
    for nome, fattore in (("mzlab", 2.0), ("altro", 3.0)):
        righe = [{"mz": 100.0 + i, "rt": 2.0, "aree": [fattore * 100.0, fattore * 100.0, 50.0, 50.0]} for i in range(6)]
        (tmp_path / f"{nome}.json").write_text(A.a_json(A.tabella(nome, "D1", "predefinita", ["SA1", "SA2", "SB1", "SB2"], righe)))
    assert R.main(["--verita", str(tmp_path / "v.json"), "--tipo", "d1", "--rt-tol", "0.2", "--gruppi", "A=SA1,SA2", "B=SB1,SB2", "--tabelle", str(tmp_path / "mzlab.json"),
                   str(tmp_path / "altro.json"), "--out", str(tmp_path / "o")]) == 0
    r = json.loads((tmp_path / "o" / "rapporto.json").read_text())
    assert r["strumenti"]["mzlab/predefinita"]["P1"]["n"] == 6 and r["strumenti"]["mzlab/predefinita"]["P1"]["valore"] == 1.0   # the unseen compound is left out
    assert abs(r["strumenti"]["mzlab/predefinita"]["P2"]["valore"] - 1.0) < 1e-9 and abs(r["strumenti"]["altro/predefinita"]["P2"]["valore"] - np.log2(3)) < 1e-5
