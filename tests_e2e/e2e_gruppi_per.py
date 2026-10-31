"""«Group by» of the side list: Matrix gives MQ 20 / TW 9 / one file out; «Then by Condition» gives `MQ · H2O2`; «Contains» a whole word; Ctrl+Z / Ctrl+Shift+Z restore
labels and key; a reload keeps both; the menu and the dialogs work with the keyboard only; opening them shifts nothing (CLS 0, same boxes); axe is clean; no engine miss."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
AXE = axe_path()
D = pathlib.Path(__file__).resolve().parent / "shots" / "gruppi_per_tmp"; D.mkdir(parents=True, exist_ok=True)
NAMES = ["MQ_blk", "MQ_Wash_1", "MQ_Wash_2", "TW_blk", "MQ_std_BTZ_1ppm", "QC_Std_5ppm_mid"]
NAMES += [f"MQ_{c}_{t}" for c in ("H2O2", "N2", "O2", "PMS") for t in ("T00", "T01", "T02", "T05")]
NAMES += [f"TW_{c}_{t}" for c in ("AIR", "H2O2") for t in ("T00", "T01", "T02", "T05")]
# MQ 4 + 16 = 20 (blk, Wash x2, std), TW 1 + 8 = 9, QC 1: the matrix MQ/TW/ungrouped split of the study
assert len(NAMES) == 30
ds.full_scan(D / "src.mzML", 5, np.random.default_rng(2))
PATHS = []
for n in NAMES:
    q = D / f"{n}.mzML"; shutil.copyfile(D / "src.mzML", q); PATHS.append(str(q))
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gruppi_per.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
LABELS = "Object.fromEntries(E.files.filter(f => !f.gone).map(f => [fileBase(f.file).replace(/\\.mzML$/, ''), f.ugrp || null]))"
COUNT = "(l) => E.files.filter(f => f.ugrp === l).length"
ITEM = "document.querySelectorAll('.gby-menu [role=menuitemradio]')"
def count(pg, label): return pg.evaluate(COUNT, label)
def item(pg, text): return pg.locator(".gby-menu [role=menuitemradio]", has_text=text).first
def open_menu(pg):
    pg.click("#sb-by"); attendi(pg, f"!!document.querySelector('.gby-menu') && {ITEM}.length > 3", 10000)
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
r = Run(port=8962, wd="/tmp/wd_gruppi_per")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", PATHS)
        pg.click("text=Carica dati"); ready(pg, timeout=180000); drawer(pg)
        pg.wait_for_selector("#flst .fl .nm", timeout=30000)
        pg.evaluate("I18N.set('en')"); pg.wait_for_function("window.qqReady === true && I18N.lang === 'en'", timeout=60000)
        def watch():
            pg.evaluate("""() => { window.__cls = 0; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; }).observe({type: 'layout-shift', buffered: false}); }""")
            if AXE: pg.add_script_tag(path=AXE)
        watch()
        BOX = "['#tools', '#dpanels', '#sb-header'].map(s => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height].map(Math.round).join(); }).join('|')"
        def matrix():
            assert pg.evaluate("E.files.filter(f => !f.gone).length") == 30
            open_menu(pg)
            assert item(pg, "None").get_attribute("aria-checked") == "true"
            item(pg, "Matrix").click(); attendi(pg, "!!E.ugBy && E.ugBy.by === 'matrix'", 10000); fermo(pg, "#flst")
            assert (count(pg, "MQ"), count(pg, "TW")) == (20, 9), (count(pg, "MQ"), count(pg, "TW"))
            assert pg.evaluate("E.files.filter(f => !f.gone && !f.ugrp).length") == 1
            assert pg.evaluate("E.ugOrder") == ["MQ", "TW"], pg.evaluate("E.ugOrder")
            heads = pg.evaluate("[...document.querySelectorAll('#flst .fgh[data-ug]')].map(g => g.dataset.ug)")
            assert heads == ["MQ", "TW"], heads
        step("Group by → Matrix: MQ 20, TW 9, one file out of every group", matrix)
        def then():
            open_menu(pg); item(pg, "Then by").click(); attendi(pg, f"{ITEM}.length > 2 && !!document.querySelector('.gby-menu')", 5000)
            pg.locator(".gby-menu [role=menuitemradio]", has_text="Condition").first.click()
            attendi(pg, "E.ugBy && E.ugBy.then === 'cond'", 10000); fermo(pg, "#flst")
            assert count(pg, "MQ · H2O2") == 4, pg.evaluate(LABELS)
        step("Then by Condition gives «MQ · H2O2» with 4 files", then)
        def undo_redo():
            pg.evaluate("document.activeElement && document.activeElement.blur()")
            pg.keyboard.press("Control+z"); attendi(pg, "!E.ugBy || !E.ugBy.then", 5000)
            assert (count(pg, "MQ"), count(pg, "TW")) == (20, 9) and count(pg, "MQ · H2O2") == 0
            pg.keyboard.press("Control+z"); attendi(pg, "!E.ugBy && !E.files.some(f => f.ugrp)", 5000)
            pg.keyboard.press("Control+Shift+z"); attendi(pg, "!!E.ugBy && E.ugBy.by === 'matrix' && !E.ugBy.then", 5000)
            assert count(pg, "MQ") == 20
            pg.keyboard.press("Control+Shift+z"); attendi(pg, "E.ugBy && E.ugBy.then === 'cond'", 5000)
            assert count(pg, "MQ · H2O2") == 4
        step("Ctrl+Z / Ctrl+Shift+Z restore the labels and the key, one step each", undo_redo)
        def contains():
            open_menu(pg); item(pg, "Contains").click(); attendi(pg, "!!document.querySelector('dialog.gby-dlg[open]')", 5000)
            pg.locator("dialog.gby-dlg input[type=text]").fill("O2")
            attendi(pg, "document.querySelector('dialog.gby-dlg .gby-prev li') && !document.querySelector('dialog.gby-dlg button:not(.sec)').disabled", 10000)
            prev = pg.evaluate("[...document.querySelectorAll('dialog.gby-dlg .gby-prev li')].map(l => l.textContent)")
            assert prev == ["O2 · 4", "Other · 26"], prev
            pg.locator("dialog.gby-dlg button:not(.sec)").click(); attendi(pg, "E.ugBy && E.ugBy.by === 'manuale'", 10000); fermo(pg, "#flst")
            assert count(pg, "O2") == 4 and count(pg, "Other") == 26, pg.evaluate(LABELS)
        step("Contains «O2» matches the whole word only (4 files), the rest is «Other»", contains)
        def reload():
            saved(pg); pg.reload(); attendi(pg, "typeof E !== 'undefined' && E.files.some(f => f.ugrp)", 120000); ready(pg)
            assert pg.evaluate("E.ugBy && E.ugBy.by") == "manuale" and count(pg, "O2") == 4, pg.evaluate("E.ugBy")
        step("a reload keeps the groups and the key", reload)
        def keyboard():
            drawer(pg); watch(); fermo(pg, "#flst"); global box0; box0 = pg.evaluate(BOX); pg.locator("#sb-by").focus(); pg.keyboard.press("Enter"); attendi(pg, f"{ITEM}.length > 3", 10000)
            assert pg.evaluate("document.activeElement.closest('.gby-menu') !== null"), "the focus is not in the menu"
            assert pg.evaluate("document.querySelector('.gby-menu').getAttribute('role')") == "menu"
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("End"); pg.keyboard.press("Home")
            assert "None" in pg.evaluate("document.activeElement.textContent")
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('.gby-menu')", 3000)
            assert pg.evaluate("document.activeElement.id") == "sb-by", "the focus does not return to the button"
            pg.keyboard.press("Enter"); attendi(pg, f"{ITEM}.length > 3", 10000)
            while "Token position" not in pg.evaluate("document.activeElement.textContent"): pg.keyboard.press("ArrowDown")
            pg.keyboard.press("Enter"); attendi(pg, "!!document.querySelector('dialog.gby-dlg[open]')", 5000)
            assert pg.evaluate("document.activeElement.closest('dialog') !== null"), "the focus is not in the dialog"
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('dialog.gby-dlg')", 3000)
            assert pg.evaluate("document.activeElement.id") == "sb-by"
            pg.keyboard.press("Enter"); attendi(pg, f"{ITEM}.length > 3", 10000)
            while "None" not in pg.evaluate("document.activeElement.textContent"): pg.keyboard.press("ArrowUp")
            pg.keyboard.press("Space"); attendi(pg, "!E.ugBy && !E.files.some(f => f.ugrp)", 5000)
        step("the menu and the dialogs work with the keyboard only (arrows, Enter, Space, Esc), the focus returns to the button", keyboard)
        def axe():
            if not AXE: raise AssertionError("axe-core not available")
            for what in ("menu", "dialog"):
                if what == "menu": open_menu(pg)
                else:
                    pg.keyboard.press("Escape"); open_menu(pg); item(pg, "Contains").click(); attendi(pg, "!!document.querySelector('dialog.gby-dlg[open]')", 5000)
                v = pg.evaluate(RUN, "body"); assert not v, (what, v)
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('dialog.gby-dlg')", 3000)
        step("axe: the menu and the dialog have no serious or critical violation", axe)
        def stable():
            cls = pg.evaluate("window.__cls"); assert cls < 0.001, f"layout shift {cls}"
            assert pg.evaluate(BOX) == box0, (box0, pg.evaluate(BOX))
            assert not pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_MISS")
        step("no layout shift, same boxes of the tools and the header, no engine miss", stable)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
shutil.rmtree(D, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
