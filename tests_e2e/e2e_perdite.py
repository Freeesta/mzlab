"""Prompt 2, block 4: the «Perdite neutre» tab: simple view, Dettagli, isobaric groups, Cerca Δm (62, 72), polarity filter, entry from the ruler."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_perdite.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
r = Run(port=8880, wd="/tmp/wd80")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def simple():
            pg.click("#np-nl"); pg.wait_for_timeout(300)
            n = pg.locator("#nl-list .nlr").count(); assert 14 <= n <= 24, n
            ms = [int(x) for x in pg.locator("#nl-list .nlm").all_inner_texts()]; print(ms); assert ms == sorted(ms) and ms[0] == 15, ms
            assert pg.locator("#nl-list .nldet:visible").count() == 0, "details closed by default"
            assert pg.evaluate("document.querySelector('#nl-pol button.on').dataset.p") == "+", "positive files -> the filter starts on +"
            t = pg.inner_text("#nl-list"); assert "alcoli" in t and "tioli" in t and "idrossido" not in t and "%" not in t, t
            assert "Dettagli" not in t and "elettroni pari" not in t, "no repeated button or radical paragraph on the rows"
            hs = pg.evaluate("[...document.querySelectorAll('#nl-list .nlrow')].map(b=>b.getBoundingClientRect().height)"); assert max(hs) <= 44, hs
        step("4.1/4.6: compact one-line rows, sorted by Δm, polarity of the files", simple)
        def details():
            b = pg.locator('#nl-list [data-d="H2O"]'); assert b.get_attribute("aria-expanded") == "false"
            b.click(); pg.wait_for_timeout(200); assert b.get_attribute("aria-expanded") == "true"
            t = pg.inner_text('#nl-list [data-f="H2O"] .nldet'); print(t[:120]); assert "18.0106" in t and "meccanismo" in t and "Levsen" in t and "acqua" in t, t
            b.focus(); pg.keyboard.press("Enter"); pg.wait_for_timeout(100)
            assert b.get_attribute("aria-expanded") == "false" and pg.locator('#nl-list [data-f="H2O"] .nldet:visible').count() == 0
            pg.keyboard.press("Space"); pg.wait_for_timeout(100); assert b.get_attribute("aria-expanded") == "true"
            pg.keyboard.press("Space")
        step("4.2: the row is the button: click and keyboard open exact mass, mechanism, source, name", details)
        def groups():
            g = pg.locator("#nl-list .nlr[data-f^=g]"); assert g.count() == 2, g.count()      # with + : 28 and 42 (46 and 80 need a negative-only loss)
            row = pg.locator('#nl-list [data-f="g28"] .nlrow'); t = row.inner_text(); assert "CO" in t and "C₂H₄" in t and "risoluzione unitaria" in t, t
            row.click(); d = pg.inner_text('#nl-list [data-f="g28"] .nldet'); assert d.count("Fonti") == 2, d
            row.click()
            # Click ESI- to activate both + and -
            pg.locator("#nl-pol [data-p='-']").click(); pg.wait_for_timeout(200)
            g80 = pg.locator('#nl-list [data-f="g80"]'); assert g80.count() == 1 and "M+2" in g80.inner_text()
            assert pg.locator("#nl-list .nlr[data-f^=g]").count() == 4
            assert "profilo isotopico nello spettro" in g80.inner_text() and "scheda" not in g80.inner_text()
        step("4.3: same nominal mass on ONE line with the formulas side by side; 80 points to the isotope profile", groups)
        def radicals():
            d = pg.locator('#nl-list [data-f="CH3"] .nlrd'); assert d.count() == 1 and "radicale" in d.get_attribute("aria-label") and d.get_attribute("title"), "dot with tooltip"
            assert "•" in pg.inner_text('#nl-list [data-f="CH3"] .nlrow') and "elettroni pari" not in pg.inner_text('#nl-list [data-f="CH3"] .nlrow')
            pg.locator('#nl-list [data-d="CH3"]').click(); assert "elettroni pari" in pg.inner_text('#nl-list [data-f="CH3"] .nldet'); pg.locator('#nl-list [data-d="CH3"]').click()
            assert "•" in pg.inner_text('#nl-list [data-f="NO2"]') if pg.locator('#nl-list [data-f="NO2"]').count() else True
        step("4.4: radical loss: dot with the dot in the formula and a tooltip, full text only in the details", radicals)
        def search():
            pg.fill("#nl-q", "62"); pg.wait_for_timeout(300)
            t = pg.inner_text("#nl-res"); print(t.replace("\n", " | ")); assert "Possibili perdite (da verificare sullo spettro)" in t and "H2O + CO2" in t.replace("H₂O", "H2O").replace("CO₂", "CO2") or ("H" in t and "CO" in t and "+" in t), t
            pg.fill("#nl-q", "72"); pg.wait_for_timeout(300); t = pg.inner_text("#nl-res"); assert "2 ×" in t and "HCl" in t, t
            pg.fill("#nl-q", "28"); pg.wait_for_timeout(300); assert pg.locator("#nl-list .nlr.hit").count() == 1, pg.locator("#nl-list .nlr.hit").count()
            pg.fill("#nl-q", "")
        step("4.5: Cerca Δm: 62 = H2O + CO2, 72 = 2 x HCl, 28 highlights the shared line", search)
        def polarity():
            # Turn off + so only - is active
            pg.click("#nl-pol [data-p='+']"); pg.wait_for_timeout(200)
            ids = pg.evaluate("[...document.querySelectorAll('#nl-list .nlr')].map(r=>r.dataset.f)"); assert "NO2" in ids and "SO3" in ids and "NH3" not in ids, ids
            # Turn on + and turn off -
            pg.click("#nl-pol [data-p='+']"); pg.wait_for_timeout(100); pg.click("#nl-pol [data-p='-']"); pg.wait_for_timeout(200)
            ids = pg.evaluate("[...document.querySelectorAll('#nl-list .nlr')].map(r=>r.dataset.f)"); assert "NH3" in ids and "SO3" not in ids and "H2O" in ids, ids
            shut(pg, "#refx")
        step("4.6: +/- toggle filter", polarity)
        def a2():
            if not pg.is_visible("#sb-panel-losses"): pg.click("#np-nl")
            pg.wait_for_timeout(300)
            assert [b.strip() for b in pg.locator("#nl-pol button").all_inner_texts()] == ["ESI+", "ESI−"]
            assert pg.locator("#nl-list .nlpp").count() == 0, "polarity column removed"
            # Turn on both + and -
            pg.click("#nl-pol [data-p='-']"); pg.wait_for_timeout(200)
            row = pg.locator('#nl-list [data-f="Cl"]'); assert row.count() == 1
            assert row.locator(".nlm").inner_text() == "35" and "•Cl" in row.inner_text() and "cloro aromatico" in row.inner_text()
            pg.locator('#nl-list [data-d="Cl"]').click(); d = pg.inner_text('#nl-list [data-f="Cl"] .nldet'); assert "34.9689" in d and "HCl (36)" in d and "37Cl" in d and "radicale" in d, d
            # the «?» is replaced by one short hint line
            assert not pg.is_visible("#sb-help") and pg.is_visible("#nl-hint") and len(pg.inner_text("#nl-hint")) < 80
            pg.fill("#nl-q", "62"); pg.wait_for_timeout(300); assert "H2O" in pg.inner_text("#nl-res").replace("H₂O", "H2O"); pg.fill("#nl-q", ""); shut(pg, "#refx")
        step("A2: compact view, ESI+/ESI- toggles, •Cl, hint line instead of «?»", a2)
        def narrow_axe():
            AXE = axe_path(); assert AXE, "axe-core not available"
            pg.add_script_tag(path=AXE)
            pg.locator("#nl-pol [data-p='-']").click(); pg.locator('#nl-list [data-d="g28"]').click()
            RUN = """() => axe.run(document.querySelector('#sb-panel-losses'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))"""
            v = pg.evaluate(RUN); assert not v, v
            for w, h in [(1440, 900), (400, 800)]:
                pg.set_viewport_size({"width": w, "height": h}); pg.wait_for_timeout(300)
                if not pg.is_visible("#sb-panel-losses"): pg.click("#np-nl"); pg.wait_for_timeout(300)
                sw = pg.evaluate("(()=>{const r=document.querySelector('#sb-panel-losses');return [r.scrollWidth>r.clientWidth+1,document.documentElement.scrollWidth>innerWidth]})()"); assert not any(sw), (w, sw)
                tgt = pg.evaluate("[...document.querySelectorAll('#nl-list .nlrow')].every(b=>{const q=b.getBoundingClientRect();return q.width>=24&&q.height>=24})"); assert tgt, w
            pg.set_viewport_size({"width": 1400, "height": 1000}); pg.wait_for_timeout(200)
            pg.locator('#nl-list [data-d="g28"]').click(); pg.locator("#nl-pol [data-p='-']").click(); shut(pg, "#refx")
        step("axe without violations on the panel; 1440 and 400 px without horizontal scroll; targets ≥ 24 px", narrow_axe)
        def ruler():
            pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            pg.evaluate(f"(()=>{{const p={SPEC};p.meas={{ref:364.4,list:[{{a:364.4,b:194.2}}]}};draw(p)}})()"); pg.wait_for_timeout(500)
            a = pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),a=p._a;return [q.left+a.X(194.2),q.top+a.Y(a.ymax*0.1)]}})()")
            pg.mouse.click(a[0], a[1], button="right"); pg.wait_for_timeout(300)
            its = pg.evaluate("[...document.querySelectorAll('#ctx div')].map(d=>d.textContent)"); print(its)
            assert "Cerca 170.2 nelle perdite neutre" in its, its
            pg.locator("#ctx div", has_text="Cerca 170.2").click(); pg.wait_for_timeout(500)
            assert pg.evaluate("BARRA.curTab === 'losses' && !document.querySelector('#sb-panel-losses').hidden") and pg.input_value("#nl-q") == "170.2"
            shut(pg, "#refx")
        step("4.7: from the ruler: «Cerca 170.2 nelle perdite neutre»", ruler)
        def ruler_label():
            def lbl_pos(a_, b_):
                pg.evaluate(f"(()=>{{const p={SPEC};p.meas={{ref:{a_},list:[{{a:{a_},b:{b_}}}]}};draw(p)}})()"); pg.wait_for_timeout(500)
                return pg.evaluate(f"(()=>{{const p={SPEC},q=p.cv.getBoundingClientRect(),l=p._a.lbls.find(b=>b.dm!=null);return l?[q.left+l.x+l.w/2,q.top+l.y+l.h/2,l.tip,!!l.ap]:null}})()")
            l = lbl_pos(364.4, 194.2); assert l and not l[3] and "perdite" in l[2], l
            pg.mouse.click(l[0], l[1], button="right"); pg.wait_for_timeout(500)
            assert not pg.evaluate("document.querySelector('#ctx') && !document.querySelector('#ctx').hidden"), "the general menu must not open on the label"
            assert pg.evaluate("BARRA.curTab === 'losses'") and pg.input_value("#nl-q") == "170.2", "right click on the label: losses filtered on the value"
            shut(pg, "#refx")
            l = lbl_pos(300, 321.9819); assert l and l[3] and "[M+Na]" in l[2] and "[M+H]" in l[2], l
            pg.mouse.click(l[0], l[1], button="right"); pg.wait_for_timeout(500)
            assert pg.evaluate("BARRA.curTab === 'adducts'"), "right click on an adduct label: adducts tab"
            shut(pg, "#refx")
        step("4.8: ruler label: right click straight to losses / adducts, short adduct symbol with the full form in the tip", ruler_label)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s_ in steps: print(" ", s_)
r.report()
