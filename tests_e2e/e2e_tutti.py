"""Single view (low-resolution world): Full Scan, MS2 and MRM blocks on the same page from the start, one experiment type per panel; no tabs, no overview button,
no switch; a click on a file of another group makes its type the current one and goes to its block."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tutti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
VIS = "E.panels.filter(p=>p.el&&p.el.style.display!=='none'&&p.el.offsetParent).map(p=>p.tab)"
r = Run(port=8990, wd="/tmp/wd_tutti")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_FullMass-t15"), mz("B_MS2-t15"), mz("B_MRM-t0")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=3", timeout=30000)
        pg.click("text=Carica dati"); ready(pg)
        def together():
            pg.wait_for_function("new Set(E.panels.map(p=>p.tab)).size>=3", timeout=30000); pg.wait_for_timeout(1500)
            v = pg.evaluate(VIS); assert {"full", "ms2", "mrm"} <= set(v), v
            ys = pg.evaluate("(()=>{const m={};E.panels.forEach(p=>{(m[p.tab]=m[p.tab]||[]).push([p.y,p.y+p.h])});return Object.fromEntries(Object.entries(m).map(([t,a])=>[t,[Math.min(...a.map(x=>x[0])),Math.max(...a.map(x=>x[1]))]]))})()")
            order = sorted(ys, key=lambda t: ys[t][0])
            for a, b in zip(order, order[1:]): assert ys[a][1] <= ys[b][0] + 1, (a, b, ys)          # blocks one under the other, never overlapping
            n = pg.evaluate("document.querySelectorAll('#flst .fl input[data-k]').length"); assert n == 3, n          # one list with the three files, all live
            assert pg.locator("#dtabs, #ovbtn, #uip-tog").count() == 0 and pg.evaluate("'tog' in UIP") is False, "no tabs, no overview button, no switch"
            assert pg.evaluate("STATO.verifica()") == [], pg.evaluate("STATO.verifica()")
        step("three blocks one under the other from the start; one file list with every file", together)
        def rule():
            for pl in ("full", "ms2", "mrm"):
                assert pg.evaluate(f"E.panels.filter(p=>p.tab==='{pl}'&&p._a&&p._a.sr).every(p=>p._a.sr.every(s=>E.files[s.k].kind==='{pl}'))"), pl      # a panel draws one experiment type only
        step("a panel holds only the files of its experiment type", rule)
        def tools():
            assert pg.is_visible("#np-mrm") and pg.is_visible("#np-map") and pg.is_visible("#np-chrom"), "buttons of every type present"
        step("the toolbar offers the buttons of every experiment type that has files", tools)
        def goto():
            open_kind(pg, "mrm"); pg.wait_for_timeout(3500)
            assert pg.evaluate("E.tab") == "mrm"
            top = pg.evaluate("E.panels.find(p=>p.tab==='mrm').el.getBoundingClientRect().top"); assert -5 < top < 600, top      # the page went to the MRM block
            open_kind(pg, "ms2"); pg.wait_for_timeout(3500)
            assert pg.evaluate("E.tab") == "ms2" and "full" in pg.evaluate(VIS)
        step("a click on a file of another group makes its type the current one and goes to its block", goto)
        def empty_area():
            for kind in ("full", "ms2", "mrm"):
                for k in pg.evaluate("E.files.filter(f=>f.vis).map(f=>f.k)"):
                    pg.locator(f"#flst .fl input[data-k='{k}']").uncheck()
                pg.wait_for_timeout(500)
                assert pg.evaluate("E.panels.length") == 0, "every file hidden: no panel left"
                open_kind(pg, kind); pg.wait_for_timeout(2500)
                assert pg.evaluate(f"E.panels.some(p=>p.tab==='{kind}'&&(p.type==='chrom'||p.type==='mrm'))"), f"{kind}: the TIC appears"
        step("empty work area: a click on a file of any type opens its TIC", empty_area)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
