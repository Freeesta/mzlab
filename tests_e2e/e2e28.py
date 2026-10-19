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
        ROWS = "[...document.querySelectorAll('#ad-tbl .ar')].map(r=>[r.className,r.querySelector('.arn').textContent,r.querySelector('.ars').textContent,(r.querySelector('.arm')||{textContent:''}).textContent.replace('m/z','').trim()])"
        def reopen():
            pg.evaluate("document.querySelectorAll('dialog[open]').forEach(d=>d.close())"); pg.click("#np-ad"); pg.wait_for_timeout(400)
        def table():
            pg.click("#np-ad"); pg.wait_for_timeout(500)
            t = pg.inner_text("#refbody"); assert "nominale" not in t and "nominal" not in t.lower() and "Altri" in t and "M +" not in t and "Huang" not in t and "Keller" not in t, t[:600]
            assert pg.locator("#refbody a").count() == 0, "no link to the Theory in the panel"
            assert pg.locator("#refbody .help-q, #refbody [data-help]").count() == 0, "no question mark"
            pg.fill("#ad-in", "363.0665"); pg.wait_for_timeout(400)
            rows = pg.evaluate(ROWS)
            shown = [x[1] for x in rows]; assert shown == ["[M+H]+", "[M+NH4]+", "[M+Na]+", "[M+K]+"], shown      # ESI+ common ones until More is opened
            assert rows[0][2] == "+1.0073" and rows[0][3] == "364.0738" and rows[2][3] == "386.0557", rows
            pg.click("#ad-pol button[data-p='-1']")
            neg = pg.evaluate(ROWS); assert [x[1] for x in neg] == ["[M-H]-", "[M+HCOO]-", "[M+Cl]-"], neg
            assert neg[0][2] == "\u22121.0073", neg
            pg.click("#ad-pol button[data-p='1']")
            pg.locator("#ad-tbl .adtog").first.click()
            more = [x[1] for x in pg.evaluate(ROWS)]; assert "[2M+H]+" in more and "[M+H-H2O]+" in more, more
            assert pg.evaluate("document.activeElement.classList.contains('adtog')"), "focus stays on More"
            pg.fill("#ad-in", "363.0665"); pg.wait_for_timeout(300)
            assert "[2M+H]+" in [x[1] for x in pg.evaluate(ROWS)], "More stays open while typing"
            pg.locator("#ad-tbl .adtog").first.click()
            assert "[2M+H]+" not in [x[1] for x in pg.evaluate(ROWS)]
        step("list: one row per adduct, shift and m/z, ESI+/ESI- switch, More, no references", table)
        def rows_open():
            pg.fill("#ad-in", ""); pg.wait_for_timeout(200)
            b = pg.locator("#ad-tbl .arow").first
            assert b.get_attribute("aria-expanded") == "false" and not pg.locator("#ad-tbl .adnote").first.is_visible()
            b.click(); assert b.get_attribute("aria-expanded") == "true" and pg.locator("#ad-tbl .adnote").first.is_visible()
            note = pg.inner_text("#ad-tbl .adnote >> nth=0"); assert 0 < len(note.split()) <= 14, note
            b.click(); assert not pg.locator("#ad-tbl .adnote").first.is_visible()
        step("a click on the row opens and closes its note", rows_open)
        def lr():
            pg.evaluate("window.__fake={gone:false,polarity:'positive',name:'x'}; E.files.push(window.__fake); HR.__ish=HR.isHr; HR.isHr=()=>false")
            reopen()
            try:
                pg.fill("#ad-in", "363.0665"); pg.wait_for_timeout(300)
                rows = pg.evaluate(ROWS); assert [x[2] for x in rows] == ["+1", "+18", "+23", "+39"], rows
                assert [x[3] for x in rows] == ["364", "381", "386", "402"], rows
                assert pg.locator("#ad-tbl details.ad-diffs").count() == 0, "no pair table in LR"
                pg.click("#ad-pol button[data-p='-1']")
                assert [x[2] for x in pg.evaluate(ROWS)] == ["\u22121", "+45", "+35"], pg.evaluate(ROWS)
                assert not __import__("re").search(r"\d\.\d", pg.inner_text("#ad-tbl")), "no decimals in LR"
                pg.click("#ad-pol button[data-p='1']")
                pg.locator("#ad-tbl .adtog").first.click()
                assert not __import__("re").search(r"\d\.\d", pg.inner_text("#ad-tbl")), "no decimals in LR (More)"
                pg.locator("#ad-tbl .adtog").first.click()
            finally:
                pg.evaluate("E.files.splice(E.files.indexOf(window.__fake),1); HR.isHr=HR.__ish")
        step("LR: shifts and m/z rounded to the unit, no decimals, no pair table", lr)
        def hr_pairs():
            reopen()
            d = pg.locator("details.ad-diffs"); assert d.count() == 1 and d.get_attribute("open") is None, "pairs closed by default"
            d.locator("summary").click()
            rows = pg.evaluate("[...document.querySelectorAll('.ad-diffs tbody tr')].map(tr=>[...tr.children].map(c=>c.textContent))")
            assert 5 <= len(rows) <= 6 and rows[0][0] == "+21.9819", rows
            t = pg.inner_text(".ad-diffs"); assert "Huang" not in t and pg.locator(".ad-diffs a").count() == 0, t
        step("HR: characteristic pairs in a closed section, no source, no link", hr_pairs)
        def geometry():
            sz = pg.evaluate("[...document.querySelectorAll('#ad-tbl .arow, #ad-tbl .adtog, #ad-pol button')].map(b=>{const r=b.getBoundingClientRect();return [r.width,r.height]})")
            assert all(w >= 24 and h >= 24 for w, h in sz), sz
            pg.evaluate("(()=>{const b=document.querySelector('#refbody'); b.style.maxHeight='800px'})()")
        step("targets >= 24 px", geometry)
        def widths():
            for w in (280, 400, 560):
                pg.evaluate("w=>{const b=document.querySelector('#refbody'); b.style.width=w+'px'}", w); pg.wait_for_timeout(150)
                g = pg.evaluate("[document.querySelector('#ad-tbl').scrollWidth, document.querySelector('#ad-tbl').clientWidth, document.querySelector('#refbody').scrollWidth, document.querySelector('#refbody').clientWidth]")
                assert g[0] <= g[1] + 1 and g[2] <= g[3] + 1, (w, g)          # nothing cut, no sideways scroll
        step("widths 280 / 400 / 560 px: no cut, no sideways scroll", widths)
        def themes():
            for th in ("light", "dark"):
                pg.evaluate("t=>document.documentElement.setAttribute('data-theme',t)", th); pg.wait_for_timeout(100)
                c = pg.evaluate("(()=>{const n=document.querySelector('#ad-tbl .ars'),h=document.querySelector('#ad-tbl .arn');return [getComputedStyle(n).color,getComputedStyle(h).color]})()")
                assert c[0] != c[1], (th, c)
            pg.evaluate("document.documentElement.removeAttribute('data-theme')")
        step("themes: shift dimmed in light and dark", themes)
        def keyboard():
            pg.focus("#ad-in"); pg.keyboard.press("Tab"); pg.keyboard.press("Tab"); pg.keyboard.press("Tab")
            assert pg.evaluate("document.activeElement.classList.contains('arow')"), pg.evaluate("document.activeElement.outerHTML.slice(0,80)")
            pg.keyboard.press("Enter"); assert pg.evaluate("document.activeElement.getAttribute('aria-expanded')") == "true"
            pg.keyboard.press("Enter"); assert pg.evaluate("document.activeElement.getAttribute('aria-expanded')") == "false"
        step("keyboard: Tab reaches a row, Enter opens and closes it", keyboard)
        def i18n():
            for lg in ("en", "it"):
                pg.evaluate("l=>I18N.setLang?I18N.setLang(l):0", lg); reopen()
                t = pg.inner_text("#ad-tbl"); assert "tbl." not in t, t
        step("i18n: no missing keys in en and it", i18n)
        def narrow():
            pg.set_viewport_size({"width": 400, "height": 800}); pg.wait_for_timeout(300)
            assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth"), "no horizontal scroll at 400 px"
            pg.set_viewport_size({"width": 1440, "height": 900}); pg.wait_for_timeout(300)
        step("400 px: no horizontal scroll", narrow)
        def axe():
            AXE = axe_path(); assert AXE, "axe-core not available"
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("""() => axe.run(document.querySelector('#refdlg'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))"""); assert not v, v
        step("axe without violations", axe)
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
            qa = pg.evaluate("QADD.map(a=>[a.n,a.shift,a.k,a.z])")
            for n, shift, k, z in qa:
                if k == 1 and abs(z) == 1 and n in ADDUCT_SHIFT: assert abs(shift - ADDUCT_SHIFT[n]) < 2e-5, (n, shift, ADDUCT_SHIFT[n])
            for n, shift, mult in ionfamily.ADDUCTS:
                if mult == 1:
                    q = [a for a in qa if a[0] == n]; assert q and abs(q[0][1] - shift) < 2e-4, (n, shift, q)
        step("the lists (tables.js, draw.js, elements.py, ionfamily.py) agree on every shared adduct", aligned)
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
