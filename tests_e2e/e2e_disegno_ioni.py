"""Disegno, «Expected ions» card: without files the table is titled as theory and «Show XIC» is off with its reason; with a synthetic high-resolution file
the rows carry the file's decimals and ppm window, and draw → «Show expected ions» → «Show XIC» (3 actions) opens the Data XIC of [M+H]+ of caffeine with a
peak at its RT. Needs site/static/wasm-feat/grafo (tools/build_feat_wasm.sh); SKIP without it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
if not (ROOT / "site" / "static" / "wasm-feat" / "manifest.json").exists():
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))

CAF, MH = "Cn1cnc2c1c(=O)n(C)c(=O)n2C", 195.087652          # caffeine, C8H10N4O2: [M+H]+ computed apart (AME masses)
HRD = HERE / "shots" / "disegno_ioni_tmp"; HRD.mkdir(parents=True, exist_ok=True)
ds.hr_series(HRD / "CAF_t0.mzML", np.random.default_rng(7), [(MH, 5.0, 0.06, 6e5)])
KJ = "document.querySelector('#kframe').contentWindow.ketcher"

def rows(pg):
    pg.wait_for_selector("[data-testid=ions-table] tbody tr", timeout=30000)
    return pg.evaluate("[...document.querySelectorAll('[data-testid=ions-table] tbody tr')].map(t=>[t.querySelector('th').firstChild.textContent,t.children[1].firstChild.textContent,t.querySelector('button').disabled,t.querySelector('button').title])")

r = Run(port=8994, wd="/tmp/wd_disegno_ioni")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        def empty():
            pg.click("#ions-go"); pg.wait_for_selector("#ions-body .muted", timeout=10000)
            assert "Disegna prima" in pg.inner_text("#ions-body"), pg.inner_text("#ions-body")
        step("an empty canvas asks for a structure", empty)
        def theory():
            pg.evaluate(f"{KJ}.setMolecule('{CAF}')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 14")
            pg.click("#ions-go"); rs = rows(pg)
            assert pg.inner_text("#ions-h") == "Ioni attesi (teoria)", pg.inner_text("#ions-h")
            got = {n: (mz, off, why) for n, mz, off, why in rs}
            assert got["[M+H]+"][0] == "195.0877" and got["[M+Na]+"][0] == "217.0696" and got["[M-H]-"][0] == "193.0731", got
            assert got["[M+2H]2+"][0] == "98.0475" and "[M-2H]2-" not in got, got
            assert all(off and "alta risoluzione" in why for _, off, why in got.values()), got
            box = pg.locator(".ions-xic").first.bounding_box(); assert box["height"] >= 24 and box["width"] >= 24, box
        step("without a file: theory, the m/z of caffeine, XIC buttons off with the reason", theory)
        def hr():
            pg.click("#nav [data-v=data]")
            pg.set_input_files("#pick", [str(HRD / "CAF_t0.mzML")])
            open_first(pg); pg.wait_for_function("typeof E!=='undefined' && E.files.length>=1", timeout=60000); ready(pg)
            pg.click("#nav [data-v=draw]"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            n0 = pg.evaluate("E.panels.filter(p=>p.type==='xic').length")
            pg.click("#ions-go")                                                                      # action 2 (action 1 = the drawing)
            pg.wait_for_function("document.getElementById('ions-h').textContent === 'Ioni attesi' && !!document.querySelector('.ions-xic:not([disabled])')", timeout=30000); rs = rows(pg)
            assert pg.inner_text("#ions-h") == "Ioni attesi", pg.inner_text("#ions-h")
            dec = pg.evaluate("(()=>{const f=HR.xicFiles().find(x=>HR.isHr(x,x.lv));return HR.prof(f,f.lv).dec})()")
            mh = next(x for x in rs if x[0] == "[M+H]+"); assert mh[1] == f"{MH:.{dec}f}" and not mh[2], (mh, dec)
            assert "ppm" in pg.inner_text("#ions-body"), pg.inner_text("#ions-body")
            pg.locator("[data-testid=ions-table] tbody tr", has=pg.locator("th", has_text="[M+H]+")).first.locator("button").click()   # action 3
            pg.wait_for_function(f"!document.getElementById('v-data').hidden && E.panels.filter(p=>p.type==='xic').length>{n0}", timeout=30000); ready(pg)
            tr = pg.evaluate("(()=>{const p=E.panels.filter(p=>p.type==='xic').pop(),t=p.traces[0];return [t.mz,t.ion,t.label]})()")
            assert abs(tr[0] - MH) < 1e-4 and tr[1] and "[M+H]+" in tr[2] and "ppm" in tr[2], tr
        step("with a high-resolution file: decimals of the file, and draw → Expected ions → Show XIC opens the XIC of [M+H]+", hr)
        def keyboard():
            pg.click("#nav [data-v=draw]"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            pg.focus("#ions-go"); pg.keyboard.press("Enter"); rows(pg)
            pg.locator(".ions-xic").first.focus(); assert pg.evaluate("document.activeElement.classList.contains('ions-xic')")
            assert pg.evaluate("document.activeElement.getAttribute('aria-label')").startswith("Mostra l'XIC di [M+H]+")
        step("the card works from the keyboard and every XIC button has its name", keyboard)
        def menu():
            pg.evaluate("document.getElementById('ions-body').innerHTML=''")
            pg.evaluate("""(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()""")
            row = pg.locator("#ctx div[data-row=expected-ions]"); row.wait_for(timeout=10000)
            assert row.inner_text() == "Ioni attesi", row.inner_text()
            row.click(); rs = rows(pg); assert any(n == "[M+H]+" for n, *_ in rs), rs
        step("the canvas menu has the «Expected ions» row and it fills the card", menu)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
