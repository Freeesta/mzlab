"""Tables and the landing-page comparison of misura_payload.py (split from it for the file-size limit): `report` prints markdown from the run-*.json of a
directory, `run_compare` measures FCP, requests and bytes of any public page with the same throttling profiles. Measures, never asserts; not run in CI."""
from __future__ import annotations

import json, statistics, time
from pathlib import Path


def _mp():
    import misura_payload as m   # late: misura_payload imports this module
    return m


# --- comparison of other landing pages -----------------------------------------------------------------------------------------------------------------
GEN = r"""(() => { const v = window.__g = { fcp: 0, lcp: 0 }; try { new PerformanceObserver(l => l.getEntries().forEach(e => { if (e.name === 'first-contentful-paint') v.fcp = e.startTime; })).observe({ type: 'paint', buffered: true }); } catch (_) {}
  try { new PerformanceObserver(l => l.getEntries().forEach(e => { v.lcp = e.startTime; })).observe({ type: 'largest-contentful-paint', buffered: true }); } catch (_) {} })();"""


def run_compare(b, url, profile, cpu):
    c = b.new_context(viewport={"width": 1400, "height": 900}); c.add_init_script(GEN)
    pg = c.new_page()
    if _mp().PROFILES[profile] or cpu > 1:
        s = c.new_cdp_session(pg)
        if _mp().PROFILES[profile]:
            rate, rtt = _mp().PROFILES[profile]
            s.send("Network.enable"); s.send("Network.emulateNetworkConditions", {"offline": False, "latency": rtt * 1000, "downloadThroughput": rate / 8, "uploadThroughput": 1_500_000 / 8})
        if cpu > 1:
            s.send("Emulation.setCPUThrottlingRate", {"rate": cpu})
    t = time.time()
    pg.goto(url, wait_until="load", timeout=180000)
    _mp().poll(pg, "__g.fcp > 0", 60)
    _mp().poll(pg, "performance.now() > __g.fcp + 3000", 30)
    r = pg.evaluate("""() => ({ fcp: __g.fcp, lcp: __g.lcp, nav: performance.getEntriesByType('navigation')[0].toJSON(),
        res: performance.getEntriesByType('resource').map(e => [e.name, e.initiatorType, e.startTime, e.responseEnd, e.transferSize, e.encodedBodySize, e.decodedBodySize]) })""")
    c.close()
    nav = r["nav"]
    docs = [(0, nav["responseEnd"], nav["encodedBodySize"], nav["decodedBodySize"])]
    rs = [(e[2], e[3], e[5], e[6]) for e in r["res"]]
    allr = docs + rs
    by = lambda t: [x for x in allr if x[1] <= t]
    f = r["fcp"]
    return {"url": url, "profile": profile, "cpu": cpu, "fcp": round(f), "lcp": round(r["lcp"]), "req_fcp": len(by(f)), "enc_kb_fcp": round(sum(x[2] for x in by(f)) / 1024, 1),
            "dec_kb_fcp": round(sum(x[3] for x in by(f)) / 1024, 1), "req_total": len(allr), "enc_kb_total": round(sum(x[2] for x in allr) / 1024, 1),
            "dec_kb_total": round(sum(x[3] for x in allr) / 1024, 1), "load": round(nav["loadEventEnd"]), "wall_s": round(time.time() - t, 1)}


# --- report: tables in markdown from the run-*.json of a directory ------------------------------------------------------------------------------------
def med(xs):
    xs = [x for x in xs if x is not None]
    return statistics.median(xs) if xs else None


