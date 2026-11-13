"""Disegno: the canvas menu has no header line; its name (aria-label) says what it acts on («Selezione (n atomi)» or «Tutto il disegno»), its rows come from
the TPDraw.menu registry (ordered, filtered by selection / whole drawing, one failing row never hides the others) and stay keyboard menu items. No bond rows
(«Spezza», «Taglia») and no «Ioni attesi» row."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=9139, wd="/tmp/wd_menu_selezione")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.addFragment('CCO')"); wait_for_js(pg, f"!{KJ}.editor.struct().isBlank()")
        OPEN = """(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"""
        def sel(bond):                                                      # bond index to select, None = clear the selection
            js = "null" if bond is None else "{bonds:[ids[%d]]}" % bond
            pg.evaluate(f"(()=>{{const K={KJ};const ids=[...K.editor.struct().bonds.keys()];K.editor.selection({js})}})()")
        def opened(head, bond):
            for _ in range(40):                                           # under load Ketcher may need a moment before the menu answers
                pg.evaluate("document.querySelector('#ctx').hidden = true"); sel(bond); pg.evaluate(OPEN)
                try: wait_for_js(pg, f"(()=>{{const m=document.querySelector('#ctx');return !m.hidden&&m.getAttribute('aria-label')==={head!r}&&!!m.querySelector('[role=menuitem]')}})()", timeout=1500); return
                except Exception: pass
            raise AssertionError("menu named %r did not open: %r" % (head, pg.inner_text("#ctx")))
        # the outside lookups (submenu rows link-pubchem and link-other, order 17) come only with the grafo .wasm and are checked by e2e_menu_link: here they must sit between info and center
        def rows():
            rr = pg.evaluate("[...document.querySelectorAll('#ctx div[role=menuitem]')].map(d=>d.dataset.row)")
            ln = [i for i, x in enumerate(rr) if x.startswith("link-")]
            assert not ln or (rr[ln[0] - 1] == "info" and rr[ln[-1] + 1] == "center" and ln == list(range(ln[0], ln[-1] + 1))), rr
            return [x for x in rr if not x.startswith("link-")]
        def whole():
            opened("Tutto il disegno", None)
            assert pg.evaluate("document.querySelector('#ctx').getAttribute('aria-label')") == "Tutto il disegno"
            assert not pg.evaluate("document.querySelector('#ctx .ctx-head')") and pg.evaluate("document.querySelector('#ctx').firstElementChild.getAttribute('role')") == "menuitem"
            rr = rows(); assert rr[:3] == ["copy-smiles", "info", "center"] and "expected-ions" not in rr, rr
        step("no selection: no header line, the menu is named «Tutto il disegno»", whole)
        def part():
            opened("Selezione (2 atomi)", 0)
            rr = rows(); assert rr[:3] == ["copy-smiles", "info", "center"] and not {"break-bonds", "cut-bonds", "expected-ions"} & set(rr), rr
            t = pg.locator("#ctx div[data-row=copy-smiles]").inner_text(); assert "CC" in t and "O" not in t, t     # the piece, not the whole drawing
        step("a selected bond: menu named «Selezione (2 atomi)», SMILES of the piece, no bond rows", part)
        def added():
            pg.evaluate("""(() => { window._got = null;
              TPDraw.menu.add({ id: 'test-sel', when: 'selection', order: 12, label: c => 'Prova ' + c.atoms.size, run: c => { window._got = [c.atoms.size, c.whole, c.bonds.length]; } });
              TPDraw.menu.add({ id: 'test-whole', when: 'whole', order: 25, label: 'draw.ctx.center', run: () => {} });
              TPDraw.menu.add({ id: 'test-bad', order: 16, show: () => { throw new Error('boom'); }, label: 'x', run: () => {} }); })()""")
            ids = pg.evaluate("TPDraw.menu.ids()"); assert ids.index("test-sel") == ids.index("copy-smiles") + 1, ids
            opened("Selezione (2 atomi)", 0)
            rr = rows(); assert rr[:4] == ["copy-smiles", "test-sel", "info", "center"] and "test-whole" not in rr and "test-bad" not in rr, rr
            pg.locator("#ctx div[data-row=test-sel]").click()                # select, right click, row: two clicks
            assert pg.evaluate("window._got") == [2, False, 1], pg.evaluate("window._got")
            assert pg.locator("#ctx").is_hidden()
            opened("Tutto il disegno", None)
            rr = rows(); assert "test-whole" in rr and "test-sel" not in rr, rr
        step("rows added through TPDraw.menu.add: ordered, filtered, a failing row left out", added)
        def keys():
            opened("Tutto il disegno", None); pg.evaluate("document.querySelector('#ctx').hidden = true")
            pg.locator("#kframe").focus()
            pg.evaluate("(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.KeyboardEvent('keydown',{key:'F10',shiftKey:true,bubbles:true}))})()")
            wait_for_js(pg, "!document.querySelector('#ctx').hidden && document.activeElement && document.activeElement.dataset.row === 'copy-smiles'")
            pg.keyboard.press("ArrowDown"); assert pg.evaluate("document.activeElement.dataset.row") == "info"
            pg.keyboard.press("ArrowUp"); pg.keyboard.press("ArrowUp"); assert pg.evaluate("document.activeElement.getAttribute('role')") == "menuitem"   # arrows wrap over the rows
            pg.keyboard.press("Escape"); assert pg.locator("#ctx").is_hidden()
        def axe_menu():
            if not axe_path(): raise AssertionError("axe-core not available (pip install axe-playwright-python)")
            opened("Selezione (2 atomi)", 0)
            if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=axe_path())
            v = pg.evaluate("""() => axe.run({include: [['#ctx']]}, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21aa','wcag22aa']}})
              .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))"""); assert not v, v
            pg.evaluate("document.querySelector('#ctx').hidden = true")
        step("axe: the open menu has no serious violation", axe_menu)
        step("Shift+F10 focuses the first row, arrows wrap, Esc closes", keys)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
