"""HRMS Target Screening view: the page loads its engine (liste-target-core.js) on demand, so a synthetic one-compound run finds its peak and no request goes unserved.
The spectra are injected in the session cache (MZLAB_SPECTRA_CACHE) around the first compound of the Keller list: no data file needed.
Needs the built site (python3 tools/build_site.py). Run from the repo root: python3 tests_e2e/e2e_liste_target.py"""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8830 + 100, wd="/tmp/wd_liste_target")
SPECTRA = """(mz0) => {
  const sp = [];
  for (let i = 0; i < 60; i++) {
    const rt = 1 + i * 0.05, g = Math.exp(-Math.pow((rt - 2.5) / 0.15, 2) / 2);
    sp.push({ level: 1, rt, mz: [mz0, mz0 + 1.003355], intensity: [1e6 * g + 50, 2e5 * g + 40] });
  }
  window.MZLAB_SPECTRA_CACHE = sp;
}"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def engine_loaded():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())")
            pg.evaluate("LISTE.openTargetScreening({})"); pg.wait_for_selector("#ts-start", timeout=20000)
            n = pg.evaluate("Object.keys(window.LISTE_TARGET_CORE || {}).length"); assert n > 5, "engine not loaded: LISTE_TARGET_CORE has %d members" % n
        step("the target view loads its engine", engine_loaded)
        def finds_peak():
            mz = pg.evaluate("(async () => { LISTE.ensure(); await LISTE.ready(); return LISTE.ensure().pos[0].mz; })()")
            pg.evaluate(SPECTRA, mz)
            pg.select_option("#ts-list-sel", "keller"); pg.click("#ts-start")
            pg.wait_for_function("document.querySelector('#ts-status').textContent.includes('finished')", timeout=60000)
            assert pg.locator("#ts-tbody tr[data-m-idx]").count() >= 1, pg.inner_text("#ts-status")
        step("a synthetic one-compound run gives at least one match", finds_peak)
        def no_miss():
            miss = pg.evaluate("window.MZLAB_LR_MISS || []"); assert not miss, miss
        step("no request left unserved by the engine", no_miss)
        def label():
            # the test page runs in Italian: compare with the catalogue entry of the current language, and make sure it is not the key itself
            t = pg.inner_text("#lst-view-seg button[data-v=target]").strip(); want = pg.evaluate("I18N.t('lst.view.target')")
            assert t == want and t != "lst.view.target" and len(t) > 5, repr((t, want))
        step("the view label comes from the catalogue", label)
finally:
    r.close()
for s in steps: print("  ", s)
r.report()
# Chromium notes the Ketcher iframe (allow-scripts + allow-same-origin, by design) on every page load: not an error of this view
real_errs = [e for e in r.errs if "can escape its sandboxing" not in str(e)]
sys.exit(0 if all(s[1] == "ok" for s in steps) and not real_errs else 1)
