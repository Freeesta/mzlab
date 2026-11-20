"""The sample table and the side-panel file list stay in sync (synthetic files, built site with the Rust engine): add, remove, rename, reorder, role, group, time,
selection, undo/redo and resuming a saved session, with every edit made in either place showing in the other without a reload.
Run from the repo root: python3 tests_e2e/e2e_samples_sync.py"""
# covers: mzlab/web/shell.js, mzlab/web/explore.js, mzlab/web/samples.js, mzlab/web/samples-table.js, mzlab/web/samples-doc.js, mzlab/web/lista-file.js, mzlab/web/lista-file-menu.js
import sys, os, shutil, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
from lib import *
import synth

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_samples_sync.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split("\n")[:12])[:1400]))

tmp = tempfile.mkdtemp(prefix="samples_sync_")
paths = synth.make_series(tmp, [0, 10, 30])          # s_t0, s_t10, s_t30, blank, std_5ppm
extra = os.path.join(tmp, "extra"); os.makedirs(extra)
more = [os.path.join(extra, "s_t60.mzML")]; synth.write_full(more[0], 60, seed=9)
# what the table says per row and what the side list shows per row, read from the DOM (never from the model)
TABLE = """() => [...document.querySelectorAll('#flist tbody tr[data-nm]')].map(tr => ({ name: tr.dataset.nm.replace(/\\.mzML$/, ''),
    use: tr.querySelector('input[data-k=use]').checked, role: tr.querySelector('select[data-k=type]').value, time: tr.querySelector('input[data-k=time]').value }))"""
PANEL = """() => [...document.querySelectorAll('#flst .fl[data-k]:not(.pr)')].map(e => ({ label: e.querySelector('.nm').textContent, name: E.files[+e.dataset.k].file.replace(/\\.mzML$/, ''), tag: e.querySelector('small').textContent.trim(),
    cur: e.classList.contains('cur') }))"""
def tag_of(role, time):
    return {"blank": "blk", "standard": "std"}.get(role) or ((time or "?") + " min")

r = Run(port=8873, wd=os.path.join(tmp, "wd"))
def both(pg, want=None):
    """Table and side list hold the same files (the table is sorted by role and time, the side list keeps the injection order: only the sets are compared),
    and the tag of every panel row follows the role and time of its table row."""
    t, p = pg.evaluate(TABLE), pg.evaluate(PANEL)
    # a removed file keeps its row in the table, unticked («Removed · Undo» ticks it again): the ticked rows are the files of the side list
    t = [x for x in t if x["use"]]
    assert sorted(x["name"] for x in t) == sorted(x["name"] for x in p), ("files", [x["name"] for x in t], [x["name"] for x in p])
    by = {x["name"]: x for x in p}
    for a in t:
        if a["role"] in ("sample", "blank", "standard"): assert by[a["name"]]["tag"] == tag_of(a["role"], a["time"]), (a, by[a["name"]])
    if want is not None: assert sorted(x["name"] for x in t) == sorted(want), [x["name"] for x in t]
    return t, p
def wait_both(pg, want=None, timeout=15000):
    """Wait on the real condition (the two views agree), then report the failure the assertion gives."""
    import time as _t
    t0 = _t.time(); err = None
    while _t.time() - t0 < timeout / 1000:
        try: return both(pg, want)
        except AssertionError as e: err = e; frames(pg, 2)
    raise err
def install(pg): pg.evaluate("() => { window.tick = n => document.querySelector('#flist tr[data-nm=\"' + n + '.mzML\"] input[data-k=use]').checked; }")
def table_view(pg):
    if pg.is_hidden("#wtable"): pg.click("#samptab")
    pg.wait_for_selector("#wtable", state="visible")
