"""«What can this number be?» under the floating Calculator (Disegno), the Adducts and the Neutral losses tabs of Adducts & losses (no data file needed):
groups for 214 / 30 / 18 / 100+114 / 21.98, resolution detected from the number (no switch), polarity, no observed field, no layout jump, cross-links, keystroke time, no engine miss."""
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
        attendi(pg, "window.qqReady && !document.querySelector('#start').hidden"); pg.evaluate("setView('draw')")
        attendi(pg, "!!document.querySelector('#np-calc2') && !!document.querySelector('#np-calc2').offsetParent")
        pg.click("#np-calc2")
        attendi(pg, "!document.querySelector('#calcdlg').hidden && typeof CALCQUI !== 'undefined' && CALCQUI.calc")
        def type_(v):
            pg.fill("#calcin", v)
            pg.evaluate("document.querySelector('#calcin').dispatchEvent(new Event('input',{bubbles:true}))")
            frames(pg, 2); frames(pg, 25)
        def groups(): return pg.evaluate("[...document.querySelectorAll('#calcout > details.cq-g, #calcout > details')].map(d => d.dataset.g || d.id)")
        def txt(): return pg.inner_text("#calcout")
        def mode(): return pg.get_attribute("#calcctl .cq-ctl", "data-mode")       # the detected resolution: "hr" or "lr" (no text line, a data attribute)
        def msrc(): return pg.get_attribute("#calcctl .cq-ctl", "data-src")          # where it comes from: files, formula, decimals, short, none
        def mname(): return pg.get_attribute("#calcctl .cq-ctl", "data-name")
        def height(): return pg.evaluate("document.documentElement.scrollHeight")
        def auto_default():
            assert pg.locator("#calcctl [data-r]").count() == 0 and pg.locator("#calcobs").count() == 0, "no High/Low switch, no observed field"
            assert mode() == "lr" and msrc() not in ("files", "formula", "decimals"), (mode(), msrc())
            assert pg.is_visible("#calcctl") and pg.is_visible("#calcout")
        step("no file: low resolution by default, said in small text; no switch, no observed field", auto_default)
        def lr_214():
            type_("214")
            assert mode() == "lr" and msrc() == "short", (mode(), msrc())
            assert pg.get_attribute("#calcctl [data-u=da]", "aria-pressed") == "true" and pg.get_attribute("#calcctl [data-u=ppm]", "aria-pressed") == "false", "Da preset in low resolution"
            assert pg.input_value("#calctol") == "0.5"
        step("214: low resolution from the number, 0.5 Da preset", lr_214)
        def unit_switch():
            type_("214"); n_da = pg.evaluate("document.querySelectorAll('#calcout .cq-g').length")
            pg.click("#calcctl [data-u=ppm]"); frames(pg, 25)
            assert pg.input_value("#calctol") == "5" and pg.get_attribute("#calcctl [data-u=ppm]", "aria-pressed") == "true", "ppm: its own value (5)"
            assert pg.evaluate("CALCQUI.opts(CALCQUI.arg('214',0)).unit") == "ppm" and pg.evaluate("CALCQUI.opts(CALCQUI.arg('214.0896',0)).unit") == "ppm"
            pg.evaluate("CALCQUI.set({unit: {hr: null, lr: 'da'}})"); frames(pg, 25)            # the same unit again: the same groups as before
            assert pg.input_value("#calctol") == "0.5" and pg.evaluate("document.querySelectorAll('#calcout .cq-g').length") == n_da
            assert pg.evaluate("JSON.parse(localStorage.getItem(NOMI.KEY('calcqui'))).unit.lr") == "da", "remembered in the settings"
        step("the Da / ppm selector switches the unit, keeps each value and remembers the choice per mode", unit_switch)
        def lr_30():
            type_("30"); assert "CH2O" in txt() or "CH\u2082O" in txt(), txt()[:300]
        step("30 lists CH2O among the neutral losses", lr_30)
        def lr_18():
            type_("18"); assert "perdite" in str(groups()), groups(); assert "H2O" in txt() or "H\u2082O" in txt()
        step("18 lists water", lr_18)
        def sum_():
            type_("100+114"); assert pg.inner_text("#calcsum") is not None
            assert pg.evaluate("CALCQUI.arg(document.querySelector('#calcin').value, 0).v") == 214
        step("100+114: the argument is the result 214", sum_)
        def hr_diff():
            type_("21.98")
            assert mode() == "lr", mode()
            type_("21.9819")
            assert mode() == "hr" and msrc() == "decimals", mode()
            assert pg.input_value("#calctol") == "5" and pg.get_attribute("#calcctl [data-u=ppm]", "aria-pressed") == "true", "ppm preset in high resolution"
            assert "Na" in txt() and "H" in txt(), txt()[:300]
        step("21.9819 (3+ decimals): high resolution detected, ppm tolerance, the difference Na - H", hr_diff)
        def formula_mode():
            type_("C9H10N2O"); assert mode() == "hr" and msrc() == "formula", mode()
        step("a formula is searched in high resolution", formula_mode)
        def pol():
            P = "[...document.querySelectorAll('#calcpol button')].map(b => b.dataset.p + ':' + b.getAttribute('aria-pressed')).join(' ')"
            pg.click("#calcpol button[data-p='-1']"); frames(pg, 25)
            assert pg.evaluate(P) == "1:true -1:true" and pg.evaluate("CALCQUI.pol()") == 0, pg.evaluate(P)
            pg.click("#calcpol button[data-p='1']"); frames(pg, 25)
            assert pg.evaluate(P) == "1:false -1:true" and pg.evaluate("CALCQUI.pol()") == -1, pg.evaluate(P)
            pg.click("#calcpol button[data-p='-1']"); frames(pg, 25)
            assert pg.evaluate(P) == "1:true -1:false", "switching off the last one switches on the other"
        step("ESI+ and ESI− toggle on their own: both can be on, one always is", pol)
        def files_decide():
            pg.evaluate("CALCQUI.follow(() => ({ hr: false, name: 'QTRAP 3200' }), () => 1)"); type_("21.9819")
            assert mode() == "lr" and "QTRAP 3200" in mname() and pg.input_value("#calctol") == "0.5", (mode(), mname())
            pg.evaluate("CALCQUI.follow(() => ({ hr: true, name: 'Orbitrap Exploris 120' }), () => 1)"); type_("214")
            assert mode() == "hr" and "Exploris" in mname() and pg.input_value("#calctol") == "5", (mode(), mname())
            pg.evaluate("CALCQUI.follow(() => null, () => 1)"); type_("")
        step("an open file decides the resolution and names the instrument", files_decide)
        def no_jump():
            type_("")
            ys = {}
            for v in ("", "214", "214.0896", "C9H10N2O", "100+114"):
                type_(v); ys[v] = pg.evaluate("[document.querySelector('#calcsum'), document.querySelector('#calcpad'), document.querySelector('#calctape')].map(e => e.getBoundingClientRect().top)")
            assert all(abs(a - b) < 1 for a, b in zip(ys[""], ys["214"])) and all(abs(a - b) < 1 for a, b in zip(ys[""], ys["214.0896"])), ys
        step("typing a number moves nothing above the groups (the result row has its space from the start)", no_jump)
        def timing():
            type_("")
            t = pg.evaluate("""async () => { const el = document.querySelector('#calcin'); const ts = [];
              for (const v of ['2','21','214','2143','214.1']) { el.value = v; const t0 = performance.now(); el.dispatchEvent(new Event('input', {bubbles:true})); await CALCQUI.render(CALCQUI.calc); ts.push(performance.now() - t0); }
              return ts }""")
            assert max(t) <= 30, t
        step("search time per keystroke <= 30 ms", timing)
        def apri():
            pg.evaluate("CALCQUI.apri(214)"); attendi(pg, "BARRA.curTab === 'addloss' && !!document.querySelector('#calcin').offsetParent")
            assert pg.input_value("#calcin") == "214" and pg.evaluate("BARRA.curSub") == "losses"
            pg.evaluate("BARRA.close(false); setView('draw')"); attendi(pg, "!!document.querySelector('#np-calc2').offsetParent")
            if not pg.is_visible("#calcdlg"): pg.click("#np-calc2")
        step("CALCQUI.apri(214) puts the number in the field of Adducts & losses (Dati view)", apri)
        def shot(name, w):
            if not SHOTS: return
            pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 25)
            pg.screenshot(path=f"{SHOTS}/{name}_{w}.png")
        for w in (320, 480): shot("calc", w)
        pg.set_viewport_size({"width": 1500, "height": 2200})
        # the Adducts panel
        def adducts():
            pg.click("#np-addloss"); attendi(pg, "document.querySelector('#refdlg') && !document.querySelector('#refdlg').hidden || document.querySelector('#sb-panel-adducts') && !document.querySelector('#sb-panel-adducts').hidden")
            q = pg.query_selector("#calcin")
            assert q
            q.fill("21.98"); frames(pg, 25)
            assert "Na" in pg.inner_text("body")
        step("Adducts panel opens and answers a number", adducts)
        def to_loss():
            pg.click("#reftitle ~ *", timeout=1000) if False else None
            pg.click("#np-addloss") if not pg.evaluate("BARRA.curTab==='addloss'") else None; pg.click("#al-t-losses"); attendi(pg, "!document.querySelector('#sb-panel-losses').hidden && document.querySelector('#nl-res')")
            pg.fill("#calcin", "30"); frames(pg, 25)
            t = pg.inner_text("#sb-panel-losses"); assert "CH2O" in t or "CH₂O" in t, t[:300]
            assert pg.locator("#al-ctl [data-r]").count() == 0 and pg.get_attribute("#al-ctl .cq-ctl", "data-mode") == "lr"
        step("Neutral losses panel: shared groups for 30", to_loss)
        def link():
            pg.click("#al-t-adducts"); frames(pg, 25)
            assert pg.evaluate("!document.querySelector('#sb-panel-adducts').hidden")
            assert pg.input_value("#calcin") == "30", pg.input_value("#calcin")
        step("one field for the tabs: Perdite -> Addotti keeps the number", link)
        def nolr():
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
        step("no engine miss", nolr)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if any(x[1] != "ok" for x in steps) else 0)
