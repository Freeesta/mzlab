"""Daily and queue statistics of tools/ci_metrics.py: the per-day table, the COO row, the `coda` snapshot of the open PRs and its summary.

Standard library only; `api` is injected so the tests use a fake one. Real reds exclude cancelled runs and the aggregate gate.
"""
from __future__ import annotations

import statistics
import time
from collections import Counter, defaultdict
from datetime import datetime

NON_VERI = ("annullato", "aggregato")      # failed-job categories that are not a real red: superseded by a push / the gate repeating another red
COLONNE_COO = ["date", "merged", "median_open_to_merge_min", "max_open_to_merge_min", "open_prs_end", "chats_running", "note"]


def _ok(v: list) -> list:
    return [x for x in v if x is not None]


def media(v: list) -> float:
    v = _ok(v)
    return sum(v) / len(v) if v else 0


def mediana(v: list) -> float:
    v = _ok(v)
    return statistics.median(v) if v else 0


def massimo(v: list) -> float:
    v = _ok(v)
    return max(v) if v else 0


def aperte_a_fine_giorno(ev: list[dict], giorni: list[str]) -> dict[str, int]:
    """PRs open at the end of each UTC day, from the opened/reopened and merged/closed rows (PRs opened before the data started are unknown)."""
    mosse = sorted(((r["ts_utc"], r.get("pr"), r["azione"] in ("aperta", "riaperta")) for r in ev
                    if r.get("tipo") == "pr" and r.get("azione") in ("aperta", "riaperta", "unita", "chiusa")), key=lambda m: m[0])
    stato: dict = {}
    i, out = 0, {}
    for d in sorted(giorni):
        while i < len(mosse) and mosse[i][0][:10] <= d:
            stato[mosse[i][1]] = mosse[i][2]
            i += 1
        out[d] = sum(stato.values())
    return out


def giornaliero(ev: list[dict]) -> list[dict]:
    """One row per UTC day: runs, re-runs, mean queue, failed jobs by category (`rossi_veri` leaves out cancelled runs and the aggregate gate), PRs opened that day
    and open at the end of it, PRs merged, open-to-merge and green-to-merge as mean, median and max, commits per merged PR, conflicts."""
    giorni: dict[str, dict] = defaultdict(lambda: {"run": 0, "rilanci": 0, "coda": [], "job_rossi": Counter(), "pr_rosse": set(), "pr_aperte": 0, "pr_unite": 0,
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
            if r["categoria"] not in NON_VERI and r.get("pr"):
                g["pr_rosse"].add(r["pr"])
        elif t == "conflitto":
            g["conflitti"] += 1
        elif t == "pr" and r["azione"] == "aperta":
            g["pr_aperte"] += 1
        elif t == "pr" and r["azione"] == "unita":
            g["pr_unite"] += 1
            for k in ("aperta_s", "verde_a_unione_s", "commit_n"):
                g[k].append(r.get(k))
    fine = aperte_a_fine_giorno(ev, list(giorni))
    cat = sorted({c for g in giorni.values() for c in g["job_rossi"]})
    return [{"giorno": d, "run": g["run"], "rilanci": g["rilanci"], "coda_media_s": round(media(g["coda"])),
             "pr_aperte_nel_giorno": g["pr_aperte"], "pr_aperte_fine_giorno": fine[d], "pr_unite": g["pr_unite"],
             "aperta_media_s": round(media(g["aperta_s"])), "aperta_mediana_s": round(mediana(g["aperta_s"])), "aperta_max_s": round(massimo(g["aperta_s"])),
             "verde_a_unione_media_s": round(media(g["verde_a_unione_s"])), "verde_a_unione_mediana_s": round(mediana(g["verde_a_unione_s"])),
             "verde_a_unione_max_s": round(massimo(g["verde_a_unione_s"])), "commit_per_pr_media": round(media(g["commit_n"]), 1), "conflitti": g["conflitti"],
             "rossi_veri": sum(n for c, n in g["job_rossi"].items() if c not in NON_VERI), "pr_con_rosso_vero": len(g["pr_rosse"]),
             **{"rossi_" + c: g["job_rossi"][c] for c in cat}} for d, g in sorted(giorni.items())]


def righe_coo(ev: list[dict]) -> list[dict]:
    """The daily row in the column layout of the COO's `misure/giornaliero.csv`; chats_running and note are for the COO to fill."""
    return [{"date": g["giorno"], "merged": g["pr_unite"], "median_open_to_merge_min": round(g["aperta_mediana_s"] / 60, 1),
             "max_open_to_merge_min": round(g["aperta_max_s"] / 60, 1), "open_prs_end": g["pr_aperte_fine_giorno"], "chats_running": "", "note": ""}
            for g in giornaliero(ev)]


def pr_con_rosso_vero(ev: list[dict]) -> tuple[int, int]:
    """(PRs with at least one real red job, PRs with at least one recorded run)."""
    viste = {r["pr"] for r in ev if r["tipo"] == "run" and r.get("pr")}
    rosse = {r["pr"] for r in ev if r["tipo"] == "job_fallito" and r.get("pr") and r["categoria"] not in NON_VERI}
    return len(rosse), len(viste | rosse)


def righe_coda(api, ora: str, attesa=time.sleep) -> list[dict]:
    """Snapshot of the open (non-draft) PRs: how many, behind main, in conflict, green and waiting to be merged, the oldest open and the oldest green."""
    from ci_metrics import secs, verde_il
    prs = [p for p in api("/pulls?state=open&per_page=100") if not p.get("draft")]
    indietro = sporche = verdi = 0
    eta_aperta, eta_verde = [], []
    for p in prs:
        for _ in range(4):
            d = api(f"/pulls/{p['number']}")
            if d.get("mergeable") is not None:
                break
            attesa(2)
        stato = d.get("mergeable_state")
        indietro += stato == "behind"
        sporche += stato == "dirty"
        eta_aperta.append(secs(p["created_at"], ora))
        verde = verde_il(p["head"]["sha"], api) if stato != "dirty" else None
        if verde:
            verdi += 1
            eta_verde.append(secs(verde, ora))
    return [{"ts_utc": ora, "tipo": "coda", "k": ora, "aperte": len(prs), "indietro": indietro, "sporche": sporche, "verdi_in_attesa": verdi,
             "piu_vecchia_aperta_s": max(_ok(eta_aperta), default=None), "piu_vecchia_verde_s": max(_ok(eta_verde), default=None)}]


def tabella_coda(ev: list[dict], out=print) -> None:
    """Latest queue snapshot and the mean/max of each count over the window."""
    from ci_metrics import hm
    co = [r for r in ev if r["tipo"] == "coda"]
    if not co:
        return
    u = co[-1]
    out(f"\nQueue of open PRs (snapshots at each push to main, {len(co)} in the window)")
    out(f"  latest {u['ts_utc']}: open {u['aperte']}, behind {u['indietro']}, in conflict {u['sporche']}, green and waiting {u['verdi_in_attesa']}, "
        f"oldest open {hm(u['piu_vecchia_aperta_s'] or 0)}, oldest green {hm(u['piu_vecchia_verde_s'] or 0)}")
    out("  mean/max: " + ", ".join(f"{n} {media([r[c] for r in co]):.1f}/{massimo([r[c] for r in co])}"
                                   for n, c in (("open", "aperte"), ("behind", "indietro"), ("conflict", "sporche"), ("green waiting", "verdi_in_attesa"))))
