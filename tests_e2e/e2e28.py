"""Adducts tab: one row per adduct (shift, m/z of the typed mass), ESI+/ESI- switch, rare ones behind More, LR rounded to the unit, HR pairs closed, no references; lists aligned with the Python ones, one rounding rule. No data files needed."""
# covers: mzlab/web/tavola-periodica.js, mzlab/web/tables.js, mzlab/web/elements.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from lib import *
from mzlab.chem.elements import ADDUCT_SHIFT, MASS, ELECTRON, round_half_up
from mzlab import ionfamily
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e28.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8828 + 100, wd="/tmp/wd28")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def reflists():
            pg.evaluate("document.querySelector('#refdlg').close()")
            pg.evaluate("LISTE.open({})"); pg.wait_for_timeout(1500)
            # two built-in lists (known contaminants, pesticides): one switch each, titles without the reference
            n = pg.evaluate("document.querySelectorAll('#lst-src-checks input[type=checkbox]').length"); assert n == 2, n
            t = pg.inner_text("#lst-src-checks"); assert "Known contaminants (Keller et al.)" in t and "Anal. Chim" not in t and "contaminants" in t.lower() and "pesticides" in t.lower(), t
            assert pg.locator("#lst-mode-seg").count() == 0 and pg.locator("#lst-view-seg").count() == 0, "no files: HR version, no mode switch, no tab bar (target screening is behind its flag)"
            assert pg.is_visible("#lst-q") and pg.is_visible("#lst-tol") and pg.is_visible("#lst-unit") and pg.locator("#lst-pol").count() == 0, "one search field, the shared tolerance (no polarity buttons)"
            assert pg.is_visible("#lst-count") and "matches" in pg.inner_text("#lst-count"), pg.inner_text("#lst-count")
            assert pg.locator("#lst-tbody tr").first.locator("td").count() == 9 and "." in pg.inner_text("#lst-tbody tr >> nth=0")
            # a simulated user list adds its own switch
            pg.evaluate("LISTE.addUserListFromCsv('prova.csv','mz,name,formula,polarity\\n100.1,Test,,1\\n')"); pg.wait_for_timeout(600)
            pg.evaluate("LISTE.open({})"); pg.wait_for_timeout(800)
            n = pg.evaluate("document.querySelectorAll('#lst-src-checks input[type=checkbox]').length"); assert n == 3, n
        step("Reference lists: one switch per list, the user list adds one", reflists)
        def add_list():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())")
            pg.evaluate("LISTE.open({})"); wait_for_js(pg, "!!document.querySelector('#lst-lists')")
            # what each list is: link with the DOI, count, date, and «Source unknown» for the pesticides
            pg.evaluate("document.querySelector('#lst-about').open=true")
            t = pg.inner_text("#lst-lists")
            assert "Known contaminants (Keller et al.)" in t and "ions from" in t and "2026-10-09" in t and "Source unknown" in t, t
            assert pg.locator("#lst-lists a[href='https://doi.org/10.1016/j.aca.2008.04.043']").count() >= 1
            # a CSV with a wrong row: its line number is shown, nothing is added until the valid rows are accepted
            csv = "mz,name,formula,polarity\n100.1,Good,,+\nbad,Bad row,,\n"
            pg.set_input_files("#lst-add-file", files=[{"name": "mine.csv", "mimeType": "text/csv", "buffer": csv.encode()}]); wait_for_js(pg, "!!document.querySelector('#lst-add-valid')")
            assert "Line 3" in pg.inner_text("#lst-add-err") and pg.locator("#lst-src-checks input[type=checkbox]").count() == 3
            pg.click("#lst-add-valid"); wait_for_js(pg, "document.querySelectorAll('#lst-src-checks input[type=checkbox]').length==4")
            n = pg.locator("#lst-src-checks input[type=checkbox]").count(); assert n == 4, n
            # rename, then remove
            pg.evaluate("document.querySelector('#lst-about').open=true")
            pg.click("[data-ren]"); pg.fill(".lst-ren", "Renamed list"); pg.keyboard.press("Enter"); wait_for_js(pg, "document.querySelector('#lst-src-checks').innerText.includes('Renamed list')")
            assert "Renamed list" in pg.inner_text("#lst-src-checks")
            pg.evaluate("document.querySelector('#lst-about').open=true")
            pg.locator(".lst-info:has-text(\"Renamed list\") [data-del-ul]").click(); wait_for_js(pg, "!document.querySelector('#lst-src-checks').innerText.includes('Renamed list')")
            assert "Renamed list" not in pg.inner_text("#lst-src-checks")
        step("Reference lists: add a CSV with a wrong row, rename, remove; provenance of the lists", add_list)
        def lists_lr():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())")
            pg.evaluate("window.__fake={gone:false,polarity:'positive',name:'x'}; E.files.push(window.__fake); HR.__ish=HR.isHr; HR.isHr=()=>false")
            try:
                pg.evaluate("LISTE.open({})"); pg.wait_for_selector("#lst-tbody tr", timeout=8000)
                assert pg.locator("#lst-tbody tr").first.locator("td").count() == 4 and pg.locator("#lst-table-wrap thead th:visible").count() == 4
                assert pg.locator("#lst-view-seg").count() == 0 and pg.is_visible("#lst-tol") and not pg.is_visible("#lst-cont-signs")
                assert pg.locator("#lst-unit button.on").inner_text() == "Da" and pg.input_value("#lst-tol") == "0.5", "unit resolution: Da, as in Adducts & losses"
                import re
                mzs = pg.evaluate("[...document.querySelectorAll('#lst-tbody tr:not(#lst-more) td:first-child')].map(t=>t.textContent)"); assert mzs and all(re.fullmatch(r"\d+\.\d{4}", m) for m in mzs), mzs[:5]      # exact m/z with 4 decimals in LR too
                pg.fill("#lst-q", "63,3"); wait_for_js(pg, "document.querySelector('#lst-tbody').textContent.includes('PEG')", 5000); t = pg.inner_text("#lst-tbody"); assert "63." in t and "PEG" in t, t[:200]      # comma decimals rank by m/z
                import json; prev = pg.evaluate("document.querySelector('#lst-tbody').textContent"); pg.fill("#lst-q", "peg"); wait_for_js(pg, "document.querySelector('#lst-tbody').textContent !== " + json.dumps(prev), 5000); t2 = pg.inner_text("#lst-tbody"); assert "PEG" in t2, t2[:200]      # text filters by name
                assert pg.locator("#lst-pol-seg").count() == 0 and pg.locator("#lst-txt").count() == 0, "one field, no polarity toggle"
                pols = set(pg.evaluate("[...document.querySelectorAll('#lst-tbody tr td:last-child .pol')].map(e=>e.textContent.trim())")); assert pols <= {"+", "\u2212", "\u00b1"} and pols, pols
                # the Data-view drawer scrolls inside #lst-table-wrap (the observer root): scrolling it loads more rows
                pg.fill("#lst-q", ""); wait_for_js(pg, "document.querySelector('#lst-more')", 5000)
                n0 = pg.locator("#lst-tbody tr:not(#lst-more)").count()
                pg.evaluate("(()=>{const w=document.querySelector('#lst-table-wrap');w.scrollTop=w.scrollHeight})()")
                wait_for_js(pg, f"document.querySelectorAll('#lst-tbody tr:not(#lst-more)').length>{n0}", 5000)
                assert "Keller" not in pg.inner_text("#lst-ref-box") and pg.locator("#lst-ref-box a").count() == 0
                assert not pg.is_visible("#sb-help")
                pg.set_viewport_size({"width": 400, "height": 800}); pg.wait_for_timeout(200)
                assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth"), "no horizontal scroll at 400 px"
                pg.set_viewport_size({"width": 1440, "height": 900})
            finally:
                pg.evaluate("E.files.splice(E.files.indexOf(window.__fake),1); HR.isHr=HR.__ish")
        step("Reference lists in LR: four columns, m/z rounded, no references", lists_lr)
        def lists_tolerance():
            # the tolerance is the one of Adducts & losses (same remembered choice) and the search honours Da and ppm
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())")
            pg.evaluate("CALCQUI.set({ppm:'5', da:'0.5', unit:{hr:null, lr:null}})")
            pg.evaluate("LISTE.open({})"); pg.wait_for_selector("#lst-tbody tr", timeout=8000)
            mz = pg.evaluate("LISTE.ensure().pos[300].mz")
            q = "%.4f" % (mz + 0.02)                                   # 4 decimals: high resolution, ppm by default
            pg.fill("#lst-q", q); wait_for_js(pg, "document.querySelector('#lst-unit button.on').textContent === 'ppm'", 5000)
            wait_for_js(pg, "!document.querySelector('#lst-more') && document.querySelectorAll('#lst-tbody tr').length > 0", 5000)   # the Rust lookup answered: the long browse list (with its sentinel) is gone
            n_ppm = pg.locator("#lst-tbody tr .pol").count()           # 5 ppm is far below 0.02 Da at this m/z unless the list is dense there
            pg.click("#lst-unit button[data-u=da]"); pg.fill("#lst-tol", "0.05"); wait_for_js(pg, "CALCQUI.st.da === '0.05'", 5000)
            wait_for_js(pg, "/Da/.test(document.querySelector('#lst-tbody').innerText)", 5000)      # the new window's answer is on screen (errors in Da)
            assert pg.evaluate("JSON.parse(localStorage.getItem(NOMI.KEY('calcqui'))).unit.hr") == "da" and pg.evaluate("CALCQUI.st.da") == "0.05", "remembered like the Adducts & losses panel"
            hits = pg.evaluate("[...document.querySelectorAll('#lst-tbody tr')].map(r => r.innerText)")
            assert any("Da" in h for h in hits) and "match" in pg.inner_text("#lst-count"), (hits[:2], pg.inner_text("#lst-count"))
            assert pg.evaluate("LISTE.find(LISTE.ensure(), 0, %s, 5).length" % q) == n_ppm, "ppm window of the engine unchanged"
            pg.fill("#lst-tol", "0.00001"); wait_for_js(pg, "CALCQUI.st.da === '0.00001'", 5000)
            wait_for_js(pg, f"document.querySelectorAll('#lst-tbody tr .pol').length <= {n_ppm}", 5000)
            assert pg.locator("#lst-tbody tr .pol").count() <= n_ppm
            pg.evaluate("CALCQUI.set({ppm:'5', da:'0.5', unit:{hr:null, lr:null}})")
        step("Reference lists: tolerance control of Adducts & losses, Da and ppm honoured, choice remembered", lists_tolerance)
        def ptable():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())")
            pg.click("#np-pt"); pg.wait_for_timeout(500)
            assert pg.locator("#reftabs").count() == 0 and pg.inner_text("#reftitle").strip() == "Periodic table"
            js = """th=>{document.documentElement.setAttribute('data-theme',th);
              const cv=document.createElement('canvas').getContext('2d',{willReadFrequently:true}), px=c=>{cv.clearRect(0,0,1,1);cv.fillStyle='#000';cv.fillStyle=c;cv.fillRect(0,0,1,1);return [...cv.getImageData(0,0,1,1).data]};
              const lum=([r,g,b])=>{const f=v=>{v/=255;return v<=.03928?v/12.92:((v+.055)/1.055)**2.4};return .2126*f(r)+.7152*f(g)+.0722*f(b)};
              const worst=[]; let min=99;
              document.querySelectorAll('.pt-c').forEach(c=>{const bg=px(getComputedStyle(c).backgroundColor);
                c.querySelectorAll('b,span,small').forEach(e=>{if(!e.textContent.trim())return;const fg=px(getComputedStyle(e).color),a=lum(fg),b=lum(bg),r=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);if(r<min){min=r;worst.push(c.dataset.s)}})});
              const org=document.querySelector('.pt-c.org'), pl=document.querySelector('.pt-c:not(.org)');
              return {min,worst:worst.slice(-1)[0],bw:getComputedStyle(org).boxShadow,bw2:getComputedStyle(pl).boxShadow,n:document.querySelectorAll('.pt-c').length}}"""
            for th in ("light", "dark"):
                g = pg.evaluate(js, th)
                assert g["n"] >= 100 and g["min"] >= 4.5, (th, g)                 # symbol, atomic number and mass readable on every cell
                assert g["bw"] != "none" and g["bw2"] == "none", g              # highlighted elements: an inset frame, not only colour
            pg.evaluate("document.documentElement.removeAttribute('data-theme')")
            pg.click("#refx")
        step("periodic table: no tab bar, contrast >= 4.5 on every cell in light and dark, framed key elements", ptable)
        def aligned():
            qa = pg.evaluate("fetch('api/adduct_table').then(r=>r.json()).then(t=>t.rows.map(a=>[a.n,a.shift,a.k,a.z]))")
            for n, shift, k, z in qa:
                if k == 1 and abs(z) == 1 and n in ADDUCT_SHIFT: assert abs(shift - ADDUCT_SHIFT[n]) < 2e-5, (n, shift, ADDUCT_SHIFT[n])
            for n, shift, mult in ionfamily.ADDUCTS:
                if mult == 1:
                    q = [a for a in qa if a[0] == n]; assert q and abs(q[0][1] - shift) < 2e-4, (n, shift, q)
        step("the lists (Rust adduct table, draw.js, elements.py, ionfamily.py) agree on every shared adduct", aligned)
        def rounding():
            vals = [364.15, 0.25, 1.005, 2.675, 200.5, 363.0735, 364.05, 100.45, 99.95, 0.35]
            for d in (0, 1, 2):
                js = pg.evaluate("v=>v.map(x=>rh(x,%d))" % d, vals); py = [round_half_up(x, d) for x in vals]
                assert js == py, (d, js, py)
        step("JS rh and Python round_half_up give the same digits", rounding)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
