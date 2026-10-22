#!/usr/bin/env python3
"""Speed tables of the Thermo `.raw` reader against the RFR oracle (criteria A5/A6, docs/agenti/thermo-velocita.md).

  thermo_velocita.py unisci BANCO.jsonl RFR.jsonl > UNITO.tsv   join `banco/1` (ours) and `banco/1` with reader=rfr, by file id
  thermo_velocita.py tabella UNITO.tsv --sha SHA40 [--maquina TEXT]   Markdown tables per family

UNITO.tsv columns: id, family, bytes, n_scans, rfr_ok, our_ok, rfr_open_ms, rfr_read_all_ms, our_open_ms, our_read_all_ms.
Speed tables use only files that BOTH readers open; the others are counted apart with the reason. Only aggregates are printed: no file name."""
import argparse
import collections
import csv
import json
import statistics
import sys

COLS = ["id", "family", "bytes", "n_scans", "rfr_ok", "our_ok", "rfr_open_ms", "rfr_read_all_ms", "our_open_ms", "our_read_all_ms"]


def unisci(ours_path, rfr_path):
    def leggi(p):
        return {j["id"]: j for j in map(json.loads, open(p)) if j.get("schema") == "banco/1"}

    a, b = leggi(ours_path), leggi(rfr_path)
    w = csv.writer(sys.stdout, delimiter="\t", lineterminator="\n")
    w.writerow(COLS)
    for k in sorted(set(a) | set(b)):
        o, r = a.get(k), b.get(k)
        ref = o or r
        w.writerow([
            k, ref.get("family", ""), ref["bytes"], ref["n_scans"],
            "ok" if r else "fail", "ok" if o else "fail",
            r["median"]["open_ms"] if r else "", r["median"]["read_all_ms"] if r else "",
            o["median"]["open_ms"] if o else "", o["median"]["read_all_ms"] if o else "",
        ])


def num(r, k):
    return float(r[k])


def d(x, y, f=",.0f"):
    return format(x / y, f) if y > 0 else "–"


def riga(nome, rs):
    mb = sum(int(r["bytes"]) for r in rs) / 1e6
    n = sum(int(r["n_scans"]) for r in rs)
    ro, ra = sum(num(r, "rfr_open_ms") for r in rs) / 1e3, sum(num(r, "rfr_read_all_ms") for r in rs) / 1e3
    oo, oa = sum(num(r, "our_open_ms") for r in rs) / 1e3, sum(num(r, "our_read_all_ms") for r in rs) / 1e3
    rap = [num(r, "rfr_read_all_ms") / num(r, "our_read_all_ms") for r in rs if num(r, "our_read_all_ms") > 0] or [float("nan")]
    return (f"| {nome} | {len(rs)} | {mb:,.0f} | {n:,} | {ro:.1f} / {oo:.1f} | {ra:.1f} / {oa:.1f} | "
            f"{d(n, ra)} / {d(n, oa)} | {d(mb, ra)} / {d(mb, oa)} | {statistics.median(rap):.1f}× |")


def tabella(path, sha, maquina):
    rows = list(csv.DictReader(open(path), delimiter="\t"))
    comuni = [r for r in rows if r["rfr_ok"] == "ok" and r["our_ok"] == "ok"]
    fam = collections.defaultdict(list)
    for r in comuni:
        fam[r["family"] or "(none)"].append(r)
    print(f"main `{sha}` · {maquina or 'machine not declared'} · files opened by both readers: {len(comuni)} of {len(rows)}\n")
    print("| Family | Files | MB | Scans | Open s (RFR / ours) | Read all s (RFR / ours) | Scans/s (RFR / ours) | MB/s (RFR / ours) | Median file ratio |")
    print("|---|---|---|---|---|---|---|---|---|")
    for k, v in sorted(fam.items(), key=lambda x: -len(x[1])):
        print(riga(k, v))
    print(riga("**All**", comuni))
    apart = collections.Counter(
        "RFR opens, ours does not" if r["rfr_ok"] == "ok" else "ours opens, RFR does not" if r["our_ok"] == "ok" else "neither opens"
        for r in rows if not (r["rfr_ok"] == "ok" and r["our_ok"] == "ok"))
    print("\nListed apart (not in the speed table): " + (", ".join(f"{v} {k}" for k, v in apart.items()) or "none"))


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    s = p.add_subparsers(dest="cmd", required=True)
    u = s.add_parser("unisci")
    u.add_argument("ours")
    u.add_argument("rfr")
    t = s.add_parser("tabella")
    t.add_argument("unito")
    t.add_argument("--sha", required=True)
    t.add_argument("--maquina")
    a = p.parse_args()
    unisci(a.ours, a.rfr) if a.cmd == "unisci" else tabella(a.unito, a.sha, a.maquina)
