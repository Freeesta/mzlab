"""tools/verifica.py --budget: once the time is spent nothing starts and a run is cut, so the report is always written; part 1 takes fewer e2e."""
import sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import verifica


def test_budget_spent_runs_nothing(tmp_path, monkeypatch):
    monkeypatch.setattr(verifica, "DEADLINE", time.time() - 5)
    rc, out, dt = verifica.sh([sys.executable, "-c", "print('ran')"], tmp_path / "x.log", 600)
    assert rc == 124 and out.startswith("TIMEOUT") and dt == 0.0 and verifica.judge(out, rc)


def test_budget_cuts_a_running_test(tmp_path, monkeypatch):
    monkeypatch.setattr(verifica, "DEADLINE", time.time() + 2)
    t = time.time()
    rc, out, _ = verifica.sh([sys.executable, "-c", "import time; time.sleep(30)"], tmp_path / "x.log", 600)
    assert rc == 124 and time.time() - t < 10 and "TIMEOUT" in out


def test_parts_cover_every_test_once_and_part_one_is_lighter():
    names = [f"e2e_{i:03d}" for i in range(90)]
    parts = [verifica.parte_di(names, k, 5) for k in range(1, 6)]
    assert sorted(x for p in parts for x in p) == sorted(names)
    assert len(parts[0]) < min(len(p) for p in parts[1:])
