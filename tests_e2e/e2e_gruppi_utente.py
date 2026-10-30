"""User groups of the side file list: three ticked files, right click, «Group files» -> header «Group 1» at once; rename from the header (double click),
Shift+F10 opens the header menu, ungroup, undo, a reload restores the group; «Remove group from the session» takes its files away with one undo."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import re
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gruppi_utente.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
HEADS = "[...document.querySelectorAll('#flst .fgh')].map(g => g.querySelector('span:not(.chv)').textContent + '|' + g.querySelector('em').textContent)"
UG = "#flst .fgh[data-ug]"
r = Run(port=8961, wd="/tmp/wd_gruppi_utente")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg); drawer(pg)
        pg.wait_for_selector("#flst .fl .nm", timeout=10000)
        pg.evaluate("window.ask = async (_t, d) => window.__ans ?? d")
        def make():
            pg.evaluate("E.files.forEach(f => { f.vis = false; }); renderFileList()")
            pg.locator("#flst .fl .nm").first.click(button="right")
            assert not pg.locator("#ctx div", has_text=re.compile("^(Group files|Raggruppa i file)$")).count(), "«Group files» with fewer than two ticked files"
            pg.keyboard.press("Escape"); pg.evaluate("Q('#ctx').hidden = true")
            pg.evaluate("tabFiles('full').forEach(f => { f.vis = true; }); renderFileList()")
            pg.locator("#flst .fl .nm").first.click(button="right")
            pg.locator("#ctx div", has_text=re.compile("^(Group files|Raggruppa i file)$")).click()
            attendi(pg, f"!!Q('{UG}')", 5000)
            h = pg.evaluate(HEADS); assert any(re.match(r"(Group|Gruppo) 1\|3$", x) for x in h), h
            assert pg.locator(f"{UG} .chv").get_attribute("aria-label"), "the header toggle has no aria-label"
            assert pg.locator(f"{UG} + .grp-files .fl").count() == 3 and pg.locator(f"{UG} + .grp-files").is_visible()
        step("three ticked files, right click, «Group files»: header «Group 1» with 3 files", make)
        def fold():
            pg.locator(f"{UG} .chv").click(); attendi(pg, f"Q('{UG} + .grp-files').style.display === 'none'", 3000)
            pg.locator(f"{UG} .chv").focus(); pg.keyboard.press("Enter"); attendi(pg, f"Q('{UG} + .grp-files').style.display === ''", 3000)
        step("the group header folds and unfolds (click, keyboard)", fold)
        def rename():
            pg.evaluate("window.__ans = 'Blanks'"); pg.locator(f"{UG} span:not(.chv)").dblclick()
            attendi(pg, "E.files.filter(f => f.ugrp === 'Blanks').length === 3", 5000)
            assert pg.locator(f"{UG}").get_attribute("data-ug") == "Blanks" and pg.locator(f"{UG} + .grp-files").is_visible()
        step("double click on the header renames the group to «Blanks»", rename)
        def kbd():
            pg.locator(f"{UG} .chv").focus(); pg.keyboard.press("Shift+F10")
            attendi(pg, "!Q('#ctx').hidden", 3000)
            items = pg.evaluate("[...Q('#ctx').querySelectorAll('[role=menuitem]')].map(d => d.textContent)")
            assert len(items) == 3 and pg.evaluate("document.activeElement.closest('#ctx') !== null"), items
            pg.keyboard.press("Escape")
        step("Shift+F10 on the header opens its menu (Rename, Ungroup, Remove group) with the focus in it", kbd)
        def ungroup():
            pg.locator(f"{UG}").click(button="right"); pg.locator("#ctx div", has_text=re.compile("^(Ungroup|Sciogli il gruppo)$")).click()
            attendi(pg, f"!Q('{UG}') && !E.files.some(f => f.ugrp)", 5000)
            pg.evaluate("STORIA.annulla()")
            attendi(pg, "E.files.filter(f => f.ugrp === 'Blanks').length === 3", 5000)
        step("Ungroup dissolves the group; undo brings it back", ungroup)
        def reload():
            saved(pg); pg.reload(); attendi(pg, f"typeof E !== 'undefined' && !!Q('{UG}')", 90000); ready(pg)
            assert pg.evaluate("E.files.filter(f => f.ugrp === 'Blanks').length") == 3
            assert any(x.startswith("Blanks|3") for x in pg.evaluate(HEADS)), pg.evaluate(HEADS)
        step("a reload restores the group", reload)
        def remove():
            drawer(pg); pg.locator(f"{UG}").click(button="right"); pg.locator("#ctx div", has_text=re.compile("(Remove group|Rimuovi il gruppo)")).click()
            attendi(pg, "E.files.every(f => f.gone)", 5000)
            pg.evaluate("STORIA.annulla()"); attendi(pg, "E.files.filter(f => !f.gone).length === 3", 5000)
        step("«Remove group from the session» takes the files away with one undo", remove)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
