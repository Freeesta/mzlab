"""Prompt 2, block 2: header of the chromatogram in one row with the Parametri popover and chips, legend inside the graph (none with one file),
no icons in the tabs (the tooltip of a tab shows Q1 -> q2 -> Q3), TIP_DELAY, whole file row clickable, gentle zoom line on the axes."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_blocco2.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
r = Run(port=8878, wd="/tmp/wd78")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def header():
            h = pg.evaluate(f"(()=>{{const p={CH},hd=p.el.querySelector('.hd').getBoundingClientRect();return [hd.height,p.el.querySelector('.cpop').hidden,!!p.el.querySelector('[data-o=kind]'),p.el.querySelector('.ttl').offsetParent===null]}})()"); print(h)
            assert h[0] < 46 and h[1] and h[2] and h[3], h          # one row, popover closed, the kind menu is the title
            pg.click(".pnl.chrom [data-a=cpar]"); pg.wait_for_timeout(200)
            assert pg.evaluate(f"!{CH}.el.querySelector('.cpop').hidden")
            pg.fill(".pnl.chrom [data-o=mz0]", "150"); pg.press(".pnl.chrom [data-o=mz0]", "Tab"); pg.fill(".pnl.chrom [data-o=mz1]", "300"); pg.press(".pnl.chrom [data-o=mz1]", "Tab"); pg.wait_for_timeout(1500)
            on = pg.evaluate(f"[{CH}.el.querySelector('[data-a=cpar]').classList.contains('on'), !!{CH}.el.querySelector('[data-a=rngx]')]"); print(on); assert on == [True, True], "the Range button is accented and has its ×"
            pg.mouse.click(5, 300); pg.wait_for_timeout(200); assert pg.evaluate(f"{CH}.el.querySelector('.cpop').hidden"), "closes on a click outside"
            pg.evaluate(f"(()=>{{const p={CH};p.mz0=p.mz1=null;ctl(p);draw(p)}})()")
        step("2.1: one-row header, Range popover, accented button", header)
        def legend():
            pg.evaluate("E.files.forEach(f=>{f.vis=true});redrawAll()"); pg.wait_for_timeout(1200)
            g = pg.evaluate(f"(()=>{{const p={CH},l=p.leg.getBoundingClientRect(),c=p.cv.getBoundingClientRect();return [l.top>=c.top,l.bottom<=c.bottom,l.right<=c.right,[...p.leg.children].filter(x=>!x.classList.contains('cmp-grad')).length]}})()"); print(g)
            assert g[0] and g[1] and g[2] and g[3] == 3, g
            pg.evaluate("E.files.forEach((f,i)=>{f.vis=i===0});redrawAll()"); pg.wait_for_timeout(1200)
            assert pg.evaluate(f"{CH}.leg.children.length") == 0 and not pg.evaluate(f"{CH}.leg.offsetParent"), "one file: no legend"
            pg.evaluate("E.files.forEach(f=>{f.vis=true});redrawAll()"); pg.wait_for_timeout(1200)
        step("2.1/2.4: legend inside the graph, none with one file", legend)
        def below():
            pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            g = pg.evaluate(f"(()=>{{const p={SPEC},l=p.leg.getBoundingClientRect(),c=p.cv.getBoundingClientRect();return [l.bottom<=c.bottom+1,l.top>c.bottom-30,l.width>100,getComputedStyle(p.leg).position]}})()"); print(g)
            assert g[0] and g[1] and g[2] and g[3] == "absolute", g          # the scan line is on the row of the x axis title
        step("2.4: scan line on the row of the x axis title", below)
        def tabs():
            assert pg.evaluate("document.querySelectorAll('#dtabs .qi, #flst .qi').length") == 0, "no icons in tabs / file list"
            pg.evaluate("scrollTo(0, 0)"); hold(pg, 300)      # a scroll event would cancel the pending tooltip
            b = pg.locator("#dtabs [data-t=ms2]"); b.hover(); hold(pg, 900)
            assert pg.evaluate("document.querySelector('#qtip').hidden"), "not yet at 0.9 s"
            hold(pg, 800); t = pg.inner_text("#qtip"); print(t[:120].replace("\n", " | "))
            assert not pg.evaluate("document.querySelector('#qtip').hidden") and "Q1" in t and "Q3" in t, t
            assert pg.evaluate("TIP_DELAY") == 1250
        step("2.3/2.5: no icons, tab label = Q1 -> q2 -> Q3 after TIP_DELAY (1250 ms)", tabs)
        def rows():
            drawer(pg); pg.evaluate("E.cur=E.files.find(f=>f.kind==='full').k;renderFileList()")
            pg.locator("#flst .fl:not(.ghost) small").nth(2).click(); pg.wait_for_timeout(300)
            c = pg.evaluate("E.cur"); assert c == pg.evaluate("tabFiles('full')[2].k"), c
            box = pg.locator("#flst .fl:not(.ghost)").nth(1).bounding_box(); pg.mouse.click(box["x"] + box["width"] - 3, box["y"] + 3); pg.wait_for_timeout(300)
            assert pg.evaluate("E.cur") == pg.evaluate("tabFiles('full')[1].k")
        step("2.6: the whole row of a file selects it", rows)
        def axis():
            pg.evaluate(f"{CH}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            q = pg.evaluate(f"(()=>{{const p={CH},c=p.cv.getBoundingClientRect(),a=p._a;return {{l:c.left,t:c.top,x:c.left+a.X(10),yb:c.top+a.H-20,x2:c.left+a.X(13),H:a.H}}}})()")
            pg.mouse.move(q["x"], q["yb"]); pg.wait_for_timeout(200)
            assert pg.evaluate(f"!{CH}.el.querySelector('.zl').hidden && {CH}.el.querySelector('.zl').classList.contains('hint')"), "hint strip over the numbers"
            pg.mouse.down(); pg.mouse.move(q["x2"], q["yb"], steps=6)
            t = pg.evaluate(f"({CH}.el.querySelector('.zl').dataset.v)"); print(t); assert t.startswith("RT ") and "min" in t, t
            bw = pg.evaluate(f"getComputedStyle({CH}.el.querySelector('.zl')).height"); assert bw == "16px", bw
            pg.screenshot(path=SH + "b2_axis.png"); pg.mouse.up(); pg.wait_for_timeout(500)
            assert pg.evaluate(f"!!{CH}.zoom")
        step("2.2: thin line with band and values on the axis", axis)
        pg.screenshot(path=SH + "b2_full.png")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s_ in steps: print(" ", s_)
r.report()
