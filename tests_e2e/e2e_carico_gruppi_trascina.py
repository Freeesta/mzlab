"""Groups in the loading table: «Group by» from the control above the table, a row dragged by its grip onto a header (real mouse), the «Move to group» button (mouse and keyboard),
two new files dropped from the desktop on a header of the table and one on a header of the side list (they take the group as they arrive); the labels are the same in table and side
list after every step, each move undoes and redoes in one step; CLS 0 and the boxes of #tools and the header stay; axe clean on the table."""
import sys, os, shutil, base64; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
AXE = axe_path()
D = pathlib.Path(__file__).resolve().parent / "shots" / "carico_gruppi_trascina_tmp"; D.mkdir(parents=True, exist_ok=True)
NAMES = ["MQ_blk", "MQ_Wash_1", "MQ_Wash_2", "TW_blk", "MQ_std_BTZ_1ppm", "QC_Std_5ppm_mid"]
NAMES += [f"MQ_{c}_{t}" for c in ("H2O2", "N2", "O2", "PMS") for t in ("T00", "T01", "T02", "T05")]
NAMES += [f"TW_{c}_{t}" for c in ("AIR", "H2O2") for t in ("T00", "T01", "T02", "T05")]
assert len(NAMES) == 30
ds.full_scan(D / "src.mzML", 5, np.random.default_rng(3))
PATHS = []
for n in NAMES + ["QC_new_1", "QC_new_2", "QC_new_3"]:
    q = D / f"{n}.mzML"; shutil.copyfile(D / "src.mzML", q); PATHS.append(q)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_carico_gruppi_trascina.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split("\n")[:8])[:500]))
TAB = "Object.fromEntries(ST.files.map(f => [f.name.replace(/\\.mzML$/, ''), f.ugrp || null]))"
SIDE = "Object.fromEntries(E.files.filter(f => !f.gone).map(f => [fileBase(f.file).replace(/\\.mzML$/, ''), f.ugrp || null]))"
def tlabel(pg, n): return pg.evaluate(f"({TAB})[{n!r}]")
def slabel(pg, n): return pg.evaluate(f"({SIDE})[{n!r}]")
def same(pg):
    """Every open file has the label of its table row, and the table shows each row under the header of its label."""
    d = pg.evaluate(f"(() => {{ const t = {TAB}, s = {SIDE}; return Object.keys(s).filter(k => (t[k] || null) !== (s[k] || null)); }})()")
    assert not d, ("table and side list differ", d)
    shown = pg.evaluate("""(() => { let g = null; const out = {}; for (const tr of document.querySelectorAll('#flist tbody tr')) { if (tr.classList.contains('tg-head')) g = tr.dataset.ug; else out[tr.dataset.nm.replace(/\\.mzML$/, '')] = g || null; } return out; })()""")
    t = pg.evaluate(TAB); bad = [k for k in t if (shown.get(k) or None) != (t[k] or None)]
    assert not bad, ("the table shows rows under another header", bad[:5])
def heads(pg): return pg.evaluate("[...document.querySelectorAll('#flist tr.tg-head')].map(h => [h.dataset.ug, h.querySelector('.muted').textContent])")
def counts(pg, *gs): return tuple(pg.evaluate("(g) => ST.files.filter(f => (f.ugrp || '') === g).length", g) for g in gs)
def trow(pg, name): return pg.locator(f'#flist tr[data-nm^="{name}."]')
def centre(loc):
    b = loc.bounding_box(); assert b, "the element is not in the page"
    return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2
THEAD = lambda g: f'#flist tr.tg-head[data-ug="{g}"]'
LIVE = "document.getElementById('gdnd-live')?.textContent || ''"
def said(pg, text):
    got = pg.evaluate(LIVE); assert got == text, (text, got)      # a page polled by requestAnimationFrame stands still during a native drag: plain evaluate
def drag(pg, name, target, hold=None):
    """A real mouse drag of a table row by its grip onto a header (a selector)."""
    x, y = centre(trow(pg, name).locator(".tg-grip")); pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(x + 4, y + 4); pg.mouse.move(x + 8, y + 8)
    tx, ty = centre(pg.locator(target)); pg.mouse.move(tx, ty, steps=10); pg.mouse.move(tx + 1, ty, steps=2)
    if hold: hold()
    pg.mouse.up()
def table(pg):
    """The sample table on screen (the work area leaves it when a file is shown or the work settles)."""
    if pg.get_attribute("#samptab", "aria-pressed") != "true": pg.click("#samptab")
    attendi(pg, "!!document.querySelector('#flist').offsetParent", 10000); fermo(pg, "#flist")
