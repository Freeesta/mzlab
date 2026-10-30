#!/usr/bin/env python3
"""Statistics of the CI bottlenecks: one JSONL line per event, no log text, no personal data.

Used by .github/workflows/stat-ci.yml (the `registra-*` commands, one event per call) and by hand (`riepilogo`).
Standard library only. The data file `eventi.jsonl` lives on the orphan branch `stat-ci` (docs/agenti/ci-veloce.md).

    python3 tools/ci_metrics.py registra-run --evento E.json --nuovi N.jsonl   # workflow_run completed: run + failed jobs
    python3 tools/ci_metrics.py registra-pr --evento E.json --nuovi N.jsonl    # pull_request_target: opened/push/merged/closed
    python3 tools/ci_metrics.py registra-conflitti --evento E.json --dati D.jsonl --nuovi N.jsonl   # push to main: new `dirty` PRs
    python3 tools/ci_metrics.py registra-recenti --dati D.jsonl --nuovi N.jsonl --cursore C.json   # scheduled: runs completed after the cursor + runner load
    python3 tools/ci_metrics.py accoda N.jsonl --dati D.jsonl [--cursore C.json]   # append without duplicates (every line gets `v`)
    python3 tools/ci_metrics.py riempi-verde --dati D.jsonl [--massimo 30]    # best-effort backfill of verde_a_unione_s (rewrites D in place)
    python3 tools/ci_metrics.py compatta --dati D.jsonl [--giorni 14]          # drop the heavy fields of old rows
    python3 tools/ci_metrics.py esporta [--dati D.jsonl] [--out DIR]           # one CSV per row type
    python3 tools/ci_metrics.py percorso [--dati D.jsonl] [--da YYYY-MM-DD]    # push -> last required check of merged PRs, by PR type
    python3 tools/ci_metrics.py riepilogo [--dati D.jsonl] [--da YYYY-MM-DD]   # default data: every events file of origin/stat-ci
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from collections import Counter, defaultdict
from datetime import datetime, timedelta, timezone

import ci_metrics_data as dati_ci
import ci_metrics_file as file_ci
from ci_metrics_file import attivo, eventi_ramo, leggi, leggi_tutti, segmenti  # noqa: F401

# Estimated minutes lost for the events that have no measured duration (printed with the summary).
CONFLITTO_S = 900
FUSIONE_S = 300
RILETTURA_S = 60
OK = ("success", "skipped", "neutral")
CHIAVE = ("tipo", "run_id", "run_attempt", "job", "pr", "azione", "sha", "k")
TIMEOUT_S = 240       # a cancelled job running at least this long, inside a run that was not cancelled, is counted as a timeout
VERSIONE = 1        # field `v` of every line; bump when a field changes meaning

# Category of a failed job from the name of its failed step (no log is read).
CATEGORIE = [
    ("aggregato", r"^Every .* job passed"),      # rust-ok: red only because another job is red or cancelled, so it is not a cause of its own
    ("soglia", r"soglie|controlla\.py|size limit|limits|threshold"),
    ("lint", r"lint|clippy|fmt|format|tsc|syntax|machete|deny|globals|i18n|sources"),
    ("test", r"test|e2e|smoke|pytest|guards|parity|verifica|determinism"),
]


def api_get(path: str):
    base = os.environ.get("GITHUB_API_URL", "https://api.github.com")
    repo = os.environ.get("GITHUB_REPOSITORY", "Freeesta/mzlab")
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "mzlab-stat-ci"}
    if os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
    req = urllib.request.Request(f"{base}/repos/{repo}{path}", headers=headers)
    # 503 "Egress is over the account limit" lasts longer than a few seconds: back off up to about a minute
    for i in range(6):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                return json.load(r)
        except Exception:
            if i == 5:
                raise
            time.sleep(2**i)


def ts(iso: str | None) -> datetime | None:
    return datetime.fromisoformat(iso.replace("Z", "+00:00")) if iso else None


def secs(a: str | None, b: str | None) -> int | None:
    if not a or not b:
        return None
    return max(0, int((ts(b) - ts(a)).total_seconds()))


def adesso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def categoria(job: dict, run_annullato: bool = False) -> tuple[str, str | None]:
    """(category, failed step name) of a job that did not succeed; `run_annullato`: the whole run was cancelled (concurrency)."""
    if job.get("conclusion") == "timed_out":
        return "timeout", None
    ferma = ("failure", "timed_out", "cancelled") if job.get("conclusion") == "cancelled" else ("failure", "timed_out")
    passo = next((s["name"] for s in job.get("steps", []) if s.get("conclusion") in ferma), None)
    if job.get("conclusion") == "cancelled":
        # a job cancelled while its run went on (or after a long run) hit its own timeout, e.g. a stuck npx install; a short one was replaced by a newer push
        lungo = (secs(job.get("started_at"), job.get("completed_at")) or 0) >= TIMEOUT_S
        return ("timeout" if lungo and not run_annullato else "annullato"), passo
    for nome, rx in CATEGORIE:
        if passo and re.search(rx, passo, re.I):
            return nome, passo
    return "altro", passo


def righe_run(evento: dict, api=api_get, fetch=dati_ci.scarica) -> list[dict]:
    return righe_run_obj(evento["workflow_run"], api, fetch)


def righe_run_obj(run: dict, api=api_get, fetch=dati_ci.scarica) -> list[dict]:
    jobs = api(f"/actions/runs/{run['id']}/jobs?per_page=100")["jobs"]
    pr = ((run.get("pull_requests") or [{}])[0]).get("number")
    base = {"pr": pr, "sha": run["head_sha"], "run_id": run["id"], "run_attempt": run.get("run_attempt", 1)}
    durate = {j["name"]: secs(j["started_at"], j["completed_at"]) or 0 for j in jobs}
    code = [secs(j["created_at"], j["started_at"]) or 0 for j in jobs]
    out = [{"ts_utc": run["updated_at"], "tipo": "run", "workflow": run["name"], "esito": run["conclusion"],
            "main": run.get("head_branch") == "main", "durata_s": secs(run.get("run_started_at"), run["updated_at"]),
            "attivo_s": sum(durate.values()), "coda_s": max(code, default=0), "jobs": durate, **base}]
    for j in jobs:
        if j.get("status") == "completed" and j.get("conclusion") not in OK:
            cat, passo = categoria(j, run.get("conclusion") == "cancelled")
            out.append({"ts_utc": j["completed_at"], "tipo": "job_fallito", "workflow": run["name"], "job": j["name"],
                        "esito": j["conclusion"], "durata_s": durate[j["name"]], "categoria": cat, "passo": passo, **base})
    out += dati_ci.righe_job(run, jobs, base) + dati_ci.righe_cache(run, jobs, base, fetch)
    if run.get("conclusion") == "cancelled":
        out += dati_ci.riga_annullo(run, jobs, base, api)
    if run["name"] == "test":
        out += dati_ci.righe_junit(run, base, api, fetch) + dati_ci.righe_wasm(run, base, api, fetch)
    return out


def righe_recenti(dati: str, api=api_get, fetch=dati_ci.scarica, ora: datetime | None = None, massimo: int = 40) -> tuple[list[dict], str]:
    """Rows of the runs completed after the cursor (`cursore.json` next to the data file), oldest first, plus the runner load; returns the new cursor too."""
    ora = ora or dati_ci.adesso_dt()
    cur = os.path.join(os.path.dirname(dati) or ".", "cursore.json")
    try:
        with open(cur, encoding="utf-8") as f:
            ultimo = json.load(f)["ultimo"]
    except (OSError, KeyError, ValueError):
        ultimo = (ora - timedelta(hours=2)).strftime("%Y-%m-%dT%H:%M:%SZ")
    dal = (ts(ultimo) - timedelta(hours=6)).strftime("%Y-%m-%d")
    runs = []
    for pagina in (1, 2, 3):
        r = api(f"/actions/runs?status=completed&created=>={dal}&per_page=100&page={pagina}").get("workflow_runs", [])
        runs += [x for x in r if x["name"] in ("test", "rust", "pages") and x["updated_at"] > ultimo]
        if len(r) < 100:
            break
    runs = sorted(runs, key=lambda x: x["updated_at"])[:massimo]
    out = [riga for run in runs for riga in righe_run_obj(run, api, fetch)]
    out += dati_ci.righe_carico(api, ora.strftime("%Y-%m-%dT%H:%M:%SZ"))
    return out, runs[-1]["updated_at"] if runs else ultimo


CAUSE_COMMIT = [("merge di main", r"Merge .*\bmain\b"), ("limite", r"limit|soglia|tetto|size|budget"),
                ("fmt/clippy", r"fmt|clippy|format|lint")]


def cause_commit(n: int, api) -> dict[str, int]:
    """Commits after the first of PR `n`, counted by cause from the title only."""
    out: Counter = Counter()
    for c in api(f"/pulls/{n}/commits?per_page=100")[1:]:
        titolo = c["commit"]["message"].split("\n")[0]
        if titolo.startswith("Merge pull request"):
            continue
        out[next((k for k, rx in CAUSE_COMMIT if re.search(rx, titolo, re.I)), "altro")] += 1
    return dict(out)


def verde_il(sha: str, api) -> str | None:
    """Time at which the last check of `sha` finished, when all the latest checks passed; None otherwise.
    The `registra` job of stat-ci is left out: the event being recorded is that job itself, still running."""
    latest = {}
    for c in sorted(api(f"/commits/{sha}/check-runs?per_page=100")["check_runs"], key=lambda c: c["id"]):
        if c["name"] != "registra":
            latest[c["name"]] = c
    if not latest or any(c["status"] != "completed" or c["conclusion"] not in OK for c in latest.values()):
        return None
    return max(c["completed_at"] for c in latest.values())


def righe_pr(evento: dict, api=api_get) -> list[dict]:
    pr, azione = evento["pull_request"], evento["action"]
    riga = {"tipo": "pr", "pr": pr["number"], "sha": pr["head"]["sha"]}
    if azione in ("opened", "reopened"):
        return [{"ts_utc": pr["created_at"] if azione == "opened" else adesso(), "azione": "aperta" if azione == "opened" else "riaperta", **riga}]
    if azione == "synchronize":
        return [{"ts_utc": adesso(), "azione": "push", **dati_ci.forma_commit(pr["head"]["sha"], api), **riga}]
    if azione != "closed":
        return []
    fine = pr.get("merged_at") or pr["closed_at"]
    riga |= {"ts_utc": fine, "azione": "unita" if pr.get("merged") else "chiusa", "aperta_s": secs(pr["created_at"], fine), "commit_n": pr.get("commits"),
             "cause_commit": (cause := cause_commit(pr["number"], api)), "merge_main": cause.get("merge di main", 0)}
    riga |= dati_ci.forma_pr(pr["number"], pr, api, file_pr)
    if pr.get("merged"):
        riga["verde_a_unione_s"] = secs(verde_il(pr["head"]["sha"], api), fine)
    return [riga]


def riempi_verde(righe: list[dict], api=api_get, massimo: int = 30) -> int:
    """Best effort: fill `verde_a_unione_s` of merged-PR rows that lack it, at most `massimo` rows per call (one API call each).
    A row is tried once: `verde_provato` marks it, whether or not the checks of its head are still readable."""
    fatte = 0
    for r in righe:
        if fatte >= massimo:
            break
        if r.get("tipo") != "pr" or r.get("azione") != "unita" or r.get("verde_a_unione_s") is not None or r.get("verde_provato") or not r.get("sha"):
            continue
        fatte += 1
        r["verde_provato"] = True
        try:
            r["verde_a_unione_s"] = secs(verde_il(r["sha"], api), r["ts_utc"])
        except Exception:
            pass
    return fatte


def stato_conflitti(dati: str) -> dict[int, bool]:
    stato: dict[int, bool] = {}
    for r in leggi_tutti(dati):
        if r["tipo"] in ("conflitto", "conflitto_risolto"):
            stato[r["pr"]] = r["tipo"] == "conflitto"
    return stato


def file_pr(n: int, api) -> set[str]:
    out: set[str] = set()
    for pagina in (1, 2, 3):
        fs = api(f"/pulls/{n}/files?per_page=100&page={pagina}")
        out |= {f["filename"] for f in fs}
        if len(fs) < 100:
            break
    return out


def righe_conflitti(evento: dict, dati: str, api=api_get, attesa=time.sleep, provare: bool = False) -> list[dict]:
    if evento.get("ref") != "refs/heads/main":
        return []
    sha = evento["after"]
    commit = api(f"/commits/{sha}")
    main_file = {f["filename"] for f in commit.get("files", [])}
    m = re.search(r"Merge pull request #(\d+)|\(#(\d+)\)", commit["commit"]["message"])
    unita = int(m.group(1) or m.group(2)) if m else None
    prima = stato_conflitti(dati)

    def sporco(n: int) -> bool:
        for _ in range(4):
            d = api(f"/pulls/{n}")
            if d.get("mergeable") is not None:
                break
            attesa(2)
        return d.get("mergeable_state") == "dirty"

    def riga(n: int, comuni: list[str]) -> dict:
        veri = dati_ci.file_conflitto(n) if provare else []
        return {"ts_utc": adesso(), "tipo": "conflitto", "pr": n, "sha": sha, "pr_unita": unita,
                "file_comuni": comuni[:5], "n_file_comuni": len(comuni), "file_conflitto": veri}

    out, dubbi = [], []
    for p in api("/pulls?state=open&per_page=100"):
        n = p["number"]
        dirty = sporco(n)
        if dirty and not prima.get(n):
            comuni = sorted(file_pr(n, api) & main_file)
            if comuni:
                out.append(riga(n, comuni))
            else:
                dubbi.append(n)
        elif not dirty and prima.get(n):
            out.append({"ts_utc": adesso(), "tipo": "conflitto_risolto", "pr": n, "sha": sha})
    # Dirty with no file in common may be a transient GitHub state: it counts only if still dirty a minute later.
    if dubbi:
        attesa(RILETTURA_S)
        out += [riga(n, []) for n in dubbi if sporco(n)]
    return out


def chiave(r: dict) -> tuple:
    return tuple(r.get(k) for k in CHIAVE)


def accoda(righe: list[dict], dati: str) -> int:
    visti = {chiave(r) for r in leggi_tutti(dati)}
    nuove = [{"v": VERSIONE, **r} for r in righe if chiave(r) not in visti]
    with open(attivo(dati), "a", encoding="utf-8") as f:
        for r in nuove:
            f.write(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n")
    return len(nuove)


def avanza_cursore(nuovo: str, attuale: str) -> None:
    """Copy cursor file `nuovo` over `attuale` unless it is older: polls on push and on schedule run concurrently."""
    try:
        with open(attuale, encoding="utf-8") as f:
            vecchio = json.load(f)["ultimo"]
        with open(nuovo, encoding="utf-8") as f:
            if vecchio >= json.load(f)["ultimo"]:
                return
    except (OSError, KeyError, ValueError):
        pass
    shutil.copyfile(nuovo, attuale)


def hm(s: float) -> str:
    s = int(s)
    return f"{s // 3600}h{s % 3600 // 60:02d}"


def media(v: list) -> float:
    v = [x for x in v if x is not None]
    return sum(v) / len(v) if v else 0


def colli(ev: list[dict]) -> list[tuple[str, int, int]]:
    """(label, count, estimated seconds lost) per bottleneck."""
    voci: dict[str, list[int]] = defaultdict(lambda: [0, 0])

    def somma(nome, n, perso):
        voci[nome][0] += n
        voci[nome][1] += perso

    runs = [r for r in ev if r["tipo"] == "run"]
    for r in ev:
        if r["tipo"] == "job_fallito":
            somma(f"job failed: {r['job']} [{r['categoria']}]", 1, r.get("durata_s") or 0)
    for r in runs:
        if r.get("run_attempt", 1) > 1:
            somma(f"re-run of {r['workflow']}", 1, r.get("durata_s") or 0)
        if r.get("coda_s"):
            somma(f"queue wait: {r['workflow']}", 1, r["coda_s"])
    per_pr = defaultdict(list)
    for r in runs:
        if r["workflow"] == "test" and r.get("pr") and r.get("run_attempt", 1) == 1:
            per_pr[r["pr"]].append(r.get("durata_s") or 0)
    for durate in per_pr.values():
        if len(durate) > 1:
            somma("CI redone (test runs after the first, per PR)", len(durate) - 1, sum(sorted(durate)[:-1]))
    for r in ev:
        if r["tipo"] == "conflitto":
            fs = r.get("file_comuni") or []
            for f in fs or ["(no file in common)"]:
                somma(f"conflicts: {f}", 1, CONFLITTO_S // max(1, len(fs)))
        if r["tipo"] == "pr" and r.get("merge_main"):
            somma("merges of main into the PR branch", r["merge_main"], r["merge_main"] * FUSIONE_S)
        if r["tipo"] == "pr" and r.get("verde_a_unione_s"):
            somma("wait between green CI and merge", 1, r["verde_a_unione_s"])
    return sorted(((k, v[0], v[1]) for k, v in voci.items()), key=lambda x: -x[2])


def riepilogo(ev: list[dict], da: str | None = None, out=print) -> None:
    if da:
        ev = [r for r in ev if r["ts_utc"][:10] >= da]
    if not ev:
        out("no events")
        return
    out(f"events: {len(ev)}, from {min(r['ts_utc'] for r in ev)} to {max(r['ts_utc'] for r in ev)} (UTC)")
    prs = [r for r in ev if r["tipo"] == "pr"]
    unite = [r for r in prs if r["azione"] == "unita"]
    pr_visti = {r["pr"] for r in prs}
    push = Counter(r["pr"] for r in prs if r["azione"] == "push")
    runs = [r for r in ev if r["tipo"] == "run"]
    ci_pr = defaultdict(lambda: [0, 0, 0])
    for r in runs:
        if r.get("pr"):
            ci_pr[r["pr"]][0] += r.get("durata_s") or 0
            ci_pr[r["pr"]][1] += r.get("coda_s") or 0
            ci_pr[r["pr"]][2] += r["workflow"] == "test" and r.get("run_attempt", 1) == 1
    out("\nPull requests")
    out(f"  opened {sum(r['azione'] in ('aperta', 'riaperta') for r in prs)}, merged {len(unite)}, closed unmerged {sum(r['azione'] == 'chiusa' for r in prs)}")
    out(f"  open to merge: mean {hm(media([r['aperta_s'] for r in unite]))}; green CI to merge: mean {hm(media([r.get('verde_a_unione_s') for r in unite]))}")
    out(f"  pushes per PR: mean {media(list(push.values())):.1f}; test runs per PR: mean {media([v[2] for v in ci_pr.values()]):.1f}; "
        f"CI duration per PR: mean {hm(media([v[0] for v in ci_pr.values()]))}, of which queue {hm(media([v[1] for v in ci_pr.values()]))}")
    cause: Counter = Counter()
    for r in prs:
        cause.update(r.get("cause_commit") or {})
    chiusi = [r for r in prs if r["azione"] in ("unita", "chiusa")]
    out("  pushes per PR by cause (commits after the first): " + (", ".join(f"{k} {v / len(chiusi):.1f}" for k, v in cause.most_common()) if chiusi else "-"))
    out(f"  merges of main per PR (closed PRs): mean {media([r.get('merge_main') for r in prs if r['azione'] in ('unita', 'chiusa')]):.1f}; PRs seen {len(pr_visti)}")
    fall = [r for r in ev if r["tipo"] == "job_fallito"]
    out("\nJobs failing most often")
    for (job, cat), n in Counter((r["job"], r["categoria"]) for r in fall).most_common(10):
        out(f"  {n:4d}  {job} [{cat}]")
    tempi = defaultdict(list)
    for r in runs:
        for nome, d in (r.get("jobs") or {}).items():
            tempi[nome].append(d)
    out("\nMean job time")
    for nome, d in sorted(tempi.items(), key=lambda x: -media(x[1]))[:10]:
        out(f"  {hm(media(d))}  {nome} ({len(d)} runs)")
    conf = [r for r in ev if r["tipo"] == "conflitto"]
    out(f"\nConflicts with main: {len(conf)} (PR becoming dirty)")
    for giorno, n in sorted(Counter(r["ts_utc"][:10] for r in conf).items()):
        out(f"  {giorno}  {n}")
    for pr, n in Counter(r["pr"] for r in conf).most_common(5):
        out(f"  PR #{pr}: {n}")
    out("\nReds per hour of the day (UTC) and per day")
    ore = Counter(r["ts_utc"][11:13] for r in fall)
    out("  hours: " + (", ".join(f"{h}h:{n}" for h, n in ore.most_common(5)) or "-"))
    out("  days:  " + (", ".join(f"{g}:{n}" for g, n in Counter(r["ts_utc"][:10] for r in fall).most_common(5)) or "-"))
    dati_ci.tabelle(ev, out)
    out(f"\nTop 10 bottlenecks by estimated time lost (conflict = {CONFLITTO_S // 60} min, merge of main = {FUSIONE_S // 60} min; the others are measured)")
    for nome, n, perso in colli(ev)[:10]:
        out(f"  {hm(perso):>7}  x{n:<4d} {nome}")


def scrivi_righe(path: str, righe: list[dict]) -> None:
    with open(path, "w", encoding="utf-8") as f:
        f.writelines(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in righe)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("comando", choices=["registra-run", "registra-pr", "registra-conflitti", "registra-recenti", "accoda", "compatta", "riempi-verde", "esporta", "riepilogo", "percorso"])
    ap.add_argument("file", nargs="?", help="accoda: JSONL with the new lines")
    ap.add_argument("--evento", default=os.environ.get("GITHUB_EVENT_PATH"))
    ap.add_argument("--dati", help="events file (default: eventi.jsonl, plus its eventi-YYYY-MM[-N].jsonl parts; riepilogo and esporta: origin/stat-ci)")
    ap.add_argument("--nuovi", help="write the new lines here instead of appending to --dati")
    ap.add_argument("--cursore", help="registra-recenti: write the new cursor here; accoda: copy this cursor next to --dati")
    ap.add_argument("--out", default="stat-ci-csv", help="esporta: folder of the CSV files")
    ap.add_argument("--giorni", type=int, default=14, help="compatta: rows older than this lose their heavy fields")
    ap.add_argument("--da")
    ap.add_argument("--massimo", type=int, default=30, help="riempi-verde: rows tried per call")
    a = ap.parse_args()
    if a.comando in ("riepilogo", "esporta", "percorso"):
        ev = leggi_tutti(a.dati) if a.dati else eventi_ramo()
        if a.comando == "esporta":
            print("\n".join(dati_ci.esporta(ev, a.out)))
        elif a.comando == "percorso":
            for tipo, r in dati_ci.percorso_critico(ev, a.da or "").items():
                print(f"{tipo}: n={r['n']} median {r['mediana']} min, max {r['max']}; slowest jobs: " + ", ".join(f"{n} {m}" for n, m in r["lenti"]))
        else:
            riepilogo(ev, a.da)
        return 0
    dati = a.dati or "eventi.jsonl"
    if a.comando == "accoda":
        print(f"{accoda(leggi(a.file), dati)} new lines")
        if a.cursore and os.path.exists(a.cursore):
            avanza_cursore(a.cursore, os.path.join(os.path.dirname(dati) or ".", "cursore.json"))
        return 0
    if a.comando == "compatta":
        for p in segmenti(dati):
            if os.path.getsize(p) < file_ci.LIMITE_B:
                scrivi_righe(p, dati_ci.compatta(leggi(p), dati_ci.adesso_dt(), a.giorni))
        return 0
    if a.comando == "riempi-verde":
        restano = a.massimo
        for p in reversed(segmenti(dati)):
            if restano <= 0 or os.path.getsize(p) >= file_ci.LIMITE_B:
                continue
            righe = leggi(p)
            restano -= riempi_verde(righe, massimo=restano)
            scrivi_righe(p, righe)
        print(f"{a.massimo - restano} rows tried")
        return 0
    if a.comando == "registra-recenti":
        righe, cursore = righe_recenti(dati)
        if a.cursore:
            with open(a.cursore, "w", encoding="utf-8") as f:
                json.dump({"ultimo": cursore}, f)
    else:
        with open(a.evento, encoding="utf-8") as f:
            evento = json.load(f)
        righe = {"registra-run": lambda: righe_run(evento), "registra-pr": lambda: righe_pr(evento),
                 "registra-conflitti": lambda: righe_conflitti(evento, dati, provare=True)}[a.comando]()
    if a.nuovi:
        scrivi_righe(a.nuovi, righe)
    else:
        accoda(righe, dati)
    print(f"{len(righe)} events")
    return 0


if __name__ == "__main__":
    sys.exit(main())
