"""Synthetic series with identity truth (tools/valida/sintetici_identita.py): determinism, truth format, families, in-source fragments, isomers, chimeric MS2, blank."""
import csv
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
if not (ROOT / "TP_Mine" / "py").exists():
    pytest.skip("TP_Mine/py not available", allow_module_level=True)
import valida_feat as VF  # noqa: E402
from valida import sintetici_identita as SI  # noqa: E402
from valida import tabelle as T  # noqa: E402
from mzlab.chem import elements as E  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

SEME = 7


def _sha(d: Path) -> dict:
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(d.iterdir())}


@pytest.fixture(scope="module")
def serie(tmp_path_factory):
    return SI.genera(SEME, "piccolo", tmp_path_factory.mktemp("sin"))


@pytest.fixture(scope="module")
def verita(serie):
    return json.loads((serie / "verita.json").read_text(encoding="utf-8"))


def _riga(verita, id):
    return next(r for r in verita["ioni"] if r["id"] == id)


def test_same_seed_same_bytes(serie, tmp_path):
    assert _sha(SI.genera(SEME, "piccolo", tmp_path)) == _sha(serie)
    assert sorted(_sha(serie)) == sorted(["SIN_blank.mzML", "SIN_dark.mzML", "SIN_t0.mzML", "SIN_t05min.mzML", "SIN_t10min.mzML", "SIN_t20min.mzML", "verita.csv", "verita.json"])


def test_other_seed_other_data(serie, tmp_path):
    assert _sha(SI.genera(SEME + 1, "piccolo", tmp_path))["verita.csv"] != _sha(serie)["verita.csv"]


def test_truth_format(serie, verita):
    rows = list(csv.DictReader(open(serie / "verita.csv", encoding="utf-8")))
    assert list(rows[0]) == SI.COLONNE and len(rows) == len(verita["ioni"])
    assert {r["ruolo"] for r in rows} >= {"progenitore", "tp1", "isotopo", "addotto", "isf", "isf_finto", "isomero", "chimerica", "fondo"}
    assert all(r["id"].startswith("SIN-") for r in rows)                                    # invented molecules, no compound names
    ioni = VF.carica_verita(serie / "verita.json")                                           # V0 reads the new truth as it is
    assert ioni[0]["parent"] and {x["id"] for x in ioni[1:]} == {r["id"] for r in verita["tps"]}
    assert {r["ruolo"] for r in verita["tps"]} <= {"tp1", "tp2", "isomero"}
    assert all(r["ion"] and r["rt_obs"] for r in verita["tps"])


def test_family_masses_and_isotopes(verita):
    p = _riga(verita, "SIN-001")
    assert abs(p["mz"] - E.ion_mz(E.mass(E.parse_formula(p["ion"])) - E.MASS["H"], "[M+H]+")) < 1e-3
    na, nh4 = _riga(verita, "SIN-001_Na"), _riga(verita, "SIN-001_NH4")
    assert na["mz"] - p["mz"] == pytest.approx(SI.NA_DELTA, abs=2e-5) and nh4["mz"] - p["mz"] == pytest.approx(SI.NH4_DELTA, abs=2e-5)
    assert E.ADDUCT_SHIFT["[M+Na]+"] - E.ADDUCT_SHIFT["[M+H]+"] == pytest.approx(SI.NA_DELTA, abs=1e-5)
    m1, m2 = _riga(verita, "SIN-001_M1"), _riga(verita, "SIN-001_M2")
    assert 0.9 < m1["mz"] - p["mz"] < 1.1 and 1.9 < m2["mz"] - p["mz"] < 2.1
    assert float(m1["note"].split("=")[1]) > 5                                                # declared relative abundance (S, C22)
    assert abs(na["rt_obs"][0] - p["rt_obs"][0]) <= 0.01 and abs(nh4["rt_obs"][0] - p["rt_obs"][0]) <= 0.01      # co-apex


