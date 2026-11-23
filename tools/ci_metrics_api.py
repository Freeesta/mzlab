"""GitHub API reads of the stat-ci poll: rate-limit aware, bounded, and loud about what it could not read.

The workflow token (GITHUB_TOKEN) has a primary quota of 1000 requests per hour shared by every job of the repository, so the poll keeps a
reserve for the other jobs, waits as GitHub asks on a 403/429 only when the wait is short, and gives up (ApiDenied) after that. Once it
gave up, the rest of the process fails fast instead of waiting again for every call. `quota_line()` and the body of each denial are
printed, so a failure says whether it was the quota, a secondary limit or a missing permission.
"""
from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone

MAX_WAIT_S = 120.0     # a longer wait than this is not worth a runner minute: give up, the next poll resumes from the cursor
RESERVE = 200          # requests of the hourly quota left for the other jobs; the poll stops reading below it
DENIED_RETRIES = 3

_state: dict = {"calls": 0, "remaining": None, "denied": None}


class ApiDenied(Exception):
    """The API kept answering 403/429 (rate limit or forbidden), or the quota reserve was reached; the caller keeps what it already read."""


def warn(msg: str) -> None:
    """Every automatic skip is logged on stderr (and as a workflow annotation), never silent."""
    print(f"::warning::stat-ci: {msg}", file=sys.stderr)


def reset() -> None:
    _state.update(calls=0, remaining=None, denied=None)


def denied() -> str | None:
    """Why the API stopped answering in this process (None while it did not): the poll turns it into a visible row."""
    return _state["denied"]


def quota_line() -> str:
    return f"api: {_state['calls']} requests, {_state['remaining']} left of the hourly quota (reserve {RESERVE})"


def note(headers) -> None:
    _state["calls"] += 1
    try:
        _state["remaining"] = int(headers.get("X-RateLimit-Remaining"))
    except (TypeError, ValueError):
        pass


def _asked(err, now: float | None = None) -> float | None:
    """Seconds the server asks to wait (Retry-After, else the reset of an exhausted quota); None when it says nothing."""
    h = err.headers or {}
    try:
        if h.get("Retry-After"):
            return max(1.0, float(h["Retry-After"]))
        if h.get("X-RateLimit-Remaining") == "0" and h.get("X-RateLimit-Reset"):
            return max(1.0, float(h["X-RateLimit-Reset"]) - (time.time() if now is None else now))
    except ValueError:
        pass
    return None


def denied_wait(err, now: float | None = None) -> float:
    """Seconds to wait after a 403/429: what the server asks, else 30; capped at MAX_WAIT_S."""
    asked = _asked(err, now)
    return min(MAX_WAIT_S, 30.0 if asked is None else asked)


def _detail(err) -> str:
    """Message of the error body and the rate-limit headers: the evidence that tells the quota from a missing permission."""
    try:
        msg = str(json.loads(err.read().decode("utf-8", "replace")).get("message", ""))[:160]
    except Exception:
        msg = ""
    h = err.headers or {}
    hdr = " ".join(f"{k}={h[k]}" for k in ("Retry-After", "X-RateLimit-Remaining", "X-RateLimit-Reset", "X-RateLimit-Resource") if h.get(k))
    return f"{msg} [{hdr}]".strip()


def handle_denied(err, path: str, attempt: int) -> None:
    """After a 403/429: sleep as asked and return so the caller retries, or raise ApiDenied (permission problem, long wait, retries used up)."""
    detail = _detail(err)
    asked = _asked(err)
    if "not accessible by integration" in detail:
        why = "missing permission of the job token"
    elif asked is not None and asked > MAX_WAIT_S:
        why = f"asked to wait {asked:.0f} s"
    elif attempt >= DENIED_RETRIES:
        why = "still denied after the retries"
    else:
        w = denied_wait(err)
        warn(f"HTTP {err.code} on {path}, retrying in {w:.0f} s: {detail}")
        time.sleep(w)
        return
    _state["denied"] = f"HTTP {err.code} on {path}: {why}: {detail}"
    raise ApiDenied(_state["denied"]) from err


def check_reserve(path: str) -> None:
    """Fail fast once the API denied this process, or when only the reserve of the hourly quota is left."""
    if _state["denied"]:
        raise ApiDenied(f"{path}: not tried, the API already denied this run ({_state['denied']})")
    if _state["remaining"] is not None and _state["remaining"] < RESERVE:
        _state["denied"] = f"hourly quota down to {_state['remaining']} requests, reserve {RESERVE} left for the other jobs"
        raise ApiDenied(_state["denied"])


DENIAL_POLLS = 3       # denied polls within DENIAL_WINDOW_S that make a denial persistent
DENIAL_WINDOW_S = 3600


def _now() -> datetime:
    return datetime.now(timezone.utc)


def denial_row(command: str, reason: str) -> dict:
    """The API stopped a poll: kept as data (key = 10-minute bucket + command) so a repeated denial can be counted and seen."""
    now = _now().strftime("%Y-%m-%dT%H:%M:%SZ")
    return {"ts_utc": now, "tipo": "api_denied", "k": f"{now[:15]}-{command}", "command": command, "reason": reason[:300]}


def denials_exit(ev: list[dict], now: datetime | None = None) -> int:
    """3 when at least DENIAL_POLLS denied polls were recorded in the last hour: one transient denial is a warning, a persistent one is a red run."""
    since = ((now or _now()) - timedelta(seconds=DENIAL_WINDOW_S)).strftime("%Y-%m-%dT%H:%M:%SZ")
    recent = [r for r in ev if r.get("tipo") == "api_denied" and r["ts_utc"] >= since]
    if len({r["k"] for r in recent}) < DENIAL_POLLS:
        return 0
    print(f"::error::stat-ci: the GitHub API denied {len(recent)} polls in the last hour; last reason: {recent[-1]['reason']}", file=sys.stderr)
    return 3


def headers() -> dict:
    h = {"Accept": "application/vnd.github+json", "User-Agent": "mzlab-stat-ci"}
    if os.environ.get("GITHUB_TOKEN"):
        h["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
    return h


def api_url(path: str) -> str:
    return f"{os.environ.get('GITHUB_API_URL', 'https://api.github.com')}/repos/{os.environ.get('GITHUB_REPOSITORY', 'Freeesta/mzlab')}{path}"


def api_get(path: str):
    check_reserve(path)
    req = urllib.request.Request(api_url(path), headers=headers())
    # 503 "Egress is over the account limit" lasts longer than a few seconds: back off up to about a minute
    denials = 0
    for i in range(6):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                note(r.headers)
                return json.load(r)
        except urllib.error.HTTPError as e:
            if e.code in (403, 429):
                handle_denied(e, path, denials)
                denials += 1
                continue
            if i == 5:
                raise
            time.sleep(2**i)
        except Exception:
            if i == 5:
                raise
            time.sleep(2**i)
    raise ApiDenied(f"no answer on {path}")
