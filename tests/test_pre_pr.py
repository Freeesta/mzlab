"""tools/pre_pr.py: fast round by default in the cloud, documents-only round, exit code of the fast round."""
import importlib.util
import subprocess
import sys
from pathlib import Path

RADICE = Path(__file__).resolve().parent.parent


def carica():
    sys.path.insert(0, str(RADICE / "tools"))
    s = importlib.util.spec_from_file_location("pre_pr", RADICE / "tools" / "pre_pr.py")
    m = importlib.util.module_from_spec(s)
    s.loader.exec_module(m)
    return m


def test_cloud_da_variabile(monkeypatch):
    m = carica()
    monkeypatch.setenv("CLAUDE_CODE_REMOTE", "true")
    assert m.in_cloud()


def test_solo_doc_usa_la_lista_di_ci_solo_doc():
    m = carica()
    assert m.ci_solo_doc.solo(["docs/agenti/test.md", "AGENTS.md"])
    assert not m.ci_solo_doc.solo(["docs/agenti/test.md", "tools/pre_pr.py"])
    assert not m.ci_solo_doc.solo([])


def test_passi_solo_doc_sono_economici():
    nomi = [n for n, _ in carica().passi_solo_doc()]
    assert nomi[:2] == ["mappa del codice", "soglie di dimensione"]
    assert not any("clippy" in n or "e2e" in n for n in nomi)


def test_veloce_non_esce_con_2_per_gli_e2e():
    testo = (RADICE / "tools" / "pre_pr.py").read_text(encoding="utf-8")
    assert "--completo" in testo and "return 2" in testo and "if veloce and not solo_doc" in testo
