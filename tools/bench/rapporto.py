#!/usr/bin/env python3
"""Report of the benchmark: the measures of each tool, the paired comparisons of mzFinder with the others (difference, 95 % interval from the bootstrap of the
compounds, exact McNemar, Holm over the primary endpoints x competitors, pre-registered non-inferiority margins), as a deterministic JSON (sorted keys, rounded
numbers, no date, no time) and a Markdown summary. It writes numbers, not sentences of §3.5: those are decided by people on the numbers.

    python3 tools/bench/rapporto.py --d2 --seme 7 --profilo piccolo --out DIR      # mzFinder on a synthetic series: DIR/bench1.json + rapporto.json/.md
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
    __package__ = "tools.bench"

from . import FORMATO, metriche, statistica as S  # noqa: E402
from valida.rapporto import a_json as _a_json  # noqa: E402  (after `.`: it puts tools/ on the path)
from .adatta import a_json as tabella_a_json, corri_mzfinder_sintetico  # noqa: E402

ENDPOINT = {"P1": ("richiamo", "alto", "media"), "P2": ("mediana", "basso", "mediana"), "P3": ("integrita", "alto", "media"),
            "P4": ("richiamo_candidati", "alto", "media"), "P5": ("falsi", "basso", "media")}
VALORE = {"P1": "richiamo", "P2": "mediana", "P3": "integrita", "P4": "richiamo_candidati", "P5": "falsi"}


def misura(verita: list[dict], t: dict, gruppi: dict | None = None) -> dict:
    """The measures of one table: P1-P5 per compound and the secondary numbers."""
    return {"strumento": t["strumento"], "config": t["config"], "dataset": t["dataset"], **metriche.tutte(verita, t, gruppi)}


def unisci(misure: list[dict]) -> dict:
    """Concatenates the measures of the same tool on several series (the three seeds of D2): per-compound lists are joined and each compound is labelled with
    its series, so that the bootstrap resamples inside each series. Summary numbers are recomputed from the joined lists."""
    out = {k: misure[0][k] for k in ("strumento", "config", "dataset")}
    out["strati"] = [i for i, m in enumerate(misure) for _ in m["P1"]["per_composto"]]
    for p in ("P1", "P2", "P3", "P4", "P5"):
        per = [x for m in misure for x in m[p]["per_composto"]]
        out[p] = {"per_composto": per, "n": len(per)}
    out["strati_p"] = {p: [i for i, m in enumerate(misure) for _ in m[p]["per_composto"]] for p in ("P1", "P2", "P3", "P4", "P5")}
    return out


def _vec(m: dict, p: str) -> list:
    return [float("nan") if x is None else x for x in m[p]["per_composto"]]


def _strati(m: dict, p: str):
    return m.get("strati_p", {}).get(p)


def riassunto(m: dict) -> dict:
    """Value and interval of each primary endpoint of one tool."""
    out = {}
    for p, (_, _, quale) in ENDPOINT.items():
        out[p] = S.intervallo(_vec(m, p), quale, _strati(m, p))
    return out


def confronto(a: dict, altri: list[dict]) -> list[dict]:
    """Paired comparisons a - b for each competitor and primary endpoint (the same compounds), with Holm over all of them. A positive difference is better for
    the endpoints where higher is better (P1, P3, P4); for P2 and P5 a negative one is better."""
    righe = []
    for b in altri:
        for p, (_, direzione, quale) in ENDPOINT.items():
            va, vb = _vec(a, p), _vec(b, p)
            if len(va) != len(vb):
                raise ValueError(f"{p}: the tools are not measured on the same compounds")
            d = S.differenza(va, vb, quale, _strati(a, p))
            riga = {"endpoint": p, "contro": f"{b['strumento']}/{b['config']}", "direzione": direzione, "differenza": d}
            if quale == "media" and set(x for x in va + vb if x == x) <= {0, 1}:
                riga["mcnemar"] = S.mcnemar_esatto([int(x) for x in va], [int(x) for x in vb])
            righe.append(riga)
    h = S.holm([r["differenza"]["p"] if r["differenza"]["p"] is not None else 1.0 for r in righe])
    for r, adj, rif in zip(righe, h["p_corretto"], h["rifiutata"]):
        r["p_holm"], r["rifiutata_holm"] = adj, rif
        if r["endpoint"] in S.MARGINI:
            dirz, marg = S.MARGINI[r["endpoint"]]
            r["margine"], r["non_inferiore"] = marg, S.non_inferiore(r["differenza"], dirz, marg)
        r["superiore"] = S.superiore(r["differenza"], r["direzione"], rif)
    return righe


def rapporto(dataset: str, misura_a: dict, altri: list[dict] | None = None) -> dict:
    """The document `bench-rapporto/1` of one dataset: the summary of every tool and the comparisons of the first with the others."""
    altri = altri or []
    return {"formato": "bench-rapporto/1", "tabella": FORMATO, "dataset": dataset, "bootstrap": {"n": S.N_BOOT, "seme": S.SEME}, "margini": {k: list(v) for k, v in S.MARGINI.items()},
            "strumenti": {f"{m['strumento']}/{m['config']}": riassunto(m) for m in [misura_a] + altri}, "confronti": confronto(misura_a, altri) if altri else []}


def a_json(r: dict) -> str:
    return _a_json(r)


def _f(x, k=3):
    return "n/a" if x is None else f"{x:.{k}f}"


def a_markdown(r: dict) -> str:
    L = [f"# Benchmark {r['dataset']}", "", f"Bootstrap of the compounds: {r['bootstrap']['n']} resamples, seed {r['bootstrap']['seme']}; 95 % percentile intervals.", "",
         "| tool | endpoint | n | value | 95 % interval |", "|---|---|---|---|---|"]
    for nome, e in r["strumenti"].items():
        L += [f"| {nome} | {p} | {v['n']} | {_f(v['valore'])} | {_f(v['ic_basso'])} to {_f(v['ic_alto'])} |" for p, v in e.items()]
    if r["confronti"]:
        L += ["", "## Comparisons (first tool minus the other)", "", "| against | endpoint | difference | 95 % interval | p (Holm) | non-inferior | superior |", "|---|---|---|---|---|---|---|"]
        for c in r["confronti"]:
            d = c["differenza"]
            L.append(f"| {c['contro']} | {c['endpoint']} | {_f(d['valore'])} | {_f(d['ic_basso'])} to {_f(d['ic_alto'])} | {_f(c['p_holm'], 4)} | {c.get('non_inferiore', '')} | {c['superiore']} |")
    return "\n".join(L) + "\n"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--d2", action="store_true", help="run mzFinder on a synthetic series")
    ap.add_argument("--seme", type=int, default=7)
    ap.add_argument("--profilo", default="piccolo")
    ap.add_argument("--out", required=True)
    a = ap.parse_args(argv)
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    t, verita = corri_mzfinder_sintetico(a.seme, a.profilo, out / "lavoro")
    (out / "bench1.json").write_text(tabella_a_json(t), encoding="utf-8")
    r = rapporto(f"D2/{a.profilo}/{a.seme}", misura(verita, t))
    (out / "rapporto.json").write_text(a_json(r), encoding="utf-8")
    (out / "rapporto.md").write_text(a_markdown(r), encoding="utf-8")
    print(a_markdown(r))
    return 0


if __name__ == "__main__":
    sys.exit(main())
