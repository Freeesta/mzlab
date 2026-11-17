"""Waits, inside a CI job, until other jobs of the same workflow run have finished: exit 0 when all of them succeeded, 1 as soon as one did not.
sito-rust.yml's `sito-e2e` starts with `sito-changes` instead of after the three `sito-wasm-*` jobs, so it installs, builds the site and the native
release binaries of its slow e2e while the .wasm build; it calls this just before downloading their artifacts (a job is completed after its upload).
It reads the latest execution of each job across attempts (`filter=latest`): after «Re-run failed jobs» of sito-e2e alone, the .wasm jobs of attempt 1 count.
Usage: ci_attendi_job.py NAME [NAME ...]; reads GITHUB_TOKEN, GITHUB_REPOSITORY, GITHUB_RUN_ID, GITHUB_API_URL (job needs `actions: read`)."""
import json
import os
import sys
import time
import urllib.error
import urllib.request
from typing import Callable

POLL = 10
# its own limit, apart from the test time of the step that follows: MZLAB_ATTENDI_MIN (minutes) overrides it
TIMEOUT = float(os.environ.get("MZLAB_ATTENDI_MIN") or 45) * 60
# after a refused request (403/429 rate limit, 5xx) the next poll waits longer, up to this long
BACKOFF_MAX = 300
# a named job absent from the list for this many polls does not exist in the run (renamed, or not in this workflow): fail now, not after TIMEOUT
MISSING_POLLS = 6


def jobs_of_run() -> list[dict]:
    api = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    url = f"{api}/repos/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}/jobs?filter=latest&per_page=100"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {os.environ['GITHUB_TOKEN']}", "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)["jobs"]


def retry_delay(e: OSError, fails: int) -> float:
    """Seconds before the next poll after a failed request: Retry-After when the API gives it (403/429), else 10 s doubling per consecutive failure."""
    if isinstance(e, urllib.error.HTTPError):
        try:
            return min(BACKOFF_MAX, max(POLL, float(e.headers.get("Retry-After", ""))))
        except (TypeError, ValueError):
            pass
    return min(BACKOFF_MAX, POLL * 2 ** min(fails, 5))


def verdict(jobs: list[dict], names: list[str]) -> str | None:
    """'ok' when every named job succeeded, 'fail: ...' when one finished otherwise, None while any is still running or not listed yet."""
    by = {j["name"]: j for j in jobs}
    if any(n not in by for n in names):
        return None
    bad = [f"{n}={by[n]['conclusion']}" for n in names if by[n]["status"] == "completed" and by[n]["conclusion"] != "success"]
    if bad:
        return "fail: " + ", ".join(bad)
    return "ok" if all(by[n]["status"] == "completed" for n in names) else None


def wait(names: list[str], fetch: Callable[[], list[dict]] = jobs_of_run, sleep: Callable[[float], None] = time.sleep, timeout: float = TIMEOUT) -> int:
    start = time.monotonic()
    absent = 0
    fails = 0
    delay = POLL
    while True:
        try:
            jobs = fetch()
            listed = {j["name"] for j in jobs}
            gone = [n for n in names if n not in listed]
            absent = absent + 1 if gone else 0
            v = f"fail: not in this run: {', '.join(gone)}" if absent >= MISSING_POLLS else verdict(jobs, names)
            fails, delay = 0, POLL
        except OSError as e:  # a transient API error (a rate limit too) must not fail the job: back off, then try again
            fails += 1
            delay = retry_delay(e, fails - 1)
            print(f"API error, retrying in {delay:.0f} s: {e}", flush=True)
            v = None
        if v == "ok":
            print("finished: " + ", ".join(names), flush=True)
            return 0
        if v:
            print(v, flush=True)
            return 1
        if time.monotonic() - start > timeout:
            print(f"still running after {timeout / 60:.0f} min: " + ", ".join(names), flush=True)
            return 1
        sleep(delay)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    sys.exit(wait(sys.argv[1:]))
