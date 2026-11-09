"""Times the site load of a maker's file (a synthetic Thermo .raw of crates/mzlab-lettori/examples/thermo_sintetico, no laboratory data): from the drop to the
sample row being usable, the last event of phase «read» (the copy, or the builder hashing a member itself), and the SHA-256 of the .mzidx (it must not change).
A Playwright context is ephemeral like a private window: Chromium keeps about 1 GiB of OPFS there, so a bigger file is read in place; --profilo runs in a
persistent profile (a normal window). Each run reloads the page after the last sample was removed. Needs the site with the readers modules
(tools/prepara_sito.py, then tools/build_lettori_wasm.sh site/static).

    python3 tests_e2e/misura_raw_carico.py FILE.raw [--giri 5] [--out run.json] [--cpu 1] [--profilo]"""
import argparse, json, os, shutil, statistics, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

EV = """addEventListener('mzlab-index', e => { const d = e.detail; (window.__ev = window.__ev || []).push([performance.now(), d.phase || '', d.frac]); });"""
# a persistent context looks for its own headless build: where a Chromium of the machine is given (cloud sessions), that one, with headless as launch() has it
CHROME = {"executable_path": "/opt/pw-browsers/chromium", "args": ["--headless=new"]} if os.path.isfile("/opt/pw-browsers/chromium") else {}
SHAS = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx'); const o = {};
  for await (const [n, h] of d.entries()) if (n.endsWith('.mzidx')) o[n] = [...new Uint8Array(await crypto.subtle.digest('SHA-256', await (await h.getFile()).arrayBuffer()))].map(x => x.toString(16).padStart(2, '0')).join('');
  return o; }"""


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("file"); ap.add_argument("--giri", type=int, default=5); ap.add_argument("--out"); ap.add_argument("--cpu", type=float, default=1); ap.add_argument("--profilo", action="store_true")
    a = ap.parse_args()
    r = Run(port=8981, wd="/tmp/wd_misura_raw"); runs = []; dirs = []
    try:
        with sync_playwright() as p:
            if a.profilo:
                d = tempfile.mkdtemp(prefix="mzlab-profilo-"); dirs.append(d)
                pg = p.chromium.launch_persistent_context(d, viewport={"width": 1500, "height": 2200}, reduced_motion="reduce", **CHROME).new_page()
                pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
                pg.goto(f"http://127.0.0.1:{r.port}/")
            else:
                pg = r.page(p)
            pg.add_init_script(EV)
            if a.cpu > 1: pg.context.new_cdp_session(pg).send("Emulation.setCPUThrottlingRate", {"rate": a.cpu})
            for i in range(a.giri):
                pg.reload(); wait_for_js(pg, "window.qqReady === true", 60000)
                pg.evaluate("window.__ev = []")
                t0 = pg.evaluate("performance.now()")
                pg.set_input_files("#pick", [a.file])
                wait_for_js(pg, "ST.files.length === 1 && ST.files[0].kind && UPBUSY === 0 && OPENNOW.busy() === 0", 600000)
                t1 = pg.evaluate("performance.now()"); ev = pg.evaluate("window.__ev")
                reads = [t for t, ph, _ in ev if ph == "read"]
                read = (reads[-1] - t0) if reads else None
                runs.append({"total_ms": round(t1 - t0), "read_ms": round(read) if read else None, "sha": pg.evaluate(SHAS)})
                print(json.dumps(runs[-1]["total_ms"]), runs[-1]["read_ms"], flush=True)
                t2 = pg.evaluate("performance.now()"); pg.evaluate("removeAllFiles()"); wait_for_js(pg, "ST.files.length === 0 && UPBUSY === 0", 600000)
                runs[-1]["remove_ms"] = round(pg.evaluate("performance.now()") - t2)
    finally:
        r.close()
        for d in dirs: shutil.rmtree(d, ignore_errors=True)
    med = lambda k: statistics.median([x[k] for x in runs if x[k] is not None]) if any(x[k] is not None for x in runs) else None
    out = {"file_bytes": os.path.getsize(a.file), "n": len(runs), "total_ms": {"median": med("total_ms"), "min": min(x["total_ms"] for x in runs), "max": max(x["total_ms"] for x in runs)},
           "read_ms": med("read_ms"), "shas": sorted({json.dumps(x["sha"], sort_keys=True) for x in runs})}
    print(json.dumps(out, indent=1))
    if a.out: open(a.out, "w").write(json.dumps({"summary": out, "runs": runs}, indent=1))


if __name__ == "__main__":
    main()
