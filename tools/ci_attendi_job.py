"""Waits, inside a CI job, until other jobs of the same workflow run have finished: exit 0 when all of them succeeded, 1 as soon as one did not.
sito-rust.yml's `sito-e2e` starts with `sito-changes` instead of after the three `sito-wasm-*` jobs, so it installs, builds the site and the native
release binaries of its slow e2e while the .wasm build; it calls this just before downloading their artifacts (a job is completed after its upload).
Usage: ci_attendi_job.py NAME [NAME ...]; reads GITHUB_TOKEN, GITHUB_REPOSITORY, GITHUB_RUN_ID, GITHUB_RUN_ATTEMPT, GITHUB_API_URL (job needs `actions: read`)."""
import json
import os
import sys
import time
import urllib.request
from typing import Callable

POLL = 10
TIMEOUT = 45 * 60


def jobs_of_run() -> list[dict]:
    api = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    url = f"{api}/repos/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}/attempts/{os.environ.get('GITHUB_RUN_ATTEMPT', '1')}/jobs?per_page=100"
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {os.environ['GITHUB_TOKEN']}", "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)["jobs"]


def verdict(jobs: list[dict], names: list[str]) -> str | None:
    """'ok' when every named job succeeded, 'fail: ...' when one finished otherwise (or is missing), None while any is still running."""
    by = {j["name"]: j for j in jobs}
    missing = [n for n in names if n not in by]
    if missing:
        return None
    bad = [f"{n}={by[n]['conclusion']}" for n in names if by[n]["status"] == "completed" and by[n]["conclusion"] != "success"]
    if bad:
        return "fail: " + ", ".join(bad)
    return "ok" if all(by[n]["status"] == "completed" for n in names) else None


def wait(names: list[str], fetch: Callable[[], list[dict]] = jobs_of_run, sleep: Callable[[float], None] = time.sleep, timeout: float = TIMEOUT) -> int:
    start = time.monotonic()
    while True:
        try:
            v = verdict(fetch(), names)
        except OSError as e:  # a transient API error must not fail the job: try again at the next poll
            print(f"API error, retrying: {e}", flush=True)
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
        sleep(POLL)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    sys.exit(wait(sys.argv[1:]))
