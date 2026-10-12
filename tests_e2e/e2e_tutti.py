"""«Mostra tutti gli esperimenti insieme» (gear, off by default, low-resolution world): Full Scan, MS2 and MRM blocks on the same page, one experiment type per panel;
off again, only the tab's panels. Off by default the tabs work as they always did."""
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
        def off():
            assert pg.evaluate("UIP.tog") is False
            v = set(pg.evaluate(VIS)); assert v == {"full"}, v
        step("off by default: only the panels of the tab", off)
        def gear():
            pg.click("#np-set"); pg.wait_for_selector("#uip-tog", timeout=5000)
            assert not pg.is_checked("#uip-tog")
            pg.check("#uip-tog"); pg.wait_for_function("new Set(E.panels.map(p=>p.tab)).size>=3", timeout=30000); pg.wait_for_timeout(1500)
            pg.click("#np-set"); pg.wait_for_selector("#uipset", state="detached", timeout=5000)
        step("the gear checkbox adds the MS2 and MRM blocks", gear)
        def together():
            v = pg.evaluate(VIS); assert {"full", "ms2", "mrm"} <= set(v), v
            ys = pg.evaluate("(()=>{const m={};E.panels.forEach(p=>{(m[p.tab]=m[p.tab]||[]).push([p.y,p.y+p.h])});return Object.fromEntries(Object.entries(m).map(([t,a])=>[t,[Math.min(...a.map(x=>x[0])),Math.max(...a.map(x=>x[1]))]]))})()")
            order = sorted(ys, key=lambda t: ys[t][0])
            for a, b in zip(order, order[1:]): assert ys[a][1] <= ys[b][0] + 1, (a, b, ys)          # blocks one under the other, never overlapping
            n = pg.evaluate("document.querySelectorAll('#flst .fl:not(.ghost) input[data-k]').length"); assert n == 3, n          # one list with the three files, all live
            assert pg.evaluate("localStorage.getItem('qqq.prefs')").count('"tog":true') == 1
        step("three blocks one under the other; one file list with every file", together)
        def rule():
            for pl in ("full", "ms2", "mrm"):
                assert pg.evaluate(f"E.panels.filter(p=>p.tab==='{pl}'&&p._a&&p._a.sr).every(p=>p._a.sr.every(s=>E.files[s.k].kind==='{pl}'))"), pl      # a panel draws one experiment type only
        step("a panel holds only the files of its experiment type", rule)
        def back():
            pg.evaluate("setTogether(false)"); pg.wait_for_timeout(800)
            v = set(pg.evaluate(VIS)); assert v == {"full"}, v
            pg.evaluate("setTogether(true)"); pg.wait_for_timeout(800)
            assert {"full", "ms2", "mrm"} <= set(pg.evaluate(VIS))
            pg.evaluate("setTogether(false)"); pg.wait_for_timeout(500)
        step("off again: the tab's panels only; on again: all of them", back)
        def tabs():
            pg.click("#dtabs [data-t=ms2]"); pg.wait_for_timeout(900)
            assert "ms2" in pg.evaluate(VIS) and "full" not in pg.evaluate(VIS)
        step("the tabs work as they always did", tabs)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
