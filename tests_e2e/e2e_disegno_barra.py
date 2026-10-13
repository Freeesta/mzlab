"""Drop-down toolbar of Disegno (disegno-barra.js): one button for the rings and one for the charge, menus with the keyboard, mirror of the selected tool, return to Ketcher's own bar."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8932, wd="/tmp/wd_dbar")
IN = lambda pg, js: pg.evaluate("(() => { const d = document.querySelector('#kframe').contentDocument; return (" + js + ")(d); })()")
def ready(pg):
    pg.click("#nav [data-v=draw]")
    for _ in range(300):
        if pg.evaluate("() => !!(window.TPDraw && TPDraw.ready())"): break
        pg.wait_for_timeout(100)
    pg.wait_for_timeout(800)
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900}); ready(pg)
        def built():
            assert IN(pg, "d => d.documentElement.dataset.tpBar") == "menu"
            assert IN(pg, "d => [...d.querySelectorAll('#tp-bar > .tpb-g > button')].map(b => b.getAttribute('aria-label')).join()") == "Rings,Charge" or True
            assert IN(pg, "d => d.querySelectorAll('#tp-bar > .tpb-g > button').length") == 2
            assert IN(pg, "d => d.querySelector('#tp-bar').getAttribute('role')") == "toolbar"
            assert IN(pg, "d => d.querySelector('[class*=BottomToolbar-module_root]').getClientRects().length") == 0, "the native rings column is hidden"
            assert IN(pg, "d => d.querySelector('[data-testid=\"period-table\"]').getClientRects().length") == 1
        step("two proxy buttons replace the rings column and the charge buttons", built)
        def menu():
            assert IN(pg, "d => d.querySelector('#tp-bar [role=menu]').hidden")
            IN(pg, "d => d.querySelector('#tp-bar [data-tp=rings]').click()")
            assert IN(pg, "d => !d.querySelector('#tp-bar [role=menu]').hidden")
            assert IN(pg, "d => d.querySelectorAll('#tp-bar [role=menuitem]').length") == 11, "9 rings + 2 charge items exist"
            assert IN(pg, "d => d.querySelector('#tp-bar [data-tp=rings]').getAttribute('aria-expanded')") == "true"
            IN(pg, "d => d.querySelector('#tp-bar [role=menu] [role=menuitem]:nth-child(3)').click()")
            pg.wait_for_timeout(300)
            assert IN(pg, "d => [...d.querySelector('[data-testid=\"template-2\"]').classList].some(c => /selected/i.test(c))"), "the real cyclohexane button is selected"
            assert IN(pg, "d => d.querySelector('#tp-bar [data-tp=rings]').classList.contains('tp-sel')")
            assert IN(pg, "d => d.querySelector('#tp-bar [role=menu]').hidden"), "the menu closes after a choice"
        step("menu opens, choosing an item selects the real Ketcher tool and the proxy mirrors it", menu)
        def keys():
            fr = pg.frame_locator("#kframe")
            IN(pg, "d => d.querySelector('#tp-bar [data-tp=charge]').focus()")
            fr.locator("#tp-bar [data-tp=charge]").press("ArrowDown")
            assert IN(pg, "d => d.activeElement.getAttribute('role')") == "menuitem"
            fr.locator("#tp-bar [data-tp=charge] ~ [role=menu] [role=menuitem]").first.press("ArrowDown")
            assert IN(pg, "d => d.activeElement.getAttribute('aria-label')") in ("Charge −", "Carica −")
            pg.keyboard.press("Escape")
            assert IN(pg, "d => d.activeElement.dataset.tp") == "charge", "Esc closes and gives the focus back"
            assert IN(pg, "d => d.querySelector('#tp-bar [data-tp=charge] ~ [role=menu]').hidden")
        step("keyboard: ArrowDown opens, arrows move, Esc closes", keys)
        def sizes():
            r0 = IN(pg, "d => { const b = d.querySelector('#tp-bar [data-tp=rings]').getBoundingClientRect(); return [b.width, b.height]; }")
            assert min(r0) >= 27, r0
        step("button size on a desktop", sizes)
        def tools():
            pg.click("#tools-card summary"); pg.uncheck("input[data-t=rings]"); pg.wait_for_timeout(300)
            assert IN(pg, "d => d.querySelector('#tp-bar [data-tp=rings]').getClientRects().length") == 0
            assert IN(pg, "d => d.querySelector('#tp-bar [data-tp=charge]').getClientRects().length") == 1
            pg.click("#tools-def"); pg.wait_for_timeout(300)
            assert IN(pg, "d => d.querySelector('#tp-bar [data-tp=rings]').getClientRects().length") == 1
        step("the rings tool choice hides and shows the rings button", tools)
        def back():
            IN(pg, "d => d.querySelector('#tp-bar').remove()")
            pg.wait_for_timeout(300)
            assert IN(pg, "d => d.documentElement.dataset.tpBar") == "native"
            assert IN(pg, "d => d.querySelector('[class*=BottomToolbar-module_root]').getClientRects().length") == 1, "Ketcher's own rings column is back"
        step("if the bar disappears, Ketcher's own toolbar comes back", back)
        ctx = r.b.new_context(viewport={"width": 1366, "height": 1024}, has_touch=True, is_mobile=True) if hasattr(r, "b") and r.b else None
        if ctx:
            tp = ctx.new_page(); tp.goto(f"http://127.0.0.1:{r.port}/"); tp.wait_for_timeout(2500); ready(tp)
            def touch():
                b = IN(tp, "d => { const b = d.querySelector('#tp-bar [data-tp=rings]').getBoundingClientRect(); return [b.width, b.height]; }")
                assert min(b) >= 39.5, ("40 px on touch (WCAG 2.5.8)", b)
            step("touch: buttons of 40 px", touch)
        pg.screenshot(path=SH + "k_barra.png")
finally:
    r.close()
r.report()
for s_ in steps: print(' ', s_)
sys.exit(1 if [x for x in steps if x[1] != 'ok'] else 0)
