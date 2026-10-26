"""Bench lot of smeared peaks (tools/valida/sintetici_mzml.py `genera_larghi`): determinism, exact areas, morphologies, blank, gradient background, and the area read back from a file."""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from valida import sintetici_mzml as SM  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

PICCOLO = dict(seme=5, n_campioni=2, n_larghi=18, n_stretti=6, n_fondo=20, ciclo=0.05, rt_fine=30.0)      # coarse cycle and few background channels: fast


def _trapz(y, t) -> float:
    return float(np.sum((y[1:] + y[:-1]) / 2 * np.diff(t)))


def _sha(d: Path) -> dict:
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(d.iterdir())}


@pytest.fixture(scope="module")
def lotto(tmp_path_factory):
    d = tmp_path_factory.mktemp("larghi")
    return d, SM.genera_larghi(d, **PICCOLO)


def test_stesso_seme_stessi_byte(lotto, tmp_path):
    d, _ = lotto
    SM.genera_larghi(tmp_path, **PICCOLO)
    assert _sha(d) == _sha(tmp_path)
    SM.genera_larghi(tmp_path / "altro", **{**PICCOLO, "seme": 6})
    assert _sha(tmp_path / "altro")["verita.json"] != _sha(d)["verita.json"]


def test_lotto_e_verita(lotto):
    d, v = lotto
    assert sorted(p.name for p in d.iterdir()) == ["blank_01.mzML", "camp_01.mzML", "camp_02.mzML", "verita.json"]
    assert json.loads((d / "verita.json").read_text()) == v and v["file"][0] == "blank_01.mzML"
    larghi = [r for r in v["ioni"] if r["ruolo"] == "broad"]
    assert len(larghi) == 18 and {r["morfologia"] for r in larghi} == {"a_gauss", "a_emg", "b_doppio", "c_banda"}
    for r in v["ioni"]:
        assert set(r) >= {"ruolo", "rt", "fwhm_s", "rt_min", "rt_max", "area_attesa", "mz", "componenti"}
        assert r["rt_min"] < r["rt"] < r["rt_max"] and r["area_attesa"][0] == 0.0 and len(r["area_attesa"]) == len(v["file"])
    for r in larghi:
        assert 58 <= r["fwhm_s"] <= 245                                                    # 60-240 s (numeric FWHM of the sum)
        if r["morfologia"] == "c_banda":
            assert r["fwhm_s"] == pytest.approx(150, abs=1)
    por = {r["id"]: r for r in v["ioni"]}
    sub = [r for r in v["ioni"] if r["morfologia"] == "stretto_su_banda"]
    assert sub and all(r["fwhm_s"] == pytest.approx(8, abs=0.3) and abs(r["mz"] / por[r["su"]]["mz"] - 1) <= 3e-6 + 1e-9 for r in sub)
    assert {sum(1 for r in sub if r["su"] == b["id"]) for b in larghi if b["morfologia"] == "c_banda"} <= {1, 2}                # 1-2 narrow peaks per band


def test_doppio_plateau_sella_al_60_per_cento(lotto):
    _, v = lotto
    for r in (r for r in v["ioni"] if r["morfologia"] == "b_doppio"):
        c = r["componenti"]
        assert len(c) == 2 and (c[1]["mu"] - c[0]["mu"]) * 60 == pytest.approx(90, abs=0.01)
        t = np.arange(c[0]["mu"] - 3, c[1]["mu"] + 3, 0.001)
        y = SM._curva(c, t)
        m = np.flatnonzero((y[1:-1] > y[:-2]) & (y[1:-1] >= y[2:])) + 1
        assert len(m) == 2 and y[m[0]:m[1] + 1].min() / min(y[m]) == pytest.approx(0.60, abs=0.005)


def test_area_attesa_e_l_integrale_numerico(lotto):
    _, v = lotto
    for r in v["ioni"]:
        c = r["componenti"]
        t = np.arange(min(x["mu"] - 12 * x["sigma"] for x in c), max(x["mu"] + 12 * x["sigma"] + 30 * x["tau"] for x in c), 0.0005)
        assert _trapz(SM._curva(c, t), t) == pytest.approx(r["area_attesa"][1], rel=0.01)
    assert any(r["morfologia"] == "a_emg" for r in v["ioni"])                            # the EMG normalisation is among those checked


def test_snr_in_gamma(lotto):
    _, v = lotto
    for r in v["ioni"]:
        if r["morfologia"] in ("a_gauss", "a_emg", "b_doppio"):
            assert 4 <= r["componenti"][0]["h"] / SM.N0 <= 12


def test_blank_senza_composti_con_fondo_a_gradiente(lotto):
    d, v = lotto
    bg = Run(d / "blank_01.mzML")
    camp = Run(d / "camp_01.mzML")
    assert len(bg.scans) == len(camp.scans) and bg.scans[0].level == 1
    larghi = next(r for r in v["ioni"] if r["morfologia"] == "a_gauss")
    def traccia(run, mz):
        out = []
        for s in run.scans:
            x, y = run.read(s.index)
            out.append(float(y[np.abs(x - mz) / mz < 5e-6].max(initial=0.0)))
        return np.array(out)
    assert traccia(bg, larghi["mz"]).max() < 5e4 and traccia(camp, larghi["mz"]).max() > 2e4          # nothing in the blank at the compound m/z
    # a background channel follows G(t): later (more organic) scans are higher than the first ones
    mzs = sorted({round(float(m), 6) for m in np.random.default_rng([PICCOLO["seme"], 77]).uniform(150, 800, PICCOLO["n_fondo"])})
    x0, y0 = bg.read(bg.scans[0].index)
    xf, yf = bg.read(bg.scans[-1].index)
    i0 = np.argmin(np.abs(x0 - mzs[0])); i1 = np.argmin(np.abs(xf - mzs[0]))
    assert abs(x0[i0] / mzs[0] - 1) < 5e-6 and yf[i1] > 2 * y0[i0]


def test_area_leggendo_il_file(lotto):
    """The area of a high S/N narrow compound recovered from the XIC of the file (sum of heights x cycle) agrees with area_attesa."""
    d, v = lotto
    r = max((r for r in v["ioni"] if r["morfologia"] == "a_gauss"), key=lambda r: r["componenti"][0]["h"])
    run = Run(d / "camp_01.mzML")
    t, y = [], []
    for s in run.scans:
        x, yy = run.read(s.index)
        t.append(s.rt); y.append(float(yy[np.abs(x - r["mz"]) / r["mz"] < 5e-6].max(initial=0.0)))
    t, y = np.array(t), np.array(y)
    base = np.median(y[(t < r["rt_min"] - 1) | (t > r["rt_max"] + 1)])
    ok = (t >= r["rt_min"] - 0.5) & (t <= r["rt_max"] + 0.5)
    assert _trapz(y[ok] - base, t[ok]) == pytest.approx(r["area_attesa"][1], rel=0.15)
