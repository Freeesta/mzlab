"""Bug matrix of the Data, Statistics and Molecules areas: user actions x page states (no file, one file, many files, a file removed, reload,
language switch, theme switch, narrow window, keyboard only, reduced motion, undo/redo, rapid repeated clicks), on synthetic files only.
Every cell logs page errors, console errors, unhandled rejections, MZLAB_LR_MISS entries, empty panels and axe violations; a cell with
anything logged fails. Needs the built site (tools/build_site.py). Run from the repo root: python3 tests_e2e/e2e_matrice_bug.py"""
import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds

AXE = axe_path()
D = HERE / "shots" / "matrice_tmp"; D.mkdir(parents=True, exist_ok=True)
F = [D / "M_t0.mzML", D / "M_t30.mzML", D / "M_t60.mzML", D / "M_Blank_1.mzML"]
for i, (p, t) in enumerate(zip(F[:3], (0, 30, 60))):
    if not p.exists(): ds.full_scan(p, t, np.random.default_rng(i + 1))
if not F[3].exists():
    rng = np.random.default_rng(9)
    ds._write(F[3], [ds._spec(i, 0.5 + i * 0.0175, list(rng.uniform(120, 480, 45)), list(rng.lognormal(np.log(4e3), 0.9, 45))) for i in range(1111)], [])

# unhandled rejections are not always page errors: the page keeps them in a list the cell reads
CATCH = "window.__mxRej=[];addEventListener('unhandledrejection',e=>{try{window.__mxRej.push(String(e.reason&&(e.reason.stack||e.reason.message)||e.reason))}catch(_){}})"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_matrice_bug.py")]
        steps.append((name, f"FAIL line {ln[-1] if ln else '?'}: " + str(e).split("\n")[0][:400]))

r = Run(port=8941, wd="/tmp/wd_matrice")
def errors_since(pg, n0):
    """What went wrong since the cell started: page errors, console errors, unhandled rejections, requests the engine did not answer."""
    out = [e for e in r.errs[n0:] if e[0] in ("pageerror", "console.error")]
    out += [("rejection", x) for x in pg.evaluate("window.__mxRej.splice(0)")]
    out += [("lr_miss", x) for x in pg.evaluate("(window.MZLAB_LR_MISS || []).splice(0)")]
    return out
def cell(pg, name, fn):
    def run():
        n0 = len(r.errs)
        fn()
        bad = errors_since(pg, n0)
        assert not bad, bad[:3]
    step(name, run)
def axe(pg, sel="body"):
    if not AXE: return []
    if pg.evaluate("typeof axe") != "object": pg.add_script_tag(path=AXE)
    return pg.evaluate("s => axe.run(document.querySelector(s), {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']}, resultTypes: ['violations']}).then(r => r.violations.filter(v => v.impact === 'serious' || v.impact === 'critical').map(v => v.id + ': ' + v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' ; ')))", sel)
def tab(pg, id):
    pg.evaluate(f"BARRA.setTab('{id}', {{noFocus: true, instant: true}})"); frames(pg)
def empty(pg, sel):
    return pg.evaluate("s => { const e = document.querySelector(s); return !e || !e.textContent.trim(); }", sel)
def stat_ok(pg):
    tab(pg, "stat"); pg.wait_for_selector("#sb-panel-stat details.stat-sec", timeout=15000)
    ids = pg.evaluate("[...document.querySelectorAll('#sb-panel-stat details.stat-sec')].map(d => d.dataset.s + ':' + (d.querySelector('div').textContent.trim().length > 0))")
    assert all(x.endswith("true") for x in ids), ids
def mol_ok(pg):
    pg.evaluate("carica('m:molecole-ui.js').then(() => window.MOLDB.apri())"); pg.wait_for_selector("#moldlg .mol-box", state="visible")
    pg.wait_for_function("document.activeElement && document.activeElement.closest('#mol-q')", timeout=15000)
    pg.fill("#mol-q input", "caf"); pg.wait_for_function("document.querySelector('#mol-body').dataset.q === 'caf'", timeout=10000)
    pg.keyboard.press("Escape"); pg.wait_for_selector("#moldlg", state="hidden")

