"""H3b: the toolbar of the high-resolution bench (in the place of the tabs), the cells of every experiment on the page together, the file marks.
Synthetic Orbitrap DDA file; the real infusion MSn file and the TiO2 series of the private data repository (skipped without them). Low-resolution world untouched."""
import sys, os, glob; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_banco_barra.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrbarra")
r = Run(port=8978, wd="/tmp/wd_hrbarra")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def tabs_gone():
            assert pg.is_visible("#hrbar") and pg.locator("#dtabs").count() == 0, "the toolbar of the bench is the only bar"
            n = pg.locator("#hrbar button[data-hb]").count(); assert n >= 14, n
            assert not pg.is_visible("#g-add"), "the old «+ Cromatogramma» group is replaced by the menu «Cella»"
        step("toolbar instead of the tabs, with its buttons", tabs_gone)
        def no_badges():
            assert pg.locator("#flst .hrb").count() == 0 and pg.locator("#flst .fl").count() >= 1
        step("no marks (HR, DDA, profile) in the rows of the files", no_badges)
        def add_del():
            n0 = pg.evaluate("E.panels.length")
            pg.click("#hrbar [data-hb=add]"); pg.click("#ctx >> text=Cromatogramma"); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.panels.length") == n0 + 1 and pg.evaluate("E.active.type") == "chrom"
            pg.click("#hrbar [data-hb=del]"); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels.length") == n0
        step("«Cella» adds a chromatogram (active), the bin removes the active cell", add_del)
        def decimals():
            assert pg.locator("#hrbar [data-hb=dec]").count() == 0, "the decimals field left the bar"
            pg.evaluate("setActive(E.panels.find(p=>p.type==='spec'))"); drawer(pg, "disp"); pg.wait_for_selector("#dsp-0", timeout=5000)
            pg.select_option("#dsp-0", "3"); pg.wait_for_timeout(800)
            assert pg.evaluate("UIP.hrDec") == 3
            pg.select_option("#dsp-0", "4"); pg.wait_for_timeout(500)
        step("decimals of the m/z from the display options", decimals)
        def norm():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'))"); pg.click("#hrbar [data-hb=norm]"); pg.wait_for_timeout(1200)
            ym = pg.evaluate("E.panels.find(p=>p.type==='chrom')._a.ymax"); assert abs(ym - 108) < 0.5, ym
            pg.click("#hrbar [data-hb=norm]"); pg.wait_for_timeout(800)
            assert pg.evaluate("E.panels.find(p=>p.type==='chrom')._a.ymax") > 1000
        step("0-100 ⇄ absolute from the toolbar", norm)
        def steps_():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))")
            box = pg.locator(".pnl.chrom canvas").first.bounding_box(); pg.mouse.click(box["x"] + box["width"] * 0.4, box["y"] + box["height"] * 0.5); pg.wait_for_timeout(1200)
            c0 = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').cur")
            pg.click("#hrbar [data-hb=next]"); pg.wait_for_timeout(1200)
            c1 = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').cur"); assert c1 is not None and c0 is not None and c1 > c0, (c0, c1)
            pg.click("#hrbar [data-hb=prev]"); pg.wait_for_timeout(1200)
            assert pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').cur") < c1
        step("previous / next scan from the toolbar", steps_)
        def pin():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='spec'&&p.link!=null&&p.dda==null))"); pg.click("#hrbar [data-hb=pin]"); pg.wait_for_timeout(500)
            assert pg.evaluate("E.active.pin") is True
            pg.click("#hrbar [data-hb=pin]"); pg.wait_for_timeout(500); assert not pg.evaluate("E.active.pin")
        step("pin from the toolbar", pin)
        def copy_xls():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))")
            pg.evaluate("window.__clip=null;Object.defineProperty(navigator,'clipboard',{value:{writeText:t=>{window.__clip=t;return Promise.resolve()}},configurable:true})")
            pg.click("#hrbar [data-hb=copy]"); pg.click("#ctx >> text=Dati"); pg.wait_for_timeout(800)
            t = pg.evaluate("window.__clip"); assert t and "\t" in t and "RT" in t.split("\n")[0], (t or "")[:120]
            with pg.expect_download(timeout=15000) as d: pg.click("#hrbar [data-hb=xls]")
            assert d.value.suggested_filename.endswith(".xlsx"), d.value.suggested_filename
        step("copy the data as text; export to Excel", copy_xls)
        def cross():
            c = "E.panels.find(p=>p.type==='chrom')"
            key = pg.evaluate(f"BANCO.groupsOf(E.files[{c}.k??E.files.find(f=>f.kind==='full').k].k||E.files.find(f=>f.kind==='full').k).then(g=>g.find(x=>x.level===2).key)")
            pg.evaluate(f"BANCO.groupsOf(E.files.find(f=>f.kind==='full').k).then(g=>BANCO.setFilter({c}, '{key}', g))"); pg.wait_for_timeout(2500)
            v = pg.evaluate(f"(()=>{{const p={c};return {{tab:p.tab,flv:p.flv,vis:getComputedStyle(p.el).display!=='none',n:p._a&&p._a.sr?p._a.sr.length:-1,k:p._a&&p._a.sr&&p._a.sr[0]?E.files[p._a.sr[0].k].kind:null}}}})()")
            assert v["tab"] == "ms2" and v["flv"] == 2 and v["vis"] and v["n"] >= 1 and v["k"] == "ms2", v
        step("a filter of another experiment moves the cell to it: the cell stays on the page and draws the MS2 file", cross)
        def retype():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='ms2'))"); n = pg.evaluate("E.panels.length")
            pg.click("#hrbar [data-hb=type]"); pg.click("#ctx >> text=Mappa RT-m/z"); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.panels.length") == n and pg.evaluate("E.active.type") == "map", pg.evaluate("E.active&&E.active.type")
        step("change of type: chromatogram → map in the same place", retype)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
