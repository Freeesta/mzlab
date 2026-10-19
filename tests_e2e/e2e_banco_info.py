"""H3c: the information bar of the high-resolution bench: header of the scan, list of scans, file and instrument, composite spectrum, and the tabs that open the
functions that already exist. Synthetic Orbitrap DDA file; the infusion MSn file of the private data repository (skipped without it). Nothing in the low-resolution world."""
import sys, os, glob; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from dati_percorsi import percorso  # noqa: E402  (tools/ is on sys.path through lib)
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_banco_info.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrinfo")
r = Run(port=8984, wd="/tmp/wd_hrinfo")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1500})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        tab = lambda t: (pg.evaluate(f"BARRA.setTab('{t}')"), pg.wait_for_timeout(900))
        body = lambda: pg.inner_text("#hri-body")
        def visible():
            assert pg.locator("#stbar [data-cmd=scan], #stbar [data-cmd=form], #stbar [data-cmd=idn]").count() == 3 and not pg.is_visible("#hrinfo"), "closed until a tool is chosen"
            tab("hdr")                                       # an information tool of the toolbar brings the panel up
            assert pg.is_visible("#hrinfo") and pg.evaluate("BARRA.curTab") == "hdr"
            assert pg.evaluate("getComputedStyle(document.querySelector('#dfiles')).position") == "sticky", "pinned, the panel stays on screen while the page scrolls"
        step("information tools of the toolbar (six), in the pinned panel", visible)
        def hdr():
            pg.evaluate("setActive(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'))")
            box = pg.locator(".pnl.chrom canvas").first.bounding_box(); pg.mouse.click(box["x"] + box["width"] * 0.45, box["y"] + box["height"] * 0.5); pg.wait_for_timeout(1500)
            tab("hdr"); pg.wait_for_function("document.querySelector('#hri-body').innerText.includes('ms level')", timeout=15000)
            t = body(); assert "Filtro" in t and "FTMS" in t and "Tutti i parametri" in t, t[:300]
            rt0 = pg.evaluate("[...document.querySelectorAll('#hri-body tr')].find(r=>r.innerText.startsWith('RT'))?.innerText")
            pg.click("#hrbar [data-hb=next]"); pg.wait_for_timeout(1500); pg.wait_for_function("1")
            rt1 = pg.evaluate("[...document.querySelectorAll('#hri-body tr')].find(r=>r.innerText.startsWith('RT'))?.innerText")
            assert rt0 and rt1 and rt0 != rt1, (rt0, rt1)
        step("tab 1: the header of the scan under the cursor, with every parameter of the file; it follows the next scan", hdr)
        def lst():
            tab("lst"); pg.wait_for_selector("#hri-list .r", timeout=15000)
            t = pg.inner_text("#hri-body").split("\n")[0]; n = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full')._a.sr[0].x.length"); assert str(n) in t, (t, n)
            c0 = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').cur")
            pg.evaluate("document.querySelector('#hri-list').scrollTop=2000"); pg.wait_for_timeout(300)
            pg.locator("#hri-list .r").nth(2).click(); pg.wait_for_timeout(800)
            c1 = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full').cur"); assert c1 != c0, (c0, c1)
            pg.click("#hri-list .hd span[data-c=tic]"); pg.wait_for_timeout(300)
            a = pg.evaluate("[...document.querySelectorAll('#hri-list .r')].slice(0,4).map(r=>+r.children[3].innerText)"); assert a == sorted(a), a
        step("tab 2: the list of the scans (all of them, scrolling), a row moves the cell, a column sorts", lst)
        def fil():
            tab("fil"); t = body(); assert "Orbitrap Exploris 120" in t and "Scansioni per tipo" in t and "Status log" in t, t[:300]
        step("tab 3: file and instrument as the mzML says them", fil)
        def launchers():
            tab("cmp"); pg.wait_for_selector("#cx-mz", timeout=10000); pg.click("#cx-exp"); pg.wait_for_selector("#cmp-mz", timeout=10000); pg.evaluate("document.querySelector('#bigdlg').close()")
            tab("iso"); pg.fill("#hri-f", "C13H24N4O3S"); pg.fill("#hri-r", "35000"); pg.click("#hri-go"); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.panels.some(p=>p.type==='spec'&&p.iso&&p.iso.formula==='C13H24N4O3S'&&p.iso.R===35000)")
            tab("idn"); assert "Identifica tutte" in body()
        step("composition, isotopes and libraries open the functions that already exist", launchers)
        def persist():
            assert pg.evaluate("JSON.parse(localStorage.getItem('mzlab.banco.info')).tab") == "idn"
            h = pg.evaluate("document.querySelector('#hrinfo').offsetHeight"); assert h >= 160
        step("the tab and the height are remembered", persist)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
r = Run(port=8985, wd="/tmp/wd_lrinfo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "B_FullMass-t0.mzML")]); pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=60000); pg.click("text=Carica dati"); ready(pg)
        step("low-resolution world: no information bar", lambda: (_ for _ in ()).throw(AssertionError("visible")) if pg.is_visible("#hrinfo") else None)
    r.close()
except Exception as e:
    steps.append(("lr", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
HRMS = percorso("dati/HRMS")    # None without the private data repository: the block is skipped
MSN = next(iter(glob.glob(f"{HRMS}/*/*direct-infusion_MSn.mzML")), None) if HRMS else None
if MSN:
    r = Run(port=8986, wd="/tmp/wd_msninfo")
    try:
        with sync_playwright() as p:
            pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1500})
            load(pg, [MSN], 8000, 1)
            def msn():
                pg.wait_for_timeout(2500); pg.evaluate("BARRA.setTab('fil')"); pg.wait_for_timeout(1500)
                t = pg.inner_text("#hri-body"); assert "MS8" in t or "MS8:" in t.replace(" ", "") or "ms8" in t.lower(), t[:400]
                assert "317 > 261 > 244" in t, t[-600:]
            step("infusion MSn: the file tab lists the scan types up to MS8, one per path", msn)
        r.close()
    except Exception as e:
        steps.append(("msn", "FAIL " + str(e)[:300]))
        try: r.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
