"""Disegno: «Informazioni…» of the canvas menu opens a dialog with the `sel/1` numbers of the Rust graph (formula, monoisotopic mass, average weight,
rings with the aromatic ones, heavy atoms) and InChI/InChIKey from Indigo, for the whole drawing or the selected piece (with the note that open valences
are filled with H); every copy button copies its own value, and the dialog has no serious axe violation."""
import sys, os, re; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=9140, wd="/tmp/wd_menu_info")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        pg.evaluate("window._cp = []; navigator.clipboard.writeText = t => { window._cp.push(t); return Promise.resolve(); }")
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.addFragment('CC(=O)Nc1ccc(F)cc1')"); wait_for_js(pg, f"!{KJ}.editor.struct().isBlank()")
        OPEN = """(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"""
        def sel(bond):
            js = "null" if bond is None else "{bonds:[ids[%d]]}" % bond
            pg.evaluate(f"(()=>{{const K={KJ};const ids=[...K.editor.struct().bonds.keys()];K.editor.selection({js})}})()")
        def info(bond, title):
            pg.evaluate("document.querySelector('#bigdlg').open && document.querySelector('#bigdlg').close()")
            for _ in range(40):                                           # under load Ketcher may need a moment before the menu answers
                pg.evaluate("document.querySelector('#ctx').hidden = true"); sel(bond); pg.evaluate(OPEN)
                try: wait_for_js(pg, "!document.querySelector('#ctx').hidden && !!document.querySelector('#ctx div[data-row=info]')", timeout=1500); break
                except Exception: pass
            pg.locator("#ctx div[data-row=info]").click()
            wait_for_js(pg, f"document.querySelector('#bigdlg').open && document.querySelector('#bigtt').textContent === {title!r}", timeout=60000)
        cells = lambda: dict(pg.evaluate("[...document.querySelectorAll('#bigbody tr')].map(t => [t.querySelector('th').textContent, t.querySelector('td').textContent])"))
        def whole():
            info(None, "Informazioni: tutto il disegno")
            c = cells()
            assert (c["Formula"], c["Massa monoisotopica"], c["Anelli"], c["Atomi pesanti"], c["Carica"]) == ("C8H8FNO", "153.05899", "1 (aromatici: 1)", "11", "0"), c
            assert c["Peso molecolare medio"] == "153.16", c
            assert "m/z (con la carica disegnata)" not in c and not pg.locator("#info-part").count()
            wait_for_js(pg, "/^[A-Z]{14}-[A-Z]{10}-[A-Z]$/.test(document.querySelector('#info-inchikey').textContent)", timeout=60000)
            assert pg.inner_text("#info-inchi").startswith("InChI=1S/C8H8FNO/")
        step("whole drawing: formula, masses, rings and InChIKey of 4'-fluoroacetanilide", whole)
        def copies():
            pg.evaluate("window._cp = []")
            pg.locator("#bigbody tr:has(th:text-is('Formula')) button").click()
            pg.locator("#bigbody tr:has(th:text-is('Massa monoisotopica')) button").click()
            pg.locator("#bigbody tr:has(th:text-is('InChIKey')) button").click()
            pg.locator("#bigbody tr:has(th:text-is('Molfile')) button").click()
            cp = pg.evaluate("window._cp")
            assert cp[:2] == ["C8H8FNO", "153.05899"] and re.match(r"^[A-Z]{14}-[A-Z]{10}-[A-Z]$", cp[2]) and "V2000" in cp[3], cp
            assert pg.locator("#bigbody tr:has(th:text-is('Formula')) button").get_attribute("aria-label") == "Copia: Formula"
        step("copy buttons copy formula, mass, InChIKey and molfile", copies)
        def axe_dlg():
            if not axe_path(): raise AssertionError("axe-core not available (pip install axe-playwright-python)")
            if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=axe_path())
            v = pg.evaluate("""() => axe.run({include: [['#bigdlg']]}, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21aa','wcag22aa']}})
              .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))"""); assert not v, v
            pg.keyboard.press("Escape"); assert not pg.evaluate("document.querySelector('#bigdlg').open")
        step("axe: the dialog has no serious violation; Esc closes it", axe_dlg)
        def part():
            info(0, "Informazioni: selezione")                            # the first bond: the methyl and the carbonyl carbon, open valences filled with H
            c = cells()
            assert (c["Formula"], c["Anelli"], c["Atomi pesanti"]) == ("C2H6", "0 (aromatici: 0)", "2"), c
            assert "valenze aperte" in pg.inner_text("#info-part")
        step("a selected bond: the numbers of the piece, with the note on the open valences", part)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