# low-resolution world: untouched
r = Run(port=8979, wd="/tmp/wd_lrbarra")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [D / "B_FullMass-t0.mzML"]); pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        def lr():
            assert not pg.is_visible("#hrbar") and pg.is_visible("#g-add")
        step("low-resolution world: the tabs and the old buttons are there, no toolbar of the bench", lr)
    r.close()
except Exception as e:
    steps.append(("lr", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
DATI = None
for c in (os.environ.get("MZLAB_DATI"), os.environ.get("QQQ_DATI"), "/home/user/mzlab-dati", "/home/user/QqQ-lab-dati"):
    if c and glob.glob(c + "/HRMS/*/*direct-infusion_MSn.mzML"): DATI = c; break
if DATI:
    MSN = glob.glob(DATI + "/HRMS/*/*direct-infusion_MSn.mzML")[0]
    TIO = [glob.glob(DATI + f"/HRMS/*/*_TiO2_t0{n}min.mzML")[0] for n in ("10", "20", "45")]
    r = Run(port=8980, wd="/tmp/wd_msnbarra")
    try:
        with sync_playwright() as p:
            pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
            load(pg, [MSN], 8000, 1)
            def msn():
                pg.wait_for_timeout(3000)
                tabs = pg.evaluate("E.panels.map(p=>p.type+':'+p.tab)")
                assert "chrom:infusion" in tabs and "spec:infusion" in tabs, tabs
                assert "MSn" in pg.inner_text("#flst") or "MS8" in pg.inner_text("#flst"), pg.inner_text("#flst")[:300]
            step("infusion MSn: one item, TIC and spectrum of its richest node, file marked MSn", msn)
        r.close()
    except Exception as e:
        steps.append(("msn", "FAIL " + str(e)[:300]))
        try: r.close()
        except Exception: pass
    r = Run(port=8981, wd="/tmp/wd_tio3")
    try:
        with sync_playwright() as p:
            pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
            pg.set_input_files("#pick", TIO); pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=3", timeout=180000); pg.click("text=Carica dati")
            pg.wait_for_function("E.files.length>=6", timeout=420000); pg.wait_for_timeout(4000)
            def three():
                drawer(pg, "file")                                          # high resolution: only the first file is open at the start, a click on the box opens each of the others
                for k in (2, 4): pg.click(f'#flst input[data-k="{k}"]'); pg.wait_for_timeout(600)
                pg.evaluate("E.browse=false;renderNav();redrawAll()"); pg.wait_for_timeout(3500)
                n = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full')._a.sr.length"); assert n == 3, n
            step("three TiO2 files together: the same trace of the three files overlaid in the chromatogram", three)
        r.close()
    except Exception as e:
        steps.append(("tio2", "FAIL " + str(e)[:300]))
        try: r.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
