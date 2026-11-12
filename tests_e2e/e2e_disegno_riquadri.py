"""Disegno, side cards that fold (disegno-riquadri.js): on a first visit Quick draw, Expected properties and Export canvas come first and open, every other
card is folded to its title row; the chevron toggles a card by click, Enter and Space with aria-expanded and its aria-label; the choice survives a reload
(also for a <details> card); the explanations are tooltips and hidden descriptions, not paragraphs; a result opens its folded card; «Nascondi i riquadri»
and the Draw tour (the labels step opens its folded card and folds it again) still work."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
STATE = """[...document.querySelectorAll('#v-draw aside > .card:not([hidden])')].map(c => [c.id, c.tagName === 'DETAILS' ? c.open
  : c.querySelector('.rq-tg').getAttribute('aria-expanded') === 'true', c.getBoundingClientRect().height])"""
exp = lambda pg, cid: pg.get_attribute(f"#{cid} .rq-tg", "aria-expanded")
r = Run(port=8981, wd="/tmp/wd_disegno_riquadri")
try:
    with sync_playwright() as p:
        pg = r.page(p, draw_cards=True); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready()) && !!document.querySelector('#chk-card .rq-tg') && !!document.querySelector('#cip-card .rq-tg')", timeout=60000)
        def defaults():
            st = pg.evaluate(STATE); ids = [s[0] for s in st]
            assert ids[:3] == ["quick-card", "prop-card", "exp-card"] and all(s[1] for s in st[:3]), st
            assert not any(s[1] for s in st[3:]) and {"lb-card", "cut-card", "mech-card", "chk-card", "cip-card", "anal-card"} <= set(ids[3:]), st
            assert all(s[2] < 60 for s in st[3:]), st                                     # a folded card is its title row
            assert pg.is_visible("#ex-smi") and pg.is_visible("#ex-png") and pg.is_visible("#ex-link") and not pg.is_visible("#cut-line") and not pg.is_visible("#lb-f")
            assert pg.get_attribute("#quick-card .rq-tg", "aria-label") == "Comprimi il riquadro «Disegna veloce»"
            assert pg.get_attribute("#cut-card .rq-tg", "aria-label") == "Espandi il riquadro «Taglio dei legami»"
            b = pg.locator("#cut-card .rq-tg").bounding_box(); assert b["height"] >= 24 and b["width"] >= 24, b
        step("first visit: the three cards first and open, the rest folded", defaults)
        def no_prose():
            for cid, key in (("cut-card", "draw.cut.help"), ("mech-card", "draw.mech.help"), ("chk-card", "draw.check.intro"), ("cip-card", "draw.cip.intro")):
                txt = pg.evaluate(f"I18N.t('{key}')")
                assert pg.get_attribute(f"#{cid} .rq-h", "title").startswith(txt), cid
                d = pg.get_attribute(f"#{cid} .rq-tg", "aria-describedby").split()[0]
                assert pg.evaluate(f"document.getElementById('{d}').textContent") == txt and pg.evaluate(f"getComputedStyle(document.getElementById('{d}')).clip") != "auto", cid
            assert "non la risposta" in pg.get_attribute("#cut-card .rq-h", "title") and "non dice se il meccanismo" in pg.get_attribute("#mech-card .rq-h", "title")
            pg.click("#cut-card .rq-tg"); pg.click("#mech-card .rq-tg")
            vis = pg.evaluate("[...document.querySelectorAll('#cut-card-b p, #mech-card-b p')].filter(p => p.offsetHeight).length")
            assert vis == 0, vis
            pg.click("#cut-card .rq-tg"); pg.click("#mech-card .rq-tg")
        step("the explanations are header tooltips and hidden descriptions, never visible paragraphs", no_prose)
        def click():
            pg.click("#cut-card .rq-tg")
            assert exp(pg, "cut-card") == "true" and pg.is_visible("#cut-line") and pg.get_attribute("#cut-card .rq-tg", "aria-label").startswith("Comprimi")
            pg.click("#cut-card .rq-tg")
            assert exp(pg, "cut-card") == "false" and not pg.is_visible("#cut-line")
            pg.click("#anal-card summary"); wait_for_js(pg, "document.getElementById('anal-card').open")
        step("a click opens and folds a card; a <details> card opens from its title", click)
        def keyboard():
            pg.focus("#mech-card .rq-tg"); pg.keyboard.press("Enter")
            assert exp(pg, "mech-card") == "true" and pg.is_visible("#mech-arrow")
            pg.keyboard.press("Space"); assert exp(pg, "mech-card") == "false" and not pg.is_visible("#mech-arrow")
            pg.keyboard.press("Space"); assert exp(pg, "mech-card") == "true"
        step("Enter and Space toggle the focused card", keyboard)
        def reload():
            pg.click("#quick-card .rq-tg"); assert exp(pg, "quick-card") == "false"
            pg.reload(); wait_for_js(pg, "!!document.querySelector('#v-draw aside[data-rq]')", timeout=60000)
            assert exp(pg, "quick-card") == "false" and exp(pg, "mech-card") == "true" and exp(pg, "cut-card") == "false" and exp(pg, "prop-card") == "true"
            assert pg.evaluate("document.getElementById('anal-card').open") and pg.is_visible("#mech-arrow") and not pg.is_visible("#ex-smi")
            assert pg.get_attribute("#quick-card .rq-tg", "aria-label").startswith("Espandi")
            pg.click("#quick-card .rq-tg"); pg.click("#mech-card .rq-tg"); pg.click("#anal-card summary"); wait_for_js(pg, "!document.getElementById('anal-card').open")
        step("the choice is remembered after a reload", reload)
        def result():
            pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            assert exp(pg, "chk-card") == "false"
            pg.evaluate(f"{KJ}.setMolecule('CCO')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 3")
            pg.evaluate("import('./static/disegno-verifica.js').then(m => m.checkNow())")      # «Controlla»: its status line is the result
            wait_for_js(pg, "document.querySelector('#chk-card .rq-tg').getAttribute('aria-expanded') === 'true'", timeout=30000)
            pg.click("#chk-card .rq-tg"); assert exp(pg, "chk-card") == "false"
            pg.reload(); wait_for_js(pg, "!!document.querySelector('#v-draw aside[data-rq]')", timeout=60000)
            assert exp(pg, "chk-card") == "false"                                          # opened by a result, not by the student: not stored
        step("a result in a folded card opens it, without storing it", result)
        def hide_all():
            pg.click("#side-toggle"); wait_for_js(pg, "document.getElementById('v-draw').classList.contains('noside')")
            assert pg.evaluate("document.getElementById('v-draw').classList.contains('noside')") and not pg.is_visible("#quick-card")
            pg.click("#side-toggle"); wait_for_js(pg, "!document.getElementById('v-draw').classList.contains('noside')")
            assert pg.is_visible("#quick-card") and exp(pg, "quick-card") == "true" and exp(pg, "lb-card") == "false"
        step("«Nascondi i riquadri» still hides and shows them all, the folds kept", hide_all)
        def tour():
            pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            pg.evaluate("TOUR.disegno()"); wait_for_js(pg, "!!document.getElementById('tour-dialog')")
            for _ in range(3): pg.evaluate("TOUR.avanti()")
            wait_for_js(pg, "document.querySelector('#lb-card .rq-tg').getAttribute('aria-expanded') === 'true'")
            assert pg.evaluate("TOUR.active") and pg.inner_text("#tour-dialog h3"), "tour"
            assert exp(pg, "lb-card") == "true" and pg.is_visible("#lb-f"), "labels step: card not opened"
            pg.evaluate("TOUR.avanti()"); wait_for_js(pg, "document.querySelector('#lb-card .rq-tg').getAttribute('aria-expanded') === 'false'")
            assert exp(pg, "lb-card") == "false", "labels card not folded again"
            pg.keyboard.press("Escape"); wait_for_js(pg, "!TOUR.active")
            assert "lb-card" not in (pg.evaluate("localStorage.getItem('mzlab.disegno.schede')") or "")
        step("the tour opens the folded labels card for its step and folds it again", tour)
        def english():
            pg.goto(f"http://127.0.0.1:{r.port}/?lang=en#/draw"); wait_for_js(pg, "!!document.querySelector('#v-draw aside[data-rq]')", timeout=60000)
            assert pg.get_attribute("#cut-card .rq-tg", "aria-label") == "Expand the «Bond cuts» panel"
            assert "not the answer" in pg.get_attribute("#cut-card .rq-h", "title")
        step("English labels and tooltips", english)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
