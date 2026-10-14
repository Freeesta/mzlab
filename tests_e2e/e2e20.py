"""MS2/Full scan: live spectrum under the chromatogram (double click), frozen older ones, group move, arrows (MS2: nearest scan with data), reload."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e20.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8820, wd="/tmp/wd20")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in ["B_FullMass-t0", "B_FullMass-t15", "B_MS2-t15", "B_MS2-t45"]])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)      # opened file by file: the view starts on the first file of the list
        def cx(x):   # screen point of RT x on the first chromatogram of the current tab
            return pg.evaluate("""x=>{const p=E.panels.find(q=>q.tab===E.tab&&q.type==='chrom');p.el.scrollIntoView({block:'start'});const r=p.cv.getBoundingClientRect();return {px:r.left+p._a.X(x),py:r.top+r.height*0.4}}""", x)
        order = lambda: pg.evaluate("E.panels.filter(p=>p.tab===E.tab).sort((a,b)=>a.y-b.y).map(p=>p.type+(p.link?'*':'')+(p.type==='spec'?'@'+(+((p.r0+p.r1)/2).toFixed(1)):''))")
        def dbl(x):
            c = cx(x); pg.mouse.dblclick(c["px"], c["py"]); pg.wait_for_timeout(1200)
        def triple():
            for x in (6.0, 9.0, 14.3): dbl(x)
            o = order(); print(o)
            specs = [t for t in o if t.startswith("spec")]
            assert o[0] == "chrom" and len(specs) == 4, o              # chromatogram first, then 3 new + the initial one
            assert o[1].startswith("spec*") and abs(float(o[1].split("@")[1]) - 14.3) < 0.3, o       # newest right under, and live
            assert sum(1 for t in o if t.startswith("spec*")) == 1, o   # only one live
            ttl = pg.evaluate("E.panels.filter(p=>p.tab===E.tab&&p.type==='spec').sort((a,b)=>a.y-b.y).map(p=>p.el.querySelector('.fol').textContent)"); print(ttl)
            assert "pann. " in ttl[0] and all("pann. " not in t and "Fisso" in t for t in ttl[1:]), ttl
            rt = pg.evaluate("E.panels.filter(p=>p.tab===E.tab&&p.type==='spec').sort((a,b)=>a.y-b.y).map(p=>p.el.querySelector('.rtl').textContent)"); assert all("RT" in t and "min" in t for t in rt), rt      # the time is written once, next to the title
        step("full scan: 3 double clicks -> newest on top and live, older frozen with the RT next to the title", triple)
        def single():
            before = pg.evaluate("E.panels.filter(p=>p.type==='spec'&&p.tab===E.tab).map(p=>[p.id,p.r0])")
            c = cx(11.0); pg.mouse.click(c["px"], c["py"]); pg.wait_for_timeout(900)
            after = pg.evaluate("E.panels.filter(p=>p.type==='spec'&&p.tab===E.tab).map(p=>[p.id,p.r0])")
            ch = [a for a, b in zip(after, before) if a != b]; assert len(ch) == 1, (before, after)
            live = pg.evaluate("E.panels.find(p=>p.type==='spec'&&p.tab===E.tab&&p.link).id"); assert ch[0][0] == live
        step("single click moves only the live spectrum", single)
        def group():
            pg.click("#np-chrom")
            attendi(pg, "E.panels.filter(p=>p.tab===E.tab&&p.type==='chrom').length>=2")
            fermo(pg, "#dpanels .pnl.chrom .hd")
            names = lambda: pg.evaluate("(()=>{const ps=E.panels.filter(p=>p.tab===E.tab).sort((a,b)=>a.y-b.y);return ps.map(p=>p.type==='chrom'?'C'+p.id:p.type==='spec'?'s'+(p.src||p.link||'-')+'_'+p.id:'?')})()")
            n0 = names(); print(n0); assert n0[0].startswith("C") and n0[-1].startswith("C") and n0[0] != n0[-1], n0
            pg.evaluate("window.scrollTo({top:0,behavior:\"instant\"})"); attendi(pg, "window.scrollY === 0"); fermo(pg, "#dpanels .pnl.chrom .hd"); box = pg.locator("#dpanels .pnl.chrom .hd").first.bounding_box()
            dist = pg.evaluate("(()=>{const c=E.panels.filter(p=>p.tab===E.tab&&p.type==='chrom').sort((a,b)=>a.y-b.y);return c[1].y+c[1].h-c[0].y})()")
            vp = pg.viewport_size; pg.set_viewport_size({"width": vp["width"], "height": int(box["y"] + 8 + dist + 40)})      # the pointer must stay inside the viewport: Firefox drops the mouse events that fall outside it
            fermo(pg, "#dpanels .pnl.chrom .hd"); box = pg.locator("#dpanels .pnl.chrom .hd").first.bounding_box()
            pg.mouse.move(box["x"] + 8, box["y"] + 8); pg.mouse.down(); pg.mouse.move(box["x"] + 8, box["y"] + 8 + dist, steps=10); pg.mouse.up()
            pg.wait_for_function("function (id) { return E.panels.filter(p=>p.tab===E.tab).sort((a,b)=>a.y-b.y)[0].id === id; }", arg=int(n0[-1][1:]), timeout=30000)
            n1 = names(); print(n1)
            c1 = n0[0][1:]
            assert n1[0] == n0[-1] and n1[1] == "C" + c1 and all(x.startswith("s" + c1 + "_") for x in n1[2:]), n1      # the group moved as a block under the other chromatogram
            # a spectrum dragged alone moves alone
            pg.evaluate("window.scrollTo({top:0,behavior:\"instant\"})"); pg.wait_for_timeout(500); pg.locator("#dpanels .pnl.spec .hd").first.scroll_into_view_if_needed(); pg.wait_for_timeout(300); sp = pg.locator("#dpanels .pnl.spec .hd").first.bounding_box()
            pg.mouse.move(sp["x"] + 14, sp["y"] + 8); pg.mouse.down(); pg.mouse.move(sp["x"] + 14, max(5, sp["y"] - 400), steps=8); pg.mouse.up(); pg.wait_for_timeout(900)
            print(sp); n2 = names(); print(n2); assert len(n2) == len(n1) and n2 != n1
        step("dragging a chromatogram moves its spectra with it; a spectrum alone moves alone", group)
        def mv():                           # up/down arrows carry the children
            pg.evaluate("movePanel(E.panels.find(p=>p.type==='chrom'),1)"); pg.wait_for_timeout(300); o = order(); print(o)
            ys = pg.evaluate("(()=>{const c=E.panels.find(p=>p.tab===E.tab&&p.type==='chrom');return [c.y,...E.panels.filter(p=>p.src===c.id||p.link===c.id).map(p=>p.y)]})()"); print(ys)
        step("movePanel keeps the group", mv)
        def reload():
            o1 = order(); n = pg.evaluate("E.panels.length")
            pg.evaluate("uiSave(true)")         # the notebook is written in the background (two delays in a row): wait until it holds the panels as they are, then reload
            pg.wait_for_function("fetch('api/notebook').then(r => r.json()).then(j => JSON.stringify(((j.ui || {}).panels || []).map(p => [p.type, p.y])) === JSON.stringify(E.panels.map(p => [p.type, p.y])))", timeout=60000)
            pg.reload(); pg.wait_for_function(f"typeof E !== 'undefined' && E.panels.length === {n} && (E.pending || []).length === 0", timeout=60000); ready(pg)      # the files come back one by one, the panels with the last one
            o2 = order(); print(o1, o2); assert o1 == o2, (o1, o2)
        step("reload restores order, link and src", reload)
        # MS2 arrows (nearest scan with data): covered by e2e_scroll «MS2: arrows jump over empty scans»
        pg.screenshot(path=SH + "200_ms2.png", full_page=True)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
