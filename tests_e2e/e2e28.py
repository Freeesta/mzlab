"""Addotti tab: compact table (no nominal column), common adducts first, the rest folded, no pair tool, lists aligned with the Python ones, one rounding rule (JS rh = Python round_half_up). No data files needed."""
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
        ROWS = "[...document.querySelectorAll('#ad-tbl table.adt tr.adr')].map(tr=>[tr.className,...[...tr.children].map(c=>c.textContent)])"
        def table():
            pg.click("#np-ad"); pg.wait_for_timeout(500)
            t = pg.inner_text("#refbody"); assert "nominale" not in t and "nominal" not in t.lower() and "Altri addotti (meno comuni)" in t and "M +" not in t and "m/z =" not in t, t[:600]
            pg.fill("#ad-in", "363.0665"); pg.wait_for_timeout(400)
            rows = pg.evaluate(ROWS)
            hrow = [x for x in rows if x[1].startswith("[M+H]")][0]; assert "exp" in hrow[0] and hrow[2] == "+1.0073" and hrow[3] == "364.0738", hrow
            assert [x for x in rows if x[1].startswith("[M-H]-")][0][2] == "\u22121.0073"
            na = [x for x in rows if x[1].startswith("[M+Na]+")][0]; assert "exp" in na[0] and na[3] == "386.0557", na
            k = [x for x in rows if x[1].startswith("[M+K]+")][0]; assert "exp" in k[0], k
            shown = [x[1] for x in rows]; assert shown == ["[M+H]+", "[M+NH4]+", "[M+Na]+", "[M+K]+", "[M-H]-", "[M+HCOO]-", "[M+Cl]-"], shown      # only the common ones until the fold is opened
            pg.locator("#ad-tbl .adtog").first.click(); pg.wait_for_timeout(200)
            more = [x[1] for x in pg.evaluate(ROWS)]; assert "[2M+H]+" in more and "[M+H-H2O]+" in more, more
            assert pg.evaluate("document.activeElement.classList.contains('adtog')"), "focus stays on the fold button"
            pg.fill("#ad-in", "363.0665"); pg.wait_for_timeout(300)
            assert "[2M+H]+" in [x[1] for x in pg.evaluate(ROWS)], "the fold stays open while typing"
            pg.locator("#ad-tbl .adtog").first.click(); pg.wait_for_timeout(200)
            assert "[2M+H]+" not in [x[1] for x in pg.evaluate(ROWS)]
        step("table: no nominal column, signed shifts without «M +», expected adducts first, fold", table)
        def structure():
            # real table, row headers, one note row (colspan) under every adduct, thin group headers, 3 columns only
            assert pg.evaluate("document.querySelectorAll('#ad-tbl table.adt thead th[scope=col]').length") == 3
            assert pg.evaluate("[...document.querySelectorAll('#ad-tbl table.adt tr.adr th')].every(h=>h.scope==='row')")
            assert pg.evaluate("[...document.querySelectorAll('#ad-tbl table.adt tr.adr')].every(tr=>tr.nextElementSibling.classList.contains('adn') && tr.nextElementSibling.children.length===1 && tr.nextElementSibling.children[0].colSpan===3)")
            assert pg.evaluate("document.querySelectorAll('#ad-tbl tr.adh th[scope=colgroup]').length") == 2
            note = pg.evaluate("document.querySelector('#ad-tbl tr.adn td').textContent.split(/\\s+/).length"); assert note <= 10, note
            assert pg.evaluate("document.querySelector('#ad-tbl .adtog').getAttribute('aria-expanded')") == "false"
        step("structure: table with row headers, colspan note row, ESI group headers", structure)
        def geometry():
            pg.evaluate("(()=>{const b=document.querySelector('#refbody'); b.style.width='400px'; b.style.maxHeight='800px'})()"); pg.wait_for_timeout(200)
            pg.locator("#ad-tbl .adtog").first.click(); pg.wait_for_timeout(200)
            g = pg.evaluate("""(()=>{const t=document.querySelector('#ad-tbl table.adt'),rows=[...t.querySelectorAll('tr.adr')];
              const top=t.getBoundingClientRect().top, last=rows[11].nextElementSibling.getBoundingClientRect().bottom;
              return {h:Math.max(...rows.map(r=>r.getBoundingClientRect().height)),n:rows.length,span:last-top}})()""")
            assert g["h"] <= 36 and g["n"] >= 12 and g["span"] <= 800, g         # 12 adducts (data + note rows) fit in 800 px
            pg.fill("#ad-in", ""); pg.locator("#ad-tbl .adtog").first.click(); pg.locator("#ad-tbl .adtog").nth(1).click(); pg.wait_for_timeout(150)
        step("geometry: data rows <= 36 px", geometry)
        def widths():
            for w in (280, 400, 560):
                pg.evaluate("w=>{const b=document.querySelector('#refbody'); b.style.width=w+'px'; b.style.maxHeight='800px'}", w); pg.wait_for_timeout(150)
                g = pg.evaluate("(()=>{const t=document.querySelector('#ad-tbl table.adt'); return [t.scrollWidth, t.parentElement.clientWidth, document.querySelector('#refbody').scrollWidth, document.querySelector('#refbody').clientWidth]})()")
                assert g[0] <= g[1] + 1 and g[2] <= g[3] + 1, (w, g)          # nothing cut, no sideways scroll
        step("widths 280 / 400 / 560 px: no cut, no sideways scroll", widths)
        def themes():
            for th in ("light", "dark"):
                pg.evaluate("t=>document.documentElement.setAttribute('data-theme',t)", th); pg.wait_for_timeout(100)
                c = pg.evaluate("(()=>{const n=document.querySelector('#ad-tbl tr.adn td'),h=document.querySelector('#ad-tbl tr.adr th');return [getComputedStyle(n).color,getComputedStyle(h).color,getComputedStyle(document.querySelector('#refbody')).backgroundColor]})()")
                assert c[0] != c[1], (th, c)
            pg.evaluate("document.documentElement.removeAttribute('data-theme')")
        step("themes: note text dimmed in light and dark", themes)
        def keyboard():
            pg.focus("#ad-in"); pg.keyboard.press("Tab")
            assert pg.evaluate("document.activeElement.classList.contains('adtog')"), "Tab reaches the fold button"
            pg.keyboard.press("Enter"); pg.wait_for_timeout(200)
            assert "[2M+H]+" in [x[1] for x in pg.evaluate(ROWS)] or "[M+H-H2O]+" in [x[1] for x in pg.evaluate(ROWS)]
            pg.keyboard.press("Enter"); pg.wait_for_timeout(200)
        step("keyboard: Tab then Enter open and close the fold", keyboard)
        def diffs():
            d = pg.evaluate("[...document.querySelectorAll('.ad-diffs tbody tr')].map(tr=>[...tr.children].map(c=>c.textContent))")
            assert 5 <= len(d) <= 6 and d[0][0] == "+21.9819", d
            t = pg.inner_text(".ad-diffs"); assert "Huang" in t and "nominal" not in t.lower(), t
            assert pg.evaluate("document.querySelectorAll('.ad-diffs a').length") == 1
            a = pg.locator(".ad-diffs a"); assert a.get_attribute("href") == "static/teoria/06-esi.html#addotti-differenze" and "noopener" in a.get_attribute("rel") and a.get_attribute("aria-label")
        step("pairs: at most 6 rows, source line, no nominal, one link to the Theory", diffs)
        def theory():
            reqs = []
            with pg.context.expect_page() as np_:
                pg.locator(".ad-diffs a").focus(); pg.keyboard.press("Enter")
            tp = np_.value; tp.on("request", lambda q: reqs.append(q.url)); tp.wait_for_load_state(); tp.wait_for_timeout(800)
            assert tp.url.endswith("/static/teoria/06-esi.html#addotti-differenze"), tp.url
            h = tp.evaluate("document.querySelector('#addotti-differenze').getBoundingClientRect().top"); assert -5 < h < 700, h
            assert tp.evaluate("document.querySelectorAll('#addotti-differenze + p + .tw caption').length") == 1
            assert tp.evaluate("document.querySelectorAll('#addotti-differenze + p + .tw tbody tr').length") == 14
            hd = tp.evaluate("[...document.querySelectorAll('#addotti-differenze + p + .tw thead th')].map(h=>h.scope+':'+h.textContent)")
            assert hd == ["col:Coppia di addotti", "col:\u0394 m/z esatto", "col:\u0394 nominale", "col:Modo", "col:Nota e significato didattico"], hd
            tp.set_viewport_size({"width": 390, "height": 800}); tp.wait_for_timeout(300)
            assert tp.evaluate("document.documentElement.scrollWidth <= window.innerWidth"), "no horizontal page scroll at 390 px"
            for th in ("light", "dark"):
                tp.evaluate(f"document.documentElement.setAttribute('data-theme','{th}')")
            bad = [u for u in reqs if any(k in u.lower() for k in ("pyodide", ".wasm", "mzlab.zip"))]; assert not bad, bad
            tp.close()
        step("full table opens in the Theory (14 rows, headers, 390 px, no engine)", theory)
        def reflists():
            pg.evaluate("document.querySelector('#refdlg').close()")
            pg.evaluate("LISTE.open({})"); pg.wait_for_timeout(1500)
            n = pg.evaluate("document.querySelectorAll('#lst-src-checks input[type=checkbox]').length"); assert n == 0, "one list only: no switch"
            t = pg.inner_text("#lst-src-checks"); assert "Keller" in t, t
            # two lists (the built-in one + a simulated user list): the switches come back
            pg.evaluate("LISTE.addUserListFromCsv('prova.csv','mz,name,formula,polarity\\n100.1,Test,,1\\n')"); pg.wait_for_timeout(600)
            pg.evaluate("LISTE.open({})"); pg.wait_for_timeout(800)
            n = pg.evaluate("document.querySelectorAll('#lst-src-checks input[type=checkbox]').length"); assert n == 2, n
        step("Reference lists: no switch with one list, switches with two", reflists)
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
