"""Measure the browser reset on the built site: «Reset completo» (QQRESET.run("all")), «File» (run("files")) and the error-banner reset (qqHardReset),
step by step (tests_e2e/lib_reset.py probe), and what they leave behind. Measures, never asserts. One JSON line per run, a summary table at the end.
Variants: a = idle engine with filler files only, a2 = idle with two example files open, b = reset while the same tab builds an index,
c = a second tab of the same context open and idle, d = a second tab building an index, e = the user lists read once in the tab (liste.js keeps a connection).
The coiReload guard of lib.py is set: the first-visit isolation reload of browser.js isolateOnce never happens, the numbers do not include it.
Usage: python3 tools/prepara_sito.py; python3 tests_e2e/misura_reset.py [--rep 3] [--only all_a_50,files_b_50] [--out FILE]"""
# covers: mzlab/web/reset.js, mzlab/web/browser.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_reset import probe, seed, state, steps, HARNESS_LS
import argparse, json, statistics, time

EX = ROOT / "mzlab" / "web" / "esempi"
GUARD = "try{sessionStorage.setItem('mzlab.coiReload',new URLSearchParams(location.search).get('ripn')||'1');}catch(_){}"
HINTS = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
BUSY = "(typeof UPBUSY !== 'undefined' && UPBUSY > 0) || (typeof OPENNOW !== 'undefined' && OPENNOW.busy() > 0) || (typeof PROGRESSO !== 'undefined' && PROGRESSO.busy())"
READY = "typeof S !== 'undefined' && window.qqReady === true"
QUIET = "window.qqReady === true && (window.__qqF || 0) === 0 && Date.now() - (window.__qqT || 0) > 1000"


def scenarios():
    out = {}
    for mode in ("all", "files", "hard"):
        for n in (1, 50, 240):
            out[f"{mode}_a_{n}"] = (mode, "a", n)
        for v in ("a2", "b", "c", "d", "e"):
            out[f"{mode}_{v}_50"] = (mode, v, 50)
    return out


def open_page(ctx, base):
    pg = ctx.new_page(); pg.goto(base); pg.wait_for_function(READY, timeout=90000); return pg


def load_examples(pg):
    stage(pg, [EX / "FullScan_t0.mzML", EX / "FullScan_t30.mzML"]); pg.click("#opbtn"); ready(pg)


def start_busy(pg, big):
    """Give the big synthetic file to the start screen and return as soon as the page is copying or indexing it."""
    pg.set_input_files("#pick", [str(big)])
    pg.wait_for_function(BUSY, timeout=60000)


def say(*a):
    if VERBOSE: print("   ..", *a, file=sys.stderr, flush=True)
VERBOSE = False


def run_one(b, base, mode, var, n, big):
    ctx = b.new_context(lang_auto=True, locale="en-US", viewport={"width": 1280, "height": 900})
    ctx.add_init_script(GUARD); ctx.add_init_script(HINTS); mk = probe(ctx)
    errs = []
    try:
        docs = []          # top-level document requests of the tab: the reset's reload, and the isolating reload of isolateOnce if one follows
        pg = open_page(ctx, base); pg.on("pageerror", lambda e: errs.append(str(e)[:160]))
        pg.on("request", lambda q: docs.append((time.time(), q.url)) if q.resource_type == "document" and q.frame == pg.main_frame else None)
        say("page ready")
        pg2 = open_page(ctx, base) if var in ("c", "d") else None       # opened before the seed: its engine boots over an empty store, not over the filler
        if var in ("a2", "e"): load_examples(pg)
        if var == "e": pg.evaluate("LISTE.ready()")
        seed_s = seed(pg, n); say("seeded", seed_s)
        before = state(pg)
        if var == "b": start_busy(pg, big)
        if var == "d": start_busy(pg2, big)
        busy = {"tab": pg.evaluate(BUSY), "tab2": pg2.evaluate(BUSY) if pg2 else None}
        conn = pg.evaluate("__rs.conn.open - __rs.conn.close")
        n_docs = len(docs)
        pg.evaluate("() => { __rs.mark('t'); %s; }" % ("qqHardReset()" if mode == "hard" else f"QQRESET.run('{mode}')"))
        t_old = pg.evaluate("__rs.t0")
        pg.wait_for_function(f"window.__rs && __rs.t0 > {t_old} && __rs.ev.some(e => e[1] === 'ready')", timeout=180000)
        say("new page ready"); prev, cur, mid = mk.pair(pg)
        st = steps(prev, cur, mid); m = {"cur": cur, "mid": mid}
        wait_for_js(pg, QUIET, 60000)
        after = state(pg)
        tab2 = None
        if pg2:
            try: tab2 = pg2.evaluate("({ files: typeof E !== 'undefined' ? E.files.length : -1, failed: window.qqFailed || null, err: window.MZLAB_LR_ERR || null, busy: %s })" % BUSY)
            except Exception as e: tab2 = {"err": str(e)[:120]}
        rec = {"doc_loads": len(docs) - n_docs, "mode": mode, "var": var, "n": n, "seed_s": seed_s, "busy_at_reset": busy, "lists_conn_at_reset": conn, **st,
               "ls_at_start": [k for k in ((m["mid"][0] if m["mid"] else m["cur"])["ls0"] or []) if k not in HARNESS_LS], "sessione_at_start": "mzlab.sessione" in ((m["mid"][0] if m["mid"] else m["cur"])["ls0"] or []),
               "ls_at_start_last": [k for k in (m["cur"]["ls0"] or []) if k not in HARNESS_LS], "coi": pg.evaluate("crossOriginIsolated"),
               "before": {"usage": before["usage"], "stores": before["stores"]},
               "after": {**after, "ls": [k for k in after["ls"] if k not in HARNESS_LS]}, "tab2": tab2, "errs": errs[:5]}
        return rec
    finally:
        ctx.close()


