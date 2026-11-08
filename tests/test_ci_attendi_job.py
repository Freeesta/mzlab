"""tools/ci_attendi_job.py: sito-e2e goes on only when every .wasm job succeeded, and stops at the first one that did not."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import ci_attendi_job as w  # noqa: E402

NAMES = ["sito-wasm-core", "sito-wasm-feat-a"]


def job(name, status, conclusion=None):
    return {"name": name, "status": status, "conclusion": conclusion}


def run(states, timeout=1e9):
    seq = iter(states)
    slept = []
    code = w.wait(NAMES, fetch=lambda: next(seq), sleep=slept.append, timeout=timeout)
    return code, len(slept)


def test_waits_until_all_succeeded():
    code, polls = run([
        [job("sito-wasm-core", "in_progress"), job("sito-wasm-feat-a", "queued")],
        [job("sito-wasm-core", "completed", "success"), job("sito-wasm-feat-a", "in_progress")],
        [job("sito-wasm-core", "completed", "success"), job("sito-wasm-feat-a", "completed", "success")],
    ])
    assert (code, polls) == (0, 2)


def test_stops_at_the_first_failure():
    code, polls = run([
        [job("sito-wasm-core", "completed", "failure"), job("sito-wasm-feat-a", "in_progress")],
    ])
    assert (code, polls) == (1, 0)


def test_a_cancelled_or_skipped_job_is_not_success():
    for c in ("cancelled", "skipped"):
        assert w.verdict([job("sito-wasm-core", "completed", c), job("sito-wasm-feat-a", "completed", "success")], NAMES).startswith("fail")


def test_a_job_not_listed_yet_keeps_waiting():
    assert w.verdict([job("sito-wasm-core", "completed", "success")], NAMES) is None


def test_a_job_never_listed_fails_after_a_few_polls():
    code, polls = run([[job("sito-wasm-core", "completed", "success")]] * (w.MISSING_POLLS + 1))
    assert (code, polls) == (1, w.MISSING_POLLS - 1)


def test_a_rerun_of_the_e2e_alone_counts_the_wasm_jobs_of_the_first_attempt():
    # filter=latest lists the latest execution of every job across attempts: the .wasm jobs of attempt 1, sito-e2e of attempt 2
    latest = [job("sito-e2e", "in_progress"), job("sito-wasm-core", "completed", "success"), job("sito-wasm-feat-a", "completed", "success")]
    assert run([latest]) == (0, 0)


def test_the_request_asks_for_the_latest_job_of_each_attempt(monkeypatch):
    seen = []

    class Resp:
        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

        def read(self):
            return b'{"jobs": []}'

    monkeypatch.setenv("GITHUB_REPOSITORY", "o/r")
    monkeypatch.setenv("GITHUB_RUN_ID", "7")
    monkeypatch.setenv("GITHUB_TOKEN", "t")
    monkeypatch.setattr(w.urllib.request, "urlopen", lambda req, timeout: seen.append(req.full_url) or Resp())
    assert w.jobs_of_run() == []
    assert seen == ["https://api.github.com/repos/o/r/actions/runs/7/jobs?filter=latest&per_page=100"]


def test_an_api_error_is_retried():
    calls = iter([OSError("503"), [job(n, "completed", "success") for n in NAMES]])

    def fetch():
        x = next(calls)
        if isinstance(x, Exception):
            raise x
        return x

    assert w.wait(NAMES, fetch=fetch, sleep=lambda s: None) == 0


def test_gives_up_after_the_timeout():
    code, _ = run([[job(n, "in_progress") for n in NAMES]] * 3, timeout=-1)
    assert code == 1
