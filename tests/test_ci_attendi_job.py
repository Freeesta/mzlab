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
