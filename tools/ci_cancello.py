"""Deploy gate of pages.yml: waits until the required checks of a pull request have finished on the pushed commit of main; exit 0 when all of them succeeded,
1 as soon as one did not (the deploy then does not start and the previous site stays published). Without «Require branches to be up to date» two pull requests
green alone can break main together: this is the barrier between such a commit and the public site.
Usage: ci_cancello.py [SHA]; reads GITHUB_TOKEN, GITHUB_REPOSITORY, GITHUB_SHA, GITHUB_API_URL (job needs `actions: read`)."""
import json
import os
import sys
import time
import urllib.request
from typing import Callable

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ci_attendi_job import verdict  # noqa: E402

# the summary job of each workflow: each one is red when a job it stands for is red (pytest: shards, rust-ok: every Rust job, sito: sito-e2e and its .wasm builds)
REQUIRED = {
    "test.yml": ["pytest (ubuntu-latest, 3.11)", "pytest (ubuntu-latest, 3.14)", "browser (ubuntu-latest, chromium)", "javascript", "tipi (tsc)"],
    "rust.yml": ["changes", "rust-ok"],
    "sito-rust.yml": ["sito-changes", "sito"],
}
POLL = 60
TIMEOUT = 150 * 60
# a workflow with no push run on the commit after this many polls never started (renamed, path filter): fail now, not after TIMEOUT
MISSING_POLLS = 10


def api(path: str) -> dict:
    base = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    req = urllib.request.Request(f"{base}/repos/{os.environ['GITHUB_REPOSITORY']}/{path}",
                                 headers={"Authorization": f"Bearer {os.environ['GITHUB_TOKEN']}", "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def runs_of(sha: str) -> list[dict]:
    return api(f"actions/runs?head_sha={sha}&event=push&per_page=100")["workflow_runs"]


def jobs_of(run_id: int) -> list[dict]:
    return api(f"actions/runs/{run_id}/jobs?filter=latest&per_page=100")["jobs"]


def newest(runs: list[dict]) -> dict[str, dict]:
    """The newest run of each required workflow (a re-run keeps its id and shows its latest attempt)."""
    out: dict[str, dict] = {}
    for r in runs:
        wf = r.get("path", "").split("@")[0].rsplit("/", 1)[-1]
        if wf in REQUIRED and (wf not in out or r["id"] > out[wf]["id"]):
            out[wf] = r
    return out


def check(sha: str, runs: Callable[[str], list[dict]], jobs: Callable[[int], list[dict]]) -> tuple[str | None, list[str]]:
    """('ok' | 'fail: ...' | None while running, workflows with no run yet)."""
    found = newest(runs(sha))
    missing = [wf for wf in REQUIRED if wf not in found]
    pending = False
    for wf, run in found.items():
        v = verdict(jobs(run["id"]), REQUIRED[wf])
        if v and v.startswith("fail"):
            return f"{wf} {v} ({run.get('html_url', run['id'])})", missing
        pending = pending or v is None
    return (None if pending or missing else "ok"), missing


def wait(sha: str, runs: Callable[[str], list[dict]] = runs_of, jobs: Callable[[int], list[dict]] = jobs_of,
         sleep: Callable[[float], None] = time.sleep, timeout: float = TIMEOUT) -> int:
    start = time.monotonic()
    absent = 0
    while True:
        try:
            v, missing = check(sha, runs, jobs)
            absent = absent + 1 if missing else 0
            if absent >= MISSING_POLLS:
                v = "fail: no push run on this commit: " + ", ".join(missing)
        except OSError as e:  # a transient API error must not close the gate: try again at the next poll
            print(f"API error, retrying: {e}", flush=True)
            v = None
        if v == "ok":
            print(f"gate open: the required checks of {sha[:12]} are green", flush=True)
            return 0
        if v:
            print(f"gate closed, the site is not deployed: {v}", flush=True)
            return 1
        if time.monotonic() - start > timeout:
            print(f"gate closed: required checks still running after {timeout / 60:.0f} min", flush=True)
            return 1
        sleep(POLL)


if __name__ == "__main__":
    sys.exit(wait(sys.argv[1] if len(sys.argv) > 1 else os.environ["GITHUB_SHA"]))
