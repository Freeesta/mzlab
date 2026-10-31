"""CI facts of one main commit for tools/metriche.py, read from the GitHub API (injected `api(path)`, so tests use a fake).
Run durations per workflow and per job, failed/flaky runs, PRs merged per day, time from PR open to merge. Standard library only."""
from __future__ import annotations

import re
import statistics
from datetime import datetime, timedelta, timezone

from ci_metrics import secs, ts

# Jobs that block the merge (branch protection of main) and the workflows whose runs are summarised.
REQUIRED = ("pytest (ubuntu-latest, 3.11)", "pytest (ubuntu-latest, 3.14)", "browser (ubuntu-latest, chromium)", "javascript", "tipi (tsc)", "rust-ok")
WORKFLOWS = ("test", "rust", "pages", "sito-rust")


def runs_of_commit(sha: str, api) -> dict:
    """Per workflow of the commit: wall time, attempts, conclusion and the duration of every job (latest attempt).
    `complete` is False while a run is still going, so the caller can write the part again later."""
    out: dict = {"workflows": {}, "jobs": {}, "required": {}, "complete": True}
    for run in api(f"/actions/runs?head_sha={sha}&per_page=50").get("workflow_runs", []):
        if run.get("name") not in WORKFLOWS:
            continue
        done = run.get("status") == "completed"
        out["complete"] &= done
        out["workflows"][run["name"]] = {"wall_s": secs(run.get("run_started_at"), run.get("updated_at")) if done else None,
                                         "attempts": run.get("run_attempt", 1), "conclusion": run.get("conclusion")}
        for j in api(f"/actions/runs/{run['id']}/jobs?per_page=100&filter=latest").get("jobs", []):
            if j.get("status") != "completed" or j.get("conclusion") == "skipped":
                continue
            dur = secs(j.get("started_at"), j.get("completed_at"))
            out["jobs"][j["name"]] = dur
            if j["name"] in REQUIRED:
                out["required"][j["name"]] = dur
    wf = out["workflows"].values()
    out["failed_runs"] = sum(w["conclusion"] not in (None, "success", "skipped", "neutral") for w in wf)
    # flaky: green only after a re-run (attempt > 1)
    out["flaky_runs"] = sum(w["conclusion"] == "success" and w["attempts"] > 1 for w in wf)
    out["wall_s"] = max((w["wall_s"] or 0 for w in wf), default=0) or None
    return out


def pr_number(title: str) -> int | None:
    """Squash-merge titles end with `(#123)`."""
    m = re.search(r"\(#(\d+)\)\s*$", title.strip())
    return int(m.group(1)) if m else None


def pr_open_to_merge_s(n: int | None, api) -> int | None:
    if n is None:
        return None
    pr = api(f"/pulls/{n}")
    return secs(pr.get("created_at"), pr.get("merged_at"))


def merged_window(api, now: datetime, days: int = 7) -> dict:
    """PRs merged in the last 24 h and in the last `days` days, and the median open-to-merge time of the latter."""
    since = now - timedelta(days=days)
    merged = [p for p in api("/pulls?state=closed&sort=updated&direction=desc&per_page=100") if p.get("merged_at") and ts(p["merged_at"]) >= since]
    day = [p for p in merged if ts(p["merged_at"]) >= now - timedelta(days=1)]
    times = [secs(p["created_at"], p["merged_at"]) for p in merged]
    return {"merged_24h": len(day), "merged_window": len(merged), "window_days": days,
            "open_to_merge_median_s": int(statistics.median(times)) if times else None}


def facts(sha: str, title: str, api, now: datetime | None = None) -> dict:
    """The `ci` section of a metrics line."""
    now = now or datetime.now(timezone.utc)
    out = runs_of_commit(sha, api)
    n = pr_number(title)
    out["pr"] = n
    out["pr_open_to_merge_s"] = pr_open_to_merge_s(n, api)
    out.update(merged_window(api, now))
    return out
