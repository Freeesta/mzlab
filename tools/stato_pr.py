#!/usr/bin/env python3
"""One line per open PR of Freeesta/mzlab: head SHA, required checks, mergeable state, auto-merge.

Read-only, standard library only. Used by the COO instead of checking PRs by hand.
Usage: python3 tools/stato_pr.py [--stato PATH]  (PATH: stato.json of the private repo;
default $MZLAB_STATO or ../mzlab-dati/coordinamento/stato.json when present).
Auth: none; the cloud session proxy authenticates api.github.com.
"""
import argparse
import json
import os
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone

API = "https://api.github.com/repos/Freeesta/mzlab"


def get(url):
    req = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json", "User-Agent": "mzlab-stato-pr"})
    for i in range(4):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                return json.load(r), r.headers.get("Link", "")
        except Exception:
            if i == 3:
                raise
            time.sleep(2 ** i)


def pages(url, key=None):
    # "next" links use /repositories/<id>/, which the proxy refuses: keep our prefix, take the query.
    base = url.split("?")[0]
    while url:
        data, link = get(url)
        yield from (data[key] if key else data)
        url = None
        for part in link.split(","):
            if 'rel="next"' in part:
                url = base + "?" + part[part.find("<") + 1:part.find(">")].split("?", 1)[1]


def minutes_since(iso):
    t = datetime.fromisoformat(iso.replace("Z", "+00:00"))
    return int((datetime.now(timezone.utc) - t).total_seconds() // 60)


def check_state(sha, required):
    runs = list(pages(f"{API}/commits/{sha}/check-runs?per_page=100", "check_runs"))
    # Latest run of each name wins (re-runs).
    latest = {}
    for r in sorted(runs, key=lambda r: r["id"]):
        latest[r["name"]] = r
    sel = [latest[n] for n in required if n in latest]
    ok = sum(r["conclusion"] in ("success", "skipped", "neutral") for r in sel)
    bad = sum(r["status"] == "completed" and r["conclusion"] not in ("success", "skipped", "neutral") for r in sel)
    miss = len(required) - len(sel)
    if bad:
        verdict = "ROSSO"
    elif ok == len(required):
        verdict = "verde"
    else:
        verdict = "in corso"
    return f"{ok}/{len(required)} {verdict}" + (f" ({miss} non partiti)" if miss and not bad else "")


def row(pr, required, chats):
    d, _ = get(f"{API}/pulls/{pr['number']}")
    sha = d["head"]["sha"]
    am = "auto-merge sì" if d.get("auto_merge") else "auto-merge no"
    chat = chats.get(pr["number"], {}).get("chat", "-")
    return (pr["number"], f"#{pr['number']:<5} {sha} {check_state(sha, required):<22} "
            f"{d['mergeable_state']:<9} {am:<13} {minutes_since(d['updated_at']):>4} min  {chat}  {pr['title'][:50]}")


def main_test():
    runs, _ = get(f"{API}/actions/runs?branch=main&event=push&per_page=30")
    tests = [r for r in runs["workflow_runs"] if r["name"] == "test"]
    done = next((r for r in tests if r["status"] == "completed"), None)
    out = ["main test:"]
    if tests and tests[0] is not done:
        out.append(f"{tests[0]['status']} {tests[0]['head_sha'][:8]};")
    if done:
        out.append(f"ultimo concluso {done['conclusion']} {done['head_sha'][:8]} ({minutes_since(done['created_at'])} min fa)")
    return " ".join(out) if tests else "main test: nessun run recente"


def load_chats(path):
    path = path or os.environ.get("MZLAB_STATO") or os.path.join(os.path.dirname(__file__), "..", "..", "mzlab-dati", "coordinamento", "stato.json")
    if not os.path.exists(path):
        return {}
    data = json.load(open(path))
    data = data.get("chat", data) if isinstance(data, dict) else data
    return {e["pr"]: e for e in data if isinstance(e, dict) and e.get("pr")}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--stato")
    args = ap.parse_args()
    chats = load_chats(args.stato)
    prot, _ = get(f"{API}/branches/main/protection/required_status_checks")
    required = prot["contexts"]
    prs = list(pages(f"{API}/pulls?state=open&per_page=100"))
    with ThreadPoolExecutor(8) as ex:
        rows = sorted(ex.map(lambda p: row(p, required, chats), prs))
    print(f"{len(prs)} PR aperte; check richiesti: {len(required)}")
    for _, line in rows:
        print(line)
    print(main_test())


if __name__ == "__main__":
    sys.exit(main())
