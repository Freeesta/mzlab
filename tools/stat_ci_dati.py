"""Raw data collectors of tools/stat_ci.py: jobs and steps, caches, test durations, cancellations, PR shape, real conflicts, runner load.

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

LENTO_S = 5
AREE = [("workflow", r"^\.github/"), ("docs", r"^(docs/|AGENTS\.md|.*\.md$)"), ("generati", r"(schemi/.*\.json|globali.*\.d\.ts|stretto\.json|mappa\.md|manifest)"),
        ("rust", r"^(crates/|Cargo\.|vendor/)"), ("test", r"^tests(_e2e)?/"), ("tools", r"^tools/"), ("web", r"^mzlab/web/"), ("python", r"\.py$")]
PESANTI = ("passi", "file", "lenti", "fasi", "worker", "moduli")      # fields dropped by the compaction


def area(f: str) -> str:
    return next((n for n, rx in AREE if re.search(rx, f)), "altro")


def aree(files: list[str]) -> dict[str, int]:
    return dict(Counter(area(f) for f in files))


def scarica(path: str) -> bytes | None:
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


def passi(job: dict) -> list[list]:
    from stat_ci import secs
    return [[s["name"], secs(s.get("started_at"), s.get("completed_at")), s.get("conclusion")] for s in job.get("steps", [])]


def righe_job(run: dict, jobs: list[dict], base: dict) -> list[dict]:
    """One row per completed job: queue, duration, outcome and every step with its time (attempts are rows of their own)."""
    from stat_ci import secs
    return [{"ts_utc": j["completed_at"], "tipo": "job", "k": str(j["id"]), "workflow": run["name"], "job": j["name"], "esito": j.get("conclusion"),
             "coda_s": secs(j.get("created_at"), j.get("started_at")), "durata_s": secs(j.get("started_at"), j.get("completed_at")),
             "passi": passi(j), **base} for j in jobs if j.get("status") == "completed" and j.get("completed_at")]


def cache_da_log(testo: str) -> dict:
    """Keys asked for, hit or miss, key saved and size, from the lines the cache action prints (nothing else of the log is kept)."""
    out: dict = {"ripristinate": [], "mancate": [], "salvata": None, "mb": None}
    for riga in testo.splitlines():
        if m := re.search(r"Cache restored from key: (\S+)", riga):
            out["ripristinate"].append(m.group(1))
        elif m := re.search(r"Cache not found for input keys: (.+)$", riga):
            out["mancate"].append(m.group(1).strip()[:300])
        elif m := re.search(r"Cache saved with key: (\S+)", riga):
            out["salvata"] = m.group(1)
        elif m := re.search(r"Cache Size: ~(\d+) MB", riga):
            out["mb"] = int(m.group(1))
    return out


def righe_cache(run: dict, jobs: list[dict], base: dict, fetch) -> list[dict]:
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


def zip_artifact(run: dict, prefisso: str, api, fetch):
    """Yield (artifact name, ZipFile) of the not expired artifacts of the run whose name starts with `prefisso`."""
    for a in api(f"/actions/runs/{run['id']}/artifacts?per_page=50").get("artifacts", []):
        if not a["name"].startswith(prefisso) or a.get("expired"):
            continue
        raw = fetch(f"/actions/artifacts/{a['id']}/zip")
        if raw:
            with zipfile.ZipFile(io.BytesIO(raw)) as z:
                yield a["name"], z


def righe_junit(run: dict, base: dict, api, fetch) -> list[dict]:
    """Time per test file (setup + call + teardown) and the tests over LENTO_S, from the `junit-*` artifacts of the run; with `tempi-test.json`
    (tools/ci_pytest_tempi.py) also the setup/call/teardown split per file, the xdist worker of each file and the seconds each worker was busy."""
    out = []
    for nome_art, z in zip_artifact(run, "junit-", api, fetch):
        per_file: dict[str, list] = defaultdict(lambda: [0, 0.0])
        lenti = []
        for nome in z.namelist():
            if not nome.endswith(".xml"):
                continue
            for tc in ET.fromstring(z.read(nome)).iter("testcase"):
                cn, t = tc.get("classname", ""), float(tc.get("time", 0))
                f = (tc.get("file") or "tests/" + (cn.split(".")[1] if cn.startswith("tests.") and "." in cn else cn) + ".py")
                per_file[f][0] += 1
                per_file[f][1] += t
                if t > LENTO_S:
                    lenti.append([f"{cn}::{tc.get('name')}", round(t, 1)])
        riga = {"ts_utc": run["updated_at"], "tipo": "test_durate", "k": nome_art, "workflow": run["name"], "python": nome_art[6:],
                "file": {f: [n, round(s, 1)] for f, (n, s) in sorted(per_file.items())}, "lenti": sorted(lenti, key=lambda x: -x[1])[:30], **base}
        if "tempi-test.json" in z.namelist():
            tempi = json.loads(z.read("tempi-test.json"))
            occupato: Counter = Counter()
            for f, t in tempi.items():
                for w in t["worker"][:1]:
                    occupato[w] += t["setup"] + t["call"] + t["teardown"]
            riga["fasi"] = {f: [t["setup"], t["call"], t["teardown"]] for f, t in tempi.items()}
            riga["worker"] = {f: t["worker"] for f, t in tempi.items()}
            riga["worker_occupato_s"] = {w: round(s, 1) for w, s in sorted(occupato.items())}
        out.append(riga)
    return out


def righe_wasm(run: dict, base: dict, api, fetch) -> list[dict]:
    """Size of every built .wasm and its cap (tools/ci_wasm_dimensioni.py), from the `wasm-dimensioni` artifact of the run."""
    out = []
    for nome_art, z in zip_artifact(run, "wasm-dimensioni", api, fetch):
        moduli = json.loads(z.read("wasm-dimensioni.json"))
        margini = [100 * (1 - m["byte"] / m["tetto"]) for m in moduli.values() if m.get("tetto")]
        out.append({"ts_utc": run["updated_at"], "tipo": "wasm", "k": nome_art, "workflow": run["name"],
                    "moduli": {f: [m["byte"], m.get("tetto")] for f, m in moduli.items()}, "byte_totali": sum(m["byte"] for m in moduli.values()),
                    "margine_min_pct": round(min(margini), 1) if margini else None, **base})
    return out


def riga_annullo(run: dict, jobs: list[dict], base: dict, api) -> list[dict]:
    """A cancelled run: runner minutes already spent and the SHA that replaced it (next run of the same workflow on the branch)."""
    from stat_ci import secs
    spesi = sum(secs(j.get("started_at"), j.get("completed_at")) or 0 for j in jobs if j.get("started_at"))
    dopo = [r for r in api(f"/actions/runs?branch={run.get('head_branch')}&per_page=30").get("workflow_runs", [])
            if r.get("name") == run["name"] and r["head_sha"] != run["head_sha"] and r["created_at"] > run.get("created_at", "")]
    sost = min(dopo, key=lambda r: r["created_at"])["head_sha"] if dopo else None
    return [{"ts_utc": run["updated_at"], "tipo": "annullamento", "k": str(run["id"]), "workflow": run["name"], "minuti_runner": round(spesi / 60, 1),
             "sostituito_da": sost, **base}]


def righe_carico(api, ora: str) -> list[dict]:
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
    titolo = c["commit"]["message"].split("\n")[0]
    return {"titolo": titolo[:120], "merge_main": len(c.get("parents", [])) > 1 and bool(re.match(r"Merge .*\bmain\b", titolo)),
            "aree": aree([f["filename"] for f in c.get("files", [])])}


def forma_pr(n: int, pr: dict, api, file_pr) -> dict:
    """Shape of a closed PR: lines, files (capped), areas, doc-only, first review."""
    from stat_ci import secs
    files = sorted(file_pr(n, api))
    revs = [r["submitted_at"] for r in api(f"/pulls/{n}/reviews?per_page=100") if r.get("submitted_at")]
    a = aree(files)
    return {"righe_piu": pr.get("additions"), "righe_meno": pr.get("deletions"), "file_n": len(files), "file": files[:40], "aree": a,
            "solo_doc": bool(files) and set(a) <= {"docs"}, "prima_review_s": secs(pr["created_at"], min(revs)) if revs else None, "review_n": len(revs)}


def file_conflitto(n: int, root: str = ".") -> list[str]:
    """Files in conflict of a trial merge of the PR into origin/main (read-only `git merge-tree`; no code of the PR is run)."""
    def g(*a):
        return subprocess.run(["git", *a], cwd=root, capture_output=True, text=True)
    if g("fetch", "-q", "--filter=blob:none", "origin", "main").returncode or g("fetch", "-q", "--filter=blob:none", "origin", f"pull/{n}/head").returncode:
        return []
    r = g("merge-tree", "--write-tree", "--name-only", "origin/main", "FETCH_HEAD")
    if r.returncode == 0:
        return []
    nomi = r.stdout.split("\n\n")[0].splitlines()[1:]      # first line: tree id; then the conflicting names; messages follow a blank line
    return sorted(set(nomi))[:20]


def compatta(ev: list[dict], oggi: datetime, giorni: int = 14) -> list[dict]:
    """Rows older than `giorni`: heavy fields (steps, per-file times, slow tests) are replaced by a count, so the file stays small."""
    limite = (oggi - timedelta(days=giorni)).strftime("%Y-%m-%dT%H:%M:%SZ")
    for r in ev:
        if r["ts_utc"] < limite:
            for c in PESANTI:
                if c in r and not isinstance(r[c], int):
                    r[c + "_n"] = len(r.pop(c))
    return ev


def giornaliero(ev: list[dict]) -> list[dict]:
    """One row per UTC day: runs, re-runs, mean queue, failed jobs by category, PRs opened/merged, mean open-to-merge, green-to-merge and commits per merged PR, conflicts."""
    from stat_ci import media
    giorni: dict[str, dict] = defaultdict(lambda: {"run": 0, "rilanci": 0, "coda": [], "job_rossi": Counter(), "pr_aperte": 0, "pr_unite": 0,
                                                   "aperta_s": [], "verde_a_unione_s": [], "commit_n": [], "conflitti": 0})
    for r in ev:
        g = giorni[r["ts_utc"][:10]]
        t = r["tipo"]
        if t == "run":
            g["run"] += 1
            g["rilanci"] += r.get("run_attempt", 1) > 1
            g["coda"].append(r.get("coda_s"))
        elif t == "job_fallito":
            g["job_rossi"][r["categoria"]] += 1
        elif t == "conflitto":
            g["conflitti"] += 1
        elif t == "pr" and r["azione"] == "aperta":
            g["pr_aperte"] += 1
        elif t == "pr" and r["azione"] == "unita":
            g["pr_unite"] += 1
            for k in ("aperta_s", "verde_a_unione_s", "commit_n"):
                g[k].append(r.get(k))
    cat = sorted({c for g in giorni.values() for c in g["job_rossi"]})
    return [{"giorno": d, "run": g["run"], "rilanci": g["rilanci"], "coda_media_s": round(media(g["coda"])), "pr_aperte": g["pr_aperte"], "pr_unite": g["pr_unite"],
             "aperta_media_s": round(media(g["aperta_s"])), "verde_a_unione_media_s": round(media(g["verde_a_unione_s"])),
             "commit_per_pr_media": round(media(g["commit_n"]), 1), "conflitti": g["conflitti"],
             **{"rossi_" + c: g["job_rossi"][c] for c in cat}} for d, g in sorted(giorni.items())]


def esporta(ev: list[dict], cartella: str) -> list[str]:
    """One CSV per row type; nested values as JSON text."""
    os.makedirs(cartella, exist_ok=True)
    per_tipo: dict[str, list[dict]] = defaultdict(list)
    for r in ev:
        per_tipo[r["tipo"]].append(r)
    scritti = []
    per_tipo["giornaliero"] = giornaliero(ev)
    for tipo, righe in per_tipo.items():
        cols = sorted({c for r in righe for c in r})
        path = os.path.join(cartella, f"{tipo}.csv")
        with open(path, "w", encoding="utf-8", newline="") as f:
            w = csv.writer(f)
            w.writerow(cols)
            for r in righe:
                w.writerow([json.dumps(r[c], ensure_ascii=False, separators=(",", ":")) if isinstance(r.get(c), (dict, list)) else r.get(c, "") for c in cols])
        scritti.append(path)
    return scritti


def tabelle(ev: list[dict], out=print) -> None:
    """Extra summary tables on the raw rows (queue and steps, caches, slow test files, cancellations, hot files, time to green, runner load)."""
    from stat_ci import hm, media
    job = [r for r in ev if r["tipo"] == "job"]
    if job:
        out("\nJobs: queue, duration and slowest step (mean)")
        per = defaultdict(list)
        for r in job:
            per[r["job"]].append(r)
        for nome, rr in sorted(per.items(), key=lambda x: -media([r["durata_s"] for r in x[1]]))[:8]:
            lento: dict[str, list] = defaultdict(list)
            for r in rr:
                for p in r.get("passi", []):
                    lento[p[0]].append(p[1] or 0)
            top = max(lento.items(), key=lambda x: media(x[1]), default=("-", [0]))
            out(f"  {nome}: queue {hm(media([r['coda_s'] for r in rr]))}, run {hm(media([r['durata_s'] for r in rr]))}, slowest step '{top[0][:40]}' {hm(media(top[1]))} ({len(rr)} runs)")
    ca = [r for r in ev if r["tipo"] == "cache"]
    if ca:
        out("\nCaches (per job): hit rate and mean size saved")
        for nome in sorted({r["job"] for r in ca}):
            rr = [r for r in ca if r["job"] == nome]
            out(f"  {nome}: hit {sum(r['esito_cache'] != 'miss' for r in rr)}/{len(rr)}, saved {media([r['mb'] for r in rr if r.get('mb')]):.0f} MB")
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
        tot_fasi = [sum(f[i] for r in fs for f in r["fasi"].values()) / len(fs) for i in range(3)]
        out(f"\nTest phases (mean per run): setup {hm(tot_fasi[0])}, call {hm(tot_fasi[1])}, teardown {hm(tot_fasi[2])}")
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
    caldi = Counter(f for r in prs if r["azione"] in ("unita", "chiusa") for f in r.get("file", []))
    if caldi:
        out("\nHot files (PRs that touched them): " + ", ".join(f"{f} {n}" for f, n in caldi.most_common(6)))
    solo = [r for r in prs if r.get("solo_doc")]
    out(f"\nDoc-only PRs: {len(solo)} of {sum(r['azione'] in ('unita', 'chiusa') for r in prs)} closed")
    verde: dict = {}
    for r in ev:
        if r["tipo"] == "run" and r["workflow"] == "test" and r["esito"] == "success" and r.get("pr"):
            verde.setdefault(r["pr"], r["ts_utc"])
    primo = {r["pr"]: r["ts_utc"] for r in prs if r["azione"] == "aperta"}
    pv = [(datetime.fromisoformat(verde[p].replace("Z", "+00:00")) - datetime.fromisoformat(t.replace("Z", "+00:00"))).total_seconds() for p, t in primo.items() if p in verde]
    if pv:
        out(f"\nOpen to first green test run: mean {hm(media(pv))} ({len(pv)} PRs)")
    car = [r for r in ev if r["tipo"] == "carico"]
    if car:
        out(f"\nRunner load: max {max(r['job_in_corso'] for r in car)} jobs running, max {max(r['job_in_coda'] for r in car)} queued ({len(car)} readings)")


def adesso_dt() -> datetime:
    return datetime.now(timezone.utc)
