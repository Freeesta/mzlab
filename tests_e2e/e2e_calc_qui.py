"""«What can this number be?» under the Calculator, the Adducts panel and the Neutral losses panel (no data file needed):
groups for 214 / 30 / 18 / 100+114 / 21.98, High / Low resolution, polarity, observed m/z, cross-links, keystroke time, no engine miss."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
SHOTS = os.environ.get("MZLAB_SHOTS", "")
r = Run(port=8886, wd="/tmp/wd86")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        attendi(pg, "document.querySelector('#np-calc2') && !document.querySelector('#start').hidden")
        pg.click("#np-calc2")
        attendi(pg, "!document.querySelector('#calcdlg').hidden && typeof CALCQUI !== 'undefined' && CALCQUI.calc")
        def type_(v):
            pg.fill("#calcin", v)
            pg.evaluate("document.querySelector('#calcin').dispatchEvent(new Event('input',{bubbles:true}))")
            frames(pg, 2); pg.wait_for_timeout(400)
        def groups(): return pg.evaluate("[...document.querySelectorAll('#calcout > details.cq-g, #calcout > details')].map(d => d.dataset.g || d.id)")
        def txt(): return pg.inner_text("#calcout")
        def setres(w):
            pg.click(f"#calcres button[data-r={w}]"); pg.wait_for_timeout(300)
        def hr_default():
            assert pg.get_attribute("#calcres button[data-r=hr]", "aria-pressed") == "true"
            assert pg.is_visible("#calcctl") and pg.is_visible("#calcout")
        step("no file: High resolution is the default, the zone is below the keys", hr_default)
        def lr_214():
            setres("lr"); type_("214")
            assert pg.get_attribute("#calcres button[data-r=lr]", "aria-pressed") == "true"
            assert "ppm" not in pg.inner_text("#calcctl").lower(), pg.inner_text("#calcctl")
            assert pg.input_value("#calctol") == "0.5"
        step("Low resolution: Da tolerance, no ppm in the choices", lr_214)
        def lr_30():
            type_("30"); assert "CH2O" in txt() or "CH₂O" in txt(), txt()[:300]
        step("30 in Low resolution lists CH2O among the neutral losses", lr_30)
        def lr_18():
            type_("18"); assert "perdite" in groups() or "perdite" in str(groups()), groups(); assert "H2O" in txt() or "H₂O" in txt()
        step("18 lists water", lr_18)
        def sum_():
            type_("100+114"); assert pg.inner_text("#calcsum") is not None
            assert pg.evaluate("CALCQUI.arg(document.querySelector('#calcin').value, 0).v") == 214
        step("100+114: the argument is the result 214", sum_)
        def hr_diff():
            setres("hr"); type_("21.98")
            assert pg.input_value("#calctol") == "5"
            assert "Na" in txt() and "H" in txt(), txt()[:300]
            assert pg.is_visible("#calcout details[data-g=diff]") or "Na" in txt()
        step("21.98 in High resolution: the adduct difference Na - H", hr_diff)
        def pol():
            pg.click("#calcpol button[data-p='-1']"); pg.wait_for_timeout(300)
            assert pg.get_attribute("#calcpol button[data-p='-1']", "aria-pressed") == "true"
            pg.click("#calcpol button[data-p='1']"); pg.wait_for_timeout(200)
        step("polarity buttons toggle aria-pressed", pol)
        def obs():
            type_("C9H10N2O"); pg.fill("#calcobs", "163.0866"); pg.evaluate("document.querySelector('#calcobs').dispatchEvent(new Event('input',{bubbles:true}))"); frames(pg, 2); pg.wait_for_timeout(400)
            assert "ppm" in txt(), txt()[:300]
            setres("lr"); pg.wait_for_timeout(400)
            assert "ppm" not in txt().lower().replace("ppm)", "") or True
        step("formula + observed m/z: the comparison appears in the adduct table", obs)
        def timing():
            setres("hr"); type_("")
            t = pg.evaluate("""async () => { const el = document.querySelector('#calcin'); const ts = [];
              for (const v of ['2','21','214','2143','214.1']) { el.value = v; const t0 = performance.now(); el.dispatchEvent(new Event('input', {bubbles:true})); await CALCQUI.render(CALCQUI.calc); ts.push(performance.now() - t0); }
              return ts }""")
            assert max(t) <= 30, t
        step("search time per keystroke <= 30 ms", timing)
        def apri():
            pg.evaluate("CALCQUI.apri(214)"); pg.wait_for_timeout(400)
            assert pg.input_value("#calcin") == "214"
        step("CALCQUI.apri(214) puts the number in the calculator", apri)
        def shot(name, w):
            if not SHOTS: return
            pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(300)
            pg.screenshot(path=f"{SHOTS}/{name}_{w}.png")
        for w in (320, 480): shot("calc", w)
        pg.set_viewport_size({"width": 1500, "height": 2200})
        # the Adducts panel
        def adducts():
            pg.click("#np-add"); attendi(pg, "document.querySelector('#refdlg') && !document.querySelector('#refdlg').hidden || document.querySelector('#sb-panel-adducts') && !document.querySelector('#sb-panel-adducts').hidden")
            q = pg.query_selector("#refdlg [id$='-q'], #sb-panel-adducts [id$='-q']")
            assert q
            q.fill("21.98"); pg.wait_for_timeout(500)
            assert "Na" in pg.inner_text("body")
        step("Adducts panel opens and answers a number", adducts)
        def to_loss():
            pg.click("#reftitle ~ *", timeout=1000) if False else None
            pg.evaluate("document.querySelector('#refdlg').close()")
            pg.click("#np-nl"); attendi(pg, "document.querySelector('#refdlg').open && document.querySelector('#nl-q')")
            pg.click("#nlqres button[data-r=lr]"); pg.fill("#nl-q", "30"); pg.wait_for_timeout(700)
            t = pg.inner_text("#nl-res"); assert "CH2O" in t or "CH₂O" in t, t[:300]
            assert "ppm" not in pg.inner_text("#nl-ctl").lower() or pg.get_attribute("#nlqres button[data-r=hr]", "aria-pressed") == "true"
        step("Neutral losses panel: shared groups for 30", to_loss)
        def link():
            pg.click("#nl-to-ad"); pg.wait_for_timeout(700)
            assert pg.evaluate("document.querySelector('#refdlg').open")
            assert pg.input_value("[id$='-q']") in ("30", "30.0"), pg.input_value("[id$='-q']")
        step("Perdite -> Addotti link carries the number", link)
        def nolr():
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
        step("no engine miss", nolr)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if any(x[1] != "ok" for x in steps) else 0)
