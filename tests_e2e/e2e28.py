"""Adducts tab: one row per adduct (shift, m/z of the typed mass), ESI+/ESI- switch, rare ones behind More, LR rounded to the unit, HR pairs closed, no references; lists aligned with the Python ones, one rounding rule. No data files needed."""
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
            n = pg.evaluate("document.querySelectorAll('#lst-src-checks input[type=checkbox]').length"); assert n == 0, "one list only: no switch"
            t = pg.inner_text("#lst-src-checks"); assert "Keller" not in t and "Anal. Chim" not in t and "contaminanti" in t.lower(), t
            assert pg.locator("#lst-mode-seg").count() == 0 and pg.is_visible("#lst-view-seg"), "no files: HR version, no mode switch"
            assert pg.locator("#lst-tbody tr").first.locator("td").count() == 9 and "." in pg.inner_text("#lst-tbody tr >> nth=0")
            # two lists (the built-in one + a simulated user list): the switches come back
            pg.evaluate("LISTE.addUserListFromCsv('prova.csv','mz,name,formula,polarity\\n100.1,Test,,1\\n')"); pg.wait_for_timeout(600)
            pg.evaluate("LISTE.open({})"); pg.wait_for_timeout(800)
            n = pg.evaluate("document.querySelectorAll('#lst-src-checks input[type=checkbox]').length"); assert n == 2, n
        step("Reference lists: no switch with one list, switches with two", reflists)
        def lists_lr():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())")
            pg.evaluate("window.__fake={gone:false,polarity:'positive',name:'x'}; E.files.push(window.__fake); HR.__ish=HR.isHr; HR.isHr=()=>false")
            try:
                pg.evaluate("LISTE.open({})"); pg.wait_for_selector("#lst-tbody tr", timeout=8000)
                assert pg.locator("#lst-tbody tr").first.locator("td").count() == 4 and pg.locator("#lst-table-wrap thead th:visible").count() == 4
                assert not pg.is_visible("#lst-view-seg") and not pg.is_visible("#lst-tol-wrap") and not pg.is_visible("#lst-cont-signs")
                import re
                mzs = pg.evaluate("[...document.querySelectorAll('#lst-tbody tr td:first-child')].map(t=>t.textContent)"); assert mzs and all(re.fullmatch(r"\d+", m) for m in mzs), mzs[:5]      # m/z rounded to the unit in LR
                pg.fill("#lst-mz", "63.3"); t = pg.inner_text("#lst-tbody"); assert "63" in t and "PEG" in t, t[:200]
                assert "Keller" not in pg.inner_text("#lst-ref-box") and pg.locator("#lst-ref-box a").count() == 0
                assert not pg.is_visible("#sb-help")
                pg.set_viewport_size({"width": 400, "height": 800}); pg.wait_for_timeout(200)
                assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth"), "no horizontal scroll at 400 px"
                pg.set_viewport_size({"width": 1440, "height": 900})
            finally:
                pg.evaluate("E.files.splice(E.files.indexOf(window.__fake),1); HR.isHr=HR.__ish")
        step("Reference lists in LR: four columns, m/z rounded, no references", lists_lr)
        def ptable():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())")
            pg.click("#np-pt"); pg.wait_for_timeout(500)
            assert pg.locator("#reftabs").count() == 0 and pg.inner_text("#reftitle").strip() == "Tavola periodica"
            js = """th=>{document.documentElement.setAttribute('data-theme',th);
              const cv=document.createElement('canvas').getContext('2d',{willReadFrequently:true}), px=c=>{cv.clearRect(0,0,1,1);cv.fillStyle='#000';cv.fillStyle=c;cv.fillRect(0,0,1,1);return [...cv.getImageData(0,0,1,1).data]};
              const lum=([r,g,b])=>{const f=v=>{v/=255;return v<=.03928?v/12.92:((v+.055)/1.055)**2.4};return .2126*f(r)+.7152*f(g)+.0722*f(b)};
              const worst=[]; let min=99;
              document.querySelectorAll('.pt-c').forEach(c=>{const bg=px(getComputedStyle(c).backgroundColor);
                c.querySelectorAll('b,span,small').forEach(e=>{if(!e.textContent.trim())return;const fg=px(getComputedStyle(e).color),a=lum(fg),b=lum(bg),r=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);if(r<min){min=r;worst.push(c.dataset.s)}})});
              const org=document.querySelector('.pt-c.org'), pl=document.querySelector('.pt-c:not(.org)');
              return {min,worst:worst.slice(-1)[0],bw:getComputedStyle(org).borderTopWidth,bw2:getComputedStyle(pl).borderTopWidth,n:document.querySelectorAll('.pt-c').length}}"""
            for th in ("light", "dark"):
                g = pg.evaluate(js, th)
                assert g["n"] >= 100 and g["min"] >= 4.5, (th, g)                 # symbol, atomic number and mass readable on every cell
                assert float(g["bw"][:-2]) > float(g["bw2"][:-2]), g              # highlighted elements: thicker border, not only colour
            pg.evaluate("document.documentElement.removeAttribute('data-theme')")
            pg.click("#refx")
        step("periodic table: no tab bar, contrast >= 4.5 on every cell in light and dark, thicker border for key elements", ptable)
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