def undo(pg): pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("Control+z")
def redo(pg): pg.keyboard.press("Control+Shift+z")
def item(pg, text): return pg.locator(".gby-menu [role=menuitemradio]", has_text=text).first
def desktop_drop(pg, over, files, sel_zone):
    """The drag events of files dragged in from the desktop: they enter `sel_zone`, hover the header `over` (its place is read after the zone has shown: the side panel opens a drop box above the list) and drop there."""
    data = [[p.name, base64.b64encode(p.read_bytes()).decode()] for p in files]
    pg.evaluate("""async ([data, zone, over]) => {
        const dt = new DataTransfer(); for (const [n, b] of data) dt.items.add(new File([Uint8Array.from(atob(b), c => c.charCodeAt(0))], n));
        const z = document.querySelector(zone), h = document.querySelector(over), at = () => { const r = h.getBoundingClientRect(); return { clientX: r.x + r.width / 2, clientY: r.y + r.height / 2 }; };
        const ev = t => new DragEvent(t, { dataTransfer: dt, bubbles: true, cancelable: true, ...at() });
        z.dispatchEvent(ev('dragenter')); await new Promise(r => setTimeout(r, 150)); h.dispatchEvent(ev('dragover')); await new Promise(r => setTimeout(r, 150)); h.dispatchEvent(ev('dragover'));
        window.__ext = document.getElementById('gdnd-live')?.textContent || '';
        h.dispatchEvent(ev('drop'));
    }""", [data, sel_zone, over])
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
r = Run(port=8964, wd="/tmp/wd_carico_gruppi_trascina")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 2200})
        pg.evaluate("I18N.set('en')"); pg.wait_for_function("window.qqReady === true && I18N.lang === 'en'", timeout=60000)
        pg.set_input_files("#pick", [str(q) for q in PATHS[:30]])
        pg.click("text=Load data" if pg.locator("text=Load data").count() else "text=Carica dati"); ready(pg, timeout=180000); drawer(pg)
        pg.wait_for_selector("#flst .fl .nm", timeout=30000)
        pg.evaluate("window.__cls = 0; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; }).observe({type: 'layout-shift', buffered: false});")
        if AXE: pg.add_script_tag(path=AXE)
        BOX = "['#tools', '#sb-header', '#drop'].map(s => { const e = document.querySelector(s); if (!e || !e.offsetParent) return null; const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height].map(Math.round).join(); }).join('|')"
        def start():
            try: attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && document.querySelectorAll('#flist tbody tr').length === 30", 120000)
            except Exception: raise AssertionError(pg.evaluate("[UPBUSY, OPENNOW.busy(), ST.files.length, document.querySelectorAll('#flist tbody tr').length, document.querySelector('#v-workspace').dataset.work]"))
            table(pg)
            assert not heads(pg) and pg.is_visible("#tb-by"), "the table is as before: no group row, the control above it"
            assert pg.evaluate("document.querySelectorAll('#flist .tg-grip, #flist [data-mv]').length") == 0
        step("starting point: the table of 30 files, no group row, «Group by» above it", start)
        def by():
            table(pg); pg.click("#tb-by"); attendi(pg, "document.querySelectorAll('.gby-menu [role=menuitemradio]').length > 3", 10000)
            assert pg.get_attribute("#tb-by", "aria-expanded") == "true"
            item(pg, "Condition").click(); attendi(pg, "!!E.ugBy && E.ugBy.by === 'cond'", 10000); fermo(pg, "#flist")
            hs = heads(pg); names = [h[0] for h in hs]
            assert "PMS" in names and "H2O2" in names and names[-1] == "", hs
            assert sum(int(h[1].split()[0]) for h in hs) == 30, hs
            same(pg)
            assert pg.evaluate("[...document.querySelectorAll('#flist .tg-head .tg-fold')].every(b => b.getBoundingClientRect().width >= 24 && b.getBoundingClientRect().height >= 24)")
        step("Group by → Condition from the control above the table: a header per group and «Ungrouped», same labels in the side list", by)
        def one():
            table(pg); global box0; box0 = pg.evaluate(BOX)
            seen = []
            drag(pg, "MQ_O2_T00", THEAD("PMS"), hold=lambda: seen.extend([pg.evaluate(f"document.querySelector('{THEAD('PMS')}').classList.contains('gdnd-over')"), pg.evaluate(LIVE)]))
            assert seen == [True, "Move to PMS · 1 file"], seen
            attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 10000); fermo(pg, "#flist")
            assert tlabel(pg, "MQ_O2_T00") == "PMS" and slabel(pg, "MQ_O2_T00") == "PMS"
            assert pg.evaluate(LIVE) == "Moved to PMS", pg.evaluate(LIVE)
            same(pg)
            assert pg.evaluate("document.querySelectorAll('#flist .gdnd-over, #flist.gdnd-gap').length") == 0
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'cond'", 5000); table(pg)
            assert tlabel(pg, "MQ_O2_T00") == "O2" and slabel(pg, "MQ_O2_T00") == "O2"; same(pg)
            redo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 5000); fermo(pg, "#flist")
            assert tlabel(pg, "MQ_O2_T00") == "PMS"; same(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'cond'", 5000); table(pg)
        step("a row dragged by its grip onto the PMS header joins PMS (header highlighted, said, one undo, one redo), table and side list agree", one)
        def ungroup():
            table(pg)
            drag(pg, "MQ_PMS_T00", THEAD(""), hold=lambda: said(pg, "Remove from groups · 1 file"))
            attendi(pg, "!ST.files.find(f => f.name.startsWith('MQ_PMS_T00')).ugrp", 10000); fermo(pg, "#flist")
            assert slabel(pg, "MQ_PMS_T00") is None; same(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'cond'", 5000); table(pg)
            assert tlabel(pg, "MQ_PMS_T00") == "PMS"; same(pg)
        step("a row dragged onto the «Ungrouped» header leaves its group; undo brings it back", ungroup)
        def menu():
            table(pg)
            b = trow(pg, "MQ_N2_T01").locator("[data-mv]"); assert b.bounding_box()["width"] >= 24
            b.focus(); pg.keyboard.press("Enter"); attendi(pg, "!document.querySelector('#ctx').hidden", 5000)
            labs = pg.evaluate("[...document.querySelectorAll('#ctx button, #ctx [role=menuitem]')].map(x => x.textContent.trim())")
            assert "PMS" in labs and any("New group" in l for l in labs), labs
            pg.locator("#ctx", has_text="PMS").locator("text=PMS").first.click(); attendi(pg, "E.ugBy && E.ugBy.by === 'custom'", 5000); fermo(pg, "#flist")
            assert tlabel(pg, "MQ_N2_T01") == "PMS" and slabel(pg, "MQ_N2_T01") == "PMS"; same(pg)
            undo(pg); attendi(pg, "E.ugBy && E.ugBy.by === 'cond'", 5000); table(pg)
            assert tlabel(pg, "MQ_N2_T01") == "N2"; same(pg)
        step("«Move to group ▸» (button, opened with the keyboard) puts the row in PMS; undo", menu)
        def fold():
            table(pg)
            pg.click('#flist [data-fold="N2"]'); fermo(pg, "#flist")
            assert pg.evaluate("[...document.querySelectorAll('#flist tr[data-nm^=\"MQ_N2\"]')].every(r => r.hidden)")
            assert pg.get_attribute('#flist [data-fold="N2"]', "aria-expanded") == "false"
            pg.click('#flist [data-fold="N2"]'); fermo(pg, "#flist")
            assert pg.evaluate("[...document.querySelectorAll('#flist tr[data-nm^=\"MQ_N2\"]')].every(r => !r.hidden)")
        step("a group folds and unfolds with its 24 px button", fold)
        def drop_table():
            table(pg)
            desktop_drop(pg, THEAD("PMS"), PATHS[30:32], "#v-workspace")
            assert pg.evaluate("window.__ext") == "Add dropped files to PMS", pg.evaluate("window.__ext")
            attendi(pg, "ST.files.length === 32 && E.files.filter(f => !f.gone).length === 32 && UPBUSY === 0 && OPENNOW.busy() === 0", 120000); fermo(pg, "#flist")
            assert [tlabel(pg, f"QC_new_{i}") for i in (1, 2)] == ["PMS", "PMS"], pg.evaluate(TAB)
            assert [slabel(pg, f"QC_new_{i}") for i in (1, 2)] == ["PMS", "PMS"], pg.evaluate(SIDE)
            assert pg.evaluate("E.ugBy.by") == "custom"; same(pg)
            assert pg.locator("#flst .fgh[data-ug=\"PMS\"]").count() == 1
        step("two new files dropped from the desktop on the PMS header of the table arrive with ugrp = PMS and open into the PMS group of the side list", drop_table)
        def drop_side():
            desktop_drop(pg, '#flst .fgh[data-ug="O2"]', PATHS[32:33], "#dfiles")
            attendi(pg, "ST.files.length === 33 && E.files.filter(f => !f.gone).length === 33 && UPBUSY === 0 && OPENNOW.busy() === 0", 120000); fermo(pg, "#flist")
            assert (tlabel(pg, "QC_new_3"), slabel(pg, "QC_new_3")) == ("O2", "O2"), (tlabel(pg, "QC_new_3"), slabel(pg, "QC_new_3"), pg.evaluate("E.ugBy"), pg.evaluate("window.__ext"))
            same(pg)
        step("a file dropped from the desktop on a header of the side list takes that group", drop_side)
        def calm():
            assert pg.evaluate("window.__cls") == 0, pg.evaluate("window.__cls")
            assert pg.evaluate(BOX) == box0, (box0, pg.evaluate(BOX))
        step("no layout shift from the first drag to the settled table; #tools, the header and #drop keep their boxes", calm)
        def ax():
            if not AXE: return
            v = pg.evaluate(RUN, "#flist"); assert not v, v
        step("axe: the table with its groups has no serious or critical violation", ax)
        step("no engine miss", lambda: pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0 or (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
shutil.rmtree(D, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
