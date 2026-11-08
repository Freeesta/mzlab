"""tools/ci_e2e_cambiati.py: the job `e2e-cambiati` runs the e2e of the changed files minus the smoke round and the tests of other jobs; a core file or documents only run none."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import ci_e2e_cambiati as ci  # noqa: E402
import verifica  # noqa: E402


def test_documents_only_run_nothing():
    tests, why = ci.pick({"docs/agenti/test.md", "CLAUDE.md"})
    assert tests == [] and why


def test_a_core_file_runs_nothing_and_names_the_night():
    tests, why = ci.pick({"tests_e2e/lib.py"})
    assert tests == [] and "notte.yml" in why


def test_an_area_file_runs_its_tests_without_the_smoke_round():
    tests, why = ci.pick({"mzlab/web/draw.js"})
    assert "e2e_disegno_chimica" in tests and f"running {len(tests)}" in why
    assert not set(tests) & (set(verifica.SMOKE) | set(verifica.SMOKE_CHROMIUM_LINUX))
    assert tests == sorted(tests)


def test_tests_of_other_jobs_are_left_out():
    sito = next(n for n, need in verifica.NEEDS.items() if "sito" in need)
    tests, why = ci.pick({f"tests_e2e/{sito}.py"})
    assert tests == [] and "nothing left to run" in why


def test_a_changed_e2e_runs_itself():
    tests, _ = ci.pick({"tests_e2e/e2e_disegno_chimica.py"})
    assert tests == ["e2e_disegno_chimica"]


def test_the_output_line(tmp_path, monkeypatch):
    out = tmp_path / "out"
    monkeypatch.setenv("GITHUB_OUTPUT", str(out))
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    monkeypatch.chdir(ROOT)
    assert ci.main([head]) == 0 and out.read_text(encoding="utf-8").startswith("e2e=")
    assert ci.main([]) == 0
