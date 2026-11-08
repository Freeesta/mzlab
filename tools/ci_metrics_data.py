"""Raw data collectors of tools/ci_metrics.py: jobs and steps, caches, test durations, cancellations, PR shape, real conflicts, runner load.

Everything is a small JSON row (no log text, no personal data); `api` and `fetch` are injected so the tests use fake ones.
Also the compaction of the data file, the CSV export and the extra summary tables.
"""
from __future__ import annotations

import csv
import io
import json
import os
import re
import subprocess
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET
import zipfile
from collections import Counter, defaultdict
from datetime import datetime, timedelta, timezone

from ci_metrics_giorno import COO_COLUMNS, daily, coo_rows  # noqa: F401

SLOW_S = 5
AREAS = [("workflow", r"^\.github/"), ("docs", r"^(docs/|CLAUDE\.md|.*\.md$)"), ("generati", r"(schemi/.*\.json|globali.*\.d\.ts|stretto(-worker)?\.json|mappa\.md|manifest)"),
        ("rust", r"^(crates/|Cargo\.|vendor/)"), ("test", r"^tests(_e2e)?/"), ("tools", r"^tools/"), ("web", r"^mzlab/web/"), ("python", r"\.py$")]
# Workflows the poll measures, and those that make up the critical path of a PR (the required checks: sito-e2e of `sito-rust` is the slowest).
WORKFLOW_MISURATI = ("test", "rust", "sito-rust", "desktop", "pages")
WORKFLOW_CRITICI = ("test", "rust", "sito-rust", "desktop")
PESANTI = ("passi", "file", "lenti", "fasi", "worker", "moduli")      # fields dropped by the compaction


def area(f: str) -> str:
    return next((n for n, rx in AREAS if re.search(rx, f)), "altro")


def areas(files: list[str]) -> dict[str, int]:
    return dict(Counter(area(f) for f in files))


