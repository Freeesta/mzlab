"""Diagnosis of the Rust engine on WebKit (nightly macOS runner): two modes, both print a table and exit 0 (the workflow reads the output).

    python3 tests_e2e/webkit_sonda.py probe      # webkit_sonda.html: WebAssembly.Memory shapes in page and workers, grow in steps of 256 MB
    python3 tests_e2e/webkit_sonda.py site       # the built site (tools/prepara_sito.py): does the engine start, workers, console, errors
Environment: WK_CORES=N overrides navigator.hardwareConcurrency (the index pool is min(cores - 1, 4): WK_CORES=2 forces one worker).
Not named e2e*: tools/verifica.py does not collect it.
"""
import json, os, subprocess, sys, threading, time
from pathlib import Path
from playwright.sync_api import sync_playwright

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
PORT = 8833


def pause(seconds):
    threading.Event().wait(seconds)


def serve(root):
    p = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(root)], stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
    import urllib.request
    for _ in range(100):
        try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
        except Exception: pause(0.1)
    return p


def open_page(pw, **kw):
    """(closer, page). WK_PERSISTENT=1: a persistent context (a profile directory on disk) instead of browser.new_page(), whose context is ephemeral (no website data store on disk)."""
    if os.environ.get("WK_PERSISTENT") == "1":
        import tempfile
        ctx = getattr(pw, os.environ.get("WK_BROWSER", "webkit")).launch_persistent_context(tempfile.mkdtemp(prefix="wkprofile"), **kw)
        return ctx.close, ctx.pages[0] if ctx.pages else ctx.new_page()
    b = launch(pw)
    return b.close, b.new_page(**kw)


def launch(pw):
    # the stderr of the WebKit process (crash reports, "unknown transient reason") goes to the job log
    return getattr(pw, os.environ.get("WK_BROWSER", "webkit")).launch(**({"executable_path": os.environ["WK_EXE"], "args": ["--no-sandbox"]} if os.environ.get("WK_EXE") else {}))


def probe():
    srv = serve(HERE)
    try:
        with sync_playwright() as pw:
            close, pg = open_page(pw)
            pg.goto(f"http://127.0.0.1:{PORT}/webkit_sonda.html")
            for _ in range(200):
                if pg.evaluate("!!window.PROBE"): break
                pause(0.1)
            rec = pg.evaluate("window.PROBE") or []
            close()
    finally:
        srv.terminate()
    first = next((r for r in rec if not r["ok"]), None)
    print("PROBE context:", "persistent" if os.environ.get("WK_PERSISTENT") == "1" else "ephemeral (new_page)")
    print("PROBE | step | ok | detail")
    for r in rec: print("PROBE |", r["name"], "|", "yes" if r["ok"] else "NO", "|", r.get("v", r.get("err", "")))
    print("PROBE first failing step:", first["name"] if first else "none")


def site():
    if not (ROOT / "site" / "index.html").exists():
        print("SITE no site/ (run tools/prepara_sito.py)"); return
    srv = serve(ROOT / "site")
    cores = os.environ.get("WK_CORES")
    log, errs, workers = [], [], []
    try:
        with sync_playwright() as pw:
            close, pg = open_page(pw, viewport={"width": 1500, "height": 1000})
            pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
            if cores: pg.add_init_script(f"Object.defineProperty(navigator,'hardwareConcurrency',{{get:()=>{int(cores)}}})")
            pg.on("console", lambda m: log.append(f"{m.type}: {m.text}"[:300]) if m.type in ("error", "warning", "debug") else None)
            pg.on("pageerror", lambda e: errs.append(str(e)[:300]))
            pg.on("worker", lambda w: workers.append(w.url.rsplit("/", 1)[-1]))
            pg.on("crash", lambda: errs.append("PAGE CRASH (WebContent process died)"))
            pg.goto(f"http://127.0.0.1:{PORT}/")
            res = None
            try:
                # MZLAB_RUST_READY resolves before the boot of the engine (browser.js: lr.boot) ends: the verdict is MZLAB_LR (booted) or MZLAB_LR_ERR (boot failed)
                res = pg.evaluate("""async () => {
                    await Promise.race([window.MZLAB_RUST_READY, new Promise(x => setTimeout(x, 40000))]);
                    for (let i = 0; i < 400 && !window.MZLAB_LR && !window.MZLAB_LR_ERR; i++) await new Promise(x => setTimeout(x, 100));
                    return { booted: !!window.MZLAB_LR, bootErr: window.MZLAB_LR_ERR || null, rustErr: window.MZLAB_RUST_ERR || null, miss: (window.MZLAB_LR_MISS || []).length, isolated: self.crossOriginIsolated };
                }""")
                pause(3)         # a worker that dies at the first allocation is replaced; two deaths in a minute turn the engine off
                res["bootErrAfter"] = pg.evaluate("window.MZLAB_LR_ERR || null")
            except Exception as e:
                res = {"exception": str(e)[:300]}
            close()
    finally:
        srv.terminate()
    print("SITE context:", "persistent" if os.environ.get("WK_PERSISTENT") == "1" else "ephemeral (new_page)")
    print("SITE cores override:", cores or "none")
    print("SITE result:", json.dumps(res))
    print("SITE workers:", len(workers), sorted(set(workers)))
    for l in log[:15]: print("SITE console:", l)
    for e in errs[:10]: print("SITE pageerror:", e)
    started = bool(res and res.get("booted") and not res.get("bootErr") and not res.get("bootErrAfter") and not errs and not any("engine start failed" in l for l in log))
    print("SITE engine starts:", "yes" if started else "NO")


if __name__ == "__main__":
    {"probe": probe, "site": site}[sys.argv[1] if len(sys.argv) > 1 else "probe"]()
