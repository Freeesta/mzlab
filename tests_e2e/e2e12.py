# e2e12: experiment type column, sidebar, no Esporta/Integrazioni buttons, integration icons, MS2 chromatogram, PNG without cursor, Metodo layout, italic m/z
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
def flst_own(pg):
    """text of the file list without the greyed rows (and heading) of the other tabs"""
    return pg.evaluate("[...document.querySelectorAll('#flst .fgh:not(.sep), #flst .fl:not(.ghost)')].map(e=>e.innerText).join('\\n')")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e12.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
r = Run(port=8822, wd="/tmp/wd12")
F = FILES + ["B_FullMass-t10"]      # a sixth file of the B series (the old extra file was not in the data repository)
with sync_playwright() as p:
    pg = r.page(p)
    pg.set_input_files("#pick", [mz(f) for f in F]); pg.wait_for_timeout(2500)
    pg.screenshot(path=SH + "120_start.png")
    def column():
        rows = pg.evaluate("ST.files.map(f=>[f.name,f.kind])")
        d = {n: (k,) for n, k in rows}
        assert d["B_MRM-t0.mzML"][0] == "mrm" and d["B_MS2-t15.mzML"][0] == "ms2" and d["B_FullMass-t0.mzML"] == ("full",), d
        assert "Esperimento" in pg.inner_text("#flist") and "MS2 (Product Ion)" in pg.inner_text("#flist") and "MRM" in pg.inner_text("#flist") and "Full Scan" in pg.inner_text("#flist") and "EMS" not in pg.inner_text("#flist")
    step("start screen detects the experiment from the content", column)
    pg.click("text=Carica dati"); ready(pg)
    pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()")
    pg.screenshot(path=SH + "121_data.png")
    def side():
        t = flst_own(pg); assert "scan MS1" not in t and "RT 0.5" not in t and "Doppio clic" not in pg.inner_text("#dfiles"), t
        assert "FULL SCAN" in t.upper() and "MS2" not in t and "MRM" not in t and "EMS" not in t, t
        d = pg.inner_text("#dtabs"); assert "Full Scan" in d and "MS2 (Product Ion)" in d and "MRM" in d and "Tempi ed esperimenti" in d, d
        assert pg.locator("#flst .fgh").count() >= 1 and pg.locator("#flst .tag").count() == 0 and pg.locator('#flst .hq').count() == 0
        assert pg.locator("#np-export, #np-ints, #kindinfo").count() == 0
        assert pg.locator("#credits").count() == 0 and pg.locator('.pnl.chrom [data-o=norm]').count() == 0
        fw = pg.evaluate("document.querySelector('.pnl.chrom [data-o=mz0]').offsetWidth"); assert fw < 90, fw
    step("sidebar slim, no EMS, no Esporta/Integrazioni/Rilevato", side)
    def integ():
        c = pg.locator('.pnl.chrom').first
        c.locator('[data-a=iauto]').click(); box = c.locator("canvas").bounding_box()
        x = pg.evaluate("E.panels[0]._a.X(14.33)"); pg.mouse.click(box["x"] + x, box["y"] + 120); pg.wait_for_timeout(500)
        assert pg.is_visible("#askdlg") and "non si integra" in pg.inner_text("#asktxt") and pg.evaluate("E.panels[0].ints.length") == 0
        pg.click("#askno"); c.locator('[data-a=iauto]').click()
        # zoom tool on the chromatogram
        c.locator('[data-a=izoom]').click()
        pg.mouse.move(box["x"] + pg.evaluate("E.panels[0]._a.X(13)"), box["y"] + 100); pg.mouse.down(); pg.mouse.move(box["x"] + pg.evaluate("E.panels[0]._a.X(16)"), box["y"] + 100, steps=5); pg.mouse.up(); pg.wait_for_timeout(500)
        z = pg.evaluate("E.panels[0].zoom"); assert z and abs(z[0] - 13) < 0.1 and abs(z[1] - 16) < 0.1, z
        assert c.locator('[data-a=fit]').is_enabled(); c.locator('[data-a=fit]').click(); c.locator('[data-a=izoom]').click(); assert c.locator('[data-a=fit]').is_disabled()
        # XIC panel with a chosen window, then auto + manual integration on a chosen file
        pg.click("#np-xic"); pg.fill("#xic-mz", "364"); pg.click("#xic-go"); ready(pg)
        xi = pg.evaluate("E.panels.findIndex(p=>p.type==='xic')"); xp = pg.locator('.pnl.xic').first
        assert pg.evaluate(f"E.panels[{xi}].traces[0].w") == 0.5
        xp.locator('[data-a=iauto]').click(); assert xp.locator('[data-a=intf]').is_visible()
        xbox = xp.locator("canvas").bounding_box(); xx = pg.evaluate(f"E.panels[{xi}]._a.X(14.33)")
        xp.locator('[data-a=intf]').select_option("all"); pg.mouse.click(xbox["x"] + xx, xbox["y"] + 120); pg.wait_for_timeout(600)
        n = pg.evaluate(f"[E.panels[{xi}].ints.length, E.panels[{xi}]._a.sr.length]"); assert n[0] == n[1] and n[0] > 1, n
        pg.screenshot(path=SH + "122_integr.png")
        pg.evaluate(f"E.panels[{xi}].ints=[]"); xp.locator('[data-a=intf]').select_option(index=2)
        pg.mouse.click(xbox["x"] + xx, xbox["y"] + 120); pg.wait_for_timeout(600)
        assert pg.evaluate(f"E.panels[{xi}].ints.length") == 1
        xp.locator('[data-a=iauto]').click(); xp.locator('[data-a=iman]').click()
        pg.mouse.move(xbox["x"] + pg.evaluate(f"E.panels[{xi}]._a.X(5)"), xbox["y"] + 100); pg.mouse.down(); pg.mouse.move(xbox["x"] + pg.evaluate(f"E.panels[{xi}]._a.X(6)"), xbox["y"] + 100, steps=5); pg.mouse.up(); pg.wait_for_timeout(500)
        assert pg.evaluate(f"E.panels[{xi}].ints.length") == 2 and xp.locator('[data-a=itab]').is_visible()
        xp.locator('[data-a=iman]').click()
        # header: only the m/z range; quick file chooser; RT readout bottom right; clear integrations
        assert xp.locator('[data-o=xlo]').input_value() == "363.8" and xp.locator('[data-o=xhi]').input_value() == "364.8" and xp.locator('#xic-q, [data-o=tol]').count() == 0
        xp.locator('[data-o=xlo]').fill("363,9"); xp.locator('[data-o=xlo]').press("Enter"); xp.locator('[data-o=xhi]').fill("364.3"); xp.locator('[data-o=xhi]').press("Enter"); pg.wait_for_timeout(1500)
        tw = pg.evaluate(f"[E.panels[{xi}].traces[0].mz, E.panels[{xi}].traces[0].w]"); assert abs(tw[0] - 364.1) < 0.011 and abs(tw[1] - 0.2) < 0.011, tw
        nv = pg.evaluate("tabFiles().filter(f=>f.vis).length")
        xp.locator('.fcount').click(); pg.wait_for_timeout(200); assert pg.locator("#fpop input").count() == pg.evaluate("tabFiles().length")
        pg.locator("#fpop input").first.uncheck(); pg.wait_for_timeout(1200); assert pg.evaluate("tabFiles().filter(f=>f.vis).length") == nv - 1
        pg.locator("#fpop .fn").nth(2).click(); pg.wait_for_timeout(1200); assert pg.evaluate("tabFiles().filter(f=>f.vis).length") == 1
        pg.locator("#fpop [data-all='1']").click(); pg.wait_for_timeout(800); pg.mouse.click(5, 5)
        assert not xp.locator(".rd").is_visible(), "the line under the graph is gone (the info is in the mouse box)"
        assert xp.locator('[data-a=iclr]').is_visible(); xp.locator('[data-a=iclr]').click(); pg.wait_for_timeout(400)
        assert pg.evaluate("E.panels.find(p=>p.type==='xic').ints.length") == 0 and not xp.locator('[data-a=iclr]').is_visible()
    step("zoom tool, TIC refuses, XIC window da-a, integration of a chosen file", integ)
    def ms2():
        pg.click("#dtabs [data-t=ms2]"); ready(pg)
        assert pg.locator("#expbar").count() == 0
        ch = pg.evaluate("tabPanels().filter(p=>p.type==='chrom').map(p=>[p.prec,p._a.sr[0].x.length])")
        assert len(ch) == 1 and ch[0][1] > 0, ch      # one pair of graphs (since 7/10); the precursor is chosen in the list
        assert flst_own(pg).upper().count("MS2 (PRODUCT ION)") == 1 and "MRM" not in flst_own(pg)
        pg.evaluate("(()=>{const p=E.panels.find(p=>p.tab==='ms2'&&p.type==='chrom');p._parOpen=true;ctl(p)})()")      # the precursor lives in the Parametri popover
        sel = pg.locator('.pnl.chrom [data-o=prec]:visible').first; assert sel.count() == 1
        pg.screenshot(path=SH + "123_ms2.png")
    step("MS2 chromatogram: choose the precursor", ms2)
    def png():
        pg.click("#dtabs [data-t=full]"); pg.evaluate("E.files.forEach(f=>f.vis=true);redrawAll()"); pg.wait_for_timeout(1200)
        assert pg.evaluate("E.panels[0].cur") is not None
        with pg.expect_download() as d: dlmenu(pg, '.pnl.chrom', 'png')
        pg.wait_for_timeout(500); print("PNG", d.value.suggested_filename); d.value.save_as("/tmp/chrom.png")
        with pg.expect_download() as d2: dlmenu(pg, '.pnl.spec', 'png')
        d2.value.save_as("/tmp/spec.png")
        assert pg.evaluate("E.panels[0]._exp") in (False, None)
    step("PNG downloads", png)
    def method():
        pg.evaluate("E.cur=E.files.findIndex(f=>f.kind==='mrm');E.browse=true;renderFileList();renderNav()")
        pg.set_input_files("input[type=file][accept*='.dam']", []) if False else None
        pg.click("#np-method"); ready(pg); t = pg.inner_text("#bigdlg"); assert "MRM" in t and "transizione" in t.lower(), t[:300]
        pg.screenshot(path=SH + "124_method_mrm.png"); pg.click("#bigx")
        pg.evaluate("E.cur=E.files.findIndex(f=>f.kind==='ms2');renderFileList();renderNav()"); pg.click("#np-method"); ready(pg)
        assert "ioni prodotto" in pg.inner_text("#bigdlg"); pg.screenshot(path=SH + "125_method_ms2.png"); pg.click("#bigx")
    step("Metodo: MRM and MS2 explained", method)
    def ital():
        n = pg.evaluate("[...document.querySelectorAll('#v-data *')].filter(e=>e.children.length==0&&e.tagName!=='I'&&e.tagName!=='OPTION'&&e.tagName!=='SCRIPT'&&/m\\/z/.test(e.textContent)).map(e=>e.outerHTML.slice(0,100))")
        assert not n, n
    step("m/z always italic in the page", ital)
    pg.wait_for_timeout(500)
    r.errs = [e for e in r.errs if "openchemlib" not in str(e) and "404" not in str(e)]
    r.close()
for s in steps: print(" ", s)
print(r.errs[:6])