def download(path: str) -> bytes | None:
    """Raw bytes of an API path whose answer is a redirect (job logs, artifact zips): the token goes to the API only, never to the storage URL."""
    base = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    repo = os.environ.get("GITHUB_REPOSITORY", "Freeesta/mzlab")
    h = {"Accept": "application/vnd.github+json", "User-Agent": "mzlab-stat-ci"}
    if os.environ.get("GITHUB_TOKEN"):
        h["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *a, **k):
            return None
    try:
        urllib.request.build_opener(NoRedirect).open(urllib.request.Request(f"{base}/repos/{repo}{path}", headers=h), timeout=60)
    except urllib.error.HTTPError as e:
        loc = e.headers.get("Location")
        if e.code in (301, 302, 303, 307) and loc:
            with urllib.request.urlopen(urllib.request.Request(loc, headers={"User-Agent": "mzlab-stat-ci"}), timeout=120) as r:
                return r.read()
    except Exception:
        pass
    return None


def steps(job: dict) -> list[list]:
    from ci_metrics import secs
    return [[s["name"], secs(s.get("started_at"), s.get("completed_at")), s.get("conclusion")] for s in job.get("steps", [])]


def job_rows(run: dict, jobs: list[dict], base: dict) -> list[dict]:
    """One row per completed job: queue, duration, outcome and every step with its time (attempts are rows of their own)."""
    from ci_metrics import secs
    return [{"ts_utc": j["completed_at"], "tipo": "job", "k": str(j["id"]), "workflow": run["name"], "job": j["name"], "esito": j.get("conclusion"),
             "coda_s": secs(j.get("created_at"), j.get("started_at")), "durata_s": secs(j.get("started_at"), j.get("completed_at")),
             "passi": steps(j), **base} for j in jobs if j.get("status") == "completed" and j.get("completed_at")]


def cache_da_log(testo: str) -> dict:
    """Keys asked for, hit or miss, key saved and size, from the lines the cache action prints (nothing else of the log is kept)."""
    out: dict = {"ripristinate": [], "mancate": [], "salvata": None, "mb": None}
    for row in testo.splitlines():
        if m := re.search(r"Cache restored from key: (\S+)", row):
            out["ripristinate"].append(m.group(1))
        elif m := re.search(r"Cache not found for input keys: (.+)$", row):
            out["mancate"].append(m.group(1).strip()[:300])
        elif m := re.search(r"Cache saved with key: (\S+)", row):
            out["salvata"] = m.group(1)
        elif m := re.search(r"Cache Size: ~(\d+) MB", row):
            out["mb"] = int(m.group(1))
    return out


def cache_rows(run: dict, jobs: list[dict], base: dict, fetch) -> list[dict]:
    out = []
    for j in jobs:
        if j.get("status") != "completed" or not any(re.search("cache", s["name"], re.I) for s in j.get("steps", [])):
            continue
        raw = fetch(f"/actions/jobs/{j['id']}/logs")
        if raw:
            c = cache_da_log(raw.decode("utf-8", "replace"))
            hit = "primary" if c["ripristinate"] and not c["salvata"] and not c["mancate"] else "restore" if c["ripristinate"] else "miss"
            out.append({"ts_utc": j["completed_at"], "tipo": "cache", "k": str(j["id"]), "workflow": run["name"], "job": j["name"], "esito_cache": hit, **c, **base})
    return out


def zip_artifact(run: dict, prefix: str, api, fetch):
    """Yield (artifact name, ZipFile) of the not expired artifacts of the run whose name starts with `prefix`."""
    for a in api(f"/actions/runs/{run['id']}/artifacts?per_page=50").get("artifacts", []):
        if not a["name"].startswith(prefix) or a.get("expired"):
            continue
        raw = fetch(f"/actions/artifacts/{a['id']}/zip")
        if raw:
            with zipfile.ZipFile(io.BytesIO(raw)) as z:
                yield a["name"], z


def junit_rows(run: dict, base: dict, api, fetch) -> list[dict]:
    """Time per test file (setup + call + teardown) and the tests over LENTO_S, from the `junit-*` artifacts of the run; with `tempi-test.json`
    (tools/ci_pytest_tempi.py) also the setup/call/teardown split per file, the xdist worker of each file and the seconds each worker was busy."""
    out = []
    for name_art, z in zip_artifact(run, "junit-", api, fetch):
        per_file: dict[str, list] = defaultdict(lambda: [0, 0.0])
        lenti = []
        for name in z.namelist():
            if not name.endswith(".xml"):
                continue
            for tc in ET.fromstring(z.read(name)).iter("testcase"):
                cn, t = tc.get("classname", ""), float(tc.get("time", 0))
                f = (tc.get("file") or "tests/" + (cn.split(".")[1] if cn.startswith("tests.") and "." in cn else cn) + ".py")
                per_file[f][0] += 1
                per_file[f][1] += t
                if t > SLOW_S:
                    lenti.append([f"{cn}::{tc.get('name')}", round(t, 1)])
        row = {"ts_utc": run["updated_at"], "tipo": "test_durate", "k": name_art, "workflow": run["name"], "python": name_art[6:].split("-")[0], **({"parte": name_art.split("-", 2)[2]} if name_art.count("-") > 1 else {}),
                "file": {f: [n, round(s, 1)] for f, (n, s) in sorted(per_file.items())}, "lenti": sorted(lenti, key=lambda x: -x[1])[:30], **base}
        if "tempi-test.json" in z.namelist():
            times = json.loads(z.read("tempi-test.json"))
            occupato: Counter = Counter()
            for f, t in times.items():
                for w in t["worker"][:1]:
                    occupato[w] += t["setup"] + t["call"] + t["teardown"]
            row["fasi"] = {f: [t["setup"], t["call"], t["teardown"]] for f, t in times.items()}
            row["worker"] = {f: t["worker"] for f, t in times.items()}
            row["worker_occupato_s"] = {w: round(s, 1) for w, s in sorted(occupato.items())}
        out.append(row)
    return out


def wasm_rows(run: dict, base: dict, api, fetch) -> list[dict]:
    """Size of every built .wasm and its cap (tools/ci_wasm_dimensioni.py), from the `wasm-dimensioni` artifact of the run."""
    out = []
    for name_art, z in zip_artifact(run, "wasm-dimensioni", api, fetch):
        modules = json.loads(z.read("wasm-dimensioni.json"))
        margins = [100 * (1 - m["byte"] / m["tetto"]) for m in modules.values() if m.get("tetto")]
        out.append({"ts_utc": run["updated_at"], "tipo": "wasm", "k": name_art, "workflow": run["name"],
                    "moduli": {f: [m["byte"], m.get("tetto")] for f, m in modules.items()}, "byte_totali": sum(m["byte"] for m in modules.values()),
                    "margine_min_pct": round(min(margins), 1) if margins else None, **base})
    return out


def cancel_row(run: dict, jobs: list[dict], base: dict, api) -> list[dict]:
    """A cancelled run: runner minutes already spent and the SHA that replaced it (next run of the same workflow on the branch)."""
    from ci_metrics import secs
    spesi = sum(secs(j.get("started_at"), j.get("completed_at")) or 0 for j in jobs if j.get("started_at"))
    after = [r for r in api(f"/actions/runs?branch={run.get('head_branch')}&per_page=30").get("workflow_runs", [])
            if r.get("name") == run["name"] and r["head_sha"] != run["head_sha"] and r["created_at"] > run.get("created_at", "")]
    sost = min(after, key=lambda r: r["created_at"])["head_sha"] if after else None
    return [{"ts_utc": run["updated_at"], "tipo": "annullamento", "k": str(run["id"]), "workflow": run["name"], "minuti_runner": round(spesi / 60, 1),
             "sostituito_da": sost, **base}]


def load_rows(api, ora: str) -> list[dict]:
    """Jobs queued and running on the whole repository now (the limit of concurrent runners)."""
    n = {"queued": 0, "in_progress": 0}
    for stato in n:
        for r in api(f"/actions/runs?status={stato}&per_page=50").get("workflow_runs", []):
            for j in api(f"/actions/runs/{r['id']}/jobs?per_page=100").get("jobs", []):
                if j.get("status") in n:
                    n[j["status"]] += 1
    return [{"ts_utc": ora, "tipo": "carico", "k": ora[:15], "job_in_coda": n["queued"], "job_in_corso": n["in_progress"]}]


def forma_commit(sha: str, api) -> dict:
    """Title, kind (merge of main or not) and areas of the files of one commit."""
    c = api(f"/commits/{sha}")
    title = c["commit"]["message"].split("\n")[0]
    return {"titolo": title[:120], "merge_main": len(c.get("parents", [])) > 1 and bool(re.match(r"Merge .*\bmain\b", title)),
            "aree": areas([f["filename"] for f in c.get("files", [])])}


def forma_pr(n: int, pr: dict, api, file_pr) -> dict:
    """Shape of a closed PR: lines, files (capped), areas, doc-only, first review."""
    from ci_metrics import secs
    files = sorted(file_pr(n, api))
    revs = [r["submitted_at"] for r in api(f"/pulls/{n}/reviews?per_page=100") if r.get("submitted_at")]
    a = areas(files)
    return {"righe_piu": pr.get("additions"), "righe_meno": pr.get("deletions"), "file_n": len(files), "file": files[:40], "aree": a,
            "solo_doc": bool(files) and set(a) <= {"docs"}, "prima_review_s": secs(pr["created_at"], min(revs)) if revs else None, "review_n": len(revs)}


def file_conflict(n: int, root: str = ".") -> list[str]:
    """Files in conflict of a trial merge of the PR into origin/main (read-only `git merge-tree`; no code of the PR is run)."""
    def g(*a):
        return subprocess.run(["git", *a], cwd=root, capture_output=True, text=True)
    if g("fetch", "-q", "--filter=blob:none", "origin", "main").returncode or g("fetch", "-q", "--filter=blob:none", "origin", f"pull/{n}/head").returncode:
        return []
    r = g("merge-tree", "--write-tree", "--name-only", "origin/main", "FETCH_HEAD")
    if r.returncode == 0:
        return []
    names = r.stdout.split("\n\n")[0].splitlines()[1:]      # first line: tree id; then the conflicting names; messages follow a blank line
    return sorted(set(names))[:20]


def compact(ev: list[dict], today: datetime, giorni: int = 14) -> list[dict]:
    """Rows older than `giorni`: heavy fields (steps, per-file times, slow tests) are replaced by a count, so the file stays small."""
    limit = (today - timedelta(days=giorni)).strftime("%Y-%m-%dT%H:%M:%SZ")
    for r in ev:
        if r["ts_utc"] < limit:
            for c in PESANTI:
                if c in r and not isinstance(r[c], int):
                    r[c + "_n"] = len(r.pop(c))
    return ev


def export(ev: list[dict], folder: str) -> list[str]:
    """One CSV per row type (nested values as JSON text), plus `giornaliero-coo.csv`: the daily row in the layout of the COO's misure/giornaliero.csv."""
    os.makedirs(folder, exist_ok=True)
    for_type: dict[str, list[dict]] = defaultdict(list)
    for r in ev:
        for_type[r["tipo"]].append(r)
    written = []
    for_type["giornaliero"] = daily(ev)
    for tipo, rows in for_type.items():
        cols = sorted({c for r in rows for c in r})
        path = os.path.join(folder, f"{tipo}.csv")
        with open(path, "w", encoding="utf-8", newline="") as f:
            w = csv.writer(f)
            w.writerow(cols)
            for r in rows:
                w.writerow([json.dumps(r[c], ensure_ascii=False, separators=(",", ":")) if isinstance(r.get(c), (dict, list)) else r.get(c, "") for c in cols])
        written.append(path)
    path = os.path.join(folder, "giornaliero-coo.csv")
    with open(path, "w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=COO_COLUMNS)
        w.writeheader()
        w.writerows(coo_rows(ev))
    return written + [path]


def tabelle(ev: list[dict], out=print) -> None:
    """Extra summary tables on the raw rows (queue and steps, caches, slow test files, cancellations, hot files, time to green, runner load)."""
    from ci_metrics import hm, media
    job = [r for r in ev if r["tipo"] == "job"]
    if job:
        out("\nJobs: queue, duration and slowest step (mean)")
        per = defaultdict(list)
        for r in job:
            per[r["job"]].append(r)
        for name, rr in sorted(per.items(), key=lambda x: -media([r["durata_s"] for r in x[1]]))[:8]:
            slow: dict[str, list] = defaultdict(list)
            for r in rr:
                for p in r.get("passi", []):
                    slow[p[0]].append(p[1] or 0)
            top = max(slow.items(), key=lambda x: media(x[1]), default=("-", [0]))
            out(f"  {name}: queue {hm(media([r['coda_s'] for r in rr]))}, run {hm(media([r['durata_s'] for r in rr]))}, slowest step '{top[0][:40]}' {hm(media(top[1]))} ({len(rr)} runs)")
    ca = [r for r in ev if r["tipo"] == "cache"]
    if ca:
        out("\nCaches (per job): hit rate and mean size saved")
        for name in sorted({r["job"] for r in ca}):
            rr = [r for r in ca if r["job"] == name]
            out(f"  {name}: hit {sum(r['esito_cache'] != 'miss' for r in rr)}/{len(rr)}, saved {media([r['mb'] for r in rr if r.get('mb')]):.0f} MB")
    tf = [r for r in ev if r["tipo"] == "test_durate"]
    if tf:
        tot: Counter = Counter()
        for r in tf:
            for f, (_, s) in (r.get("file") or {}).items():
                tot[f] += s / len(tf)
        out("\nSlowest test files (mean seconds per run)")
        for f, s in tot.most_common(8):
            out(f"  {s:7.0f}  {f}")
    fs = [r for r in tf if r.get("fasi")]
    if fs:
        tot_phases = [sum(f[i] for r in fs for f in r["fasi"].values()) / len(fs) for i in range(3)]
        out(f"\nTest phases (mean per run): setup {hm(tot_phases[0])}, call {hm(tot_phases[1])}, teardown {hm(tot_phases[2])}")
        for r in fs[-2:]:
            w = r.get("worker_occupato_s") or {}
            if w:
                out(f"  {r['python']}: xdist busy seconds per worker " + ", ".join(f"{k} {v:.0f}" for k, v in w.items()) + f" (spread {max(w.values()) - min(w.values()):.0f}s)")
    ws = [r for r in ev if r["tipo"] == "wasm"]
    if ws:
        out(f"\n.wasm: total {ws[-1]['byte_totali'] / 1e6:.1f} MB, smallest margin to a cap {ws[-1]['margine_min_pct']}% (latest of {len(ws)} runs)")
    an = [r for r in ev if r["tipo"] == "annullamento"]
    if an:
        out(f"\nCancelled runs: {len(an)}, runner minutes already spent {sum(r['minuti_runner'] for r in an):.0f}")
    prs = [r for r in ev if r["tipo"] == "pr"]
    hot = Counter(f for r in prs if r["azione"] in ("unita", "chiusa") for f in r.get("file", []))
    if hot:
        out("\nHot files (PRs that touched them): " + ", ".join(f"{f} {n}" for f, n in hot.most_common(6)))
    solo = [r for r in prs if r.get("solo_doc")]
    out(f"\nDoc-only PRs: {len(solo)} of {sum(r['azione'] in ('unita', 'chiusa') for r in prs)} closed")
    green: dict = {}
    for r in ev:
        if r["tipo"] == "run" and r["workflow"] == "test" and r["esito"] == "success" and r.get("pr"):
            green.setdefault(r["pr"], r["ts_utc"])
    first = {r["pr"]: r["ts_utc"] for r in prs if r["azione"] == "aperta"}
    pv = [(datetime.fromisoformat(green[p].replace("Z", "+00:00")) - datetime.fromisoformat(t.replace("Z", "+00:00"))).total_seconds() for p, t in first.items() if p in green]
    if pv:
        out(f"\nOpen to first green test run: mean {hm(media(pv))} ({len(pv)} PRs)")
    car = [r for r in ev if r["tipo"] == "carico"]
    if car:
        out(f"\nRunner load: max {max(r['job_in_corso'] for r in car)} jobs running, max {max(r['job_in_coda'] for r in car)} queued ({len(car)} readings)")


def critical_path(ev: list[dict], since: str = "") -> dict[str, dict]:
    """Per PR type (docs / Rust / site+other): median and max minutes from push to the last required check, and the slowest jobs, over merged PRs.

    The critical path of a head sha is the longest run among WORKFLOW_CRITICI (first attempt, green); `dal` is an ISO timestamp lower bound.
    """
    unite = {x["sha"]: x for x in ev if x.get("tipo") == "pr" and x.get("azione") == "unita"}
    corse: dict[str, dict[str, dict]] = defaultdict(dict)
    for x in ev:
        if x.get("tipo") == "run" and not x.get("main") and x.get("sha") in unite and x.get("workflow") in WORKFLOW_CRITICI \
                and x.get("run_attempt") == 1 and x.get("esito") == "success" and x["ts_utc"] >= since:
            corse[x["sha"]][x["workflow"]] = x
    durations: dict[str, list[float]] = defaultdict(list)
    jobs: dict[str, dict[str, list[float]]] = defaultdict(lambda: defaultdict(list))
    for sha, r in corse.items():
        if "test" not in r:
            continue
        pr = unite[sha]
        tipo = "docs" if pr.get("solo_doc") else "rust" if (pr.get("aree") or {}).get("rust") else "site/other"
        durations[tipo].append(max(x["durata_s"] for x in r.values()) / 60)
        for x in r.values():
            for name, d in x["jobs"].items():
                jobs[tipo][name].append(d / 60)

    def med(v):
        v = sorted(v)
        return v[len(v) // 2] if len(v) % 2 else (v[len(v) // 2 - 1] + v[len(v) // 2]) / 2

    return {t: {"n": len(v), "mediana": round(med(v), 1), "max": round(max(v), 1),
                "lenti": [(n, round(med(d), 1)) for n, d in sorted(jobs[t].items(), key=lambda kv: -med(kv[1]))[:4]]}
            for t, v in durations.items()}


def adesso_dt() -> datetime:
    return datetime.now(timezone.utc)
