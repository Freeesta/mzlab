"""The resolver of the private data repository (tools/dati_percorsi.py): new layout first, legacy as fallback, the same table as the Rust twin."""
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import dati_percorsi as dp  # noqa: E402


def _tocca(base: Path, rel: str) -> None:
    (base / rel).parent.mkdir(parents=True, exist_ok=True)
    (base / rel).write_text("x")


@pytest.fixture
def repo(tmp_path, monkeypatch):
    monkeypatch.setenv("MZLAB_DATI", str(tmp_path))
    return tmp_path


def test_no_repository_gives_none(tmp_path, monkeypatch):
    monkeypatch.setenv("MZLAB_DATI", str(tmp_path / "vuoto"))
    monkeypatch.setattr(dp, "_candidati", lambda: [tmp_path / "vuoto"])
    assert dp.radice() is None and dp.percorso("dati/HRMS") is None


def test_new_layout_is_found(repo):
    _tocca(repo, "dati/INDICE.md")
    _tocca(repo, "dati/3200 qtrap/mzML/B_FullMass-t0.mzML")
    assert dp.percorso("dati/3200 qtrap/mzML/B_FullMass-t0.mzML") == repo / "dati/3200 qtrap/mzML/B_FullMass-t0.mzML"


def test_legacy_layout_is_found_through_the_table(repo):
    for vecchio in ("mzML/B_FullMass-t0.mzML", "3200 qtrap/wiff inquinanti 25-26 B/MS2.wiff", "dati_formati/thermo_raw/a.raw",
                    "dati_formati/bruker_timstof_dia/x.d", "dati_formati/INDICE.md", "scratchpad/da_copiare/c/livelli_attesi.csv"):
        _tocca(repo, vecchio)
    assert dp.percorso("dati/3200 qtrap/mzML/B_FullMass-t0.mzML") == repo / "mzML/B_FullMass-t0.mzML"
    assert dp.percorso("dati/3200 qtrap/wiff/MS2.wiff") == repo / "3200 qtrap/wiff inquinanti 25-26 B/MS2.wiff"
    assert dp.percorso("dati/formati/thermo/raw/a.raw") == repo / "dati_formati/thermo_raw/a.raw"
    assert dp.percorso("dati/formati/bruker/timstof_dia/x.d") == repo / "dati_formati/bruker_timstof_dia/x.d"
    assert dp.percorso("dati/formati/INDICE.md") == repo / "dati_formati/INDICE.md"
    assert dp.percorso("coordinamento/da_copiare/c/livelli_attesi.csv") == repo / "scratchpad/da_copiare/c/livelli_attesi.csv"


def test_truth_files_keep_their_place_and_missing_paths_are_none(repo):
    _tocca(repo, "scratchpad/tabui_ms2_lr_truth.json")
    assert dp.percorso("scratchpad/tabui_ms2_lr_truth.json") == repo / "scratchpad/tabui_ms2_lr_truth.json"
    assert dp.percorso("dati/HRMS/non_esiste.mzML") is None


def test_table_is_well_formed():
    righe = [r.split("\t") for r in (ROOT / "tools/dati_mappa.tsv").read_text(encoding="utf-8").splitlines() if r and not r.startswith("#")]
    assert righe and all(len(r) == 2 for r in righe)
    assert len({r[0] for r in righe}) == len(righe)
