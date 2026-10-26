"""240 SCIEX pairs (.wiff + .wiff.scan; 120 EMS+, 120 EMS-) dropped together on the built site with the Rust engine (?motore=rust&lr=1), synthetic (tools/wiff_grande.py; no laboratory data):
they all load without an error and without reloading the page, the list keeps the order of the names, at most 4 index builders run together, «Cancel» in the middle stops the rest, and the same pairs again are «from cache».
Small pairs by default (about 2 min). MZLAB_CARICO_DIR=DIR measures a folder of real-size pairs (tools/wiff_grande.py --samples 1 --scans 1200 --points 3333): prints seconds, peak MB of the browser processes and of the page heap, GB of OPFS.
Needs the site and the .wasm (python tools/build_site.py + wasm, see e2e_wiff.py). Run from the repo root: python3 tests_e2e/e2e_wiff_240.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port, hold

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py + wasm")); sys.exit(0)
REAL = os.environ.get("MZLAB_CARICO_DIR")
D = Path(REAL) if REAL else Path(tempfile.mkdtemp(prefix="wiff240_"))
if not REAL:
    for pol in ("EMSp", "EMSm"):
        for i in range(1, int(os.environ.get('MZLAB_CARICO_N', 120)) + 1):
            subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(D), "--name", f"{pol}_{i:03d}", "--samples", "1", "--scans", "60", "--points", "300", "--seed", str(i)], check=True, capture_output=True)
NAMES = sorted(p.name for p in D.glob("*.wiff"))
FILES = [str(D / n) for n in NAMES] + [str(D / (n + ".scan")) for n in NAMES]
SAMPLES = [n[:-5] + ".mzML" for n in NAMES]                        # the sample of a one-sample pair is named after the file
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
for _ in range(50):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)      # the server is starting

def rss_mb():
    """sum of the resident memory of every chromium process (the pages, the workers and the browser)"""
    out = subprocess.run(["ps", "-eo", "rss,args"], capture_output=True, text=True).stdout
    return sum(int(l.split(None, 1)[0]) for l in out.splitlines()[1:] if "chrom" in l) / 1024

def follow(pg, cdp, bcdp, until, limit=1500):
    """polls every 500 ms: peak MB of the processes, peak JS heap of the page, peak idx builders (workers), longest time the visible text stayed the same"""
    peak = dict(mb=0.0, heap=0.0, builders=0, still=0.0); last, since, t0 = None, time.time(), time.time(); s = {}
    while time.time() - t0 < limit:
        s = json.loads(pg.evaluate("JSON.stringify({see: [...document.querySelectorAll('#flist [data-cvrow]')].map(e => e.innerText).slice(0, 8).join('|'), busy: UPBUSY, files: ST.files.map(f => f.name)})"))
        peak["mb"] = max(peak["mb"], rss_mb())
        m = {x["name"]: x["value"] for x in cdp.send("Performance.getMetrics")["metrics"]}; peak["heap"] = max(peak["heap"], m.get("JSHeapUsedSize", 0) / 1e6)
        peak["builders"] = max(peak["builders"], sum(1 for t in bcdp.send("Target.getTargets")["targetInfos"] if "idx-worker" in t.get("url", "")))
        if s["see"] != last: peak["still"] = max(peak["still"], time.time() - since); last, since = s["see"], time.time()
        if until(s): break
        hold(pg, 500)
    peak["s"] = time.time() - t0
    return s, peak

try:
    with sync_playwright() as p:
        # a normal window (a profile on disk): the ephemeral context of Playwright behaves like a private window, whose OPFS holds only about 200 MB (error.quota with room left, misure-formati.md)
        prof = tempfile.mkdtemp(prefix="wiff240_prof_")
        c = p.chromium.launch_persistent_context(prof, executable_path=os.environ.get("MZLAB_CHROMIUM") or None, locale="en-US", viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000)
        pg = c.pages[0] if c.pages else c.new_page(); pg.on("pageerror", lambda e: errs.append(("pageerror", str(e)))); pg.on("crash", lambda: errs.append(("crash", "page crashed")))
        loads = []; pg.on("load", lambda: loads.append(time.time()))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop"); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        cdp = c.new_cdp_session(pg); cdp.send("Performance.enable"); bcdp = cdp
        base = len(loads)
        opfs = lambda: pg.evaluate("navigator.storage.estimate().then(e => e.usage)") / 1e9

        pg.set_input_files("#pick", FILES)
        s, peak = follow(pg, cdp, bcdp, lambda s: s["busy"] == 0 and len(s["files"]) >= len(SAMPLES))
        print(f"   measure: {len(NAMES)} pairs, {peak['s']:.0f} s total, {peak['s'] / len(NAMES):.2f} s/pair, peak {peak['mb']:.0f} MB (all chromium processes), page heap {peak['heap']:.0f} MB, builders {peak['builders']}, OPFS {opfs():.2f} GB")
        if os.environ.get("MZLAB_CARICO_SOLO"): c.close(); shutil.rmtree(prof, ignore_errors=True); sys.exit(0)      # only the measure of the first drop
        def all_in():
            assert len(s["files"]) == len(SAMPLES), (len(s["files"]), s["files"][:5])
            assert pg.evaluate("CONVROWS.rows.filter(r => r.state === 'done').length") == len(SAMPLES), pg.evaluate("CONVROWS.rows.filter(r => r.state !== 'done').map(r => [r.name, r.state]).slice(0, 5)")
            assert pg.locator("#flist .cvsum .diag").count() == 0, "an error is listed"
            assert len(loads) == base, "the page reloaded"
        step("all the pairs load without an error and without reloading the page", all_in)
        order1 = [n for n in s["files"] if n in set(SAMPLES)]
        def order():                                    # the rule of the list (ordine.js: time, concentration or natural order of the names) is applied to the whole list, not to the arrival
            assert sorted(order1) == sorted(SAMPLES) and len(order1) == len(SAMPLES), (len(order1), order1[:3])
            assert pg.evaluate("(() => { const a = ST.files.map(f => f.name); sortFiles(); return JSON.stringify(a) === JSON.stringify(ST.files.map(f => f.name)); })()"), "the list is not in the order of the rule"
        step("the list is in the order of the rule, not of the arrival", order)
        def again():
            pg.set_input_files("#pick", FILES)
            t0 = time.time(); pg.wait_for_function("UPBUSY === 0 && CONVROWS.rows.length > 0", timeout=300000)
            assert len([n for n in pg.evaluate("ST.files.map(f => f.name)") if n in set(SAMPLES)]) == len(SAMPLES), "duplicates or losses"
            print(f"   measure: second drop {time.time() - t0:.1f} s")
            assert pg.locator("#flist .cvsum").count() == 0 and pg.evaluate("CONVROWS.rows.filter(r => r.state === 'done').length") == len(SAMPLES), "not all from the cache"
        step("the same pairs again enter without duplicates", again)
        def reverse():                                  # the same files dropped in the opposite order: the same list
            pg.reload(); pg.wait_for_selector("#drop"); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            pg.set_input_files("#pick", FILES[::-1])
            pg.wait_for_function(f"UPBUSY === 0 && ST.files.length >= {len(SAMPLES)}", timeout=900000)
            assert [n for n in pg.evaluate("ST.files.map(f => f.name)") if n in set(SAMPLES)] == order1
        step("dropped in the opposite order the list is the same", reverse)
        def cancel():
            pg.reload(); pg.wait_for_selector("#drop"); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            half = FILES[: len(NAMES) // 2] + FILES[len(NAMES): len(NAMES) + len(NAMES) // 2]
            pg.set_input_files("#pick", half)
            pg.wait_for_function("CONVROWS.rows.some(r => r.state === 'done')", timeout=300000)
            pg.evaluate("document.querySelector('#flist [data-cvact=cancel]').click()"); t0 = time.time()      # the button is drawn again at every row: a JS click
            pg.wait_for_function("UPBUSY === 0", timeout=60000)
            end = pg.evaluate("CONVROWS.rows.map(r => r.state)")
            assert "cancel" in end and set(end) <= {"cancel", "done"}, {k: end.count(k) for k in set(end)}
            assert time.time() - t0 < 30, f"Cancel took {time.time() - t0:.1f} s"
        step("«Cancel» in the middle stops the pairs not done", cancel)
        assert not errs, errs
        c.close(); shutil.rmtree(prof, ignore_errors=True)
finally:
    srv.terminate()
    if not REAL: shutil.rmtree(D, ignore_errors=True)
