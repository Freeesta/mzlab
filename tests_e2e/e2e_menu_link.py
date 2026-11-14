"""Disegno: the PubChem, ChEBI and BioTransformer rows of the canvas menu (`link/1` of the Rust graph), in two submenus: «PubChem» (the exact structure
first, then the engine's order) and «Altri siti» (ChEBI, BioTransformer); a submenu opens on hover, click or Right arrow (aria-haspopup, aria-expanded),
Left arrow and Esc come back to its row, rows are at least 24 px high; every row opens its URL with window.open(url, "_blank", "noopener,noreferrer") only on the click,
PubChem carries the encoded SMILES in `#query=` (and `&tab=`) and copies it too, BioTransformer copies it (toast, role=status), ChEBI carries the percent-encoded InChI, and the page
makes no request to any outside host."""
# covers: mzlab/web/disegno-menu.js, mzlab/web/disegno-link.js
import sys, os
from urllib.parse import quote, urlparse
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=9141, wd="/tmp/wd_menu_link")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        outside = []
        pg.on("request", lambda q: outside.append(q.url) if urlparse(q.url).hostname not in ("127.0.0.1", "localhost", None) and not q.url.startswith(("data:", "blob:")) else None)
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        pg.evaluate("""window._cp = []; navigator.clipboard.writeText = t => { window._cp.push(t); return Promise.resolve(); };
          window._open = []; window.open = (...a) => { window._open.push(a); return null; }; true""")
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.addFragment('CC(=O)Nc1ccc(F)cc1')"); wait_for_js(pg, f"!{KJ}.editor.struct().isBlank()")
        OPEN = """(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"""
        def menu(bond):
            js = "null" if bond is None else "{bonds:[ids[%d]]}" % bond
            for _ in range(40):                                           # under load Ketcher may need a moment before the menu answers
                pg.evaluate("document.querySelector('#ctx').hidden = true")
                pg.evaluate(f"(()=>{{const K={KJ};const ids=[...K.editor.struct().bonds.keys()];K.editor.selection({js})}})()"); pg.evaluate(OPEN)
                try: wait_for_js(pg, "!document.querySelector('#ctx').hidden && !!document.querySelector('#ctx div[data-row=link-biotransformer]')", timeout=3000); break
                except Exception: pass
            return pg.evaluate("[...document.querySelectorAll('#ctx div[data-row^=link-]')].map(d => [d.dataset.row, d.textContent, d.title])")
        def click(row):
            pg.evaluate("window._cp = []; window._open = []")
            par = pg.evaluate(f"document.querySelector('#ctx div[data-row={row}]').parentElement.previousElementSibling.dataset.row")
            pg.hover(f"#ctx div[data-row={par}]"); pg.locator(f"#ctx div[data-row={row}]").wait_for(state="visible", timeout=3000)
            pg.locator(f"#ctx div[data-row={row}]").click()
            wait_for_js(pg, "window._open.length === 1", timeout=5000)
            return pg.evaluate("window._open[0]"), pg.evaluate("window._cp")
        def whole():
            rows = menu(None)
            assert [x[0] for x in rows] == ["link-pubchem", "link-pubchem-exact", "link-pubchem-similar", "link-pubchem-sub", "link-other", "link-chebi", "link-biotransformer"], rows
            assert rows[0][1] == "PubChem" and rows[1][1] == "Questa struttura (esatta)" and "pubchem.ncbi.nlm.nih.gov" in rows[1][2], rows[:2]
            assert not pg.evaluate("window._open.length"), "nothing opens before a click"
            (url, tgt, feat), cp = click("link-pubchem-exact")
            assert (tgt, feat) == ("_blank", "noopener,noreferrer") and url.startswith("https://pubchem.ncbi.nlm.nih.gov/#query=") and "tab=" not in url, url
            assert cp and url.split("#query=")[1] == quote(cp[0], safe=""), (url, cp)
            assert cp == ["CC(=O)Nc1ccc(F)cc1"] or (len(cp) == 1 and "F" in cp[0] and "N" in cp[0]), cp
            wait_for_js(pg, "[...document.querySelectorAll('#toast-container .toast-msg')].some(t => t.textContent.includes('ricerca per identità'))")
            assert pg.get_attribute("#toast-container", "role") == "status"
        step("whole drawing: exact PubChem first, the click copies the SMILES and opens a new tab without opener", whole)
        def chebi():
            menu(None)
            (url, tgt, feat), cp = click("link-chebi")
            assert url.startswith("https://www.ebi.ac.uk/chebi/search?query=InChI%3D1S%2FC8H8FNO%2F") and "/" not in url.split("query=")[1], url
            assert feat == "noopener,noreferrer" and cp == [], (feat, cp)
        step("ChEBI: the InChI from Indigo, percent-encoded, nothing copied", chebi)
        def bio():
            menu(None)
            (url, tgt, feat), cp = click("link-biotransformer")
            assert (url, feat) == ("https://biotransformer.ca/new", "noopener,noreferrer") and len(cp) == 1, (url, cp)
            wait_for_js(pg, "[...document.querySelectorAll('#toast-container .toast-msg')].some(t => t.textContent === 'SMILES copiato: incollalo nel riquadro della struttura')")
        step("BioTransformer: SMILES copied, its prediction page opened, the toast says where to paste", bio)
        def part():
            rows = menu(0)                                                # the first bond: methyl and carbonyl carbon
            assert [x[0] for x in rows[:4]] == ["link-pubchem", "link-pubchem-exact", "link-pubchem-sub", "link-pubchem-similar"] and rows[2][1] == "Sottostruttura", rows   # exact first here too
            (url, tgt, feat), cp = click("link-pubchem-sub")
            assert len(cp) == 1 and cp[0] != "CC(=O)Nc1ccc(F)cc1", cp
            assert url == "https://pubchem.ncbi.nlm.nih.gov/#query=" + quote(cp[0], safe="") + "&tab=substructure", url
        step("a selected piece: exact structure first, then the substructure search", part)
        def keys():
            menu(None)
            par = "#ctx div[data-row=link-pubchem]"
            assert pg.get_attribute(par, "aria-haspopup") == "menu" and pg.get_attribute(par, "aria-expanded") == "false"
            assert pg.evaluate("document.querySelector('#ctx div[data-row=link-pubchem]').nextElementSibling.getAttribute('role')") == "menu"
            pg.focus(par); pg.keyboard.press("ArrowRight")
            wait_for_js(pg, "document.activeElement.dataset.row === 'link-pubchem-exact'"); assert pg.get_attribute(par, "aria-expanded") == "true"
            pg.keyboard.press("ArrowDown"); assert pg.evaluate("document.activeElement.dataset.row") == "link-pubchem-similar"
            pg.keyboard.press("ArrowLeft"); assert pg.evaluate("document.activeElement.dataset.row") == "link-pubchem" and pg.get_attribute(par, "aria-expanded") == "false"
            pg.keyboard.press("ArrowDown"); pg.keyboard.press("Enter"); wait_for_js(pg, "document.activeElement.dataset.row === 'link-chebi'")
            pg.keyboard.press("Escape"); assert pg.evaluate("document.activeElement.dataset.row") == "link-other" and pg.locator("#ctx").is_visible()
            pg.click(par); wait_for_js(pg, "!document.querySelector('#ctx div[data-row=link-pubchem]').nextElementSibling.hidden"); assert pg.locator("#ctx").is_visible()   # a click (a tap) opens it, the menu stays
            hs = pg.evaluate("[...document.querySelectorAll('#ctx [role=menuitem]')].filter(e => e.offsetParent).map(e => e.getBoundingClientRect().height)")
            assert hs and min(hs) >= 24, hs
            pg.keyboard.press("Escape"); pg.keyboard.press("Escape"); assert pg.locator("#ctx").is_hidden()
        step("submenus: aria-haspopup/expanded, Right opens on the exact search, Left and Esc come back, a click opens, rows of 24 px", keys)
        def quiet():
            assert not outside, outside[:5]
        step("no request from the page to any outside host", quiet)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