def row(pg, name): return pg.locator(f'#flist tr[data-nm="{name}.mzML"]')
def pane(pg, name): return pg.locator("#flst .fl", has=pg.locator(".nm", has_text=name)).first
def undo(pg): pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("Control+z"); frames(pg, 3)
def redo(pg): pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("Control+Shift+z"); frames(pg, 3)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        stage(pg, paths); pg.click("text=Load data"); ready(pg)
        show_table(pg); drawer(pg, "file")
        install(pg)
        base = ["s_t0", "s_t10", "s_t30", "blank", "std_5ppm"]
        step("the table and the side list start with the same files, roles and times", lambda: wait_both(pg, base))
        def role_from_table():
            row(pg, "s_t10").locator("select[data-k=type]").select_option("blank")
            wait_both(pg); assert {x["name"]: x["tag"] for x in pg.evaluate(PANEL)}["s_t10"] == "blk", pg.evaluate(PANEL)
        step("a role changed in the table shows in the side list", role_from_table)
        def time_from_table():
            i = row(pg, "s_t30").locator("input[data-k=time]"); i.fill("45"); i.press("Enter")
            wait_for_js(pg, "[...document.querySelectorAll('#flst .fl')].some(e => e.textContent.includes('45 min'))")
            wait_both(pg)
        step("a time typed in the table shows in the side list", time_from_table)
        def undo_redo():
            undo(pg); wait_both(pg)
            assert row(pg, "s_t30").locator("input[data-k=time]").input_value() == "30", "undo of the time"
            undo(pg); wait_both(pg)
            assert row(pg, "s_t10").locator("select[data-k=type]").input_value() == "sample", "undo of the role"
            redo(pg); wait_both(pg)
            assert row(pg, "s_t10").locator("select[data-k=type]").input_value() == "blank", "redo of the role"
            undo(pg); wait_both(pg)
        step("undo and redo move the table and the side list together", undo_redo)
        def rename_from_panel():
            pane(pg, "s_t10").click(button="right"); pg.click("text=Rename")
            pg.wait_for_selector("#askdlg[open]"); pg.fill("#askdlg input", "run_ten"); pg.click("#askok")
            wait_for_js(pg, "[...document.querySelectorAll('#flst .nm')].some(e => e.textContent === 'run_ten')")
            # the table's first column is the file itself (a rename is only a label, the file on disk is not touched): the row stays, with its role and time
            wait_both(pg, base)
        step("a rename in the side list is a label: the table row of that file keeps its identity", rename_from_panel)
        def remove_from_panel():
            pane(pg, "std_5ppm").locator(".fl-x").click()
            wait_for_js(pg, "!tick('std_5ppm')")
            wait_both(pg)
            undo(pg); wait_for_js(pg, "tick('std_5ppm')"); wait_both(pg)
        step("a file removed in the side list leaves the table, and Undo brings it back to both", remove_from_panel)
        def remove_from_table():
            table_view(pg)
            row(pg, "blank").locator("[data-rm]").click()
            wait_for_js(pg, "![...document.querySelectorAll('#flst .nm')].some(e => e.textContent === 'blank')")
            wait_both(pg)
            undo(pg); wait_for_js(pg, "[...document.querySelectorAll('#flst .nm')].some(e => e.textContent === 'blank')"); wait_both(pg)
        step("a file removed in the table leaves the side list, and Undo brings it back to both", remove_from_table)
        def select_both_ways():
            table_view(pg)
            pane(pg, "s_t30").locator(".nm").click()
            wait_for_js(pg, "E.files[E.cur].label === 's_t30'")
            assert [x["label"] for x in pg.evaluate(PANEL) if x["cur"]] == ["s_t30"]
            table_view(pg)          # a first click in the side list shows the charts, the table is one click away
            row(pg, "s_t0").locator("[data-open]").click()
            wait_for_js(pg, "E.files[E.cur].label === 's_t0'")
            assert [x["label"] for x in pg.evaluate(PANEL) if x["cur"]] == ["s_t0"], pg.evaluate(PANEL)
        step("selecting a file in either place selects the same file in the other", select_both_ways)
        def reorder():
            before = [x["name"] for x in pg.evaluate(PANEL)]
            pane(pg, before[-1]).drag_to(pane(pg, before[0]))
            wait_for_js(pg, "E.files[0].file !== %r" % (before[0] + ".mzML"))
            after = [x["name"] for x in pg.evaluate(PANEL)]; assert after != before
            want = "JSON.stringify(%s) === JSON.stringify(NB.session.slice().sort((a, b) => a.injection_order - b.injection_order).map(x => x.file.replace('.mzML', '')))" % __import__("json").dumps(after)
            wait_for_js(pg, want)          # the session is saved a moment after the edit
            wait_both(pg)
        step("a reorder in the side list is the injection order of the samples", reorder)
        def add_file():
            pg.click("#addf"); pg.set_input_files("#pick", more)
            wait_for_js(pg, "document.querySelectorAll('#flist tr[data-nm]').length === 6 && E.files.filter(f => !f.gone).length === 6")
            ready(pg); wait_both(pg)
        step("an added file is in the table and the side list", add_file)
        def group_both_ways():
            pane(pg, "s_t60").click(button="right"); pg.click("text=Move to group")
            pg.click("text=New group…"); pg.wait_for_selector("#askdlg[open]"); pg.fill("#askdlg input", "Series A"); pg.click("#askok")
            wait_for_js(pg, "document.querySelector('#flist').textContent.includes('Series A')")
            wait_for_js(pg, "E.files.some(f => f.ugrp === 'Series A')")          # a group of one file is listed flat in the side list: its header comes with the second file
            table_view(pg)
            row(pg, "s_t30").locator("[data-mv]").click(); pg.locator("[role=menu]:visible").get_by_text("Series A", exact=True).click()
            wait_for_js(pg, "ST.files.filter(f => f.ugrp === 'Series A').length === 2")
            wait_for_js(pg, "E.files.filter(f => f.ugrp === 'Series A').length === 2")
            wait_for_js(pg, "[...document.querySelectorAll('#flst .fgh')].filter(e => e.textContent.includes('Series A')).length === 1")
            undo(pg); wait_for_js(pg, "ST.files.filter(f => f.ugrp === 'Series A').length === 1 && E.files.filter(f => f.ugrp === 'Series A').length === 1"); wait_both(pg)
        step("a group set in the side list or in the table shows in the other, and Undo moves both", group_both_ways)
        def resume():
            saved(pg); pg.reload()
            pg.wait_for_function("typeof E !== 'undefined' && E.files.length >= 5", timeout=90000); ready(pg)
            wait_both(pg)
        step("a resumed session has the table and the side list in step", resume)
finally:
    r.close(); r.report(); shutil.rmtree(tmp, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
