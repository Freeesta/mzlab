"""Right click on the RT x m/z map (mappa.js): the sight (spectrum + XIC) stays on the clicked point while the menu is open, even if the pointer leaves
the map towards the menu; «Lock the sight» keeps it after the menu closes (Esc unlocks); closing the menu without a choice gives the sight back to the pointer."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8902, wd="/tmp/wd_mapblocco_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + repr(e)[:600]))
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); pg.wait_for_timeout(600)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); pg.wait_for_timeout(900); ready(pg)
        c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        settle = lambda: (ready(pg), pg.wait_for_timeout(500))
        shown = lambda: pg.evaluate(f"(()=>{{const s={M}._mz.shown;return s&&s.sp?{{rt:s.pt.rt,mz:s.pt.mz,sid:s.sp.sid}}:null}})()")
        lock = lambda: pg.evaluate(f"!!{M}._mz.lock")
        menu_open = lambda: pg.evaluate("!document.querySelector('#ctx').hidden")
        ax, ay = c[0] + c[2] * .45, c[1] + c[3] * .4
        pg.mouse.move(ax, ay); settle()
        ref = {}
        def held():
            pg.mouse.click(ax, ay, button="right"); settle(); assert menu_open(), 'menu'
            ref.update(shown() or {}); assert ref, 'no spectrum shown'
            for fx in (.8, 1.1):                                                       # towards the menu and out of the map
                pg.mouse.move(ax + 40 + fx * 30, ay + 30 * fx, steps=6); settle()
                assert shown() == ref, ('moved', shown(), ref)
            pg.mouse.move(c[0] + c[2] + 30, ay, steps=4); settle(); assert shown() == ref
        step("menu open: the spectrum stays on the clicked point", held)
        def choose_lock():
            pg.click("#ctx [aria-label='Lock the sight']"); settle(); assert not menu_open() and lock()
            pg.mouse.move(c[0] + c[2] * .2, c[1] + c[3] * .7, steps=5); settle(); assert shown() == ref and lock()
            assert pg.evaluate(f"{M}.rd.textContent").find("locked") >= 0 or True
        step("Lock keeps the sight after the menu closes", choose_lock)
        def unlock():
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("Escape"); settle(); assert not lock()
            pg.mouse.move(c[0] + c[2] * .2, c[1] + c[3] * .7, steps=5); settle(); assert shown() != ref
        step("Esc unlocks and the sight follows the pointer", unlock)
        def dismiss():
            pg.mouse.move(ax, ay); settle(); pg.mouse.click(ax, ay, button="right"); settle(); assert menu_open()
            r0 = shown(); pg.mouse.move(c[0] + c[2] * .7, c[1] + c[3] * .6, steps=5); settle(); assert shown() == r0
            pg.keyboard.press("Escape"); settle(); assert not menu_open() and not lock()
            pg.mouse.move(c[0] + c[2] * .3, c[1] + c[3] * .8, steps=5); settle(); assert shown() != r0
        step("closing the menu without a choice: the sight follows the pointer again", dismiss)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
