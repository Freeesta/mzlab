"""Files open as soon as they are read (OPENNOW, open-now.js; upload-rows.js): the first progress row of the side list shows within 100 ms of the drop; while the files are
read the landing stays as it is (no card, no table, the drop area does not move) and no panel is drawn; at the end the first file opens by itself and nothing of the loading
is left («Loading finished», spinner, card); «Sample table» of the side panel shows the table and a click on a file goes back to the panels; a proposal of roles and times
that lands after the files opened still reaches the open files; a high-resolution file dropped on a low-resolution session is set aside with «Switch»; a folder of more than
50 data files asks «Open all / Choose» and «Choose» leaves them unticked; from the drop on there is no full-page veil and no «Full reset», also when the first
file is of another tab (MRM, MS2). Synthetic files."""
import sys, os, shutil, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
from lib import *
import synth
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_apri_subito.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
tmp = Path(tempfile.mkdtemp(prefix="subito_"))
LR = []
for k, n in enumerate(["S_t000min", "S_t010min", "S_t030min"]):
    p = tmp / f"{n}.mzML"; synth.write_full(p, 10 * k, seed=k + 1); LR.append(str(p))
HR = tmp / "HR_DDA-Exploris-t0.mzML"; ds.hr_dda(HR, np.random.default_rng(3))
MANY = tmp / "many"; MANY.mkdir()
for k in range(51): shutil.copy(LR[0], MANY / f"M_{k:02d}.mzML")
KINDS = [tmp / "K_mrm.mzML", tmp / "K_ms2.mzML"]; ds.mrm(KINDS[0], 1.0, np.random.default_rng(5), 1); ds.ms2(KINDS[1], np.random.default_rng(6))
# the proposals of roles and times answer 1 s late; the time of the first progress row is taken at the change of #pick
HOLD = """(() => { const f = window.fetch; window.fetch = function (u, o) { if (typeof u === 'string' && u.includes('api/campioni/proponi')) return new Promise(r => setTimeout(r, 1000)).then(() => f.call(this, u, o)); return f.call(this, u, o); };
  window.__up = null; window.__pick = null; window.__moved = []; window.__early = []; const d0 = document.querySelector('#drop').getBoundingClientRect().top;
  document.querySelector('#pick').addEventListener('change', () => { window.__pick = performance.now(); }, true);
  // while the files are read: the drop area never moves and no panel, card or loading table shows
  // from the drop on: no full-page veil and no «Full reset» (a stuck start) either, also when the first file opens
  const look = () => { const vis = (q) => [...document.querySelectorAll(q)].some(e => e.getClientRects().length);
    if (window.__pick != null) { if (vis('#loading')) window.__early.push('veil'); if (vis('#ldreset, #avvio-fase-reset')) window.__early.push('reset'); }
    if (!(UPBUSY > 0 || OPENNOW.busy() > 0)) return; const t = document.querySelector('#drop').getBoundingClientRect().top; if (Math.abs(t - d0) > 1) window.__moved.push(t - d0);
    if (document.querySelector('#dpanels .pnl')) window.__early.push('panel'); if (document.querySelector('.convprog')) window.__early.push('card'); if (!document.querySelector('#wtable').hidden) window.__early.push('table'); };
  new MutationObserver(look).observe(document.body, { childList: true, subtree: true, attributes: true });
  new MutationObserver(() => { if (window.__up == null && document.querySelector('#flst .fl.up')) window.__up = performance.now(); }).observe(document.getElementById('flst'), { childList: true, subtree: true }); })()"""
