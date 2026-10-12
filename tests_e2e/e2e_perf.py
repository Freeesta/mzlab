"""C1: the ?perf meter. Absent without ?perf; with it: a box with api calls (with no Python timing: the engine of the site is Rust), panel draws and a «Copia» button."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_perf.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8893, wd="/tmp/wd93")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def off():
            assert pg.locator("#perfbox").count() == 0 and pg.evaluate("typeof window.PERF") == "undefined", "the meter must not exist without ?perf"
        step("no meter without ?perf", off)
        pg.goto(f"http://127.0.0.1:{r.port}/?perf=1"); pg.wait_for_timeout(800)
        def box():
            assert pg.locator("#perfbox").count() == 1 and pg.is_visible("#perfbox"), "box"
            assert pg.is_visible("#perf-copy") and pg.is_visible("#perf-clear")
        step("the box is there with ?perf", box)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_timeout(2500)
        def rows():
            kinds = pg.evaluate("[...new Set(PERF.rows.map(r => r.kind))]")
            assert "api" in kinds and "disegno" in kinds, kinds
            assert any(r["what"].startswith("explore") for r in pg.evaluate("PERF.rows")), "api/explore not measured"
            assert pg.evaluate("PERF.rows.some(r => r.kind === 'disegno' && /draw (chrom|xic|spec)/.test(r.what))"), "panel draws"
        step("api calls and panel draws are measured", rows)
        def nopython():
            assert pg.evaluate("fetch('api/perf').then(r => r.json())") == {"files": []}, "the engine of the site has no Python timings"
            assert not pg.evaluate("PERF.rows.some(r => r.kind === 'python')"), "Python timings shown"
        step("api/perf: the Rust engine reports no Python timings", nopython)
        def text():
            pg.wait_for_timeout(600)
            t = pg.inner_text("#perf-out"); assert "api" in t and "riassunto" in t and "disegno" in t, t[:400]
        step("the box shows rows and a summary", text)
        def copy():
            pg.click("#perf-copy"); pg.wait_for_timeout(300)
            assert pg.inner_text("#perf-copy") == "Copiato"
            pg.click("#perf-clear"); pg.wait_for_timeout(500)
            assert pg.evaluate("PERF.rows.length") <= 3
        step("«Copia» works and «Azzera» empties the rows", copy)
        def shortcut():
            pg.keyboard.press("Control+Alt+KeyP"); pg.wait_for_timeout(200)
            assert not pg.is_visible("#perfbox"); pg.keyboard.press("Control+Alt+KeyP"); pg.wait_for_timeout(200); assert pg.is_visible("#perfbox")
        step("Ctrl+Alt+P hides and shows the box", shortcut)
        pg.screenshot(path=SH + "perf.png")
finally:
    r.close(); r.report()
bad = [s for s in steps if s[1] != "ok"]
for n, st in steps: print(("FAIL " if st != "ok" else "ok   ") + n + ("" if st == "ok" else " -> " + st))
if bad: sys.exit(1)
