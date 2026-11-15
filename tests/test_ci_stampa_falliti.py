"""tools/ci_stampa_falliti.py: the failed tests and stages are readable in the job log, without the artifact."""
import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "tools" / "ci_stampa_falliti.py"


def run(tmp_path, report, logs):
    (tmp_path / ".verifica" / "log").mkdir(parents=True)
    (tmp_path / ".verifica" / "ultimo.md").write_text(report, encoding="utf-8")
    for n, t in logs.items():
        (tmp_path / ".verifica" / "log" / f"{n}.log").write_text(t, encoding="utf-8")
    return subprocess.run([sys.executable, str(SCRIPT)], cwd=tmp_path, capture_output=True, text=True, encoding="utf-8").stdout


def test_failed_e2e_and_site_build_tails_are_printed(tmp_path):
    out = run(tmp_path, "# Verifica: 2 FAIL\n\n- FAIL e2e_a (3 s): x\n- OK   e2e_b\n- FAIL sito per gli e2e (tools/prepara_sito.py) (362 s): vedi log\n",
              {"e2e_a": "line-a\n", "e2e_b": "NOT-PRINTED\n", "prepara_sito": "cargo exploded\n", "prova_hr": "NOT-PRINTED-HR\n"})
    assert "===== e2e_a" in out and "line-a" in out and "cargo exploded" in out and "NOT-PRINTED" not in out


def test_no_report_is_said(tmp_path):
    r = subprocess.run([sys.executable, str(SCRIPT)], cwd=tmp_path, capture_output=True, text=True)
    assert r.returncode == 0 and "No .verifica/ultimo.md" in r.stdout
