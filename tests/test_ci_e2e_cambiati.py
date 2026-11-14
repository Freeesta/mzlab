"""tools/ci_e2e_cambiati.py: the job `e2e-cambiati` runs the e2e of the changed files minus the smoke round and the tests of other jobs; a core file runs a bounded safety set, documents only run none."""
import json
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


def test_a_core_file_runs_a_bounded_safety_set():
    tests, why = ci.pick({"tests_e2e/lib.py"})
    cost = json.loads(ci.COSTS.read_text(encoding="utf-8"))
    assert tests and tests == sorted(tests) and len(set(tests)) == len(tests) and "areas covered" in why
    assert all((ROOT / "tests_e2e" / f"{n}.py").exists() for n in tests)
    assert sum(cost[n] for n in tests) <= ci.BUDGET_S and max(cost[n] for n in tests) <= ci.TEST_CAP_S
    assert not set(tests) & (set(verifica.SMOKE) | set(verifica.SMOKE_CHROMIUM_LINUX))
    assert not any(verifica.NEEDS.get(n, set()) & ci.ELSEWHERE for n in tests)


def test_the_safety_set_is_the_same_for_every_core_file():
    assert ci.pick({"tools/pre_pr.py"})[0] == ci.pick({"tests_e2e/lib.py"})[0]


def test_an_area_file_runs_its_tests_without_the_smoke_round():
    tests, why = ci.pick({"mzlab/web/draw.js"})
    assert "e2e_lingua_disegno" in tests and f"running {len(tests)}" in why
    assert not set(tests) & (set(verifica.SMOKE) | set(verifica.SMOKE_CHROMIUM_LINUX))
    assert tests == sorted(tests)


def test_tests_of_other_jobs_are_left_out():
    sito = next(n for n, need in verifica.NEEDS.items() if "sito" in need)
    tests, why = ci.pick({f"tests_e2e/{sito}.py"})
    assert tests == [] and "nothing left to run" in why


def test_tests_that_need_the_ci_wasm_are_left_out_but_run_by_verifica():
    wasm = next(n for n, need in verifica.NEEDS.items() if "wasm" in need)
    tests, why = ci.pick({f"tests_e2e/{wasm}.py"})
    assert tests == [] and "nothing left to run" in why
    assert not verifica.NEEDS[wasm] & {"sito", "dam", "veri"}          # the complete round of the night still runs it


def test_a_changed_e2e_runs_itself():
    tests, _ = ci.pick({"tests_e2e/e2e_lingua_disegno.py"})
    assert tests == ["e2e_lingua_disegno"]


def test_the_output_line(tmp_path, monkeypatch):
    out = tmp_path / "out"
    monkeypatch.setenv("GITHUB_OUTPUT", str(out))
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    monkeypatch.chdir(ROOT)
    assert ci.main([head]) == 0 and out.read_text(encoding="utf-8").startswith("e2e=")
    assert ci.main([]) == 0
