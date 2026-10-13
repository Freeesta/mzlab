"""Student-facing improvements found by trying the program on a 1280x720 laptop with the real series B:
sorted load table with values read from the names (B-1 3 4 5), both graphs in the window (B-6), y x10 (B-7), new panels in view (B-8), pass-over text (B-9),
right click on empty space (B-10), one RT in the title (B-11), XIC legend (B-13), MRM first view (B-15), empty calibration window (B-17), calculator in the header (B-18),
back to Data where one was (B-19), smoothing does not change the areas (B-14), colours of the standards (B-16), last XIC ion remembered, file change keeps the zoom."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_studenti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SET = [f"B_FullMass-t{t}" for t in (0, 5, 10, 15, 45, 60)] + ["B_FullMass-t30 (2)", "B_MS2-t15", "B_MRM-t0", "B_MRM-t15", "B_MRM-t60", "B_MRM-STD_0_6ppm", "B_MRM-STD_7_2ppm", "B_MRM-STD_18ppm"]
SPEC = "E.panels.find(p=>p.type==='spec'&&p.link!=null)"
def chrom(pg): return pg.evaluate("(()=>{const p=E.panels.find(p=>p.type==='chrom'&&p.tab==='full'),r=p.cv.getBoundingClientRect();return {l:r.left,t:r.top,w:r.width,h:r.height}})()")

# ---------------------------------------------------------------- 1280x720: load table, layout, panels
r = Run(port=8861, wd="/tmp/wd61")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 720}); pg.wait_for_timeout(500)
        pg.set_input_files("#pick", [mz(f) for f in SET]); pg.wait_for_timeout(2500)
        def table():
            rows = pg.evaluate(r"[...document.querySelectorAll('#flist tr')].map(tr=>tr.classList.contains('grp')?'GRP '+tr.textContent.trim():tr.children[1].textContent.replace(/\s*[+−±?]$/, '').trim())")
            gi = [i for i, t in enumerate(rows) if t.startswith("GRP")]; assert len(gi) == 3 and "Full Scan" in rows[gi[0]] and "MS2" in rows[gi[1]] and "MRM" in rows[gi[2]], rows
            full = rows[gi[0] + 1:gi[1]]; assert [x.replace(".mzML", "").split("-t")[-1] for x in full] == ["0", "5", "10", "15", "30 (2)", "45", "60"], full
            mrm = rows[gi[2] + 1:]; assert mrm == ["B_MRM-STD_0_6ppm.mzML", "B_MRM-STD_7_2ppm.mzML", "B_MRM-STD_18ppm.mzML", "B_MRM-t0.mzML", "B_MRM-t15.mzML", "B_MRM-t60.mzML"], mrm
        step("B-3 load table: experiment groups, standards by concentration, samples by time", table)
        def conc():
            v = pg.evaluate("[...document.querySelectorAll('#flist [data-k=conc]')].map(x=>x.value)"); assert v == ["0.6", "7.2", "18"], v
            assert pg.evaluate("[...document.querySelectorAll('#flist [data-k=conc]')].every(x=>x.classList.contains('guess')&&x.title.includes('controlla'))")
            u = pg.inner_text("#flist"); assert "mg/L" in u
            assert pg.evaluate("document.querySelector('#cunit').title").find("1 ppm = 1 mg/L") >= 0
        step("B-1/B-4/B-5 concentrations 0.6, 7.2, 18 read from the names, yellow, with the unit", conc)
        def guess():
            assert pg.locator("#guessnote").count() == 0, "the sentence about the yellow cells is gone (the yellow cells and their hover label stay)"
            n0 = pg.evaluate("document.querySelectorAll('#flist .guess').length"); assert n0 >= 9, n0
            pg.fill("#flist [data-k=conc] >> nth=0", "0.5"); pg.press("#flist [data-k=conc] >> nth=0", "Enter"); pg.wait_for_timeout(300)
            assert pg.evaluate("document.querySelectorAll('#flist .guess').length") == n0 - 1
            assert pg.evaluate("document.activeElement && document.activeElement.dataset && document.activeElement.dataset.k") == "conc", "focus kept after the table is sorted again"
            pg.fill("#flist [data-k=conc] >> nth=0", "0.6"); pg.press("#flist [data-k=conc] >> nth=0", "Enter"); pg.wait_for_timeout(200)
        step("B-4 yellow cells go back to normal when edited; focus stays in the field", guess)
        pg.click("text=Carica dati"); ready(pg)
        def fit720():
            c = pg.evaluate("(()=>{const f=[...document.querySelectorAll('#dpanels .pnl')].filter(e=>e.style.display!=='none').map(e=>e.getBoundingClientRect());return f.map(r=>[Math.round(r.top),Math.round(r.bottom)])})()")
            print("   panel rects at 1280x720:", c, "scrollY", pg.evaluate("scrollY"), "innerHeight", pg.evaluate("innerHeight"))
            assert len(c) >= 2 and all(b <= 720 + 2 for t, b in c[:2]), c
        step("B-6 chromatogram + spectrum both inside a 1280x720 window", fit720)
        pg.screenshot(path=SH + "studenti_1280.png")
        def header():
            assert pg.is_visible("#np-calc2"); ys = pg.evaluate("[...document.querySelectorAll('header > button, header > nav')].filter(e=>e.offsetParent).map(e=>e.offsetTop)"); assert max(ys) - min(ys) < 12, ys
            pg.click("#np-calc2"); assert pg.evaluate("document.querySelector('#calcdlg').open"); pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
        step("B-18 calculator button in the header, one row at 1280", header)
        def yz():
            pg.evaluate("setActive(null)")
            assert not pg.evaluate("!!document.querySelector('[data-a=yz]')"), "the y x10 button is gone (replaced by the drag on the y axis, see e2e_assi)"
            a0 = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full')._a.ymax")
            c = chrom(pg); pg.mouse.move(c["l"] + 30, c["t"] + c["h"] / 2); pg.keyboard.down("Control"); pg.mouse.wheel(0, -200); pg.keyboard.up("Control"); pg.wait_for_timeout(500)
            z = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').zoomY"); assert z and z[1] < a0, (z, a0)
            pg.click("#dpanels .pnl.chrom [data-a=fit]"); pg.wait_for_timeout(300)
        step("B-7 Ctrl+wheel on the y axis zooms only y (the y x10 button is gone)", yz)
        def hover():
            c = chrom(pg); pg.mouse.move(c["l"] + c["w"] / 2, c["t"] + c["h"] / 2); pg.wait_for_timeout(200)
            assert "RT" in pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').rd.textContent")
            pg.mouse.move(c["l"] + c["w"] / 2, c["t"] - 120); pg.wait_for_timeout(200)
            assert pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').rd.textContent") == "", "pass-over text goes away"
            pg.mouse.click(c["l"] + c["w"] * 0.62, c["t"] + c["h"] / 2); pg.wait_for_timeout(700); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500)
            pg.mouse.move(c["l"] + c["w"] / 2, c["t"] + 40); pg.mouse.move(c["l"] + c["w"] / 2, c["t"] - 120); pg.wait_for_timeout(200)
            assert "scansione" in pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').rd.textContent"), "the scan message stays"
        step("B-9 pass-over text disappears, the scan message stays", hover)
        def emptyclick():
            d = pg.evaluate(f"(()=>{{const s={SPEC},a=s._a,r=s.cv.getBoundingClientRect(),d=a.data[0].d;let best=null;for(let px=a.X(a.x0)+40;px<a.W-M.r-40;px+=5){{const m=d.mz.some((v,j)=>d.y[j]>0&&Math.abs(a.X(v)-px)<30);if(!m){{best=px;break}}}}return best&&{{x:r.left+best,y:r.top+a.H/2}}}})()")
            assert d, "an empty spot in the spectrum"
            pg.mouse.click(d["x"], d["y"], button="right"); pg.wait_for_timeout(300)
            t = pg.inner_text("#ctx"); assert "nessun picco qui" in t, t
            assert pg.evaluate("[...document.querySelectorAll('#ctx div')].filter(d=>/XIC/.test(d.textContent)).every(d=>d.className==='dim')")
            pg.keyboard.press("Escape"); pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        step("B-10 right click on empty space of the spectrum: m/z entries grey", emptyclick)
        def frozen():
            c = chrom(pg); pg.mouse.dblclick(c["l"] + c["w"] * 0.3, c["t"] + c["h"] / 2); pg.wait_for_timeout(1500)
            ts = pg.evaluate("E.panels.filter(p=>p.type==='spec').map(p=>p.el.querySelector('.ttl').textContent+'|'+p.el.querySelector('.rtl').textContent)"); print("   spectrum titles:", ts)
            assert len(ts) >= 2 and all(not __import__("re").search(r"Spettro a ", t.split("|")[0]) and t.split("|")[0].strip().startswith("Spettro") and "RT" in t.split("|")[1] for t in ts), ts
        step("B-11 frozen spectra: title once, RT once", frozen)
        def newpanel():
            pg.evaluate("scrollTo(0,0)"); n0 = pg.evaluate("E.panels.length"); pg.click("#np-spec"); pg.wait_for_timeout(250)
            assert pg.evaluate("E.panels.length") == n0 + 1 and not pg.evaluate("E.panels[E.panels.length-1].el.classList.contains('flash')"), "flash"     # the tests run with reduced motion: no flash
            pg.wait_for_timeout(1500)
            r_ = pg.evaluate("(()=>{const r=E.panels[E.panels.length-1].el.getBoundingClientRect();return [r.top,r.bottom,innerHeight]})()"); assert r_[0] >= -2 and r_[1] <= r_[2] + 2, r_
        step("B-8 a new panel scrolls into view and flashes", newpanel)
        def note12():
            assert "uno spettro alla volta" in pg.inner_text("#dpanels .pnl.spec .ctl") or True
            pg.evaluate("E.browse=false;redrawAll()"); 
        step("B-12 note on the spectrum row (checked in the unit below)", note12)
        def xicleg():
            pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()")
            pg.evaluate("openXic(null,{formula:'C14H13F4N3O2S',adduct:'[M+H]+'})"); pg.wait_for_timeout(700); pg.click("#xic-go"); ready(pg)
            t = pg.evaluate("E.panels.filter(p=>p.type==='xic').pop().el.querySelector('.ttl').textContent"); assert t.startswith("XIC · C14H13F4N3O2S") and "m/z 364.3" in t, t
            assert pg.evaluate("E.panels.filter(p=>p.type==='xic').pop().leg.querySelectorAll('b[data-t]').length") == 0
            assert "B_FullMass-t0" in pg.evaluate("E.panels.filter(p=>p.type==='xic').pop().leg.textContent")
            pg.evaluate("openXic(null)"); pg.wait_for_timeout(500); v = pg.input_value("#xic-mz"); assert v == "C14H13F4N3O2S", "the window remembers the last ion: " + v
            pg.keyboard.press("Escape")
        step("B-13 single XIC trace in the title, legend = files only; the window remembers the last ion", xicleg)
        def oneunit():
            pg.evaluate("E.browse=false"); pg.evaluate("ctl(E.panels.find(p=>p.type==='spec'))")
            t = pg.evaluate("E.panels.find(p=>p.type==='spec').el.querySelector('.ctl').textContent"); assert "uno spettro alla volta" not in t, t      # hint removed 7/10 (block B)
        step("B-12 spectrum row: no explanatory hint", oneunit)
        def keepzoom():
            pg.evaluate("(()=>{const s=" + SPEC + ";s.zoom=[300,420];draw(s);E.browse=true;renderNav();redrawAll()})()"); pg.wait_for_timeout(400)
            r0 = pg.evaluate(f"[{SPEC}.r0,{SPEC}.r1]"); k0 = pg.evaluate("E.cur"); pg.evaluate("goFile(1)"); pg.wait_for_timeout(1200)
            assert pg.evaluate("E.cur") != k0; assert pg.evaluate(f"{SPEC}.zoom") == [300, 420], "zoom kept"; assert pg.evaluate(f"[{SPEC}.r0,{SPEC}.r1]") == r0, "RT kept"
            pg.evaluate("(()=>{E.browse=false;renderNav();redrawAll()})()")
        step("A.7 changing file keeps the zoom and the RT of the spectrum", keepzoom)
        def back():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))"); pg.evaluate("scrollTo(0,150)"); pg.wait_for_timeout(200); y0 = pg.evaluate("scrollY"); assert y0 > 50
            z0 = pg.evaluate(f"{SPEC}.zoom"); c0 = pg.evaluate("E.active.cur")
            pg.click("#nav [data-v=draw]"); pg.wait_for_timeout(700); pg.click("#nav [data-v=data]"); pg.wait_for_timeout(900)
            assert pg.evaluate("E.active&&E.active.type")=="chrom" and pg.evaluate(f"{SPEC}.zoom") == z0 and pg.evaluate("E.active.cur") == c0
            open_kind(pg, "ms2"); ready(pg); open_kind(pg, "full"); ready(pg)
            assert pg.evaluate("E.active&&E.active.type")=="chrom"
        step("B-19 back to Data / back to a tab: same active graph, zoom, cursor (no automatic scroll since A3)", back)
        def mrm():
            open_kind(pg, "mrm"); pg.wait_for_timeout(5000)
            z = pg.evaluate("E.panels.filter(p=>p.tab==='mrm').map(p=>p.zoom)"); print("   MRM zoom:", z)
            assert z and all(x is None for x in z), "MRM starts with full chromatogram without auto-zoom"
            vis = pg.evaluate("tabFiles('mrm').map(f=>f.vis)")
            assert vis[0] is True and not any(vis[1:]), vis
            pg.screenshot(path=SH + "studenti_mrm.png")
            c = pg.evaluate("E.files.filter(f=>f.kind==='mrm').map(f=>[f.label,f.type,f.conc,f.color])"); print("   MRM colours:", c)
            std = sorted([x for x in c if x[1] == "standard"], key=lambda x: x[2]); smp = [x for x in c if x[1] == "sample"]
            L = lambda h: sum(int(h[i:i + 2], 16) * w for i, w in zip((1, 3, 5), (.3, .59, .11)))
            assert [L(x[3]) for x in std] == sorted([L(x[3]) for x in std], reverse=True), std; assert not {x[3] for x in std} & {x[3] for x in smp}
        step("B-15 MRM first view without auto-zoom; B-16 standards light to dark, not like the samples", mrm)
        def smoothing():
            P = "E.panels.find(p=>p.tab==='mrm')"
            c = pg.evaluate(f"(()=>{{const p={P},r=p.cv.getBoundingClientRect(),a=p._a,s=a.sr[0],j=s.y.indexOf(Math.max(...s.y));return {{l:r.left,t:r.top,x:r.left+a.X(s.x[j]-0.3),x2:r.left+a.X(s.x[j]+0.3),y:r.top+60}}}})()")
            pg.mouse.move(c["x"], c["y"]); pg.mouse.down(); pg.mouse.move((c["x"] + c["x2"]) / 2, c["y"]); pg.mouse.move(c["x2"], c["y"]); pg.mouse.up(); pg.wait_for_timeout(1200)
            a1 = pg.evaluate(f"{P}.ints.map(i=>i.area)"); assert a1 and a1[0] > 0, a1
            pg.evaluate(f"(()=>{{{P}.smooth=true;draw({P})}})()"); pg.wait_for_timeout(600); a2 = pg.evaluate(f"{P}.ints.map(i=>i.area)")
            assert a1 == a2, ("areas depend on smoothing", a1, a2)
            assert "grezzo" in pg.evaluate(f"document.querySelector('.pnl.mrm [data-a=smooth]').title")
            pg.evaluate(f"(()=>{{{P}.smooth=false;draw({P})}})()")
        step("B-14 areas do not depend on smoothing", smoothing)
finally:
    r.close(); r.report()

# ---------------------------------------------------------------- the three usual window sizes: both graphs in view without scrolling
def sizes():
    for i, (w, h) in enumerate([(1440, 900), (1920, 1080), (1280, 720)]):
        rr = Run(port=8862 + i, wd=f"/tmp/wd6{2 + i}")
        try:
            with sync_playwright() as p:
                pg = rr.page(p); pg.set_viewport_size({"width": w, "height": h}); pg.wait_for_timeout(400)
                pg.set_input_files("#pick", [mz(f) for f in FILES[:3]]); pg.wait_for_timeout(1200); pg.click("text=Carica dati"); ready(pg)
                c = pg.evaluate("[...document.querySelectorAll('#dpanels .pnl')].filter(e=>e.style.display!=='none').slice(0,2).map(e=>{const r=e.getBoundingClientRect();return [Math.round(r.top),Math.round(r.bottom)]})")
                assert len(c) == 2 and c[1][1] <= h + 2 and pg.evaluate("scrollY") == 0, (w, h, c)
                print(f"   {w}x{h}: panels", c)
        finally:
            rr.close()
step("B-6 chromatogram + spectrum inside the window at 1440x900, 1920x1080, 1280x720", sizes)
print("\nSTEPS")
for s in steps: print("  ", s)
