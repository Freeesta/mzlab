"""Search box (palette, Ctrl/⌘K): a number is matched in Rust against the neutral losses, the adducts and the lab contaminants (the chip says
«compatible», never a verdict); a formula gives its mass and ions; a word gives commands and Teoria. Keyboard, it/en, axe, 400 px, no engine miss."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ricerca.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
AXE = axe_path()
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))"""
r = Run(port=8902, wd="/tmp/wd_ricerca")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        if AXE: pg.add_script_tag(path=AXE)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")])
        pg.click("#opbtn"); ready(pg)
        def opn():
            pg.evaluate("document.activeElement && document.activeElement.blur()"); pg.keyboard.press("Control+k"); pg.wait_for_selector("#paletta:not([hidden])")
        txt = lambda: pg.inner_text("#pal-l")
        def typ(q, want=None):
            pg.fill("#pal-in", q)
            if want: attendi(pg, f"document.querySelector('#pal-l').innerText.includes({want!r})")
        def empty():
            opn(); t = txt(); assert "Prova: 18, C6H12O6+Na, smoothing" in t, t
            assert pg.locator("#pal-l li[role=option]").count() <= 12, "no endless command list"
            pg.keyboard.press("Escape")
        step("empty field: a short list and the line «Prova»", empty)
        def number():
            opn(); typ("18", "H2O"); t = txt()
            assert "Perdita" in t and "Addotto" in t, t
            assert "Δ" not in t, "low resolution: compatibility only, no decimals"
            assert pg.evaluate("window.MZLAB_LR_MISS || []") == [], pg.evaluate("window.MZLAB_LR_MISS")
        step("18: a neutral loss and an adduct are compatible (LR, no deltas)", number)
        def keys():
            n0 = pg.get_attribute("#pal-in", "aria-activedescendant"); pg.keyboard.press("ArrowDown")
            assert pg.get_attribute("#pal-in", "aria-activedescendant") != n0 and pg.locator("#pal-l [aria-selected=true]").count() == 1
            pg.keyboard.press("ArrowUp"); pg.keyboard.press("Enter"); attendi(pg, "document.querySelector('#paletta').hidden")
            attendi(pg, "BARRA.curTab === 'losses'"); assert pg.input_value("#nl-q") == "18", "the loss panel opens with the number"; attendi(pg, "document.activeElement.id === 'nl-q'"); pg.evaluate("BARRA.close(false)")
        step("↑↓ and Enter open the panel of the first result", keys)
        def esc():
            opn(); typ("18", "Perdita"); pg.keyboard.press("Escape"); attendi(pg, "document.querySelector('#paletta').hidden")
            assert not pg.is_visible("#paletta"), (pg.evaluate("document.activeElement.id||document.activeElement.tagName"), pg.evaluate("document.querySelector('#paletta').hidden"))
        step("Esc closes", esc)
        def formula():
            opn(); typ("C6H12O6", "180"); t = txt(); assert "Formula" in t, t
            typ("C6H12O6+Na", "203"); assert "203" in txt(), txt()
            pg.keyboard.press("Escape")
        step("formula: mass and ions, also with the adduct typed", formula)
        def word():
            opn(); typ("smoothing", "Smoothing"); assert pg.locator("#pal-l li[role=option]").count() >= 1
            typ("calcol", "Calcolatrice"); pg.keyboard.press("Escape")
        step("word: the commands", word)
        def axe_ok():
            if not AXE: return
            opn(); typ("18", "Perdita"); v = pg.evaluate(RUN, "#paletta"); assert not v, v; pg.keyboard.press("Escape")
        step("axe clean with results", axe_ok)
        def narrow():
            ctx = r.b.new_context(viewport={"width": 400, "height": 800}, locale="it-IT", reduced_motion="reduce")
            q = ctx.new_page(); q.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            q.goto(f"http://127.0.0.1:{r.port}/?telefono=0"); ready(q)
            try:
                stage(q, [EX / "FullScan_t0.mzML"]); q.click("text=Carica dati"); ready(q)
                q.click("#tb-search"); q.wait_for_selector("#paletta:not([hidden])"); q.fill("#pal-in", "18")
                attendi(q, "document.querySelector('#pal-l').innerText.includes('Addotto')")
                assert q.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth") <= 1, "no sideways scroll"
                assert q.evaluate("(()=>{const b=document.querySelector('#paletta .pal-b').getBoundingClientRect();return b.left>=0&&b.right<=innerWidth})()")
            finally:
                ctx.close()
        step("400 px: the box fits", narrow)
        def english():
            pg.evaluate("I18N.set('en')"); attendi(pg, "I18N.lang === 'en'"); ready(pg); opn(); typ("18", "loss")
            t = txt(); assert "loss" in t and "Adduct" in t, t
            pg.fill("#pal-in", ""); attendi(pg, "document.querySelector('#pal-l').innerText.includes('Try: 18')")
            pg.keyboard.press("Escape")
        step("English labels", english)
        step("no engine miss and no console error", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
finally:
    r.close()
for s in steps: print("  ", s)
print("--- BROWSER ERRORS", len(r.errs))
for e in r.errs: print(e)
bad = [e for e in r.errs if "escape its sandboxing" not in str(e)]   # Ketcher iframe, known
sys.exit(1 if any(s[1] != "ok" for s in steps) or bad else 0)
