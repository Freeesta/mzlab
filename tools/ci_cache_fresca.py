"""Decides whether a run on main saves its Rust `target` cache: not while the newest cache of main with the same prefix (job kind and Cargo.lock) is younger
than MZLAB_CACHE_ORE hours (default 6). Every push to main that touched a crate saved ~1 GB per job, ~130 GB a day against the 10 GB of the repository,
and the constant eviction also took the small caches (pages-*, sito-pw, playwright). Pull requests and later runs restore the older cache and rebuild only
the crates changed since (tools/ci_cargo_mtime.py). Writes `fresh=true|false` to $GITHUB_OUTPUT; any API failure means false (save as before).
Usage: ci_cache_fresca.py PREFIX; reads GITHUB_TOKEN, GITHUB_REPOSITORY, GITHUB_API_URL (job needs `actions: read`)."""
import json
import os
import sys
import time
import urllib.parse
import urllib.request
from datetime import datetime
from typing import Callable

HOURS = float(os.environ.get("MZLAB_CACHE_ORE") or 6)


def newest(prefix: str) -> list[dict]:
    """Caches of main whose key starts with `prefix`, newest first (the API matches the key as a prefix)."""
    base = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    q = urllib.parse.urlencode({"key": prefix, "ref": "refs/heads/main", "sort": "created_at", "direction": "desc", "per_page": 5})
    req = urllib.request.Request(f"{base}/repos/{os.environ['GITHUB_REPOSITORY']}/actions/caches?{q}",
                                 headers={"Authorization": f"Bearer {os.environ['GITHUB_TOKEN']}", "Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r).get("actions_caches", [])


def age_hours(prefix: str, caches: Callable[[str], list[dict]], now: float) -> float | None:
    """Age of the newest cache of main with that prefix, None when there is none."""
    created = [datetime.fromisoformat(c["created_at"].replace("Z", "+00:00")).timestamp()
               for c in caches(prefix) if c.get("key", "").startswith(prefix) and c.get("created_at")]
    return (now - max(created)) / 3600 if created else None


def fresh(prefix: str, caches: Callable[[str], list[dict]] | None = None, now: float | None = None, hours: float = HOURS) -> bool:
    try:
        age = age_hours(prefix, caches or newest, time.time() if now is None else now)
    except Exception as e:  # noqa: BLE001 - an API failure must never stop a save
        print(f"cache list failed ({e}): saving")
        return False
    if age is None:
        print(f"no cache of main starts with {prefix}: saving")
        return False
    print(f"newest cache of main with {prefix} is {age:.1f} h old (limit {hours:g} h): {'not saving' if age < hours else 'saving'}")
    return age < hours


def main(argv: list[str]) -> int:
    out = fresh(argv[1])
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
            f.write(f"fresh={str(out).lower()}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
