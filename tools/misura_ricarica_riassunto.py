"""Summary of the JSON written by tests_e2e/misura_ricarica.py: one table row per step (cold, warm, file, reset, warm-after-reset) and, with
--dettaglio STEP, the waterfall of that step (what was fetched before the first paint and before the page was usable, by kind, the slowest files,
the long tasks, what the service worker asked the network). Used by docs/studi/VELOCITA_RICARICA.md.
"""
from __future__ import annotations

import argparse, json
from collections import Counter, defaultdict


def kind(path: str) -> str:
    p = path.split("?", 1)[0]
    if p.endswith(".wasm"): return "wasm"
    if "/lang/" in p: return "lang"
    if "/vendor/" in p: return "vendor"
    if "/mzfinder/" in p or "tpmine" in p: return "tabui"
    if p.endswith((".js", ".mjs")): return "js"
    if p.endswith(".css"): return "css"
    if p.endswith((".json", ".csv")): return "data"
    if p.endswith((".png", ".svg", ".ico", ".woff2", ".woff")): return "img/font"
    if "api/" in p: return "api"
    return "other"


def row(name: str, st: dict) -> str:
    pages = st["pages"]
    if not pages:
        return f"| {name} | 0 | | | | | | {st['usable'] - st['t0']} | | | {len(st.get('sw', []))} | |"
    last = pages[-1]
    m, t0 = last["marks"], st["t0"]
    rel = lambda x: str(x - t0) if x else "-"
    lt = [d for pg in pages for _, d in pg.get("lt", [])]
    nreq = sum(len(pg.get("res", [])) for pg in pages)
    sw = st.get("sw", [])
    # the worker fetches with cache "no-cache": a revalidated file comes back as 200 from the HTTP cache with a few hundred bytes on the wire
    sw304 = sum(1 for r in sw if r[2] == 304 or r[3] < 1000)
    return (f"| {name} | {len(pages)} | {rel(last['nav'].get('ttfb'))} | {rel(m.get('fcp'))} | {rel(m.get('lcp'))} | {rel(m.get('drop'))} | {rel(m.get('ready'))} | {st['usable'] - t0} "
            f"| {rel(m.get('panel'))} | {nreq} | {len(sw)} ({sw304} revalidated) | {len(lt)} / {sum(lt)} |")


def detail(st: dict) -> None:
    t0 = st["t0"]
    for i, pg in enumerate(st["pages"]):
        m = pg["marks"]
        print(f"\n### page {i + 1}: {pg['url'].split('/')[-1]} (nav {pg['nav'].get('type')}, proto {pg['nav'].get('proto')}, t0 +{pg['t0'] - t0} ms)")
        print("marks (ms from the step start): " + ", ".join(f"{k} {v - t0}" for k, v in sorted(m.items(), key=lambda kv: kv[1])))
        if pg.get("phases"):
            print("engine phases: " + ", ".join(f"{p or '·'} {t - t0}" for t, p in pg["phases"]))
        res = pg.get("res", [])
        for lim_name, lim in (("before first paint", m.get("fcp")), ("before usable", m.get("ready") or m.get("drop")), ("all", None)):
            sel = [r for r in res if lim is None or r[2] <= lim]
            by = defaultdict(lambda: [0, 0])
            for r in sel:
                by[kind(r[0])][0] += 1; by[kind(r[0])][1] += r[6]
            tot = sum(v[1] for v in by.values())
            print(f"- {lim_name}: {len(sel)} files, {tot / 1e6:.2f} MB decoded: " + ", ".join(f"{k} {v[0]}/{v[1] / 1e3:.0f} KB" for k, v in sorted(by.items(), key=lambda kv: -kv[1][1])))
        rb = [r for r in res if r[7] == "blocking"]
        if rb:
            print(f"- render-blocking: {len(rb)} files, {sum(r[6] for r in rb) / 1e3:.0f} KB, last ends +{max(r[2] + r[3] for r in rb) - t0} ms")
        slow = sorted(res, key=lambda r: -r[3])[:8]
        print("- slowest: " + "; ".join(f"{r[0].split('/')[-1]} {r[3]} ms ({r[6] / 1e3:.0f} KB, wait {r[11]} ms, sw {r[10]})" for r in slow))
        lt = pg.get("lt", [])
        if lt:
            big = sorted(lt, key=lambda x: -x[1])[:6]
            print(f"- long tasks: {len(lt)}, {sum(d for _, d in lt)} ms; biggest " + ", ".join(f"{d} ms at +{t - t0}" for t, d in big))
    sw = st.get("sw", [])
    if sw:
        c = Counter("revalidated" if r[3] < 1000 else r[2] for r in sw)
        print(f"\n- service worker → network: {len(sw)} requests ({dict(c)}), {sum(r[3] for r in sw) / 1e6:.2f} MB on the wire, median {sorted(r[4] - r[0] for r in sw)[len(sw) // 2]} ms each")
    navs = st.get("navs", [])
    print("- navigations: " + ", ".join(f"+{t - t0} {u.split('/')[-1] or '/'}" for t, u in navs))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--dettaglio", default="")
    a = ap.parse_args()
    for f in a.files:
        d = json.load(open(f))
        print(f"\n## {f} ({d['profile']}, {d['base']})\n")
        print("| step | pages | TTFB | FCP | LCP | drop visible | engine ready | usable (isolated) | first panel | resources | SW→network | long tasks n / ms |")
        print("|---|---|---|---|---|---|---|---|---|---|---|---|")
        for k, st in d["steps"].items():
            print(row(k, st))
        for k, c in d.get("clicks", {}).items():
            if "ev" not in c:
                print(f"- click {k}: {c}"); continue
            ev = max((e[2] for e in c["ev"] if e[1] in ("click", "pointerup", "pointerdown", "keydown")), default=0)
            print(f"- click {k}: worst event {ev} ms; long tasks {[d for _, d in c['lt']]}; fetched {len(c['res'])} files {sum(r[2] for r in c['res']) / 1e3:.0f} KB")
        for k in filter(None, a.dettaglio.split(",")):
            if k in d["steps"]:
                print(f"\n## detail: {k}")
                detail(d["steps"][k])


if __name__ == "__main__":
    main()
