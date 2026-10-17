"""The high-resolution criteria without a truth (tools/valida/hr*.py): robustness of a run (runs, repeats, proposes a threshold, coherent states) and the
agreement with mzmine (D5), on a mini synthetic mzML series and mini synthetic CSV; without data every criterion is skipped, never passed. Needs cargo."""
import shutil
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402
from valida import cancello, hr, nativo  # noqa: E402
from valida import hr_nativo as H  # noqa: E402
from valida.tabelle import Tabella  # noqa: E402

COSTRUIBILE = shutil.which("cargo") is not None
FEATURES = [(210.1234, 3.1, 0.06, 4e6), (480.5, 7.0, 0.06, 3e6), (301.2, 5.0, 0.06, 8e6)]


def sano():
    """Per-file measures of a healthy run."""
    return [{"gira": True, "righe": 40, "deterministica": True, "soglia": 5e4, "incoerenti": 0, "celle": 80, "extra": 0}]


def valori(crit):
    return {c["id"]: c["ok"] for c in crit}


def test_robustness_criteria_pass_and_fail_one_by_one():
    assert all(c["ok"] is True for c in hr.criteri_robustezza(sano()))
    casi = {"hr_robustezza_gira": {"gira": False, "righe": 0}, "hr_robustezza_deterministica": {"deterministica": False},
            "hr_robustezza_soglia": {"soglia": None}, "hr_robustezza_stati_coerenti": {"incoerenti": 3}}
    for cid, mod in casi.items():
        v = valori(hr.criteri_robustezza(sano() + [{**sano()[0], **mod}]))
        assert v[cid] is False, cid
        assert sum(x is False for x in v.values()) >= 1
    assert valori(hr.criteri_robustezza(sano() + [{**sano()[0], "soglia": float("nan")}]))["hr_robustezza_soglia"] is False
    assert valori(hr.criteri_robustezza(sano() + [{**sano()[0], "soglia": 0.0}]))["hr_robustezza_soglia"] is False


def test_without_files_the_robustness_is_skipped_never_passed():
    for m in (None, []):
        assert all(c["ok"] is None for c in hr.criteri_robustezza(m))
    assert hr.trova_hr("/nessuna/cartella") is None
    c, det = hr.misura("/nessuna/cartella", None, None, Path("/nonexistent"), "2026-10-13")
    assert c["vista_tabella_hr"] is False and all(k["ok"] is None for k in c["criteri"]) and "saltato" in det["dati_dda"] and c["concordanza_mzmine"] == "non_misurata"


def test_cell_coherence():
    stato = np.array([[0, 2, 3], [0, 1, 3]])
    area = np.array([[5.0, 1.0, 0.0], [7.0, 0.0, 0.0]])
    assert hr.stati_coerenti(stato, area) == (0, 6)
    assert hr.stati_coerenti(stato, np.array([[0.0, 1.0, 0.0], [7.0, 0.0, 0.0]])) == (1, 6)           # detected with no area
    assert hr.stati_coerenti(stato, np.array([[5.0, 1.0, 2.0], [7.0, 0.0, 0.0]])) == (1, 6)           # absent with an area
    assert hr.stati_coerenti(np.array([[7]]), np.array([[1.0]])) == (1, 1)                             # a state that does not exist


def test_the_view_needs_the_robustness_and_the_csv_only_when_it_is_given():
    rob = hr.criteri_robustezza(sano())
    ok = cancello.criteri_mzmine({"richiamo": 0.95, "precisione": 0.9, "spearman": 0.95, "log2_mediano": 0.1})
    ko = cancello.criteri_mzmine({"richiamo": 0.5, "precisione": 0.9, "spearman": 0.95, "log2_mediano": 0.1})
    assert hr.vista_tabella_hr(rob + cancello.criteri_mzmine(None)) is True          # no CSV: not required
    assert hr.vista_tabella_hr(rob + ok) is True
    assert hr.vista_tabella_hr(rob + ko) is False
    assert hr.vista_tabella_hr(hr.criteri_robustezza(None) + ok) is False
    d = hr.documento(rob + ko, "2026-10-13", "1")
    assert d["format"] == "cancello_hr/1" and d["concordanza_mzmine"] == "non_passa" and d["vista_tabella_hr"] is False


