"""Rate-limit handling of the stat-ci API reads: wait as GitHub asks on 403/429, then let the caller skip the data."""
from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request


class ApiDenied(Exception):
    """The API kept answering 403/429 (rate limit or forbidden) after the retries; the caller skips the data and goes on."""


def warn(msg: str) -> None:
    """Every automatic skip is logged on stderr (and as a workflow annotation), never silent."""
    print(f"::warning::stat-ci: {msg}", file=sys.stderr)


def denied_wait(err, now: float | None = None) -> float:
    """Seconds to wait after a 403/429: Retry-After, else the X-RateLimit-Reset of an exhausted quota, else 30; capped at 120."""
    h = err.headers or {}
    try:
        if h.get("Retry-After"):
            return min(120.0, max(1.0, float(h["Retry-After"])))
        if h.get("X-RateLimit-Remaining") == "0" and h.get("X-RateLimit-Reset"):
            return min(120.0, max(1.0, float(h["X-RateLimit-Reset"]) - (time.time() if now is None else now)))
    except ValueError:
        pass
    return 30.0


def api_get(path: str):
    base = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    repo = os.environ.get("GITHUB_REPOSITORY", "Freeesta/mzlab")
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "mzlab-stat-ci"}
    if os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
    req = urllib.request.Request(f"{base}/repos/{repo}{path}", headers=headers)
    # 503 "Egress is over the account limit" lasts longer than a few seconds: back off up to about a minute
    # 403/429 (rate limits) wait as the server asks, at most 3 times, then raise ApiDenied
    denied = 0
    for i in range(6):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            if e.code in (403, 429):
                denied += 1
                if denied > 3:
                    raise ApiDenied(f"HTTP {e.code} on {path}") from e
                w = denied_wait(e)
                warn(f"HTTP {e.code} on {path}, retrying in {w:.0f} s")
                time.sleep(w)
                continue
            if i == 5:
                raise
            time.sleep(2**i)
        except Exception:
            if i == 5:
                raise
            time.sleep(2**i)
    raise ApiDenied(f"no answer on {path}")
