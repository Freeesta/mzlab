"""The example files published with the site: one button (inside box 1, under the drop zone) offers a CHOICE (Full Scan / MS2 / MRM / HRMS); only the chosen set is downloaded, on the click, with neutral names (the unknown pollutant is never named) and each set opens."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8883, wd="/tmp/wd83")
def in_order(o, names):
    """the first paint has every row; each later paint keeps the order of `names` (a row goes when its file is open)"""
    return bool(o) and o[0] == "|".join(names) and all([names.index(x) for x in l.split("|")] == sorted(names.index(x) for x in l.split("|")) for l in o)
steps = []
try:
    with sync_playwright() as p:
        pg = r.page(p)
        reqs = []; pg.on("request", lambda q: reqs.append(q.url) if "/esempi/" in q.url else None)
        pg.wait_for_timeout(1500); assert not reqs, ("nothing is downloaded before the click", reqs)
        assert pg.evaluate("(()=>{const b=document.querySelector('#demobtn').getBoundingClientRect(),d=document.querySelector('#drop').getBoundingClientRect(),c=document.querySelector('#start').getBoundingClientRect();return b.top>=d.bottom&&b.bottom<=c.bottom&&b.left>=c.left})()"), "button on the landing, under the drop zone"
        pg.click("#demobtn"); pg.wait_for_timeout(300); assert pg.is_visible("#demochoice") and pg.locator("#demochoice [data-demo]").count() == 4 and not reqs, "the button only opens the choice"
        # LOAD-UX: the rows of the whole set are in the side list at once, in the order of the set, and keep it until each file is open; one system of progress (no panel under the button, no card bar)
        pg.evaluate("""() => { window.__ord = []; window.__bars = 0; new MutationObserver(() => { const l = [...document.querySelectorAll('#fl-up [data-up]')].map(x => x.dataset.up); if (l.length) window.__ord.push(l.join('|'));
            window.__bars += document.querySelectorAll('.avanza, .loadbar').length; }).observe(document.querySelector('#flst'), { childList: true, subtree: true }); }""")
        pg.evaluate("""() => { window.__t = {}; document.addEventListener('click', e => { if (e.target.closest('[data-demo=full]')) window.__t.c = performance.now(); }, true);
            new MutationObserver(() => { if (!window.__t.r && document.querySelector('#fl-up [data-up]')) window.__t.r = performance.now(); }).observe(document.querySelector('#flst'), { childList: true, subtree: true }); }""")
        pg.click("#demochoice [data-demo=full]")
        pg.wait_for_function("document.querySelectorAll('#fl-up [data-up]').length===5", timeout=5000)
        assert pg.evaluate("[...document.querySelectorAll('#fl-up [data-up]')].map(x => x.dataset.up)") == [f"FullScan_t{t}.mzML" for t in (0, 5, 10, 15, 30)]
        pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===5", timeout=30000); pg.wait_for_timeout(500)
        o = pg.evaluate("[...new Set(window.__ord)]"); assert in_order(o, [f"FullScan_t{t}.mzML" for t in (0, 5, 10, 15, 30)]), o
        dt = pg.evaluate("window.__t.r - window.__t.c"); print("   click to the list of rows: %.0f ms" % dt); assert 0 <= dt < 1000, dt
        assert pg.evaluate("window.__bars") == 0, "never a second bar (panel of the download, bar of the card)"
        t = pg.inner_text("#flist"); assert t.count("Full Scan") >= 1 and "FullScan_t30" in t, t
        tm = pg.evaluate("[...document.querySelectorAll('#flist [data-k=time]')].map(x=>+x.value)"); assert sorted(tm) == [0, 5, 10, 15, 30], tm
        assert pg.locator("#demo a").count() == 0, "no download links any more"
        pg.wait_for_function("document.querySelectorAll('#demo .avanza').length===0", timeout=3000)          # a clean load leaves no «5 of 5 done» bar behind
        assert "flufenacet" not in pg.inner_text("body").lower() and "flufenacet" not in (pg.get_attribute("#demobtn", "title") or "").lower()
        pg.click("text=Carica dati"); ready(pg)
        assert pg.evaluate("tabFiles('full').length") == 5 and pg.evaluate("E.panels.some(p=>p.type==='spec'&&p._a)")
        assert "flufenacet" not in pg.inner_text("body").lower(), "the compound is never named after the files are open"
        assert all("FullScan" in u for u in reqs), ("only the Full Scan set was downloaded", reqs)
        steps.append(("choice Full Scan: 5 files with times 0/5/10/15/30, only that set downloaded, no links, button in box 1, no compound name, data open", "ok"))
        # the rows of the example files: the download is their first part; «Annulla» (one for everything) stops them and nothing is opened
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_selector("#demobtn")
        # the examples come from the service worker, out of reach of page.route: the second MRM file is held by a wrapper of fetch until «Annulla» aborts it
        pg.evaluate("""() => { const f0 = window.fetch; let n = 0; window.fetch = (u, o) => String(u).includes("esempi/MRM") && ++n === 2
            ? new Promise((_, no) => o.signal.addEventListener("abort", () => no(new DOMException("aborted", "AbortError")))) : f0(u, o); }""")
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=mrm]")
        pg.wait_for_function("document.querySelectorAll('#fl-up [data-up]').length===4", timeout=5000)
        assert pg.evaluate("[...document.querySelectorAll('#fl-up [data-up]')].map(x => x.dataset.up)") == [f"MRM_std_{c}ppm.mzML" for c in ("0.6", "2.4", "7.2", "18")]
        pg.wait_for_function("document.querySelector('#fl-up [data-up] [role=progressbar][aria-valuenow=\"40\"]')", timeout=20000)      # the first file is downloaded (40 % of its bar)
        assert pg.locator("#demo .avanza").count() == 0 and pg.locator(".loadbar").count() == 0
        pg.evaluate("document.querySelector('#fl-up [data-up-all]').click()")      # «Annulla» of the side list, for the whole loading
        pg.wait_for_function("document.querySelectorAll('#flist li[data-cvend=cancel]').length>=3", timeout=10000)
        assert pg.locator("#flist [data-k=conc]").count() == 0, "nothing is loaded after «Annulla»"
        steps.append(("the rows of the example files exist at once and in order, the download is part of the row, «Annulla» stops them and nothing is opened", "ok"))
        # a drop of three files: three rows at once
        pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_selector("#demobtn")
        three = [str(ROOT / "site" / "static" / "esempi" / f"FullScan_t{t}.mzML") for t in (0, 5, 10)]
        pg.evaluate("""() => { window.__ord = []; new MutationObserver(() => { const l = [...document.querySelectorAll('#fl-up [data-up]')].map(x => x.dataset.up); if (l.length) window.__ord.push(l.join('|')); })
            .observe(document.querySelector('#flst'), { childList: true, subtree: true }); }""")
        pg.set_input_files("#pick", three)
        pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===3", timeout=30000)
        o = pg.evaluate("[...new Set(window.__ord)]"); assert in_order(o, [f"FullScan_t{t}.mzML" for t in (0, 5, 10)]), o      # the three rows at once, in the order of the drop; a row goes when its file is open
        steps.append(("drop of three files: three rows at once, in the order of the drop", "ok"))
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
