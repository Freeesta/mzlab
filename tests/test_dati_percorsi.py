"""The resolver of the private data repository (tools/dati_percorsi.py): one layout, the same as the Rust twin."""
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


def test_layout_is_found(repo):
    _tocca(repo, "dati/INDICE.md")
    _tocca(repo, "dati/3200 qtrap/mzML/B_FullMass-t0.mzML")
    _tocca(repo, "coordinamento/da_copiare/c/livelli_attesi.csv")
    _tocca(repo, "scratchpad/tabui_ms2_lr_truth.json")
    assert dp.percorso("dati/3200 qtrap/mzML/B_FullMass-t0.mzML") == repo / "dati/3200 qtrap/mzML/B_FullMass-t0.mzML"
    assert dp.percorso("coordinamento/da_copiare/c/livelli_attesi.csv") is not None
    assert dp.percorso("scratchpad/tabui_ms2_lr_truth.json") is not None


def test_old_layout_is_not_a_repository_and_missing_paths_are_none(repo):
    for vecchio in ("mzML/B_FullMass-t0.mzML", "dati_formati/thermo_raw/a.raw", "scratchpad/x.json"):
        _tocca(repo, vecchio)
    assert dp.radice() is None
    _tocca(repo, "dati/INDICE.md")
    assert dp.percorso("dati/3200 qtrap/mzML/B_FullMass-t0.mzML") is None
    assert dp.percorso("dati/HRMS/non_esiste.mzML") is None


def test_only_the_resolver_reads_the_data_env_and_legacy_names():
    """One point knows the layout: nothing else reads MZLAB_DATI/QQQ_DATI or spells the legacy folders."""
    import re
    cattivi = re.compile(r"environ[^\n]*(MZLAB|QQQ)_DATI|var(_os)?\(\"(MZLAB|QQQ)_DATI|get_env\(\"DATI|dati_formati|3200 qtrap/wiff inquinanti")
    ok = {"dati_percorsi.py", "dati_percorsi.rs", "test_dati_percorsi.py"}
    trovati = []
    for sub in ("tools", "tests", "tests_e2e", "crates"):
        for p in (ROOT / sub).rglob("*"):
            if p.suffix not in {".py", ".rs"} or p.name in ok or "target" in p.parts or "golden" in p.parts:
                continue
            if cattivi.search(p.read_text(encoding="utf-8", errors="ignore")):
                trovati.append(str(p.relative_to(ROOT)))
    assert not trovati, trovati