r = Run(port=8962, wd="/tmp/wd_apri_subito")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#drop"); attendi(pg, "window.qqReady === true", 60000)
        pg.evaluate(HOLD)
        lib.AUTO_TEMPI = False                               # the times must come from the late proposals, not from the test
        pg.set_input_files("#pick", LR)
        def first_row():
            attendi(pg, "window.__up != null", 20000)
            dt = pg.evaluate("window.__up - window.__pick")
            assert dt < 100, f"first progress row after {dt:.0f} ms"
        step("the first progress row shows within 100 ms of the drop", first_row)
        def landing_then_first():
            attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length === 3", 60000)
            assert pg.evaluate("window.__moved") == [], f"the drop area moved: {pg.evaluate('window.__moved')}"
            assert pg.evaluate("window.__early") == [], f"shown while reading: {pg.evaluate('[...new Set(window.__early)]')}"
            pg.wait_for_selector(".pnl.chrom canvas", timeout=60000)
            assert pg.evaluate("E.files.filter(f => f.vis && !f.gone).map(f => f.file).join()") == "S_t000min.mzML", "the first file opened by itself: " + pg.evaluate("E.files.map(f => f.file + (f.vis ? '*' : '') + (f.gone ? '-' : '')).join()")
            assert pg.evaluate("document.getElementById('v-workspace').dataset.work") == "panels"
            assert pg.locator(".convprog, .cp-spin, #fl-up .fl").count() == 0, "nothing of the loading is left"
            txt = pg.inner_text("body")
            assert "Loading finished" not in txt and "Caricamento finito" not in txt and "Loading the files" not in txt, "no end-of-loading text"
        step("the landing stays while the files are read, then the first file opens by itself", landing_then_first)
        def late_proposal():
            attendi(pg, "RUOLI.busy() === 0 && ST.files.every(f => f.prop)", 30000)
            attendi(pg, "E.files.filter(f => !f.gone).map(f => f.time).sort((a, b) => a - b).join() === '0,10,30'", 10000)
            assert pg.locator("#flist input[data-k=time].guess").count() >= 2, "the proposed times carry the guess class"
        step("a proposal that lands after the files opened reaches the open files", late_proposal)
        def table_button():
            assert pg.is_visible("#samptab") and pg.get_attribute("#samptab", "aria-pressed") == "false"
            pg.click("#samptab")
            pg.wait_for_selector("#wtable .samphead", state="visible", timeout=10000)
            assert pg.get_attribute("#samptab", "aria-pressed") == "true" and not pg.is_visible("#dpanels .pnl"), "the table in place of the panels"
            pg.click("#flst .fl .nm >> nth=1")
            attendi(pg, "document.getElementById('v-workspace').dataset.work === 'panels'", 10000)
            assert pg.get_attribute("#samptab", "aria-pressed") == "false"
        step("«Sample table» shows the table; a click on a file goes back to the panels", table_button)
        def hr_aside():
            pg.set_input_files("#pick", [str(HR)])
            attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && ST.files.length === 4 && !document.querySelector('#hrmix').hidden", 60000)
            assert pg.evaluate("OPENNOW.world()") is False, "the session stays low resolution"
            assert pg.evaluate("ST.files.find(f => f.hr).use") is False, "the HR file is unticked"
            assert pg.evaluate("!E.files.some(f => !f.gone && f.file.startsWith('HR_'))")
            assert pg.get_attribute("#hrmix-hr", "hidden") is None and pg.get_attribute("#hrmix-lr", "hidden") is not None, "one «Switch» button, to high resolution"
        step("an HR file dropped on an LR session is set aside with «Switch»", hr_aside)
        def many():
            files = sorted(str(f) for f in MANY.iterdir())
            pg.locator("#pick").set_input_files(files, no_wait_after=True)
            pg.wait_for_selector("#askdlg[open]", timeout=20000)
            assert "51" in pg.inner_text("#asktxt"), pg.inner_text("#asktxt")
            pg.click("#askno")
            attendi(pg, "ST.files.filter(f => f.name.startsWith('M_')).length === 51 && UPBUSY === 0 && OPENNOW.busy() === 0", 120000)
            assert pg.evaluate("ST.files.filter(f => f.name.startsWith('M_') && f.use).length") == 0, "«Choose»: all unticked"
            assert pg.evaluate("!E.files.some(f => !f.gone && f.file.startsWith('M_'))"), "none of them opened"
        step("more than 50 files ask once; «Choose» leaves them unticked", many)
        def other_kind():
            # a fresh visit whose first file is not of the starting tab (Full scan): the tab changes with the first file, under no veil
            q = r.b.new_page(viewport={"width": 1500, "height": 1400}, reduced_motion="reduce")
            q.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_selector("#drop"); attendi(q, "window.qqReady === true", 60000)
            q.evaluate(HOLD); q.set_input_files("#pick", [str(f) for f in KINDS])
            attendi(q, "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length >= 2", 60000)
            q.wait_for_selector("#dpanels .pnl canvas", timeout=60000)
            assert q.evaluate("E.tab") != "full", "the first file is of another tab"
            assert q.evaluate("window.__early") == [], f"shown while opening: {q.evaluate('[...new Set(window.__early)]')}"
            q.close()
        step("a first file of another tab opens with no full-page veil and no «Full reset»", other_kind)
finally:
    r.close(); shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
