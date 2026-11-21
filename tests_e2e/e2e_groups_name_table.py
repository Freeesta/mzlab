# covers: mzlab/web/groups-tab.js, mzlab/web/groups-name.js, mzlab/web/groups.js, mzlab/web/columns-samples.js
"""Sample table: double click on a group header renames it in place (Enter saves, Esc cancels, blur saves, empty = default name, a duplicate is refused with an inline message,
F2/Enter on the focused name starts it, a double click does not fold the group); «Ungrouped» can get a label of its own; the name survives a reload of the session;
the «+» at the end of the header row opens the «Columns» chooser, a column added there appears in the table."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
HEAD = ".ldtab tr.tg-head"
NAMES = "[...document.querySelectorAll('.ldtab tr.tg-head .tg-name')].map(b => b.textContent.trim())"
r = Run(port=8967, wd="/tmp/wd_gruppi_tabella_nome")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        fs = sorted((ROOT / "tests" / "data" / "groups").glob("*.mzML"))[:3]
        stage(pg, fs)
        pg.evaluate("loadScript('groups.js').then(() => { const n = ST.files.map(f => f.name); GROUPS.set(n.slice(0, 2), 'Blank'); })")
        wait_for_js(pg, f"document.querySelectorAll('{HEAD}').length === 2", 10000)
        pg.click("text=Sample table"); wait_for_js(pg, "!!document.querySelector('.tg-name') && document.querySelector('.tg-name').offsetWidth > 0", 5000)
        def heads(): return pg.evaluate(NAMES)
        def dbl():
            assert heads() == ["Blank", "Ungrouped"], heads()
            pg.locator(f"{HEAD} .tg-name").first.dblclick()
            wait_for_js(pg, "!!document.querySelector('.tg-edit')", 3000)
            assert pg.locator(".tg-head .tg-fold").first.get_attribute("aria-expanded") == "true", "the double click folded the group"
            pg.keyboard.press("Control+a"); pg.keyboard.type("Controls"); pg.keyboard.press("Enter")
            wait_for_js(pg, "E.files.every(f => f.ugrp !== 'Blank') && ST.files.filter(f => f.ugrp === 'Controls').length === 2", 5000)
            assert heads()[0] == "Controls", heads()
        step("double click renames in place, Enter saves, the group stays open", dbl)
        def esc():
            pg.locator(f"{HEAD} .tg-name").first.dblclick(); pg.keyboard.press("Control+a"); pg.keyboard.type("Nope"); pg.keyboard.press("Escape")
            assert heads()[0] == "Controls" and pg.locator(".tg-edit").count() == 0, heads()
        step("Esc cancels", esc)
        def dup():
            pg.evaluate("GROUPS.set([ST.files[2].name], 'Samples')"); wait_for_js(pg, f"document.querySelectorAll('{HEAD}').length === 2", 5000)
            pg.locator(f"{HEAD} .tg-name").first.dblclick(); pg.keyboard.press("Control+a"); pg.keyboard.type("samples"); pg.keyboard.press("Enter")
            assert pg.locator(".tg-edit").count() == 1 and pg.locator(".tg-err").inner_text().strip(), "no inline message for a duplicate"
            assert pg.locator(".tg-edit").get_attribute("aria-invalid") == "true"
            pg.keyboard.press("Escape"); assert heads()[0] == "Controls", heads()
        step("a duplicate name is refused with an inline message", dup)
        def blur_empty():
            pg.locator(f"{HEAD} .tg-name").first.dblclick(); pg.keyboard.press("Control+a"); pg.keyboard.type("Blur name"); pg.locator("h1, body").first.click(position={"x": 2, "y": 2})
            wait_for_js(pg, "ST.files.some(f => f.ugrp === 'Blur name')", 5000)
            pg.locator(f"{HEAD} .tg-name").first.dblclick(); pg.keyboard.press("Control+a"); pg.keyboard.press("Delete"); pg.keyboard.press("Enter")
            wait_for_js(pg, "ST.files.some(f => /^Group \\d+$/.test(f.ugrp || ''))", 5000)
        step("blur saves; an empty name goes back to the default «Group N»", blur_empty)
        def kbd():
            n = pg.locator(f"{HEAD} .tg-name").first; n.focus(); assert n.get_attribute("aria-label").startswith("Rename group"), n.get_attribute("aria-label")
            pg.keyboard.press("F2"); wait_for_js(pg, "!!document.querySelector('.tg-edit')", 3000); pg.keyboard.press("Escape")
            pg.locator(f"{HEAD} .tg-name").first.focus(); pg.keyboard.press("Enter"); wait_for_js(pg, "!!document.querySelector('.tg-edit')", 3000)
            pg.keyboard.press("Control+a"); pg.keyboard.type("Kbd"); pg.keyboard.press("Enter"); wait_for_js(pg, "ST.files.some(f => f.ugrp === 'Kbd')", 5000)
        step("F2 and Enter on the focused name start the rename", kbd)
        def none():
            i = len(heads()) - 1; assert heads()[i] == "Ungrouped" or "Samples" in heads(), heads()
            pg.evaluate("GROUPS.set([], '')")
            pg.evaluate("ST.files.forEach(f => { if (f.ugrp === 'Samples') f.ugrp = undefined; }); renderFiles()")
            wait_for_js(pg, f"document.querySelectorAll('{HEAD}').length === 2 && !document.querySelector('{HEAD}[data-ug=\"Samples\"]')", 5000)
            pg.locator(f"{HEAD}[data-ug=''] .tg-name").dblclick(); pg.keyboard.press("Control+a"); pg.keyboard.type("Loose"); pg.keyboard.press("Enter")
            wait_for_js(pg, "E.ugNone === 'Loose'", 5000)
            assert pg.evaluate(NAMES)[-1] == "Loose" and not any(f.get("ugrp") == "Loose" for f in pg.evaluate("ST.files")), "a group was made"
        step("«Ungrouped» takes a label of its own, no group is made", none)
        def plus():
            b = pg.locator("#colsplus"); assert b.get_attribute("aria-label") == "Add column"
            b.focus(); pg.keyboard.press("Enter"); wait_for_js(pg, "!!document.getElementById('colsdlg')", 3000)
            pg.locator("#colsdlg [data-cc=batch]").check(); pg.locator("#colsclose").click()
            wait_for_js(pg, "!!document.querySelector('.ldtab input[data-xk=batch]')", 3000)
            try: wait_for_js(pg, "!document.getElementById('colsdlg') && document.activeElement && document.activeElement.id === 'colsplus'", 5000)      # the dialog's close event runs after the click
            except Exception: pass
            assert pg.evaluate("document.activeElement && document.activeElement.id") == "colsplus", "focus did not return to «+»"
            assert pg.evaluate("getComputedStyle(document.getElementById('colsplus').closest('th')).position") == "sticky"
        step("«+» opens the Columns chooser (keyboard), the picked column appears, «+» stays sticky", plus)
        def reload():
            wait_for_js(pg, "true"); pg.evaluate("uiSave && uiSave()"); saved(pg)
            pg.reload(); wait_for_js(pg, "typeof E !== 'undefined' && E.files && document.readyState === 'complete'", 120000)
            wait_for_js(pg, "E.files.some(f => f.ugrp === 'Kbd') || E.ugNone === 'Loose'", 30000)
            assert pg.evaluate("E.ugNone") == "Loose", pg.evaluate("E.ugNone")
        step("the names are kept after a reload of the session", reload)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
