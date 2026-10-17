"""Automatic reactivity of the parent in Tabui (TP_Mine/lazy/reatt-auto.js, TP_Mine/files/chem-worker.js; docs/agenti/reattivita.md).
The 38 synthetic files of tests/data/gruppi go in through the page. Without «mzlab.admin» a Dig fetches no .wasm «chem» and shows no bar. With it: after a Dig the panel goes
waiting -> computing -> done and the table has the values (bentazone, a public compound); a new Dig while it computes stops the worker, the panel waits and the calculation runs again after the TPs;
a cancelled Dig starts nothing; the same SMILES and options give the session's copy; the options (solvent, conformers) and «Recompute»; the time of a Dig does not grow with the reactivity armed.
Builds the site and static/wasm-feat; SKIP without cargo and wasm-bindgen. MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one. Run from the repo root: python3 tests_e2e/e2e_reattivita_auto.py"""
import os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the .wasm of mzFinder"); sys.exit(0)
tmp = Path(tempfile.mkdtemp(prefix="reatt_auto_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:
    for f in (ROOT / "TP_Mine" / "files").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
    for f in (ROOT / "TP_Mine" / "lazy").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / "lazy" / f.name)
FILES = sorted(str(p) for p in (ROOT / "tests" / "data" / "gruppi").glob("*.mzML"))
SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"          # bentazone
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
_t0 = time.time()
while time.time() - _t0 < 30:
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: pass
errs = []; reqs = []
CTL = "#tp-reatt-ctl"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1400}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("request", lambda r: reqs.append(r.url)); logs = []; pg.on("console", lambda m: logs.append(m.text[:200]))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#nav", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        pg.evaluate("localStorage.removeItem('mzlab.admin')")        # Tabui is open; from here the administrator mode is off (uipAdminOn reads it at every call)
        pg.set_input_files("#tp-in", FILES)
        pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 9", timeout=600000)
        pg.fill("#tp-smi", SMILES)
        pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('241.0641')", timeout=60000)
        chem = lambda: [u for u in reqs if "mzlab_chem_wasm" in u ]

        idle = lambda: pg.evaluate("new Promise(r => requestIdleCallback(() => setTimeout(r, 3500), {timeout: 5000}))")      # the idle moment that would start the calculation has passed
        def dig():
            """one Dig; returns its time in ms"""
            pg.wait_for_function("!document.getElementById('tp-go').disabled && !document.getElementById('tp-live')")
            t0 = time.time(); pg.click("#tp-go")
            try: pg.wait_for_function("document.getElementById('tp-msg').textContent === 'Done.'", timeout=300000)
            except Exception: raise AssertionError("Dig did not finish: " + pg.inner_text("#tp-msg") + str(pg.evaluate("[document.getElementById('tp-go').disabled, document.getElementById('tp-right').innerText.slice(0,200)]")))
            return (time.time() - t0) * 1000
        base = []
        def nonadmin():
            base.append(dig()); idle()
            assert pg.evaluate("typeof window.TPREATTAUTO === 'undefined' && typeof window.TPREATT === 'undefined'"), "scripts loaded without administrator mode"
            assert not pg.evaluate("document.querySelector('#tp-reatt .tpr') || !document.querySelector('#tp-reatt-ctl').hidden || document.querySelector('#tp-reatt-ctl').firstChild"), "panel or bar without administrator mode"
            assert not chem(), chem()
        step("without mzlab.admin a Dig shows no panel and downloads no .wasm chem", nonadmin)
        base.append(dig())

        pg.evaluate("localStorage.setItem('mzlab.admin', '1')")
        pg.fill("#tp-smi", "CCO"); pg.wait_for_function("window.TPREATTAUTO && window.TPREATTAUTO.info().smiles === 'CCO'", timeout=60000); pg.fill("#tp-smi", SMILES)           # the structure is drawn again: scripts load, the panel waits
        pg.wait_for_function("window.TPREATTAUTO && window.TPREATTAUTO.info().smiles === %r && document.querySelector('#tp-reatt .tpr')?.dataset.state === 'waiting'" % SMILES, timeout=60000)
        def idle_before():
            assert not chem(), "chem fetched before the TPs"
            assert pg.evaluate("window.TPREATTAUTO.info().phase") == "waiting" and not pg.evaluate("document.querySelector('#tp-reatt-ctl').hidden")
            assert "Recompute" in pg.inner_text(CTL) and "Solvent" in pg.inner_text(CTL) and "Conformers" in pg.inner_text(CTL)
        step("with mzlab.admin the panel waits for the TPs and nothing is fetched yet", idle_before)

        def auto():
            pg.evaluate("() => { window.__st = []; new MutationObserver(() => { const s = document.querySelector('#tp-reatt .tpr')?.dataset.state; const l = window.__st; if (s && l[l.length - 1] !== s) l.push(s); }).observe(document.getElementById('tp-reatt'), {subtree: true, childList: true, attributes: true}); return 1; }")
            pg.evaluate("() => { window.__calls = []; for (const k of ['attach','tpsStart','tpsDone','tpsEnd','cancel']) { const f = window.TPREATTAUTO[k]; window.TPREATTAUTO[k] = (...a) => { window.__calls.push(k); return f(...a); }; } }")
            t = dig()
            try: pg.wait_for_function("window.TPREATTAUTO.info().phase === 'done'", timeout=120000)
            except Exception: raise AssertionError(str((pg.evaluate("window.TPREATTAUTO.info()"), pg.inner_text("#tp-reatt")[:200], pg.inner_text(CTL)[:100], logs[-3:], pg.evaluate('window.__calls'))))
            order = pg.evaluate("window.__st")
            assert order[:2] == ["waiting", "computing"], order
            assert pg.evaluate("document.querySelectorAll('#tp-reatt .tpr-tab tbody tr').length") >= 24, "the table has the values (bentazone: 24 atoms with H)"
            st = pg.inner_text(CTL + " .tpra-st"); assert st.startswith("done in "), st
            assert pg.evaluate("window.TPREATTAUTO.info().worker"), "no worker"
            assert any(u.endswith("mzlab_chem_wasm_bg.wasm") for u in reqs), "the .wasm chem was not fetched"
            return t
        step("after the TPs: waiting -> computing -> done, the table has the values, the .wasm chem is fetched only now", auto)

        def kept():
            n = pg.evaluate("window.TPREATTAUTO.info().cached"); assert n == 1, n
            pg.click(CTL + " .tpra-re"); pg.wait_for_function("window.TPREATTAUTO.info().phase === 'done'", timeout=60000)
            pg.wait_for_function("document.querySelector('#tp-reatt-ctl .tpra-st').textContent.includes('kept from this session')", timeout=10000)
            assert pg.evaluate("window.TPREATTAUTO.info().cached") == 1
        step("the same SMILES, engine and options are not calculated again (session copy)", kept)

        def options():
            pg.select_option(CTL + " .tpra-sol", "none"); pg.fill(CTL + " .tpra-conf", "2"); pg.click(CTL + " .tpra-re")
            pg.wait_for_function("window.TPREATTAUTO.info().cached === 2 && window.TPREATTAUTO.info().phase === 'done'", timeout=120000)
            pg.click("#tp-reatt #tpr-t5"); j = pg.inner_text("#tp-reatt #tpr-p pre"); assert '"solvent": "none"' in j, j[:300]
            pg.select_option(CTL + " .tpra-sol", "water"); pg.fill(CTL + " .tpra-conf", "4")
        step("«Recompute» with the options Solvent and Conformers", options)

        def restart():
            pg.select_option(CTL + " .tpra-sol", "water"); pg.fill(CTL + " .tpra-conf", "16"); pg.click(CTL + " .tpra-re")
            pg.wait_for_function("window.TPREATTAUTO.info().phase === 'computing'", timeout=30000)
            pg.click("#tp-go")
            pg.wait_for_function("document.querySelector('#tp-reatt .tpr')?.dataset.state === 'waiting' && !window.TPREATTAUTO.info().worker", timeout=5000)
            assert pg.evaluate("window.TPREATTAUTO.info().tpsBusy"), "the TPs are running"
            assert pg.inner_text(CTL + " .tpra-st") == "waiting for TPs" and pg.locator(CTL + " .tpra-re").is_disabled(), "Recompute is enabled during the TPs"
            pg.wait_for_function("document.getElementById('tp-msg').textContent === 'Done.'", timeout=300000)
            pg.wait_for_function("window.TPREATTAUTO.info().phase === 'done'", timeout=120000)
            assert pg.evaluate("window.TPREATTAUTO.info().cached") == 3, "the new run was not calculated after the TPs"
        step("a new Dig during the calculation stops it (worker terminated), waits, and the calculation runs after the TPs", restart)

        def cancelled():
            pg.click("#tp-go"); pg.wait_for_function("window.TPLIVE && TPLIVE.phases() === 'xic'", timeout=120000)
            pg.click("#tp-live-cancel"); pg.wait_for_function("document.getElementById('tp-msg').textContent === 'Cancelled.'", timeout=30000)
            idle()
            i = pg.evaluate("window.TPREATTAUTO.info()"); assert i["phase"] == "waiting" and not i["tpsBusy"] and not i["worker"], i
            assert not pg.locator(CTL + " .tpra-re").is_disabled(), "Recompute is blocked after a cancelled Dig"
        step("a cancelled Dig starts nothing and frees «Recompute»", cancelled)

        def failed():
            pg.evaluate("window.TPREATTAUTO.cancel()")
            pg.fill("#tp-smi", "CC(C"); pg.wait_for_function("window.TPREATTAUTO.info().smiles === 'CC(C'", timeout=30000)
            pg.click(CTL + " .tpra-re")
            pg.wait_for_function("window.TPREATTAUTO.info().phase === 'failed'", timeout=60000)
            assert "failed" in pg.inner_text("#tp-reatt .tpr") and not pg.locator(CTL + " .tpra-retry").is_hidden(), "no message or no Retry"
            pg.fill("#tp-smi", SMILES); pg.wait_for_function("window.TPREATTAUTO.info().smiles === %r" % SMILES, timeout=30000)
        step("a SMILES the engine refuses gives «failed» with the reason and a «Retry» button", failed)

        def timing():
            adm = [dig(), dig()]; pg.wait_for_function("window.TPREATTAUTO.info().phase === 'done'", timeout=120000)
            a, bm = min(adm), min(base)
            print(f"Dig ms: without reactivity {base}, with it armed {adm}")
            assert a <= bm * 1.05 or a - bm <= 200, (a, bm)
        step("the time of a Dig does not grow (+5 % or 200 ms) with the reactivity armed", timing)
        step("window.MZLAB_LR_MISS is empty", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS || []).length") else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