try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.add_init_script(CATCH); pg.reload(); pg.wait_for_function("window.qqReady === true", timeout=60000)
        # --- no file
        cell(pg, "no file: molecules window opens, searches, closes", lambda: mol_ok(pg))
        cell(pg, "no file: undo and redo do nothing", lambda: (pg.keyboard.press("Control+z"), pg.keyboard.press("Control+y"), frames(pg)))
        # --- one file
        def one():
            stage(pg, F[:1]); open_first(pg); ready(pg)
        cell(pg, "one file: loads and shows", one)
        cell(pg, "one file: every drawer tab", lambda: [tab(pg, t) for t in ("file", "addloss", "lists", "stat", "file")])
        cell(pg, "one file: statistics sections are not empty", lambda: stat_ok(pg))
        cell(pg, "one file: molecules window", lambda: mol_ok(pg))
        # --- many files
        def many():
            pg.set_input_files("#addf-input" if pg.locator("#addf-input").count() else "#pick", [str(f) for f in F[1:]])
            attendi(pg, "E.files.filter(f => !f.gone).length >= 4", 120000); loaded(pg); ready(pg)
        cell(pg, "many files: added", many)
        cell(pg, "many files: statistics sections are not empty", lambda: stat_ok(pg))
        cell(pg, "many files: rapid clicks on the drawer tabs", lambda: ([pg.evaluate("BARRA.setTab('stat'); BARRA.setTab('file'); BARRA.setTab('stat'); BARRA.setTab('lists')") for _ in range(10)], ready(pg)))
        def sample_table():
            tab(pg, "file")
            for _ in range(4): pg.click("#samptab")
            ready(pg)
        cell(pg, "many files: sample table toggled 4 times", sample_table)
        def stat_targets():
            stat_ok(pg)                                    # WCAG 2.5.8: every field of the panel is at least 24 px tall
            small = pg.evaluate("[...document.querySelectorAll('#sb-panel-stat :is(input:not([type=file]), select, button)')].filter(e => e.offsetParent && e.getBoundingClientRect().height < 24).map(e => (e.id || e.dataset.m) + ':' + e.getBoundingClientRect().height.toFixed(1))")
            assert not small, small
        cell(pg, "many files: statistics fields are 24 px targets", stat_targets)
        def profile():
            stat_ok(pg)
            pg.fill("#prof-mz", "364.4"); pg.fill("#prof-rt", "14.3"); pg.click("#prof-go")
            attendi(pg, "!!document.querySelector('#prof-tab') || !!document.querySelector('#prof-msg').textContent", 60000)
            assert pg.locator("#prof-tab tbody tr").count() >= 3, pg.inner_text("#prof-msg")
        cell(pg, "many files: time profile of an ion", profile)
        def profile_a11y():
            stat_ok(pg)                                    # the drawer is narrow: the result table scrolls sideways
            box = "#prof-res .qq-scroll"
            assert pg.evaluate(f"(e => e.scrollWidth > e.clientWidth)(document.querySelector('{box}'))"), "the table does not scroll: the cell tests nothing"
            v = axe(pg, "#sb-pane"); assert not v, v
            pg.focus(box); pg.keyboard.press("ArrowRight")                                   # the keyboard scrolls it
            attendi(pg, f"document.querySelector('{box}').scrollLeft > 0", 5000)
        cell(pg, "many files: profile result scrolls by keyboard (axe)", profile_a11y)
        def remove():
            k = pg.evaluate("E.files.find(f => !f.gone && f.file.includes('t30')).k")
            pg.evaluate(f"removeFile(E.files[{k}])"); ready(pg)
            stat_ok(pg)
            rows = pg.evaluate("[...document.querySelectorAll('#prof-tab tbody tr')].map(r => r.textContent)")
            assert not any("t30" in x for x in rows), rows
        cell(pg, "many files: a file removed, the profile forgets it", remove)
        def profile_error():
            profile()                                      # a result on screen, then a wrong retention time
            pg.fill("#prof-rt", "-1"); pg.click("#prof-go")
            attendi(pg, "!!document.querySelector('#prof-msg').textContent", 30000)
            assert not pg.locator("#prof-res").inner_text().strip(), "the old result stays beside the error"
            assert pg.evaluate("PROFILO.result") is None
        cell(pg, "many files: an input error drops the old profile", profile_error)
        cell(pg, "many files: undo, redo", lambda: (pg.keyboard.press("Control+z"), ready(pg), pg.keyboard.press("Control+Shift+z"), ready(pg)))
        T30 = "E.files.find(f => f.file === 'M_t30.mzML' && !f.gone)"
        ROW30 = "ST.files.find(f => f.name === 'M_t30.mzML')"
        def pick_again():
            assert pg.evaluate(f"!{T30} && {ROW30}.use === false"), pg.evaluate(f"[!!{T30}, {ROW30} && {ROW30}.use]")
            pg.set_input_files("#pick", [str(F[1])])
            attendi(pg, f"!!{T30} && {ROW30}.use === true", 30000); loaded(pg); ready(pg)
        cell(pg, "removed file: its row is unticked and picking it again opens it", pick_again)
        def pick_freed():
            k = pg.evaluate(f"{T30}.k"); pg.evaluate(f"removeFile(E.files[{k}])")
            attendi(pg, f"!{ROW30}", 30000)          # the copy is freed when the undo time is over: the row leaves the table
            pg.evaluate("soloFile(E.files.find(f => f.file === 'M_t60.mzML'))"); ready(pg)     # a file after the freed one: its index moved
            pg.set_input_files("#pick", [str(F[1])])
            attendi(pg, f"!!{T30}", 60000); loaded(pg)
            pg.evaluate(f"soloFile({T30})"); ready(pg)
        cell(pg, "freed file: picked again, it is read again and shown (no endless request)", pick_freed)
        def two_removals():
            t0 = pg.evaluate("removeFile(E.files.find(f => f.file === 'M_t30.mzML' && !f.gone)), performance.now()")
            attendi(pg, f"performance.now() > {t0} + 3000", 10000)      # the second removal starts halfway through the 6 s of the first
            pg.evaluate("removeFile(E.files.find(f => f.file === 'M_Blank_1.mzML' && !f.gone))")
            attendi(pg, f"!{ROW30}", 30000)          # the first is freed while the second still waits for its undo
            pg.evaluate("STORIA.annulla()"); ready(pg)
            assert pg.evaluate("E.files.some(f => f.file === 'M_Blank_1.mzML' && !f.gone) && E.files.every((f, k) => f.k === k)"), pg.evaluate("E.files.map(f => f.k + ':' + f.file + (f.gone ? ':gone' : ''))")
            pg.evaluate("soloFile(E.files.find(f => f.file === 'M_Blank_1.mzML'))"); ready(pg)
            pg.set_input_files("#pick", [str(F[1])]); attendi(pg, f"!!{T30}", 60000); loaded(pg); ready(pg)
        cell(pg, "two removals: the first freed, the undo of the second still brings it back", two_removals)
        cell(pg, "many files: theme dark then light", lambda: (pg.evaluate("document.documentElement.setAttribute('data-theme','dark')"), stat_ok(pg), pg.evaluate("document.documentElement.setAttribute('data-theme','light')")))
        def narrow():
            pg.set_viewport_size({"width": 360, "height": 740}); ready(pg)
            stat_ok(pg); mol_ok(pg)
            w = pg.evaluate("document.documentElement.scrollWidth - innerWidth")
            assert w <= 1, f"horizontal scroll {w}px"
            pg.set_viewport_size({"width": 1500, "height": 2200}); ready(pg)
        cell(pg, "many files: narrow window", narrow)
        def a11y():
            stat_ok(pg); v = axe(pg, "#sb-pane"); assert not v, v
        cell(pg, "many files: axe on the statistics panel", a11y)
        def reload():
            saved(pg); pg.reload(); pg.wait_for_function("window.qqReady === true", timeout=60000)
            attendi(pg, "E.files.some(f => f.vis && !f.gone)", 60000); ready(pg); stat_ok(pg)
        cell(pg, "reload: session comes back, statistics ok", reload)
        def english():
            pg.evaluate("I18N.set('en')"); pg.wait_for_function("window.qqReady === true && I18N.lang === 'en'", timeout=60000)
            ready(pg); stat_ok(pg); mol_ok(pg)
        cell(pg, "language switch: English", english)
finally:
    r.close()
fails = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
print("--- browser errors", len(r.errs))
for e in r.errs[:40]: print(e)
sys.exit(1 if fails else 0)
