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
            n = pg.locator("#nl-list .nlr").count(); assert 18 <= n <= 24, n
            ms = [int(x) for x in pg.locator("#nl-list .nlm").all_inner_texts()]; print(ms); assert ms == sorted(ms) and ms[0] == 15, ms
            assert pg.locator("#nl-list .nldet:visible").count() == 0, "details closed by default"
            assert pg.evaluate("document.querySelector('#nl-pol button.on').dataset.p") == "+", "positive files -> the filter starts on +"
            t = pg.inner_text("#nl-list"); assert "alcoli" in t and "tioli" in t and "idrossido" not in t and "%" not in t, t
        step("4.1/4.6: simple view, sorted by Δm, polarity of the files", simple)
        def details():
            pg.locator('#nl-list [data-d="H2O"]').click(); pg.wait_for_timeout(200)
            t = pg.inner_text('#nl-list [data-f="H2O"] .nldet'); print(t[:120]); assert "18.0106" in t and "meccanismo" in t and "Levsen" in t and "acqua" in t, t
            pg.locator('#nl-list [data-d="H2O"]').click(); assert pg.locator('#nl-list [data-f="H2O"] .nldet:visible').count() == 0
        step("4.2: Dettagli: exact mass (4 decimals), mechanism, source, loss name", details)
        def groups():
            g = pg.locator("#nl-list .nlg"); assert g.count() == 2, g.count()      # with + : 28 and 42 (46 and 80 need a negative-only loss)
            t = pg.inner_text("#nl-list .nlg"); assert "risoluzione unitaria" in t
            # Click ESI- to activate both + and -
            pg.locator("#nl-pol [data-p='-']").click(); pg.wait_for_timeout(200)
            g80 = pg.locator("#nl-list .nlg", has_text="(80)"); assert g80.count() == 1 and "M+2" in g80.inner_text()
            assert pg.locator("#nl-list .nlg").count() == 4
            assert "profilo isotopico nello spettro" in g80.inner_text() and "scheda" not in g80.inner_text()
        step("4.3: same nominal mass in a box; 80 points to the isotope profile of the spectrum", groups)
        def radicals():
            t = pg.inner_text('#nl-list [data-f="CH3"]'); assert "•" in t and "elettroni pari" in t, t
            assert "•" in pg.inner_text('#nl-list [data-f="NO2"]')
        step("4.4: radical losses with the dot and the short label", radicals)
        def search():
            pg.fill("#nl-q", "62"); pg.wait_for_timeout(300)
            t = pg.inner_text("#nl-res"); print(t.replace("\n", " | ")); assert "Possibili perdite (da verificare sullo spettro)" in t and "H2O + CO2" in t.replace("H₂O", "H2O").replace("CO₂", "CO2") or ("H" in t and "CO" in t and "+" in t), t
            pg.fill("#nl-q", "72"); pg.wait_for_timeout(300); t = pg.inner_text("#nl-res"); assert "2 ×" in t and "HCl" in t, t
            pg.fill("#nl-q", "28"); pg.wait_for_timeout(300); assert pg.locator("#nl-list .nlr.hit").count() == 2, pg.locator("#nl-list .nlr.hit").count()
            pg.fill("#nl-q", "")
        step("4.5: Cerca Δm: 62 = H2O + CO2, 72 = 2 x HCl, 28 highlights two rows", search)
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
            assert row.locator(".nlm").inner_text() == "35" and "•Cl" in row.inner_text() and "aromatico" in row.inner_text()
            pg.locator('#nl-list [data-d="Cl"]').click(); d = pg.inner_text('#nl-list [data-f="Cl"] .nldet'); assert "34.9689" in d and "HCl (36)" in d and "37Cl" in d and "radicale" in d, d
            # Help button in header
            assert pg.is_visible("#sb-help") and pg.locator("#sb-help").get_attribute("data-help") == "losses"
            pg.click("#sb-help"); pg.wait_for_timeout(200)
            assert pg.is_visible("#helppop") and "cella di collisione" in pg.inner_text("#helppop")
            pg.click("#helppop .x")
            pg.fill("#nl-q", "62"); pg.wait_for_timeout(300); assert "H2O" in pg.inner_text("#nl-res").replace("H₂O", "H2O"); pg.fill("#nl-q", ""); shut(pg, "#refx")
        step("A2: compact view, ESI+/ESI- toggles, •Cl, help in header", a2)
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
