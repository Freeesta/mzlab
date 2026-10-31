"""Dragging files between the groups of the side list: a row onto a group header (real dragstart / drop with the mouse), three ticked files at once, onto the «Ungrouped» header and the gap,
a finger long press, Esc, the «Move to group» menu, Ctrl+Shift+Arrow; each step undoes and redoes in one step; a reload keeps the labels; a repaint during a drag waits; CLS 0, same boxes, axe clean."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
AXE = axe_path()
D = pathlib.Path(__file__).resolve().parent / "shots" / "gruppi_trascina_tmp"; D.mkdir(parents=True, exist_ok=True)
NAMES = ["MQ_blk", "MQ_Wash_1", "MQ_Wash_2", "TW_blk", "MQ_std_BTZ_1ppm", "QC_Std_5ppm_mid"]
NAMES += [f"MQ_{c}_{t}" for c in ("H2O2", "N2", "O2", "PMS") for t in ("T00", "T01", "T02", "T05")]
NAMES += [f"TW_{c}_{t}" for c in ("AIR", "H2O2") for t in ("T00", "T01", "T02", "T05")]
assert len(NAMES) == 30
ds.full_scan(D / "src.mzML", 5, np.random.default_rng(3))
PATHS = []
for n in NAMES:
    q = D / f"{n}.mzML"; shutil.copyfile(D / "src.mzML", q); PATHS.append(str(q))
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gruppi_trascina.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split("\n")[:8])[:500]))
LABELS = "Object.fromEntries(E.files.filter(f => !f.gone).map(f => [fileBase(f.file).replace(/\\.mzML$/, ''), f.ugrp || null]))"
def count(pg, label): return pg.evaluate("(l) => E.files.filter(f => f.ugrp === l).length", label)
def label(pg, name): return pg.evaluate(f"({LABELS})[{name!r}]")
def counts(pg): return (count(pg, "MQ"), count(pg, "TW"), pg.evaluate("E.files.filter(f => !f.gone && !f.ugrp).length"))
def tick(pg, names):
    pg.evaluate("(ns) => { E.files.forEach(f => { f.vis = ns.includes(fileBase(f.file).replace(/\\.mzML$/, '')); }); renderFileList(); }", names); fermo(pg, "#flst")
def row(pg, name):
    k = pg.evaluate(f"E.files.findIndex(f => fileBase(f.file).replace(/\\.mzML$/, '') === {name!r})")
    return pg.locator(f'#flst .nm[data-k="{k}"]').locator("xpath=ancestor::div[contains(@class,'fl')][1]")
def centre(loc):
    b = loc.bounding_box(); assert b, "the element is not in the page"
    return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2
HEAD = lambda g: f'#flst .fgh[data-ug="{g}"]'
AUTO = "#flst .fgh[data-gt]:not([data-ug])"
LIVE = "document.getElementById('gdnd-live')?.textContent || ''"
def drag(pg, name, target, hold=None):
    """A real mouse drag of a row onto a header (a selector) or a point; `hold` runs while the pointer hovers the target, before the drop."""
    x, y = centre(row(pg, name)); pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(x + 4, y + 4); pg.mouse.move(x + 8, y + 8)
    tx, ty = centre(pg.locator(target)) if isinstance(target, str) else target
    pg.mouse.move(tx, ty, steps=10); pg.mouse.move(tx + 1, ty, steps=2)
    if hold: hold()
    pg.mouse.up()
def said(text):
    got = pg.evaluate(LIVE); assert got == text, (text, got)      # a page polled by requestAnimationFrame stands still during a native drag: plain evaluate
def undo(pg): pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("Control+z")
def redo(pg): pg.keyboard.press("Control+Shift+z")
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
r = Run(port=8963, wd="/tmp/wd_gruppi_trascina")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 2200})
        pg.set_input_files("#pick", PATHS)
        pg.click("text=Carica dati"); ready(pg, timeout=180000); drawer(pg)
        pg.wait_for_selector("#flst .fl .nm", timeout=30000)
        pg.evaluate("I18N.set('en')"); pg.wait_for_function("window.qqReady === true && I18N.lang === 'en'", timeout=60000)
        pg.evaluate("window.__cls = 0; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; }).observe({type: 'layout-shift', buffered: false});")
        if AXE: pg.add_script_tag(path=AXE)
        BOX = "['#tools', '#dpanels', '#sb-header'].map(s => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height].map(Math.round).join(); }).join('|')"
        def matrix():
            pg.click("#sb-by"); attendi(pg, "document.querySelectorAll('.gby-menu [role=menuitemradio]').length > 3", 10000)
            pg.locator(".gby-menu [role=menuitemradio]", has_text="Matrix").first.click(); attendi(pg, "!!E.ugBy && E.ugBy.by === 'matrix'", 10000); fermo(pg, "#flst")
            assert counts(pg) == (20, 9, 1), counts(pg)
            tick(pg, [])
        step("starting point: Group by → Matrix (MQ 20, TW 9, one file out), nothing ticked", matrix)
        def one():
            seen = []
            drag(pg, "MQ_PMS_T00", HEAD("TW"), hold=lambda: seen.extend([pg.evaluate(f"document.querySelector('{HEAD('TW')}').classList.contains('gdnd-over')"), pg.evaluate(LIVE)]))
            assert seen == [True, "Move to TW · 1 file"], seen
            attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 10000); fermo(pg, "#flst")
            assert label(pg, "MQ_PMS_T00") == "TW" and counts(pg) == (19, 10, 1), counts(pg)
            assert pg.evaluate(LIVE) == "Moved to TW", pg.evaluate(LIVE)
            assert pg.evaluate("document.querySelectorAll('#flst .gdnd-over, #flst.gdnd-gap').length") == 0
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); assert counts(pg) == (20, 9, 1), counts(pg)
            redo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 5000); assert counts(pg) == (19, 10, 1), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); fermo(pg, "#flst")
            # the first history step lets the work area settle (the table of the empty work area hides): the boxes are taken from here
            global box0; box0 = pg.evaluate(BOX)
        step("a row dragged onto the TW header joins TW (header highlighted, «Move to TW · 1 file», «Moved to TW»); one undo and one redo", one)
        def three():
            names = ["MQ_N2_T00", "MQ_N2_T01", "MQ_N2_T02"]; tick(pg, names)
            drag(pg, "MQ_N2_T01", HEAD("TW"), hold=lambda: said("Move to TW · 3 files"))
            attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 10000); fermo(pg, "#flst")
            assert all(label(pg, n) == "TW" for n in names) and counts(pg) == (17, 12, 1), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); assert counts(pg) == (20, 9, 1), counts(pg)
            redo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 5000); assert counts(pg) == (17, 12, 1), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); tick(pg, [])
        step("three ticked files travel together, one undo step", three)
        def ungroup():
            drag(pg, "TW_AIR_T00", AUTO, hold=lambda: said("Remove from groups · 1 file"))
            attendi(pg, "!E.files.find(f => fileBase(f.file).startsWith('TW_AIR_T00')).ugrp", 10000); fermo(pg, "#flst")
            assert counts(pg) == (20, 8, 2), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); assert counts(pg) == (20, 9, 1), counts(pg)
            # the gap after the last group: the free space under the list, inside the pane
            b = pg.locator("#flst").bounding_box(); host = pg.locator("#sb-panels").bounding_box()
            assert host["y"] + host["height"] > b["y"] + b["height"] + 20, "no free space after the last group"
            drag(pg, "TW_H2O2_T05", (b["x"] + b["width"] / 2, b["y"] + b["height"] + 12))
            attendi(pg, "!E.files.find(f => fileBase(f.file).startsWith('TW_H2O2_T05')).ugrp", 10000); fermo(pg, "#flst")
            assert counts(pg) == (20, 8, 2), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000)
        step("dropped on the Ungrouped header, or on the gap after the last group, a file leaves its group; undo", ungroup)
        def midflight():
            pg.evaluate("document.querySelector('#flst .fl').dataset.mark = '1'")
            first = pg.evaluate("document.querySelector('#flst .fl .nm').textContent")
            name = pg.evaluate("(document.querySelector('#flst .fl[data-mark] .nm').textContent || '').replace(/\\.mzML$/, '')")
            def arrival():
                pg.evaluate("renderFileList()")
                assert pg.evaluate("!!document.querySelector('#flst .fl[data-mark]')"), "a repaint during the drag rebuilt the list"
            n0 = count(pg, "MQ")
            drag(pg, name, HEAD("TW"), hold=arrival)
            attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 10000); fermo(pg, "#flst")
            assert count(pg, "MQ") == n0 - 1 and pg.evaluate("document.querySelectorAll('#flst .fl').length") >= 30
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000)
        step("a repaint of the list during a drag waits (the dragged row stays), the drop still works", midflight)
        def touch():
            def finger(name, target, esc=False):
                k = pg.evaluate(f"E.files.findIndex(f => fileBase(f.file).replace(/\\.mzML$/, '') === {name!r})")
                x, y = centre(row(pg, name)); tx, ty = centre(pg.locator(target))
                ev = lambda t, px, py: f"el.dispatchEvent(new PointerEvent('{t}', {{bubbles: true, clientX: {px}, clientY: {py}, button: 0, pointerId: 7, pointerType: 'touch'}}))"
                pg.evaluate(f"(() => {{ const el = document.querySelector('#flst .nm[data-k=\"{k}\"]').closest('.fl'); {ev('pointerdown', x, y)}; }})()")
                attendi(pg, "window.GDND && GDND.busy()", 5000)
                pg.evaluate(f"(() => {{ const el = document.querySelector('#flst .nm[data-k=\"{k}\"]').closest('.fl'); {ev('pointermove', tx, ty)}; }})()")
                assert pg.evaluate(f"document.querySelector('{target}').classList.contains('gdnd-over')"), "the header is not highlighted"
                if esc: pg.keyboard.press("Escape")
                pg.evaluate(f"(() => {{ const el = document.querySelector('#flst .nm[data-k=\"{k}\"]').closest('.fl'); {ev('pointerup', tx, ty)}; }})()")
            finger("TW_blk", HEAD("MQ"))
            attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 10000); fermo(pg, "#flst")
            assert label(pg, "TW_blk") == "MQ" and counts(pg) == (21, 8, 1), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); assert counts(pg) == (20, 9, 1), counts(pg)
            finger("TW_blk", HEAD("MQ"), esc=True); fermo(pg, "#flst")
            assert label(pg, "TW_blk") == "TW" and counts(pg) == (20, 9, 1), counts(pg)
            assert pg.evaluate(LIVE) == "Move cancelled", pg.evaluate(LIVE)
            assert pg.evaluate("E.ugBy.by") == "matrix"
        step("a finger: long press, move over a header, lift drops it; Esc during the drag cancels (nothing changes)", touch)
        def menu():
            row(pg, "MQ_O2_T05").click(button="right"); attendi(pg, "!document.querySelector('#ctx').hidden", 5000)
            pg.locator("#ctx div[role=menuitem]", has_text="Move to group").click()
            attendi(pg, "[...document.querySelectorAll('#ctx div[role=menuitem]')].some(d => d.textContent === 'TW')", 5000)
            items = pg.evaluate("[...document.querySelectorAll('#ctx div[role=menuitem]')].map(d => d.textContent)")
            assert items[:2] == ["MQ", "TW"] and "New group…" in items and "Ungrouped" in items, items
            pg.locator("#ctx div[role=menuitem]", has_text="TW").click(); fermo(pg, "#flst")
            assert label(pg, "MQ_O2_T05") == "TW" and counts(pg) == (19, 10, 1), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000)
            # a new group: the name offered comes from the names of the files
            tick(pg, ["MQ_O2_T00", "MQ_O2_T01"]); row(pg, "MQ_O2_T00").click(button="right"); attendi(pg, "!document.querySelector('#ctx').hidden", 5000)
            pg.locator("#ctx div[role=menuitem]", has_text="Move to group").click()
            pg.locator("#ctx div[role=menuitem]", has_text="New group").click(); attendi(pg, "document.querySelector('#askdlg').open", 5000)
            offered = pg.input_value("#askin"); assert offered == "MQ_O2", offered
            pg.fill("#askin", "Oxygen"); pg.click("#askok"); fermo(pg, "#flst")
            assert count(pg, "Oxygen") == 2 and counts(pg) == (18, 9, 1), counts(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); assert counts(pg) == (20, 9, 1), counts(pg); tick(pg, [])
        step("right click → «Move to group ▸» lists the groups; «New group…» offers the common name of the ticked files", menu)
        def keyboard():
            tick(pg, [])
            def press(name, keys):
                row(pg, name).locator("input").focus(); pg.keyboard.press(keys); fermo(pg, "#flst")
            press("MQ_N2_T00", "Control+Shift+ArrowDown"); assert label(pg, "MQ_N2_T00") == "TW", label(pg, "MQ_N2_T00")
            assert pg.evaluate(LIVE) == "Moved to TW", pg.evaluate(LIVE)
            assert pg.evaluate("document.activeElement.matches('input[type=checkbox]') && document.activeElement.closest('.fl').textContent.includes('MQ_N2_T00')"), "the focus left the row"
            press("MQ_N2_T00", "Control+Shift+ArrowDown"); assert label(pg, "MQ_N2_T00") is None, label(pg, "MQ_N2_T00")
            assert pg.evaluate(LIVE) == "Removed from groups", pg.evaluate(LIVE)
            press("MQ_N2_T00", "Control+Shift+ArrowUp"); assert label(pg, "MQ_N2_T00") == "TW", label(pg, "MQ_N2_T00")
            press("MQ_N2_T00", "Control+Shift+ArrowUp"); assert label(pg, "MQ_N2_T00") == "MQ", label(pg, "MQ_N2_T00")
            press("MQ_N2_T00", "Control+Shift+ArrowUp"); assert pg.evaluate(LIVE) == "No more groups in this direction", pg.evaluate(LIVE)
            for _ in range(4): undo(pg); fermo(pg, "#flst")
            attendi(pg, "E.ugBy && E.ugBy.by === 'matrix'", 5000); assert counts(pg) == (20, 9, 1), counts(pg)
        step("Ctrl+Shift+↓/↑ on a row moves its file to the next / previous group, said in the live region, the focus stays; four undos", keyboard)
        def still():
            cls = pg.evaluate("window.__cls"); assert cls < 0.001, f"layout shift {cls}"
            assert pg.evaluate(BOX) == box0, (box0, pg.evaluate(BOX))
        step("no layout shift while dragging and regrouping, same boxes of the tools and the header", still)
        def reload():
            drag(pg, "MQ_PMS_T00", HEAD("TW")); attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 10000)
            saved(pg); pg.reload(); attendi(pg, "typeof E !== 'undefined' && E.files.some(f => f.ugrp)", 120000); ready(pg)
            assert label(pg, "MQ_PMS_T00") == "TW" and pg.evaluate("E.ugBy.by") == "custom"
        step("a reload keeps the labels and the key «custom»", reload)
        def axe():
            if not AXE: raise AssertionError("axe-core not available")
            pg.add_script_tag(path=AXE); pg.wait_for_function("typeof axe !== 'undefined'")
            v = pg.evaluate(RUN, "#sb-panel-file"); assert not v, v
        step("axe: the side list after the moves has no serious or critical violation", axe)
        def stable():
            pg.evaluate("renderFileList()"); drawer(pg); fermo(pg, "#flst")
            assert not pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_MISS")
        step("no engine miss", stable)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
shutil.rmtree(D, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