def med(xs):
    xs = [x for x in xs if x is not None]
    return (round(statistics.median(xs)), max(xs)) if xs else (None, None)


def summary(rows):
    keys = ["stop_to_first_rm", "opfs_ms", "nomod_retries", "idb_ms", "caches_ms", "sw_ms", "wipe_ms", "reload_to_ready_ms", "lr_clear_ms", "total_ms"]
    print("\n| scenario | " + " | ".join(keys) + " | blocked | residues |")
    print("|" + "---|" * (len(keys) + 3))
    by = {}
    for r in rows: by.setdefault(f"{r['mode']}_{r['var']}_{r['n']}", []).append(r)
    for k, rs in by.items():
        cells = []
        for key in keys:
            mm = med([r.get(key) for r in rs]); cells.append("-" if mm[0] is None else f"{mm[0]}/{mm[1]}")
        blk = sorted({x for r in rs for x in r.get("idb_blocked") or []})
        a = rs[-1]["after"]
        res = f"mzlab/:{len(a['mzlab'])} opfs:{','.join(a['opfs'])} stores:{json.dumps(a['stores'], separators=(',', ':'))} ls:{','.join(a['ls'])} sess0:{sum(r['sessione_at_start'] for r in rs)}/{len(rs)}"
        print(f"| {k} (x{len(rs)}) | " + " | ".join(cells) + f" | {','.join(blk) or '-'} | {res} |")


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--rep", type=int, default=3); ap.add_argument("--only", default=""); ap.add_argument("--out", default=""); ap.add_argument("-v", action="store_true")
    a = ap.parse_args()
    global VERBOSE; VERBOSE = a.v
    sc = scenarios(); names = [x for x in a.only.split(",") if x] or list(sc)
    rows = []
    r = Run(port=free_port(), wd="/tmp/wd_misura_reset")
    try:
        os.makedirs(r.wd, exist_ok=True)
        big = Path(r.wd) / "synt_big.mzML"
        import importlib.util          # tests_e2e/mzml_grande.py by path: lib.py puts tools/ on sys.path, whose mzml_grande.py is a script
        spec = importlib.util.spec_from_file_location("mzml_grande_e2e", Path(__file__).with_name("mzml_grande.py")); mg = importlib.util.module_from_spec(spec); spec.loader.exec_module(mg)
        mg.write(big, n_ms1=500, n_ms2=4500)
        base = f"http://127.0.0.1:{r.port}/"
        with sync_playwright() as p:
            b = p.chromium.launch(); r.b = b
            for name in names:
                mode, var, n = sc[name]
                for i in range(a.rep):
                    t = time.time()
                    try: rec = run_one(b, base, mode, var, n, big)
                    except Exception as e: rec = {"mode": mode, "var": var, "n": n, "fail": str(e)[:300]}
                    rec.update(scenario=name, rep=i, browser="chromium", wall_s=round(time.time() - t, 1))
                    print(json.dumps(rec, separators=(",", ":")), flush=True); rows.append(rec)
                    if a.out:
                        with open(a.out, "a", encoding="utf-8") as f: f.write(json.dumps(rec) + "\n")
    finally:
        r.close()
    summary([x for x in rows if "fail" not in x])
    print("WebKit, Firefox: not available (only Chromium is installed in this environment)")


if __name__ == "__main__":
    main()