def report(d: str, ref: str, tables: bool):
    runs = {}
    for f in sorted(Path(d).glob("run-*.json")):
        _, prof, cpu, mode = f.stem.split("-")
        runs[(prof, int(cpu[3:]), mode)] = json.loads(f.read_text())
    o = ["| profile | CPU | visit | runs | FCP | drop area visible | last carica.js script | engine ready | req / KB gzip / KB decoded until ready (whole chain) |", "|---|---|---|---|---|---|---|---|---|"]
    for (prof, cpu, mode), rs in sorted(runs.items(), key=lambda kv: (["none", "fast4g", "slow4g"].index(kv[0][0]), kv[0][1], kv[0][2])):
        m = lambda k: med([r["ms"][k] for r in rs])
        ch = lambda k: med([r["chain"][k] for r in rs])
        o.append(f"| {prof} | x{cpu} | {mode} | {len(rs)} | {m('fcp')} | {m('drop')} | {m('carica_last')} | {m('ready')} | {ch('req'):.0f} / {ch('enc_kb'):.0f} / {ch('dec_kb'):.0f} |")
    o += ["", "Cumulative at each milestone (files finished by then; median of runs; KB gzip -9 / KB decoded):", "", "| profile | CPU | visit | at FCP req · KB | at drop area | at last carica.js script | at engine ready |", "|---|---|---|---|---|---|---|"]
    for (prof, cpu, mode), rs in sorted(runs.items(), key=lambda kv: (["none", "fast4g", "slow4g"].index(kv[0][0]), kv[0][1], kv[0][2])):
        def c(k):
            v = [r["cum"].get(k) for r in rs if r["cum"].get(k)]
            return f"{med([x['req'] for x in v]):.0f} · {med([x['enc_kb'] for x in v]):.0f} / {med([x['dec_kb'] for x in v]):.0f}" if v else "-"
        o.append(f"| {prof} | x{cpu} | {mode} | {c('fcp')} | {c('drop')} | {c('carica_last')} | {c('ready')} |")
    tr = [(k, r["trace"]) for k, rs in runs.items() for r in rs[:1] if r.get("trace")]
    if tr:
        o += ["", "Main thread of the page (Chrome trace of the first cold run of each combination), ms from the start of the visit until the mark:", "",
              "| profile | CPU | mark | window | script | parse | render | other main-thread work | busy | idle, a request in flight | idle, nothing in flight |", "|---|---|---|---|---|---|---|---|---|---|---|"]
        for (prof, cpu, mode), t in sorted(tr, key=lambda kv: (["none", "fast4g", "slow4g"].index(kv[0][0]), kv[0][1])):
            for lab in ("fcp", "ready"):
                if lab in t and "split_" + lab in t:
                    x, sp = t[lab], t["split_" + lab]
                    o.append(f"| {prof} | x{cpu} | {lab} | {x['window_ms']} | {x['script']} | {x['parse']} | {x['render']} | {x['other_busy']} | {sp['busy_ms']} | {sp['idle_waiting_net_ms']} | {sp['idle_nothing_ms']} |")
    k = tuple(ref.split(","))
    key = (k[0], int(k[1]), k[2])
    if key in runs:
        r = sorted(runs[key], key=lambda r: r["ms"]["ready"] or 0)[len(runs[key]) // 2]
        stop = max(x for x in (r["ms"]["ready"], r["ms"]["carica_last"]) if x)
        ch = [e for e in r["table"] if e["end"] <= stop and not e["url"].endswith("[first navigation]")]
        o += ["", f"Reference run {ref} (median run by engine-ready time): {r['ms']}; chain until {stop} ms:", "", "10 biggest files of the chain (gzip -9):", "", "| # | file | kind | KB gzip | KB decoded | start | end |", "|---|---|---|---|---|---|---|"]
        for i, e in enumerate(sorted(ch, key=lambda e: -e["enc"])[:10], 1):
            o.append(f"| {i} | {e['url'].split('mzlab/')[-1]} | {e['kind']} | {e['enc'] / 1024:.1f} | {e['dec'] / 1024:.1f} | {e['start']:.0f} | {e['end']:.0f} |")
        by = {}
        for e in ch:
            a = by.setdefault(e["kind"], [0, 0, 0]); a[0] += 1; a[1] += e["enc"]; a[2] += e["dec"]
        o += ["", "Chain by kind:", "", "| kind | files | KB gzip | KB decoded |", "|---|---|---|---|"] + [f"| {k_} | {a[0]} | {a[1] / 1024:.0f} | {a[2] / 1024:.0f} |" for k_, a in sorted(by.items(), key=lambda kv: -kv[1][1])]
        if tables:
            o += ["", f"Request table of the reference run ({ref}), request order; flags: B render-blocking, P preload, C inserted by carica.js, E engine; sizes in bytes (gzip -9 / decoded); ms from the first navigation:", "",
                  "| # | url | kind | gzip B | decoded B | start | end | flags |", "|---|---|---|---|---|---|---|---|"]
            for i, e in enumerate(r["table"], 1):
                fl = ("B" if e["block"] == "blocking" else "") + ("P" if e["preload_flag"] else "") + ("C" if e["kind"].startswith("script inserted") else "") + ("E" if e["kind"].startswith("engine") else "")
                o.append(f"| {i} | {e['url'].split('mzlab/')[-1]} | {e['kind']} | {e['enc']} | {e['dec']} | {e['start']:.0f} | {e['end']:.0f} | {fl} |")
    print("\n".join(o))
