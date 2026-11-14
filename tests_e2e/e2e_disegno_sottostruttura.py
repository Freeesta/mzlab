"""Disegno, substructure search (`subdb/1` of the molkit .wasm): the right-click row of the whole drawing (or of a selection) opens the search card;
the catalogue gives candidates (never identifications), «Show» draws one with its matched atoms selected and «Back» restores the drawing; a file the user
picks (a synthetic SMILES list) is searched in memory, an unreadable entry is counted; the data table (TDATI) lists every candidate.
Run from the repo root after tools/build_site.py and tools/build_feat_wasm.sh site/static (molkit); SKIP without molkit there."""
# covers: mzlab/web/disegno-evidenza.js
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
MF = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")
if not os.path.exists(MF) or "molkit" not in json.load(open(MF, encoding="utf-8")):
    print("SKIP no molkit in site/static/wasm-feat (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8997, wd="/tmp/wd_disegno_sottostruttura")
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
OPEN = """(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"""
LIST = "# synthetic list\nOc1ccccc1 phenol-A\nCCO ethanol-B\nC1CC broken-D\nOc1ccc(O)cc1 hydroquinone-E\n"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready()) && TPDraw.menu.ids().includes('sub-find-whole')", timeout=60000)

        def menu_row(row):
            """opens the canvas menu until it lists `row`, returns its locator"""
            for _ in range(20):                                          # one right click at a time: a newer one replaces the menu of an older one still filling
                pg.evaluate(OPEN)
                try: pg.wait_for_selector(f"#ctx:not([hidden]) [data-row={row}]", timeout=3000); break
                except Exception: pass
            return pg.locator(f"#ctx [data-row={row}]")

        def search():
            busy = pg.evaluate("I18N.t('draw.sub.busy')")
            pg.click("#sub-run")
            pg.wait_for_function(f"(() => {{ const s = document.querySelector('#sub-status').textContent; return s && s !== {json.dumps(busy)}; }})()", timeout=60000)
            return pg.inner_text("#sub-status")

        def whole_catalogue():
            pg.evaluate(f"{KJ}.setMolecule('c1ccccc1O')"); frames(pg, 4)
            assert pg.locator("#sub-card").count() == 0                                          # the card is made on first use
            row = menu_row("sub-find-whole")
            assert row.count() == 1 and pg.locator("#ctx [data-row=sub-find-sel]").count() == 0, pg.inner_text("#ctx")
            row.click()
            assert pg.locator("#sub-card").is_visible()
            assert pg.inner_text("#sub-query") == pg.evaluate("I18N.t('draw.sub.query.whole', {n: 7})"), pg.inner_text("#sub-query")
            pg.wait_for_function("document.activeElement && document.activeElement.id === 'sub-run'", timeout=5000)   # focused after the fold header
            text = search()
            assert pg.get_attribute("#sub-status", "role") == "status"
            n = pg.locator("#sub-list li").count()
            assert n > 0 and text.startswith(pg.evaluate("I18N.t('draw.sub.found', {n: '', total: '', list: ''})").split(":")[0]), text
            assert "acetaminophen" in pg.inner_text("#sub-list"), pg.inner_text("#sub-list")[:300]
            show = pg.locator("#sub-list li .sub-show").first
            bb = show.bounding_box(); assert bb["width"] >= 24 and bb["height"] >= 24, bb
            assert show.get_attribute("aria-label"), "Show has no aria-label"
        step("whole drawing: menu row, card, catalogue search lists candidates (phenol -> acetaminophen)", whole_catalogue)

        def show_and_back():
            li = pg.locator("#sub-list li").filter(has_text="acetaminophen").first
            pg.evaluate("window.__cleared = []; document.addEventListener('mzlab-ev-cleared', e => window.__cleared.push(e.detail.atoms))")
            li.locator(".sub-show").click()
            pg.wait_for_function("!document.querySelector('#sub-back').hidden", timeout=20000)
            n = pg.evaluate(f"{KJ}.editor.struct().atoms.size")
            sel = pg.evaluate(f"({KJ}.editor.selection() || {{}})")
            assert n == 11 and len(sel.get("atoms", [])) == 7 and len(sel.get("bonds", [])) == 7, (n, sel)
            painted = f"{KJ}.editor.highlights.getAll().map(x => x.highlight || x).map(x => [(x.atoms || []).length, (x.bonds || []).length])"
            pg.wait_for_function(f"JSON.stringify({painted}) === '[[7,7]]'", timeout=20000)    # the moiety coloured too (disegno-evidenza.js)
            assert pg.locator("#ev-card").is_visible()
            pg.click("#sub-back")
            pg.wait_for_function("document.querySelector('#sub-back').hidden", timeout=20000)
            pg.wait_for_function("window.__cleared.length > 0", timeout=20000)
            assert pg.evaluate("window.__cleared") == [7], pg.evaluate("window.__cleared")   # cleared by «Back» (the phenol), never by painting
            assert pg.evaluate(f"{painted}.length") == 0
            assert pg.evaluate(f"{KJ}.editor.struct().atoms.size") == 7
            assert pg.inner_text("#sub-status") == pg.evaluate("I18N.t('draw.sub.restored')")
        step("«Show» draws acetaminophen with the 7 matched atoms and 7 bonds selected and coloured; «Back» restores the phenol, uncoloured", show_and_back)

        def data_table():
            pg.click("#sub-table")
            pg.wait_for_selector("dialog[open] table, .modal table", timeout=10000)
            cap = pg.evaluate("I18N.t('draw.sub.table.caption', {list: I18N.t('draw.sub.source.cat')})")
            assert pg.locator("table caption").filter(has_text=cap).count() >= 1
            pg.keyboard.press("Escape")
        step("the data table (TDATI) lists the candidates", data_table)

        def own_list_and_selection():
            pg.evaluate(f"{KJ}.setMolecule('c1ccccc1OC')"); frames(pg, 4)
            pg.evaluate(f"(() => {{ const K = {KJ}; K.editor.selection({{ atoms: [...K.editor.struct().atoms.keys()].slice(0, 7) }}); }})()")
            row = menu_row("sub-find-sel")
            assert row.count() == 1 and pg.locator("#ctx [data-row=sub-find-whole]").count() == 0, pg.inner_text("#ctx")
            row.click()
            assert pg.inner_text("#sub-query") == pg.evaluate("I18N.t('draw.sub.query.sel', {n: 7})"), pg.inner_text("#sub-query")
            pg.set_input_files("#sub-file", files=[{"name": "list.smi", "mimeType": "text/plain", "buffer": LIST.encode()}])
            pg.wait_for_function("document.querySelector('#sub-fname').textContent === 'list.smi'", timeout=10000)
            assert pg.is_checked('input[name="sub-src"][value="file"]')
            text = search()
            assert text.startswith(pg.evaluate("I18N.t('draw.sub.found', {n: 2, total: 4, list: 'list.smi'})")), text
            assert pg.evaluate("I18N.t('draw.sub.unread', {n: 1})") in text, text
            names = pg.eval_on_selector_all("#sub-list li b", "bs => bs.map(b => b.textContent)")
            assert names == ["phenol-A", "hydroquinone-E"], names
        step("selection + own SMILES list (in memory): 2 candidates of 4, one unreadable entry counted", own_list_and_selection)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if not steps or any(s[1] != "ok" for s in steps) else 0)
