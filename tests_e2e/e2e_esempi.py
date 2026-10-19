"""The example files published with the site: one button (inside box 1, under the drop zone) offers a CHOICE (Full Scan / MS2 / MRM / HRMS); only the chosen set is downloaded, on the click, with neutral names (the unknown pollutant is never named) and each set opens."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8883, wd="/tmp/wd83")
steps = []
try:
    with sync_playwright() as p:
        pg = r.page(p)
        reqs = []; pg.on("request", lambda q: reqs.append(q.url) if "/esempi/" in q.url else None)
        pg.wait_for_timeout(1500); assert not reqs, ("nothing is downloaded before the click", reqs)
        pg.click("#demobtn"); pg.wait_for_timeout(300); assert pg.is_visible("#demochoice") and pg.locator("#demochoice [data-demo]").count() == 4 and not reqs, "the button only opens the choice"
        pg.click("#demochoice [data-demo=full]"); pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===5", timeout=30000); pg.wait_for_timeout(500)
        t = pg.inner_text("#flist"); assert t.count("Full Scan") >= 1 and "FullScan_t30" in t, t
        tm = pg.evaluate("[...document.querySelectorAll('#flist [data-k=time]')].map(x=>+x.value)"); assert sorted(tm) == [0, 5, 10, 15, 30], tm
        assert pg.locator("#demo a").count() == 0, "no download links any more"
        pg.wait_for_function("document.querySelectorAll('#demo .avanza').length===0", timeout=3000)          # a clean load leaves no «5 of 5 done» bar behind
        assert pg.evaluate("(()=>{const b=document.querySelector('#demobtn').getBoundingClientRect(),d=document.querySelector('#drop').getBoundingClientRect(),c=document.querySelector('#drop').closest('.card').getBoundingClientRect();return b.top>=d.bottom&&b.bottom<=c.bottom&&b.left>=c.left})()"), "button inside box 1, under the drop zone"
        assert "flufenacet" not in pg.inner_text("body").lower() and "flufenacet" not in (pg.get_attribute("#demobtn", "title") or "").lower()
        pg.click("text=Carica dati"); ready(pg)
        assert pg.evaluate("tabFiles('full').length") == 5 and pg.evaluate("E.panels.some(p=>p.type==='spec'&&p._a)")
        assert "flufenacet" not in pg.inner_text("body").lower(), "the compound is never named after the files are open"
        assert all("FullScan" in u for u in reqs), ("only the Full Scan set was downloaded", reqs)
        steps.append(("choice Full Scan: 5 files with times 0/5/10/15/30, only that set downloaded, no links, button in box 1, no compound name, data open", "ok"))
        # the shared progress panel (avanzamento.js): item «k di n», true percentage, sign of life when nothing arrives, «Annulla», summary; nothing is opened after a cancel
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_selector("#demobtn")
        # the examples come from the service worker, out of reach of page.route: the second MRM file is held by a wrapper of fetch until «Annulla» aborts it
        pg.evaluate("""() => { const f0 = window.fetch; let n = 0; window.fetch = (u, o) => String(u).includes("esempi/MRM") && ++n === 2
            ? new Promise((_, no) => o.signal.addEventListener("abort", () => no(new DOMException("aborted", "AbortError")))) : f0(u, o); }""")
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=mrm]")
        pg.wait_for_function("(document.querySelector('#demo .avanza [data-r=item]')||{}).textContent?.startsWith('2 di 4: ')", timeout=20000)
        assert pg.get_attribute("#demo .avanza [role=progressbar]", "aria-valuenow") == "25", pg.get_attribute("#demo .avanza [role=progressbar]", "aria-valuenow")
        pg.wait_for_function("document.querySelector('#demo .avanza [data-r=beat]').textContent.includes('Ancora al lavoro')", timeout=6000)
        assert "trascorsi" in pg.inner_text("#demo .avanza [data-r=time]")
        pg.click("#demo .avanza [data-r=cancel]")
        pg.wait_for_selector("#demo .avanza [data-r=sum]", timeout=10000)
        t = pg.inner_text("#demo .avanza [data-r=sum]"); assert "1 di 4 completati" in t and "1 annullati" in t, t
        assert pg.locator("#flist [data-k=conc]").count() == 0, "nothing is loaded after «Annulla»"
        pg.click("#demo .avanza [data-r=close]"); assert pg.locator("#demo .avanza").count() == 0
        steps.append(("the shared progress panel: item, percentage, sign of life, «Annulla» stops and the summary says it", "ok"))
        # MS2: two files, times 15 and 60, the product-ion type
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500); reqs.clear()
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=ms2]"); pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===2", timeout=30000); pg.wait_for_timeout(500)
        tm = pg.evaluate("[...document.querySelectorAll('#flist [data-k=time]')].map(x=>+x.value)"); assert sorted(tm) == [15, 60], tm
        t = pg.inner_text("#flist"); assert "MS2" in t and "MS2_t15" in t, t
        assert all("MS2" in u for u in reqs), reqs
        pg.click("text=Carica dati"); ready(pg); assert pg.evaluate("tabFiles('ms2').length") == 2 and "flufenacet" not in pg.inner_text("body").lower()
        steps.append(("choice MS2: 2 files with times 15/60, MS2 tab, only that set downloaded", "ok"))
        # MRM: four standards with the concentration read from the name
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500); reqs.clear()
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=mrm]"); pg.wait_for_function("document.querySelectorAll('#flist [data-k=conc]').length===4", timeout=30000); pg.wait_for_timeout(500)
        cc = pg.evaluate("[...document.querySelectorAll('#flist [data-k=conc]')].map(x=>+x.value)"); assert sorted(cc) == [0.6, 2.4, 7.2, 18], cc
        assert all("MRM" in u for u in reqs), reqs
        pg.click("text=Carica dati"); ready(pg); assert pg.evaluate("tabFiles('mrm').length") == 4 and "flufenacet" not in pg.inner_text("body").lower()
        steps.append(("choice MRM: 4 standards with concentrations 0.6/2.4/7.2/18, MRM tab, only that set downloaded", "ok"))
        # HRMS: 4 Orbitrap DDA files (Full Scan + MS2), opened in the high-resolution mode like files loaded by hand
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500); reqs.clear()
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=hrms]"); pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length>=4", timeout=60000); pg.wait_for_timeout(500)
        assert "HRMS_t000" in pg.inner_text("#flist") and all("HRMS_t" in u for u in reqs) and len(reqs) == 4, reqs
        pg.click("text=Carica dati"); ready(pg)
        fl = pg.evaluate("E.files.map(f=>[f.file,f.lv,!!(f.prof1&&f.prof1.hr),f.prof1&&f.prof1.dec,!!f.dda])")
        assert any(f[1] == 1 and f[2] and f[3] >= 4 for f in fl) and any(f[1] == 2 and f[4] for f in fl), fl
        assert pg.evaluate("tabFiles('full').length") >= 4 and pg.evaluate("tabFiles('ms2').length") >= 4, fl
        steps.append(("choice HRMS: 4 Orbitrap DDA files, only that set downloaded, opens in high resolution with Full Scan and MS2", "ok"))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
