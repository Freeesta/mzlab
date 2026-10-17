"""The gate of the TP view (tools/valida/cancello_tp.py): the measures of the raw view on a small hand-made series with a truth, the criteria and the rule that opens the view,
and the command line (`--cancello-tp`) on the same series. No real data."""
import json
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from valida import cancello_tp as T  # noqa: E402
from valida import rapporto  # noqa: E402
from valida.nativo import Serie  # noqa: E402
from valida.tabelle import Tabella  # noqa: E402

DATA = "2026-10-12"
# rows: (mz, rt, area): 0 product A, 1 product B (isomer pair with 2), 2 product B', 3 inherited ion (constant ratio), 4 inherited ion that coelutes with a product, 5 noise
RIGHE = [(300.1000, 2.00, 5e6), (320.2000, 3.00, 4e6), (320.2000, 3.20, 3e6), (250.1000, 5.00, 2e6), (270.1500, 5.00, 1e6), (410.3000, 6.00, 1e5)]
VERITA = {"tps": [{"id": "A", "mz": 300.1, "rt_obs": [2.0]}, {"id": "B", "mz": 320.2, "rt_obs": [3.0, 3.2]}, {"id": "C", "mz": 500.0, "rt_obs": [4.0]}, {"id": "D", "mz": 270.15, "rt_obs": []}],
          "righe_isomeri": [{"id": "320-A", "gruppo": "320", "mz": 320.2, "rt_obs": 3.0, "rt_separabile": "osservato"}, {"id": "320-B", "gruppo": "320", "mz": 320.2, "rt_obs": 3.2, "rt_separabile": "osservato"},
                            {"id": "999-A", "gruppo": "999", "mz": 999.0, "rt_obs": None, "rt_separabile": "ignoto"}],
          "ereditati": [{"id": "ISF-250", "mz": 250.1, "rapporto_costante": True, "coeluisce_con_tp": None, "rt_atteso": 5.0},
                        {"id": "ISF-270", "mz": 270.15, "rapporto_costante": True, "coeluisce_con_tp": "D", "rt_atteso": 5.0},
                        {"id": "ISF-var", "mz": 410.3, "rapporto_costante": False, "coeluisce_con_tp": None, "rt_atteso": 6.0}]}


def serie(candidati=(0, 1, 2, 3, 4)) -> Serie:
    mz, rt, area = (np.array(x, dtype=float) for x in zip(*RIGHE))
    tab = Tabella("nativo", ["f1", "f2"], mz, rt, np.stack([area, area], axis=1))
    ident = [{"id": r, "totale": 5 - r, "membri": [{"mz": RIGHE[r][0], "rt": RIGHE[r][1]}], "ipotesi": None} for r in range(len(RIGHE))]
    return Serie(tabella=tab, stato=np.zeros((len(RIGHE), 2), dtype=np.uint8), identita=ident, candidati=np.array(candidati), per_file=[], versione_crate="9.9.9")


def test_raw_view_measures():
    m = T.misure_vista_grezza(VERITA, serie(), [0, 1])
    assert m["tp_veri"] == 4 and m["tp_trovati"] == 3 and m["tp_in_vista"] == 3          # C has no row; D (no RT) is found by m/z alone
    assert m["vista_dimensione"] == 5 and m["rango_mediano"] == 2.0 and m["rango_90"] <= 5
    assert m["isomeri_righe_per_gruppo"] == {"320": 2}                                     # the group with unknown RT is skipped
    assert m["ereditati_costanti"] == 2 and m["ereditati_nella_vista"] == 2
    assert m["ereditati_come_tp"] == 1 and m["ereditati_coeluiti_nella_vista"] == 1       # only the one that does not coelute with a TP is a fault


def test_view_membership_follows_the_candidates():
    m = T.misure_vista_grezza(VERITA, serie(candidati=(0,)), [0, 1])
    assert m["tp_trovati"] == 3 and m["tp_in_vista"] == 1 and m["vista_dimensione"] == 1 and m["ereditati_come_tp"] == 0


def test_criteria_and_open_rule():
    c = T.criteri_tp(T.misure_vista_grezza(VERITA, serie(), [0, 1]))
    ok = {k["id"]: k["ok"] for k in c}
    assert ok["tp_trovati"] is False and ok["vista_dimensione"] is True and ok["ereditati_come_tp"] is False       # 3 < 22; 5 <= 200; one fault
    assert ok["generazione"] is None and ok["lr_tp"] is None and ok["determinismo_tp"] is None
    g = T.cancello_tp(c, "D-X", DATA, "1")
    assert g["format"] == "cancello_tp/1" and g["vista_tp"] is False and set(g) == {"crate_version", "criteri", "data", "dataset", "format", "vista_tp"}
    # every threshold criterion passing opens it, even with the one without threshold unmeasured
    tutti = [dict(k, ok=True) if k["id"] != "lr_tp" else k for k in c]
    assert T.cancello_tp(tutti, "D-X", DATA, "1")["vista_tp"] is True
    assert T.cancello_tp(tutti[:-2] + [dict(tutti[-2], ok=None), tutti[-1]], "D-X", DATA, "1")["vista_tp"] is False        # an unmeasured one keeps it closed


def test_nothing_measured_without_truth():
    c = T.cancello_tp(T.criteri_tp(None), "D-X", DATA, "1")
    assert c["vista_tp"] is False and all(k["ok"] is None for k in c["criteri"])


def test_report_files(tmp_path):
    c, det = T.misura(VERITA, serie(), [0, 1], "D-X", DATA)
    assert c["crate_version"] == "9.9.9"
    out = rapporto.scrivi_cancello_tp(tmp_path / "r", c, det, tmp_path / "cancello_tp.json")
    assert json.loads(out[0].read_text(encoding="utf-8"))["format"] == "cancello_tp/1"
    assert "TP view: **closed**" in out[2].read_text(encoding="utf-8") and out[1].exists()
    assert out[0].read_text(encoding="utf-8") == json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
