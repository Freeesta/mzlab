"""tools/ci_cancello.py: the deploy gate opens only when the required checks of every workflow succeeded on the commit, and closes at the first one that did not."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import ci_cancello as g  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
IDS = {"test.yml": 1, "rust.yml": 2, "sito-rust.yml": 3}


def runs(*wfs):
    return [{"id": IDS[w], "path": f".github/workflows/{w}"} for w in wfs]


def jobs(status="completed", conclusion="success", over=None):
    """Every required job of each run in the same state, except the ones in `over` {name: (status, conclusion)}."""
    over = over or {}
    def f(run_id):
        wf = next(w for w, i in IDS.items() if i == run_id)
        return [{"name": n, "status": over.get(n, (status, conclusion))[0], "conclusion": over.get(n, (status, conclusion))[1]} for n in g.REQUIRED[wf]]
    return f


def run(seq_runs, seq_jobs, timeout=1e9):
    r, j = iter(seq_runs), iter(seq_jobs)
    slept = []
    state = {}
    def runs_f(_sha):
        state["jobs"] = next(j)
        return next(r)
    code = g.wait("a" * 40, runs=runs_f, jobs=lambda i: state["jobs"](i), sleep=slept.append, timeout=timeout)
    return code, len(slept)


def test_opens_when_every_required_check_is_green():
    allwf = runs(*IDS)
    code, polls = run([runs("test.yml"), allwf, allwf], [jobs(), jobs("in_progress", None), jobs()])
    assert (code, polls) == (0, 2)


def test_closes_at_the_first_red_check_without_waiting_for_the_others():
    code, polls = run([runs(*IDS)], [jobs("in_progress", None, over={"rust-ok": ("completed", "failure")})])
    assert (code, polls) == (1, 0)


def test_a_cancelled_or_skipped_summary_job_is_not_green():
    for c in ("cancelled", "skipped"):
        v, _ = g.check("x", lambda _s: runs(*IDS), jobs(over={"sito": ("completed", c)}))
        assert v.startswith("sito-rust.yml fail")


def test_a_workflow_that_never_starts_closes_the_gate():
    n = g.MISSING_POLLS
    code, polls = run([runs("test.yml", "rust.yml")] * n, [jobs()] * n)
    assert (code, polls) == (1, n - 1)


def test_the_newest_run_of_a_workflow_counts():
    old = {"id": 1, "path": ".github/workflows/test.yml"}
    new = {"id": 9, "path": ".github/workflows/test.yml@refs/heads/main"}
    assert g.newest([new, old, {"id": 5, "path": ".github/workflows/notte.yml"}]) == {"test.yml": new}


def test_still_running_at_the_timeout_closes_the_gate():
    code, _ = run([runs(*IDS)] * 3, [jobs("in_progress", None)] * 3, timeout=-1)
    assert code == 1


def test_required_names_exist_in_their_workflows():
    """A renamed summary job would close the gate on every push: each name is the `name:` (or the key) of a job of its workflow."""
    for wf, names in g.REQUIRED.items():
        text = (ROOT / ".github" / "workflows" / wf).read_text(encoding="utf-8")
        for n in names:
            base = n.split(" (")[0]
            assert f"\n  {base}:\n" in text or f"name: {base} (" in text or f"name: {n}\n" in text, (wf, n)


def test_every_required_workflow_runs_on_push_to_main():
    for wf in g.REQUIRED:
        text = (ROOT / ".github" / "workflows" / wf).read_text(encoding="utf-8")
        on = text[text.index("\non:"):text.index("\npermissions:")]
        assert "  push:\n    branches: [main]\n" in on and "paths" not in on.split("  push:")[1].split("\n  pull_request")[0], wf
