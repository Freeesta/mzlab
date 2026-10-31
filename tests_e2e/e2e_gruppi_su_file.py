"""Drop a file ONTO another file (middle third of its row, after a short rest) to make a group of the two, or join the target's group: in the loading table (grip) and in the side list (row),
a drop on the outer thirds still reorders and a quick drop in the middle makes no group, «Group with…» of the row menu does the same from the keyboard, the new group's name opens in rename
mode (Esc keeps it); every action is one undo step named after it and ten of them undo and redo one by one; a reload keeps the labels; CLS 0, same boxes, axe clean."""
import sys, os, shutil, time; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
AXE = axe_path()
D = pathlib.Path(__file__).resolve().parent / "shots" / "gruppi_su_file_tmp"; D.mkdir(parents=True, exist_ok=True)
NAMES = ["MQ_blk", "MQ_Wash_1", "MQ_Wash_2", "TW_blk", "MQ_std_BTZ_1ppm", "QC_Std_5ppm_mid"]
NAMES += [f"MQ_{c}_{t}" for c in ("H2O2", "N2", "O2", "PMS") for t in ("T00", "T01", "T02", "T05")]
NAMES += [f"TW_{c}_{t}" for c in ("AIR", "H2O2") for t in ("T00", "T01", "T02", "T05")]
assert len(NAMES) == 30
ds.full_scan(D / "src.mzML", 5, np.random.default_rng(3))
PATHS = []
for n in NAMES:
    q = D / f"{n}.mzML"; shutil.copyfile(D / "src.mzML", q); PATHS.append(str(q))
steps = []
CLSAT = []
def step(name, fn):
    try:
        fn(); steps.append((name, "ok"))
        try: CLSAT.append((name[:30], pg.evaluate("window.__cls")))
        except Exception: pass
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gruppi_su_file.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split("\n")[:8])[:500]))
TAB = "Object.fromEntries(ST.files.map(f => [f.name.replace(/\\.mzML$/, ''), f.ugrp || null]))"
SIDE = "Object.fromEntries(E.files.filter(f => !f.gone).map(f => [fileBase(f.file).replace(/\\.mzML$/, ''), f.ugrp || null]))"
def tlabel(pg, n): return pg.evaluate(f"({TAB})[{n!r}]")
def slabel(pg, n): return pg.evaluate(f"({SIDE})[{n!r}]")
# the labels of both lists and the key, independent of the order of the files
SNAP = f"JSON.stringify([Object.entries({TAB}).sort(), Object.entries({SIDE}).sort(), E.ugBy || null])"
def snap(pg): return pg.evaluate(SNAP)
def same(pg):
    d = pg.evaluate(f"(() => {{ const t = {TAB}, s = {SIDE}; return Object.keys(s).filter(k => (t[k] || null) !== (s[k] || null)); }})()")
    assert not d, ("table and side list differ", d)
def trow(pg, name): return pg.locator(f'#flist tr[data-nm^="{name}."]')
def srow(pg, name):
    k = pg.evaluate(f"E.files.findIndex(f => fileBase(f.file).replace(/\\.mzML$/, '') === {name!r})")
    return pg.locator(f'#flst .nm[data-k="{k}"]').locator("xpath=ancestor::div[contains(@class,'fl')][1]")
def box(loc):
    b = loc.bounding_box(); assert b, "the element is not in the page"; return b
def centre(loc):
    b = box(loc); return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2
LIVE = "document.getElementById('gdnd-live')?.textContent || ''"
def table(pg):
    if pg.get_attribute("#samptab", "aria-pressed") != "true": pg.click("#samptab")
    attendi(pg, "!!document.querySelector('#flist').offsetParent", 10000); fermo(pg, "#flist")
def drag(pg, src, dst, where=0.5, rest=0.8, hold=None):
    """A real mouse drag from the centre of `src` to the point `where` (0 top … 1 bottom) of the height of `dst`; it rests `rest` s there (the 600 ms of the middle third), `hold` runs before the drop."""
    x, y = centre(src); pg.mouse.move(x, y); pg.mouse.down(); pg.mouse.move(x + 4, y + 4); pg.mouse.move(x + 8, y + 8)
    b = box(dst); tx, ty = b["x"] + b["width"] / 2, b["y"] + b["height"] * where
    pg.mouse.move(tx, ty, steps=10); pg.mouse.move(tx + 1, ty, steps=2)
    if rest: time.sleep(rest); pg.mouse.move(tx, ty, steps=2)
    if hold: hold()
    pg.mouse.up()
