"""The SIM wording and controls of the Data view on the built site with the Rust engine (synthetic SIM .wiff pair of tools/wiff_sim.py, no laboratory data): the
experiment cell says «SIM · N ions» (not «no data»), the side list group, the panel title, the toolbar button and the channel picker speak of SIM ions, every ion has one
colour that is the same in every sample, the chromatogram menu holds the «Extract ion ›» submenu (colour swatch, check mark, «All», click / Ctrl / Shift rules) in place
of XIC, and the «Area» toggle fills the traces (on by default). Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_sim_ui.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="sim_ui_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "sim.json").read_text())
N = len(TRUE["ioni"])
r = Run(port=8875, wd="/tmp/wd_sim_ui")
def need(c, m): c or (_ for _ in ()).throw(AssertionError(m))
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim.wiff", D / "sim.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length >= 2 && E.files.every(f => f.sim > 0)", timeout=120000)
        step("every file counts its SIM ions", lambda: need(pg.evaluate("E.files.every(f => f.sim === %d)" % N), pg.evaluate("E.files.map(f => f.sim)")))
        def cell():
            t = pg.evaluate("ST.files.map(f => expCell(f)).join('|')")
            need("no data" not in t and "nessun dato" not in t and ("SIM" in t) and (" %d " % N in t), t)
        step("the experiment cell says SIM and the number of ions", cell)
        open_first(pg)
        step("the side list group is «SIM», not «MRM»", lambda: need(pg.evaluate("[...document.querySelectorAll('#flst .fgh')].map(e => e.textContent).join('|')").find("SIM") >= 0 and "MRM" not in pg.evaluate("[...document.querySelectorAll('#flst .fgh')].map(e => e.textContent).join('|')"), "group"))
        step("the toolbar button speaks of SIM ions", lambda: need("SIM" in pg.inner_text("#np-mrm"), pg.inner_text("#np-mrm")))
        pg.click("#np-mrm"); attendi(pg, "E.panels.some(p => p.type === 'mrm' && p._a)", timeout=60000)
        step("the panel title says SIM ions", lambda: need(pg.evaluate("E.panels.find(p => p.type === 'mrm').title") == "@sim", pg.evaluate("E.panels.find(p => p.type === 'mrm').title")))
        def colours():
            s = pg.evaluate("E.panels.find(p => p.type === 'mrm')._a.sr.map(s => [s.ion, s.color, s.k])")
            by = {}
            for ion, col, k in s: by.setdefault(ion, set()).add(col)
            need(all(len(v) == 1 for v in by.values()), by)
            need(len({next(iter(v)) for v in by.values()}) == len(by), by)
        step("one colour per ion, the same in every sample", colours)
        step("«Area» is on by default and the traces are filled", lambda: need(pg.evaluate("E.panels.find(p => p.type === 'mrm').area !== false") and pg.evaluate("document.querySelector('[data-a=area]').getAttribute('aria-pressed')") == "true", "area"))
        def toggle():
            pg.click("[data-a=area]"); need(pg.evaluate("E.panels.find(p => p.type === 'mrm').area") is False, "off")
            pg.click("[data-a=area]"); need(pg.evaluate("E.panels.find(p => p.type === 'mrm').area") is True, "on")
        step("the Area button turns the fill off and on", toggle)
        def picker():
            pg.click(".can-b, .canb"); pg.wait_for_selector(".can-row", state="visible")
            rows = pg.evaluate("[...document.querySelectorAll('.can-pop')].map(e => e.querySelectorAll('.can-row').length)")
            need(rows and all(r == N for r in rows), rows)
        step("the channel picker lists the SIM ions", picker)
        pg.keyboard.press("Escape")
        def submenu():
            box = pg.locator("canvas").first.bounding_box()
            pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right")
            pg.wait_for_selector("#ctx:not([hidden])")
            need(pg.evaluate("[...document.querySelectorAll('#ctx div[role=menuitem]')].some(e => /Extract ion|Estrai ione/.test(e.textContent))"), "entry")
            need(not pg.evaluate("[...document.querySelectorAll('#ctx div')].some(e => e.dataset.azione === 'chrom.xic')"), "XIC still there")
            pg.hover("#ctx .hsub"); need(pg.evaluate("document.querySelectorAll('#ctx .ctxsub:not([hidden]) [role=menuitemcheckbox]').length") == N + 1, "rows")
            need(pg.evaluate("document.querySelectorAll('#ctx .ctxsub .sw').length") == N, "swatches")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(2).click()
            need(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr.length") == 1, pg.evaluate("E.panels.find(p => p.type === 'mrm').tr"))
        step("the chromatogram menu extracts one ion with a click", submenu)
        def ctrl_shift():
            for mod in (None,):
                box = pg.locator("canvas").first.bounding_box()
                pg.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2, button="right"); pg.hover("#ctx .hsub")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(3).click(modifiers=["Control"])
            need(len(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr")) == 2, "ctrl")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(1).click(modifiers=["Shift"])
            need(len(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr")) >= 2, "shift")
            pg.locator("#ctx .ctxsub [role=menuitemcheckbox]").nth(0).click()
            need(pg.evaluate("E.panels.find(p => p.type === 'mrm').tr") == "", "all")
        step("Ctrl adds an ion, Shift takes a range, All clears the selection", ctrl_shift)
        def helpline():
            pg.click("#np-closeall"); attendi(pg, "!E.panels.some(p => p.el)", timeout=20000)
            attendi(pg, "(() => { const h = document.querySelector('#simhelp'); return !!h && !h.hidden && !!h.querySelector('button'); })()", timeout=20000)
            pg.click("#simhelp button"); attendi(pg, "(() => { const t = document.querySelector('#simtab'); return t && !t.hidden; })()", timeout=30000)
        step("with no graph the help line offers the SIM table and the button shows it", helpline)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
