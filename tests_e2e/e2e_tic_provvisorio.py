# e2e_tic_provvisorio: while the first file of a loading is still being indexed, the work area shows its provisional TIC (tic-provvisorio.js: the MS1 rt/tic of the scan table,
# event mzlab-index with early_tic) with its data table; the box goes when the real panel opens, and «Cancel» of the loading takes it away with no panel left behind.
# Synthetic file (tools/mzml_msconvert_sintetico.py), no laboratory data. Needs the site and the .wasm: python3 tools/build_site.py, then python3 tests_e2e/e2e_tic_provvisorio.py
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import mzml_msconvert_sintetico as gen
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
# records the first moment #ticprov is visible: whether a panel was already drawn, its title, the data table button; `cancel` clicks «Cancel» of the loading at that moment
WATCH = """(cancel) => { window.__tp = null; const look = () => { const b = document.getElementById('ticprov'); if (window.__tp || !b || b.hidden) return;
  const cv = b.querySelector('canvas');
  window.__tp = { panel: E.panels.some(p => p._a), title: b.querySelector('h3').textContent, label: cv.getAttribute('aria-label'), desc: !!cv.getAttribute('aria-describedby'), btn: !!b.querySelector('.tdbtn'), w: cv.width };
  if (cancel) document.querySelector('[data-up-all]').click(); };
  new MutationObserver(look).observe(document.body, { subtree: true, childList: true, attributes: true, attributeFilter: ['hidden'] }); }"""
d = tempfile.mkdtemp(prefix="ticprov_"); f = os.path.join(d, "tic_lento.mzML")
gen.scrivi(f, scans=2400, peaks1=1500, peaks2=150)
def shown_then_replaced():
    r = Run(port=8853, wd="/tmp/wd_tic_provvisorio")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(WATCH, False)
            pg.set_input_files("#pick", [f])
            attendi(pg, "window.__tp !== null", 120000)
            tp = pg.evaluate("window.__tp")
            assert not tp["panel"], "the provisional TIC must show before the first panel"
            assert "tic_lento.mzML" in tp["title"] and tp["label"] == tp["title"], tp
            assert tp["desc"] and tp["btn"] and tp["w"] > 0, f"the chart needs its description and «Data» table: {tp}"
            open_first(pg)
            ready(pg)
            attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0")
            assert pg.evaluate("document.getElementById('ticprov').hidden"), "the box stays once the real panel is open"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("the provisional TIC shows before the first panel, with its data table, and goes when the panel opens", shown_then_replaced)
def cancel_hides():
    r = Run(port=8854, wd="/tmp/wd_tic_provvisorio2")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(WATCH, True)
            pg.set_input_files("#pick", [f])
            attendi(pg, "window.__tp !== null && UPBUSY === 0 && OPENNOW.busy() === 0", 120000)
            assert pg.evaluate("document.getElementById('ticprov').hidden"), "the box stays after «Cancel»"
            assert pg.evaluate("E.files.filter(f => !f.gone).length") == 0, "a cancelled file was opened"
            assert pg.is_visible("#start")
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("«Cancel» during the index takes the provisional TIC away and opens nothing", cancel_hides)
shutil.rmtree(d, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