def scrivi_csv(path, nativa, nomi, scala=1.0, tieni=1.0):
    """A mini mzmine CSV from a native table: the areas times `scala`, the first `tieni` of the rows."""
    righe = list(range(int(len(nativa) * tieni)))
    area = nativa.area[righe].copy()
    cab = ["row ID", "row m/z", "row retention time"] + [f"{n}.mzML Peak area" for n in nomi]
    out = [",".join(cab)] + [",".join([str(i), repr(float(nativa.mz[r])), repr(float(nativa.rt[r]))] + [repr(float(a * scala)) for a in area[j]]) for j, r in enumerate(righe) for i in [j]]
    Path(path).write_text("\n".join(out) + "\n", encoding="utf-8")


@pytest.mark.skipif(not COSTRUIBILE, reason="needs cargo")
def test_native_run_and_d5_on_a_mini_series(tmp_path):
    rng = np.random.default_rng(3)
    mzml = []
    for nome, k in (("M_t00min", 0.0), ("M_t10min", 0.5), ("M_t20min", 1.0)):
        p = tmp_path / f"{nome}.mzML"
        ds.hr_series(p, rng, [(m, rt, sg, a * (1 + k * (i % 2))) for i, (m, rt, sg, a) in enumerate(FEATURES)], rt_end=9.0)
        mzml.append(p)
    idx = nativo.indici(mzml, tmp_path / "idx")
    m = hr.un_file(idx[1], tmp_path / "f1")
    assert m["gira"] and m["righe"] >= 3 and m["deterministica"] and m["soglia"] and m["soglia"] > 0 and m["incoerenti"] == 0, m
    assert all(c["ok"] is True for c in hr.criteri_robustezza([m]))
    run = H.esegui_senza_genitore(idx, tmp_path / "serie")
    tab, nomi = run["tabella"], [p.stem for p in mzml]
    assert len(tab) >= 3 and tab.area.shape == (len(tab), 3)
    col = [1, 2]
    perfetta = tmp_path / "ok.csv"
    scrivi_csv(perfetta, tab, nomi, scala=1.7)                                          # one global scale factor: still agrees
    crit, d = hr.concordanza_d5(Tabella("nativo", nomi, tab.mz, tab.rt, tab.area), str(perfetta), col)
    assert all(c["ok"] is True for c in crit), (crit, d)
    meta = tmp_path / "meta.csv"
    scrivi_csv(meta, tab, nomi, tieni=0.4)                                              # mzmine keeps few rows: the precision drops
    assert valori(hr.concordanza_d5(Tabella("nativo", nomi, tab.mz, tab.rt, tab.area), str(meta), col)[0])["mzmine_precisione"] is False
    assert all(c["ok"] is None for c in hr.concordanza_d5(tab, None, col)[0])


@pytest.mark.skipif(not COSTRUIBILE, reason="needs cargo")
def test_robustness_end_to_end_on_files_named_like_the_dda_ones(tmp_path):
    rng = np.random.default_rng(5)
    d = tmp_path / "dda"
    d.mkdir()
    for nome in ("Orbitrap_Exploris120_DDApos_a", "Orbitrap_Fusion_DDApos_b"):
        ds.hr_series(d / f"{nome}.mzML", rng, FEATURES, rt_end=9.0)
    c, det = hr.misura(str(d), None, None, tmp_path / "lavoro", "2026-10-13")
    v = valori(c["criteri"])
    assert det["robustezza"]["file"] == 2 and all(v[i] is True for i in hr.ID_ROBUSTEZZA), (v, det)
    assert c["vista_tabella_hr"] is True and c["concordanza_mzmine"] == "non_misurata" and "saltato" in det["csv"]
    assert "hr_robustezza_gira" in hr.markdown(c, det)


def test_trova_hr_cerca_anche_nella_radice(tmp_path, monkeypatch):
    from valida import hr
    (tmp_path / "HRMS").mkdir()
    monkeypatch.setenv("MZLAB_DATI", str(tmp_path))
    assert hr.trova_hr(None) is None
    (tmp_path / "HRMS" / "Orbitrap_Fusion_DDApos_1-2min.mzML").write_bytes(b"")
    assert hr.trova_hr(None) == tmp_path / "HRMS"