def undo(pg): pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("Control+z")
def redo(pg): pg.keyboard.press("Control+Shift+z")
def last(pg): return pg.evaluate("(STORIA.pila.v[STORIA.pila.v.length - 1] || {}).w")
def esc_rename(pg, expect):
    """The new group's name opens in rename mode with the proposed name; Esc keeps it."""
    attendi(pg, "document.getElementById('askdlg').open", 10000)
    got = pg.evaluate("document.getElementById('askin').value"); assert got == expect, (expect, got)
    pg.keyboard.press("Escape"); attendi(pg, "!document.getElementById('askdlg').open", 5000)
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
r = Run(port=8965, wd="/tmp/wd_gruppi_su_file")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 2200})
        pg.evaluate("I18N.set('en')"); pg.wait_for_function("window.qqReady === true && I18N.lang === 'en'", timeout=60000)
        pg.set_input_files("#pick", PATHS)
        pg.click("text=Load data" if pg.locator("text=Load data").count() else "text=Carica dati"); ready(pg, timeout=180000); drawer(pg)
        pg.wait_for_selector("#flst .fl .nm", timeout=30000)
        try: attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && document.querySelectorAll('#flist tbody tr').length === 30", 120000)
        except Exception: raise AssertionError(pg.evaluate("[UPBUSY, OPENNOW.busy(), ST.files.length, document.querySelectorAll('#flist tbody tr').length]"))
        pg.evaluate("window.__cls = 0; window.__sh = []; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) { window.__cls += e.value; window.__sh.push([e.value, [...e.sources].map(x => x.node && ((x.node.id || x.node.className || x.node.nodeName) + '')).join(',')]); } }).observe({type: 'layout-shift', buffered: false});")
        if AXE: pg.add_script_tag(path=AXE)
        BOX = "['#tools', '#sb-header', '#drop'].map(s => { const e = document.querySelector(s); if (!e || !e.offsetParent) return null; const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height].map(Math.round).join(); }).join('|')"
        def start():
            table(pg); global box0; box0 = pg.evaluate(BOX)
            assert not pg.evaluate("document.querySelectorAll('#flist tr.tg-head').length") and not any(pg.evaluate(f"Object.values({SIDE})"))
            assert pg.evaluate("document.querySelectorAll('#flist .tg-grip').length") == 30
        step("starting point: 30 files, no group, every table row has its grip", start)
        def outer():
            fermo(pg, "#flst"); before = snap(pg); order = pg.evaluate("E.files.map(f => f.file)")
            drag(pg, srow(pg, "TW_blk"), srow(pg, "MQ_blk"), where=0.1, rest=0)           # the top third: insert before, as ever
            attendi(pg, "E.files.findIndex(f => f.file.startsWith('TW_blk')) < E.files.findIndex(f => f.file.startsWith('MQ_blk'))", 10000)
            drag(pg, srow(pg, "MQ_blk"), srow(pg, "MQ_Wash_2"), where=0.9, rest=0)        # the bottom third: insert after
            attendi(pg, "E.files.findIndex(f => f.file.startsWith('MQ_blk')) > E.files.findIndex(f => f.file.startsWith('MQ_Wash_2'))", 10000)
            assert snap(pg) == before, "a reorder made a group"
            assert pg.evaluate("E.files.map(f => f.file)") != order
        step("a drop on the top or bottom third of a side-list row reorders and makes no group", outer)
        def quick():
            before = snap(pg)
            drag(pg, srow(pg, "TW_blk"), srow(pg, "MQ_Wash_1"), where=0.5, rest=0)         # no rest in the middle: no group
            fermo(pg, "#flst"); assert snap(pg) == before, "a quick drop in the middle made a group"
        step("a quick drop in the middle of a low (31 px) side-list row is not an «onto» (it needs the rest)", quick)
        S = []
        def chain(name, fn, key, expect=None):
            fn(); fermo(pg, "#flist"); same(pg); S.append(snap(pg))
            w = last(pg); assert w == key, (name, key, w)
        S.append(snap(pg)); pg.evaluate("window.__cls = 0; window.__sh = []")          # the reorder above moves rows on purpose: the group actions are what must not shift anything
        def new_table():
            table(pg); seen = []
            drag(pg, trow(pg, "MQ_H2O2_T01").locator(".tg-grip"), trow(pg, "MQ_H2O2_T00"), hold=lambda: seen.extend([pg.evaluate("!!document.querySelector('#flist tr.gdnd-onto')"), pg.evaluate(LIVE)]))
            assert seen == [True, "New group with MQ_H2O2_T00"], seen
            esc_rename(pg, "MQ_H2O2")
            assert tlabel(pg, "MQ_H2O2_T00") == "MQ_H2O2" == tlabel(pg, "MQ_H2O2_T01"), pg.evaluate(TAB)
            assert pg.evaluate(f"Object.values({TAB}).filter(x => x === 'MQ_H2O2').length") == 2
        step("table: MQ_H2O2_T01 dropped onto MQ_H2O2_T00 makes the group «MQ_H2O2» (2 files), its name in rename mode, Esc keeps it", lambda: chain("new_table", new_table, "groups.undo.new"))
        def new_table2():
            table(pg)
            drag(pg, trow(pg, "MQ_N2_T00").locator(".tg-grip"), trow(pg, "MQ_PMS_T00"))
            esc_rename(pg, "MQ_T00")
            assert tlabel(pg, "MQ_N2_T00") == "MQ_T00" == tlabel(pg, "MQ_PMS_T00")
        step("table: MQ_N2_T00 onto MQ_PMS_T00 makes «MQ_T00» (the tokens the names share)", lambda: chain("new_table2", new_table2, "groups.undo.new"))
        def join():
            table(pg); n = pg.evaluate("new Set(Object.values(ST.files.map(f => f.ugrp).filter(Boolean))).size")
            drag(pg, trow(pg, "MQ_O2_T00").locator(".tg-grip"), trow(pg, "MQ_H2O2_T01"), hold=lambda: assert_said(pg, "Move to MQ_H2O2 · 1 file"))
            attendi(pg, "ST.files.find(f => f.name.startsWith('MQ_O2_T00')).ugrp === 'MQ_H2O2'", 10000)
            assert pg.evaluate("new Set(ST.files.map(f => f.ugrp).filter(Boolean)).size") == n, "a group was made instead of a join"
        def assert_said(pg, text):
            got = pg.evaluate(LIVE); assert got == text, (text, got)
        step("table: a file dropped onto a grouped file joins that group (no new group)", lambda: chain("join", join, "groups.undo.join"))
        def new_side():
            drag(pg, srow(pg, "TW_AIR_T00"), srow(pg, "TW_AIR_T01"), hold=lambda: assert_said(pg, "New group with TW_AIR_T01"))
            esc_rename(pg, "TW_AIR")
            assert slabel(pg, "TW_AIR_T00") == "TW_AIR" == slabel(pg, "TW_AIR_T01")
        step("side list: TW_AIR_T00 onto TW_AIR_T01 makes «TW_AIR»", lambda: chain("new_side", new_side, "groups.undo.new"))
        def with_menu():
            table(pg)
            trow(pg, "TW_H2O2_T05").locator("[data-mv]").focus(); pg.keyboard.press("Enter"); attendi(pg, "!document.querySelector('#ctx').hidden", 5000)
            pg.locator("#ctx", has_text="Group with…").locator("text=Group with…").first.click()
            attendi(pg, "document.getElementById('askdlg').open && document.getElementById('askin').getAttribute('list')", 5000)
            pg.keyboard.type("AIR_T05"); pg.keyboard.press("Enter")
            esc_rename(pg, "TW_T05")
            assert tlabel(pg, "TW_H2O2_T05") == "TW_T05" == tlabel(pg, "TW_AIR_T05")
        step("«Group with…» of the row menu (keyboard, type-ahead) does the same as a drop onto", lambda: chain("with", with_menu, "groups.undo.new"))
        def rename():
            pg.locator('#flst .fgh[data-ug="TW_T05"] .chv').focus(); pg.keyboard.press("F2")
            attendi(pg, "document.getElementById('askdlg').open", 5000); pg.keyboard.type("Tween"); pg.keyboard.press("Enter")
            attendi(pg, "ST.files.some(f => f.ugrp === 'Tween')", 10000)
            assert tlabel(pg, "TW_AIR_T05") == "Tween" == slabel(pg, "TW_AIR_T05")
        step("F2 renames the group in the side list and the table", lambda: chain("rename", rename, "groups.undo.rename"))
        def move():
            table(pg)
            drag(pg, trow(pg, "MQ_blk").locator(".tg-grip"), pg.locator('#flist tr.tg-head[data-ug="MQ_T00"]'), rest=0)
            attendi(pg, "ST.files.find(f => f.name.startsWith('MQ_blk')).ugrp === 'MQ_T00'", 10000)
        step("a row dragged onto a header still moves to that group", lambda: chain("move", move, "groups.undo.move"))
        def ungroup():
            pg.locator('#flst .fgh[data-ug="Tween"]').click(button="right")
            attendi(pg, "!document.querySelector('#ctx').hidden", 5000); pg.locator("#ctx", has_text="Ungroup").locator("text=Ungroup").first.click()
            attendi(pg, "!ST.files.some(f => f.ugrp === 'Tween')", 10000)
        step("«Ungroup» on a side header frees the files of the table too", lambda: chain("ungroup", ungroup, "groups.undo.ungroup"))
        def by():
            table(pg); pg.click("#tb-by"); attendi(pg, "document.querySelectorAll('.gby-menu [role=menuitemradio]').length > 3", 10000)
            pg.locator(".gby-menu [role=menuitemradio]", has_text="Condition").first.click(); attendi(pg, "!!E.ugBy && E.ugBy.by === 'cond'", 10000)
        step("Group by → Condition replaces the manual groups", lambda: chain("by", by, "groups.undo.by"))
        def none():
            table(pg); pg.click("#tb-by"); attendi(pg, "document.querySelectorAll('.gby-menu [role=menuitemradio]').length > 3", 10000)
            pg.locator(".gby-menu [role=menuitemradio]", has_text="None").first.click(); attendi(pg, "!E.ugBy", 10000)
        step("Group by → None removes every group", lambda: chain("none", none, "groups.undo.none"))
        def walk():
            n = len(S) - 1
            for k in range(n, 0, -1):
                undo(pg); attendi(pg, f"{SNAP} === {S[k - 1]!r}", 8000)
            for k in range(1, n + 1):
                redo(pg); attendi(pg, f"{SNAP} === {S[k]!r}", 8000)
        step("ten mixed group actions undo one by one back to the start and redo one by one", walk)
        def calm():
            assert pg.evaluate("window.__cls") < 0.001, [pg.evaluate("[window.__cls, window.__sh]"), CLSAT]
        step("CLS 0 over the group actions (drops, menus, rename, undo, redo)", calm)
        def ax():
            if not AXE: return
            pg.add_script_tag(path=AXE); table(pg); v = pg.evaluate(RUN, "#flist"); assert not v, v
        step("axe: the table with its grips has no serious or critical violation", ax)
        def reload():
            table(pg); drag(pg, trow(pg, "MQ_N2_T01").locator(".tg-grip"), trow(pg, "MQ_N2_T02")); esc_rename(pg, "MQ_N2")
            attendi(pg, "E.files.some(f => f.ugrp === 'MQ_N2')", 10000); pg.evaluate("uiSave()"); time.sleep(0.6)
            pg.reload(); attendi(pg, "typeof E !== 'undefined' && E.files.some(f => f.ugrp)", 120000); ready(pg)
            assert slabel(pg, "MQ_N2_T01") == "MQ_N2" == slabel(pg, "MQ_N2_T02")
        step("a reload keeps the group made by a drop onto", reload)
        step("no engine miss", lambda: pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0 or (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
shutil.rmtree(D, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