def test_isf_constant_ratio_and_ms2(serie, verita):
    c = SI.costruisci(SEME, "piccolo")
    por = {o.id: o for o in c.ioni}
    for i in ("SIN-001_ISF1", "SIN-001_ISF2", "SIN-001_ISFF1", "SIN-001_ISFF2"):
        r = por[i].amp[3:] / por["SIN-001"].amp[3:]
        assert np.allclose(r, r[0]) and r[0] > 0 and float(_riga(verita, i)["note"].split("=")[1]) == pytest.approx(r[0], abs=1e-3)
    # the MS2 of the parent (file t10): true in-source fragments are in it, decoys are not
    r = Run(serie / "SIN_t10min.mzML")
    ms2 = [s for s in r.scans if s.level == 2 and abs(s.precursor - _riga(verita, "SIN-001")["mz"]) < 0.01]
    assert ms2
    mz, _ = r.read(ms2[0].index)
    has = lambda x: bool(np.any(np.abs(mz - x) / x < 5e-6))
    for i in ("SIN-001_ISF1", "SIN-001_ISF2"):
        assert has(_riga(verita, i)["mz"]) and _riga(verita, i)["isf_in_ms2"] == 1
    for i in ("SIN-001_ISFF1", "SIN-001_ISFF2"):
        assert not has(_riga(verita, i)["mz"]) and _riga(verita, i)["isf_in_ms2"] == 0


def test_isomers_valley(verita):
    pairs = [r for r in verita["ioni"] if r["ruolo"] == "isomero" and r["note"].split("isomero_di=")[1] > r["id"]]
    assert sorted(float(p["note"].split("delta_rt=")[1].split(";")[0]) for p in pairs) == [0.05, 0.1, 0.2, 0.3]
    for a in pairs:
        b = _riga(verita, a["note"].split("isomero_di=")[1])
        d, v, sg = (float(a["note"].split(k + "=")[1].split(";")[0]) for k in ("delta_rt", "valle", "sigma"))
        assert b["rt_obs"][0] - a["rt_obs"][0] == pytest.approx(d, abs=1e-3) and a["ion"] == b["ion"] and a["mz"] == b["mz"]
        assert SI.valle_relativa(d, sg, 0.8) == pytest.approx(v, abs=0.02)


def test_chimeric_purity(serie, verita):
    c = SI.costruisci(SEME, "piccolo")
    por = {o.id: o for o in c.ioni}
    livelli = sorted(r["purezza"] for r in verita["ioni"] if r["ruolo"] == "chimerica")
    assert livelli == [0.3, 0.5, 0.7, 0.95]
    r = Run(serie / "SIN_t10min.mzML")
    for o in c.ioni:
        if o.ruolo != "chimerica":
            continue
        t = por[o.progenitore]
        assert abs(o.mz - t.mz) <= SI.ISO_SEMI and t.purezza == o.purezza
        assert (t.amp[3:] / (t.amp[3:] + o.amp[3:]) == pytest.approx(o.purezza))              # true purity = share of the target in the window
        s = next(s for s in r.scans if s.level == 2 and abs(s.precursor - t.mz) < 0.01)
        assert s.iso[0] < o.mz < s.iso[1]                                                       # the partner is inside the isolation window of the MS2
        prev = r.read(s.parent)[0]
        assert np.any(np.abs(prev - o.mz) / o.mz < 5e-6) and np.any(np.abs(prev - t.mz) / t.mz < 5e-6)      # both in the preceding MS1


def test_blank_dark_t0_and_background(serie, verita):
    ioni = {r["id"]: r for r in verita["ioni"]}
    def n_scans(nome, mz):
        r = Run(serie / nome)
        def hit(s):
            x, y = r.read(s.index)
            return bool(np.any((np.abs(x - mz) / mz < 5e-6) & (y > 1e5)))              # above the noise (a noise peak can fall in the window by chance)
        return sum(1 for s in r.scans if s.level == 1 and hit(s))
    bg = ioni["SIN-BG-01"]["mz"]
    assert n_scans("SIN_blank.mzML", bg) > 400                                                 # continuous trace
    assert n_scans("SIN_blank.mzML", ioni["SIN-001"]["mz"]) == 0                               # nothing of the sample in the blank
    assert n_scans("SIN_t0.mzML", ioni["SIN-002"]["mz"]) == 0 and n_scans("SIN_dark.mzML", ioni["SIN-002"]["mz"]) == 0     # no product before the treatment
    assert n_scans("SIN_t20min.mzML", ioni["SIN-002"]["mz"]) > 10
    assert n_scans("SIN_dark.mzML", ioni["SIN-001"]["mz"]) >= n_scans("SIN_t20min.mzML", ioni["SIN-001"]["mz"])      # stable parent in the dark


def test_validation_reads_the_series(serie):
    rep = VF.valida(T.trova_serie(serie), VF.carica_verita(serie / "verita.json"), sensibilita=False, nome="sin")
    s = rep["tabelle"]["mzFinder riempita"]["tp_veri"]["riassunto"]
    assert s["valutabili"] == 15 and s["trovati"] >= 10
    assert rep["tabelle"]["mzFinder riempita"]["completezza"]["M"]["trovati"] >= 10
