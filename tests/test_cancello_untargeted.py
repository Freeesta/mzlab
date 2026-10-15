"""Gate of the engine `untargeted` (tools/valida/cancello_untargeted.py): the section from two tables, the rule of `ok`, the merge into the document of the gate without touching the rest."""
import json
import sys
from pathlib import Path

import numpy as np
import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
from valida import cancello_untargeted as U  # noqa: E402
from valida.tabelle import Tabella  # noqa: E402


def tab(mz, rt, area, nomi=("a", "b")):
    a = np.array(area, float)
    return Tabella("t", list(nomi), np.array(mz, float), np.array(rt, float), a, altezza=a / 10)


def test_identical_tables_pass():
    n = 8
    mz, rt = 200 + np.arange(n) * 7.3, 1 + np.arange(n) * 0.37
    area = np.outer(10 ** (3 + np.arange(n) % 5 + np.arange(n) / 10), [1.0, 2.0]) * 60
    s = U.sezione(tab(mz, rt, area), tab(mz, rt, area / 60), [0, 1], 1.0, "9.9.9")
    assert s["ok"] is True and s["recall"] == 1.0 and s["precision"] == 1.0 and s["protocol"] == U.PROTOCOLLO and s["crate_version"] == "9.9.9" and s["files"] == 2


def test_a_missing_row_fails_and_not_measured_is_null():
    n = 8
    mz, rt = 200 + np.arange(n) * 7.3, 1 + np.arange(n) * 0.37
    area = np.outer(10 ** (3 + np.arange(n) % 5 + np.arange(n) / 10), [1.0, 2.0]) * 60
    s = U.sezione(tab(mz[1:], rt[1:], area[1:]), tab(mz, rt, area / 60), [0, 1], 1.0, "1")
    assert s["ok"] is False and s["recall"] < 0.95
    assert U.vuota()["ok"] is None


def test_scrivi_keeps_the_other_keys(tmp_path):
    p = tmp_path / "cancello.json"
    p.write_text(json.dumps({"format": "cancello/2", "vista_candidati": True}), encoding="utf-8")
    U.scrivi(U.vuota(), p)
    d = json.loads(p.read_text(encoding="utf-8"))
    assert d["format"] == "cancello/2" and d["vista_candidati"] is True and d["untargeted"]["ok"] is None
