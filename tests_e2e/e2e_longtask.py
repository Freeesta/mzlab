"""Space (play) and the held arrow walk the scans at the same pace and cost the same: Space on a focused button does not also click it, and the main thread is not blocked longer by one than by the other."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
LONG = """() => { window.__lt = 0; try { new PerformanceObserver(l => l.getEntries().forEach(e => { window.__lt += e.duration; })).observe({type: 'longtask'}); } catch (_) { window.__lt = -1; } }"""
r = Run(port=8861, wd="/tmp/wd61")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        c = pg.evaluate("(rt)=>{const p=E.panels.find(p=>p.type==='chrom'&&p.tab==='full'),r=p.cv.getBoundingClientRect();return {px:r.left+p._a.X(rt),py:r.top+r.height/2}}", 14.0)
        pg.mouse.click(c["px"], c["py"]); pg.wait_for_timeout(900)
        cur = lambda: pg.evaluate("E.active && E.active.cur")
        def no_click():
            lock = lambda: pg.evaluate("!!E.panels.find(p=>p.type==='spec'&&p.link!=null).lock")
            before = lock()
            pg.evaluate("document.querySelector('.pnl.spec [data-a=lock]').focus()")
            pg.keyboard.press(" "); pg.wait_for_timeout(300); pg.keyboard.press(" "); pg.wait_for_timeout(300)
            assert lock() == before, "Space on a focused button must not click it"
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))"); pg.evaluate("document.activeElement.blur()")
        step("Space on a focused button only plays", no_click)
        def cost():
            pg.evaluate(LONG)
            if pg.evaluate("window.__lt") == -1: return                       # no longtask support in this browser
            pg.evaluate("window.__lt = 0"); a0 = cur(); pg.keyboard.press(" "); pg.wait_for_timeout(1500); pg.keyboard.press(" "); pg.wait_for_timeout(200)
            space, moved_s = pg.evaluate("window.__lt"), abs(cur() - a0)
            pg.evaluate("window.__lt = 0"); a1 = cur(); pg.keyboard.down("ArrowRight"); pg.wait_for_timeout(1500); pg.keyboard.up("ArrowRight"); pg.wait_for_timeout(200)
            arrow, moved_a = pg.evaluate("window.__lt"), abs(cur() - a1)
            assert moved_s > 0 and moved_a > 0, (moved_s, moved_a)
            assert space <= arrow + 400, f"Space blocks the main thread more than the arrow: {space:.0f} ms vs {arrow:.0f} ms"
        step("Space costs no more main-thread time than the held arrow", cost)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
