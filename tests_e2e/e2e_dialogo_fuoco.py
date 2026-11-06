"""Focus after a dialog closes: a window opened from the «⋯» menu of a chart header (the menu hides itself, so its item cannot take the focus back)
gives the focus to the menu's button, never to the body; with Escape and with the window's own close button. The floating calculator of the Draw view
gives it back to its toolbar button. Found by tools/fuzz_ui.py. Synthetic data."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
D = synth("dlgfuoco")
r = Run(port=free_port(), wd="/tmp/wd_dlgfuoco")
FOCUS = "(() => { const a = document.activeElement; return a === document.body ? 'body' : (a.getAttribute('aria-label') || a.id || a.tagName) })()"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1024, "height": 768})
        load(pg, [D / "B_FullMass-t0.mzML"], 4000, 1)
        more = pg.locator("#dpanels .hmb:visible").first
        def xic_esc():
            more.click(); pg.locator("#dpanels .hmore [data-a=xic]:visible").first.click(); attendi(pg, "document.querySelector('#xicdlg').open")
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('#xicdlg').open")
            try: attendi(pg, f"{FOCUS} === 'Altri strumenti del grafico'")
            except Exception: raise AssertionError(pg.evaluate("document.activeElement.outerHTML.slice(0, 160)"))
        step("XIC from the ⋯ menu, Esc: the focus goes back to the ⋯ button", xic_esc)
        def xic_cancel():
            more.click(); pg.locator("#dpanels .hmore [data-a=xic]:visible").first.click(); attendi(pg, "document.querySelector('#xicdlg').open")
            pg.click("#xic-no"); attendi(pg, "!document.querySelector('#xicdlg').open")
            try: attendi(pg, f"{FOCUS} === 'Altri strumenti del grafico'")
            except Exception: raise AssertionError(pg.evaluate("document.activeElement.outerHTML.slice(0, 160)"))
        step("XIC from the ⋯ menu, Cancel: the same", xic_cancel)
        def shortcuts():
            more.click(); pg.locator("#dpanels .hmore [data-a=log]:visible").first.focus(); pg.keyboard.press("Shift+?"); attendi(pg, "document.querySelector('#scorc') && document.querySelector('#scorc').open")
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('#scorc').open")
            attendi(pg, f"{FOCUS} !== 'body'")
        step("the shortcuts panel opened from a ⋯ menu item: Esc does not leave the focus on the body", shortcuts)
        def plain():
            pg.locator("#np-xic").focus() if pg.locator("#np-xic").is_visible() else None
            pg.evaluate("document.activeElement.blur()"); pg.evaluate("openXic(null)"); attendi(pg, "document.querySelector('#xicdlg').open")
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('#xicdlg').open")
        step("a window opened with nothing focused still closes with Esc", plain)
        def calc():
            pg.click("#nav [data-v=draw]")
            for how in ("Escape", "#calcx"):
                pg.click("#np-calc2"); attendi(pg, "!Q('#calcdlg').hidden && document.activeElement.id === 'calcin'")
                pg.keyboard.press(how) if how == "Escape" else pg.click(how); attendi(pg, "Q('#calcdlg').hidden")
                assert pg.evaluate("document.activeElement.id") == "np-calc2", (how, pg.evaluate(FOCUS))
        step("the floating calculator (Draw view), Esc and ×: the focus goes back to its toolbar button", calc)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
