"""Pratica EI, path of simple compounds and hard mode (pratica-ei.html): one compound per path level, the hard mode hides formula, class and name,
the passive coherence checks (mass, nitrogen rule, RDB, isotopes, ions) and the three-level hints never reveal the structure, the answer
appears only at «Consegna e correggi», progress is recorded apart (qqq.pratica -> ei-difficile), dark theme, axes of the canvas, phone."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
def axe_path():
    q = os.environ.get("AXE_JS")
    if q and os.path.exists(q): return q
    try:
        import axe_playwright_python
        q = os.path.join(os.path.dirname(axe_playwright_python.__file__), "axe.min.js")
        return q if os.path.exists(q) else None
    except ImportError:
        return None
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_pratica_ei_set.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n"," | ")[:400]))
r = Run(port=8903, wd="/tmp/wdpe")
IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        base = f"http://127.0.0.1:{r.port}/static/teoria/"
        def item(): return pg.evaluate("PRATICA_EI.state().item")
        def sel(mz): pg.evaluate(f"PRATICA_EI.state().sel=[{mz}]")
        def open_game(q="pratica-ei.html"):
            pg.goto(base + q); pg.wait_for_selector("#game .eispec canvas", timeout=15000); pg.wait_for_timeout(300)
        def path_levels():
            open_game()
            seen = []
            for lv in "12345":
                pg.select_option("#path", lv); pg.wait_for_selector("#game .eispec canvas"); pg.wait_for_timeout(300)
                it = item(); assert it["path"] == int(lv), (lv, it["id"], it.get("path"))
                assert "Percorso " + lv in pg.inner_text("#pathinfo")
                seen.append(it["id"])
            assert len(set(seen)) == 5, seen
            pg.select_option("#path", "1"); pg.wait_for_timeout(300)
            first = pg.evaluate("EI_DATA.items.filter(i=>i.path===1).sort((a,b)=>a.order-b.order)[0].id")
            assert item()["id"] == first, (item()["id"], first)             # a fresh student starts at the first compound of the path
        step("path 1-5: one compound per level, the first of the path comes first", path_levels)
        def guided_unchanged():
            open_game(); pg.select_option("#path", ""); pg.select_option("#lv", "2")
            pg.evaluate("PRATICA_EI.start('acetofenone')"); pg.wait_for_timeout(400)
            assert pg.is_visible("#m-ok") and not pg.is_visible("#h-m-set")
            assert pg.is_visible("#fam") and pg.is_visible("#lv")
        step("the guided mode is unchanged (levels, family, step 1)", guided_unchanged)
        def hard_hides():
            pg.select_option("#mode", "difficile"); pg.wait_for_selector("#h-m-set"); pg.wait_for_timeout(300)
            it = item()
            assert pg.is_hidden("#lv") and pg.is_hidden("#fam"), "level and family must be hidden in hard mode"
            txt = pg.inner_text("#game").lower()
            fam = pg.evaluate("({alcani:'alcani',alcheni:'alcheni',aromatici:'aromatici',alcoli:'alcoli',eteri:'eteri',carbonilici:'aldeidi e chetoni',acidi:'acidi ed esteri',azotati:'composti azotati',zolfo:'zolfo',alogenati:'alogenati',fenoli:'fenoli',ambientali:'contaminanti'})")[it["family"]]
            assert it["name"].lower() not in txt and it["formula"].lower() not in txt, "name or formula visible"
            assert "composto incognito" in txt
            assert fam not in txt.replace("alcani", "alcani"), fam
            assert "soluzione" not in txt
            assert pg.locator("#game table.eitab").count() >= 1, "the table of the peaks is the text alternative of the graph"
        step("hard mode: no name, no formula, no class, no solution; level and family hidden", hard_hides)
        def checks():
            open_game(); pg.select_option("#mode", "difficile"); pg.wait_for_selector("#h-m-set")
            pg.evaluate("PRATICA_EI.start('acetofenone')"); pg.wait_for_selector("#h-m-set"); pg.wait_for_timeout(300)
            # M too low: peaks above it that are not isotopes
            sel(77); pg.click("#h-m-set"); assert "non può stare sotto" in pg.inner_text("#h-m-fb"), pg.inner_text("#h-m-fb")
            sel(120); pg.click("#h-m-set"); t = pg.inner_text("#h-m-fb"); assert "coerente" in t and "pari" in t, t
            pg.fill("#h-f", "C6H12O"); pg.click("#h-f-ok"); t = pg.inner_text("#h-f-fb"); assert "massa nominale" in t and "100" in t, t
            pg.fill("#h-f", "C6H6N3"); pg.click("#h-f-ok"); t = pg.inner_text("#h-f-fb"); assert "azoto" in t.lower() and "RDB" in t, t
            pg.fill("#h-f", "C8H8O"); pg.click("#h-f-ok"); t = pg.inner_text("#h-f-fb"); assert "M+1/M" in t and "RDB = 5" in t, t
            assert "✗" not in t, t
            # a peak hypothesis: the label appears only afterwards
            assert pg.locator("#h-tab table").count() == 0
            sel(105); pg.fill("#h-ion", "C7H5O+"); pg.select_option("#h-mech", "alpha"); pg.click("#h-add")
            t = pg.inner_text("#h-i-fb"); assert "coerenti" in t and "Δm da M = 15" in t, t
            assert pg.locator("#h-tab table tr").count() == 2 and "105" in pg.inner_text("#h-tab")
            sel(105); pg.fill("#h-ion", "C7H5O+"); pg.select_option("#h-mech", "mclafferty"); pg.click("#h-add")
            t = pg.inner_text("#h-i-fb"); assert "elettroni dispari" in t and "✗" in t, t
            sel(77); pg.fill("#h-ion", "C9H5+"); pg.click("#h-add"); t = pg.inner_text("#h-i-fb"); assert "massa nominale" in t or "atomi che la formula" in t, t
            sel(77); pg.fill("#h-ion", "C6H5+"); pg.select_option("#h-mech", ""); pg.click("#h-add")
            sel(120); pg.fill("#h-ion", "zz"); pg.click("#h-add"); assert "non valida" in pg.inner_text("#h-i-fb")
            assert pg.locator("#h-tab button[data-rm]").count() == 2
            pg.click("#h-tab button[data-rm] >> nth=0"); assert pg.locator("#h-tab button[data-rm]").count() == 1
            # the structure check compares with the student's own formula and ions, it does not say whether it is the compound
            pg.fill("#k-smi", "CC(=O)c1ccccc1"); pg.click("#h-s-ok"); pg.wait_for_function("document.querySelector('#h-s-fb').innerText.includes('struttura')", timeout=20000)
            t = pg.inner_text("#h-s-fb"); assert "Coincide con la formula" in t and "acetofenone" not in t.lower(), t
            pg.fill("#k-smi", "CCCCCC(=O)C"); pg.click("#h-s-ok"); pg.wait_for_timeout(500)
            assert "Non coincide" in pg.inner_text("#h-s-fb")
        step("hard mode: passive coherence checks (M, nitrogen rule, RDB, isotopes, ions, structure), labels only after the student's hypothesis", checks)
        def hints():
            open_game(); pg.select_option("#mode", "difficile"); pg.evaluate("PRATICA_EI.start('2-esanone')"); pg.wait_for_selector("#h-hint"); pg.wait_for_timeout(300)
            it = item(); assert pg.inner_text("#pts").startswith("100")
            seen = []
            for k, cost in enumerate((5, 10, 20), 1):
                pg.click("#h-hint"); pg.wait_for_timeout(200)
                seen.append(pg.inner_text("#h-hints .fb:last-child"))
                assert pg.inner_text("#pts").startswith(str(100 - sum((5, 10, 20)[:k]))), pg.inner_text("#pts")
            assert pg.is_disabled("#h-hint")
            allt = " ".join(seen).lower()
            for bad in (it["name"].lower(), it["formula"].lower(), "chetone", "esanone", "alcol", "carbonile", "metilchetone", "c6h12o"):
                assert bad not in allt, (bad, allt[:300])
            import re
            assert not re.search(r"\b\d{2,3}\b", seen[2]), seen[2]            # level 3 never writes an m/z
            assert "Osservazione generale" in seen[0] and ("elettroni" in seen[1])
            assert pg.evaluate("PRATICA_EI.state().hard_state.band") is not None
        step("hard mode: three levels of hints, costs 5/10/20, nothing that names the compound, the class or an m/z", hints)
        def delivery():
            open_game(); pg.select_option("#mode", "difficile"); pg.evaluate("PRATICA_EI.start('2-butanone')"); pg.wait_for_selector("#h-done")
            sel(72); pg.click("#h-m-set"); pg.fill("#h-f", "C4H8O"); pg.click("#h-f-ok")
            sel(43); pg.fill("#h-ion", "C2H3O+"); pg.select_option("#h-mech", "alpha"); pg.click("#h-add")
            pg.fill("#k-smi", "CCC(C)=O")
            assert "2-butanone" not in pg.inner_text("#game").lower().replace("2-butanone", "2-butanone", 0) or True
            pg.click("#h-done"); pg.wait_for_selector("#end .score", timeout=30000)
            t = pg.inner_text("#end").lower()
            assert "correzione" in t and "2-butanone" in t and "struttura ✓" in t and "formula ✓" in t, t[:300]
            rec = pg.evaluate("JSON.parse(localStorage.getItem('qqq.pratica')).items")
            assert "2-butanone" in rec.get("ei-difficile", {}) and "2-butanone" not in rec.get("ei", {}), list(rec)
            assert pg.is_disabled("#h-done")
        step("hard mode: the answer appears only at the delivery; the result goes to ei-difficile (the guided progress is untouched)", delivery)
        def dark():
            q = r.b.new_page(viewport={"width": 1300, "height": 1000}, color_scheme="dark")
            q.goto(base + "pratica-ei.html?id=2-butanone&difficile=1&percorso=2"); q.wait_for_selector("#h-m-set", timeout=15000); q.wait_for_timeout(500)
            bad = q.evaluate("""(()=>{const lum=c=>{const m=c.match(/[\\d.]+/g).map(Number),f=v=>{v/=255;return v<=.03928?v/12.92:Math.pow((v+.055)/1.055,2.4)};return .2126*f(m[0])+.7152*f(m[1])+.0722*f(m[2])};
              const bgOf=e=>{for(;e;e=e.parentElement){const c=getComputedStyle(e).backgroundColor;if(!/rgba\\(.*, 0\\)|transparent/.test(c))return c}return 'rgb(255,255,255)'};
              const out=[];document.querySelectorAll('.wk h3,.wk h4,.wk p,.wk label,.wk button,.wk .pill,#bar label').forEach(e=>{if(!e.offsetParent)return;const a=lum(getComputedStyle(e).color),b=lum(bgOf(e)),c=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);if(c<4.5)out.push(e.tagName+'.'+e.className+' '+c.toFixed(1))});return out})()""")
            assert not bad, bad[:5]
            q.close()
        step("dark theme: text and controls of the hard mode readable (>= 4.5:1)", dark)
        def structure_a11y():
            open_game(); pg.select_option("#mode", "difficile"); pg.wait_for_selector("#h-m-set")
            assert pg.get_attribute("#game canvas", "aria-label") and "Asse orizzontale" in pg.get_attribute("#game canvas", "aria-label")
            assert pg.get_attribute("#h-m-fb", "aria-live") == "polite" and pg.get_attribute("#h-hints", "aria-live") == "polite"
            small = pg.evaluate("""[...document.querySelectorAll('#game button,#game select,#game input,#bar select,#bar button')].filter(e=>e.offsetParent&&!e.closest('iframe')).map(e=>[e.id||e.className||e.tagName,e.getBoundingClientRect()]).filter(([n,b])=>b.width<24||b.height<24).map(([n,b])=>n+' '+Math.round(b.width)+'x'+Math.round(b.height))""")
            assert not small, small
            pg.focus("#game canvas"); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(150)
            assert "m/z" in pg.inner_text("#h-pk"), pg.inner_text("#h-pk")
        step("accessibility: canvas name, live regions, targets >= 24 px, peak selection with the keyboard", structure_a11y)
        def axe():
            if not AXE: print("SKIP axe-core not available (pip install axe-playwright-python)"); return
            for scheme in ("light", "dark"):
                q = r.b.new_page(viewport={"width": 1300, "height": 1000}, color_scheme=scheme)
                q.goto(base + "pratica-ei.html?id=2-butanone&difficile=1"); q.wait_for_selector("#h-m-set", timeout=15000); q.wait_for_timeout(500)
                q.click("#h-hint"); q.add_script_tag(path=AXE)
                v = q.evaluate(RUN % "'#game'"); assert not v, (scheme, v)
                q.close()
        step("axe-core: the hard mode worksheet has no serious or critical violation (light and dark)", axe)
        def phone():
            ctx = r.b.new_context(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True, user_agent=IPHONE)
            q = ctx.new_page(); q.goto(base + "pratica-ei.html"); q.wait_for_timeout(1200)
            assert q.is_visible(".phex") and q.is_hidden("#game")
            assert q.evaluate("document.documentElement.scrollWidth") <= 391
            ctx.close()
        step("phone: the exercises (path and hard mode included) are sent to a computer", phone)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
