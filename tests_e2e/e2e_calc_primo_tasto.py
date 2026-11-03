"""The calculator never closes or loses its keypad by itself: whatever key is pressed first (( ) + − × ÷ , =), the panel stays open, the keypad stays, the focus stays in the field and an error is shown in place."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
KEYS = ["(", ")", "+", "−", "×", "÷", ",", "="]
KEYPAD = {"−": "-", "×": "*", "÷": "/"}          # the data-k of the keypad buttons that use another character
r = Run(port=8893, wd="/tmp/wd93")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def stays(first):
            def check(via):
                pg.fill("#calcin", "")
                if via == "pad":
                    b = pg.locator(f'#calcpad [data-k="{first}"], #calcpad [data-k="{KEYPAD.get(first, first)}"]').first
                    if b.count(): b.click()
                    else: pg.click("#calcin"); pg.keyboard.type(first)
                else:
                    pg.click("#calcin"); pg.keyboard.type(first)
                frames(pg, 3)
                assert pg.is_visible("#calcdlg") and pg.is_visible("#calcin"), "the panel closed"
                assert pg.is_visible("#calcpad"), "the keypad vanished"
                assert pg.evaluate("document.activeElement && document.activeElement.id") == "calcin", "the focus left the field"
            check("pad"); check("kbd")
        # the loading screen: the calculator is the drop-down under its button
        attendi(pg, "document.querySelector('#np-calc2') && !document.querySelector('#start').hidden")
        pg.click("#np-calc2"); attendi(pg, "!document.querySelector('#calcdlg').hidden")
        for k in KEYS: step(f"drop-down: {k!r} first keeps the panel open", lambda k=k: stays(k))
        pg.click("#calcx")
        # the Dati view: the calculator is a panel of the side drawer
        pg.set_input_files("#pick", [mz("B_FullMass-t0")]); pg.click("text=Carica dati"); ready(pg)
        pg.click("#np-calc2"); attendi(pg, "window.BARRA.curTab === 'calc' && !document.querySelector('#sb-panel-calc').hidden")
        def stays_docked(first):
            def check():
                pg.fill("#calcin", ""); pg.click("#calcin"); pg.keyboard.type(first); frames(pg, 3)
                assert pg.evaluate("window.BARRA.curTab === 'calc' && window.BARRA.isOpen('calc')") and pg.is_visible("#calcin") and pg.is_visible("#calcpad"), "the panel closed or lost its keypad"
            check()
        for k in KEYS: step(f"Dati panel: {k!r} first keeps the panel open", lambda k=k: stays_docked(k))
        def err_in_place():
            pg.fill("#calcin", ""); pg.click("#calcin"); pg.keyboard.type(")"); frames(pg, 3)
            assert pg.inner_text("#calcsum").strip() != "", "the error is shown in place"
        step("the error text is shown under the field", err_in_place)
        def one_is_enough():
            pg.fill("#calcin", ""); pg.click("#calcin"); pg.keyboard.type("("); pg.keyboard.type("2+3)"); frames(pg, 3)
            assert pg.is_visible("#calcpad") and pg.inner_text("#calcres").strip() == "5"
        step("after a first parenthesis the calculation goes on", one_is_enough)
        def well():
            pg.fill("#calcin", ""); frames(pg, 3)
            assert pg.is_visible("#calcres"), "the result well is there while the field is empty"
        step("the result well is visible while the field is empty", well)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
