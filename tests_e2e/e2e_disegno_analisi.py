"""Disegno: the «Formula analysis» card (closed by default) gives formula, exact, nominal and average mass and the mass % of the drawn molecule from the
Rust `anal/1`; «Show isotope pattern» draws the theoretical sticks (keyboard-reachable, with their TDATI table) titled as theory for the formula; an ion
shows its m/z; two molecules without a selection ask for one; the right-click row opens the card on the selection. Draw -> card -> pattern = 2 clicks. Needs site/static/wasm-feat/grafo; SKIP without it."""
# covers: mzlab/web/disegno-analisi.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
if not os.path.exists(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=9142, wd="/tmp/wd_disegno_analisi")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        wait_for_js(pg, f"!!({KJ} && {KJ}.editor)"); pg.evaluate(f"{KJ}.setMolecule('CC(=O)Nc1ccc(O)cc1')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 11")
        def closed():
            assert pg.evaluate("!document.querySelector('#anal-card').open"), "the card starts closed"
            assert pg.locator("#anal-body *").count() == 0, "nothing is computed before the card is opened"
        step("the card is closed and empty until it is opened", closed)
        def numbers():
            pg.click("#anal-card summary")                                                        # click 1
            pg.wait_for_selector("#anal-formula", timeout=15000)
            assert pg.inner_text("#anal-formula").replace("\n", "") == "C8H9NO2", pg.inner_text("#anal-formula")
            t = pg.inner_text("#anal-nums")
            assert "151.06333" in t and "151" in t and "151.16" in t, t                            # paracetamol: monoisotopic, nominal, average
            rows = pg.locator("#anal-comp tr").all_inner_texts()
            assert len(rows) == 5 and rows[1].split()[0] == "C" and "63.5" in rows[1], rows       # header + C, H, N, O; C 63.56 % by mass
            assert pg.locator("#anal-pattern").count() == 0, "the pattern opens only on a click"
        step("opening the card gives formula, masses and composition", numbers)
        def pattern():
            pg.click("#anal-show")                                                                # click 2
            pg.wait_for_selector("#anal-chart .an-st", timeout=5000)
            assert pg.get_attribute("#anal-show", "aria-expanded") == "true"
            assert "teorico" in pg.inner_text("#anal-title") and "C8H9NO2" in pg.inner_text("#anal-title"), pg.inner_text("#anal-title")
            labels = [pg.get_attribute(f"#anal-chart .an-st >> nth={i}", "aria-label") for i in range(pg.locator("#anal-chart .an-st").count())]
            assert len(labels) >= 3 and labels[0].startswith("M:") and "151.06333" in labels[0] and labels[1].startswith("M+1:"), labels
            box = pg.locator("#anal-chart .an-st rect").first.bounding_box(); assert box["width"] >= 24, box
        step("«Show isotope pattern» draws the sticks, titled as theory for the formula", pattern)
        def keys():
            pg.focus("#anal-chart .an-st >> nth=0"); pg.keyboard.press("ArrowRight")
            assert pg.evaluate("document.activeElement.dataset.i") == "1", pg.evaluate("document.activeElement.outerHTML.slice(0,80)")
            pg.keyboard.press("End"); assert pg.evaluate("document.activeElement.dataset.i") == str(pg.locator("#anal-chart .an-st").count() - 1)
        step("the arrow keys move between the sticks", keys)
        def table():
            n = pg.locator("#anal-chart .an-st").count()
            pg.click("#anal-table"); pg.wait_for_selector("#tdati table tbody tr", timeout=5000)
            assert pg.locator("#tdati tbody tr").count() == n, (pg.locator("#tdati tbody tr").count(), n)
            assert pg.locator("#tdati tbody tr").first.inner_text().split()[0] == "M"
            pg.keyboard.press("Escape")
        step("the data table of the chart lists every stick", table)
        def ion():
            pg.evaluate(f"{KJ}.setMolecule('C[NH3+]')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 2")
            pg.wait_for_function("(document.querySelector('#anal-formula')||{}).textContent === 'CH6N+'", timeout=15000)
            t = pg.inner_text("#anal-nums"); assert "m/z" in t and "32.04948" in t, t               # CH6N+: (mass - e)
            assert "32.04948" in pg.get_attribute("#anal-chart .an-st >> nth=0", "aria-label")
        step("an ion shows its m/z and the sticks are m/z", ion)
        def two():
            pg.evaluate(f"{KJ}.setMolecule('CCO.CN')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 5")
            pg.wait_for_function("/Seleziona una molecola/.test(document.querySelector('#anal-body').textContent)", timeout=15000)
        step("two molecules without a selection ask for one", two)
        def menu():
            pg.evaluate("document.querySelector('#anal-card').open = false")
            pg.evaluate(f"(()=>{{const K={KJ};const ids=[...K.editor.struct().atoms.keys()];K.editor.selection({{atoms:ids.slice(3)}})}})()")   # the CN piece
            OPEN = "(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"
            pg.evaluate(OPEN); wait_for_js(pg, "!!document.querySelector('#ctx:not([hidden]) div[data-row=analysis]')", timeout=10000)
            pg.click("#ctx div[data-row=analysis]")
            pg.wait_for_function("document.querySelector('#anal-card').open && (document.querySelector('#anal-formula')||{}).textContent === 'CH5N'", timeout=15000)
        step("the right-click row opens the card on the selection", menu)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
