"""Disegno, mechanism notation (disegno-meccanismo.js): on the radical cation of acetone a half-head arrow dragged from the O radical to a C–C bond is
well formed and drawn over the canvas; a full-head arrow from a methyl (no lone pair) is named as malformed and dashed; the keyboard path (start and
end from the selection) adds a row; lone pairs drawn beyond what the atom holds are named; «•+» puts charge and radical on the selected atom; the
arrows survive a .ket round trip and go out in the SVG export; the row button removes an arrow. The check says whether an arrow is well formed only."""
# covers: mzlab/web/disegno-meccanismo.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
OVER = "document.querySelector('#kframe').contentDocument.querySelector('svg #mz-meccanismo')"
ROWS = "[...document.querySelectorAll('#mech-list li')].map(l=>l.textContent).join(' | ')"
def mol(atoms, bonds, props):                                             # V2000 molfile: (element, x, y), bonds 1-based (a, b, order), property lines
    a = "".join(f"{x:10.4f}{y:10.4f}    0.0000 {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for e, x, y in atoms)
    b = "".join(f"{p:3d}{q:3d}{o:3d}  0\n" for p, q, o in bonds)
    return f"\n  qqq\n\n{len(atoms):3d}{len(bonds):3d}  0  0  0  0  0  0  0  0999 V2000\n{a}{b}{props}M  END\n"
ACETONE_ION = mol([("C", 0, 0), ("C", 1, 0), ("O", 1, 1), ("C", 2, 0)], [(1, 2, 1), (2, 3, 2), (2, 4, 1)], "M  CHG  1   3   1\nM  RAD  1   3   2\n")
r = Run(port=9139, wd="/tmp/wd_meccanismo")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        wait_for_js(pg, "!!document.getElementById('mech-layer')", 60000)                   # the mechanism tools are mounted
        def px(i):                                                        # page point of the i-th atom, or of the middle of atoms i and j
            return pg.evaluate(f"""(i=>{{const K={KJ},R=K.editor.render,o1=R.page2obj({{clientX:0,clientY:0}}),o2=R.page2obj({{clientX:100,clientY:100}});
              const f=document.querySelector('#kframe').getBoundingClientRect(),A=[...K.editor.struct().atoms.values()],a=i.map(k=>A[k].pp),x=a.reduce((s,q)=>s+q.x,0)/a.length,y=a.reduce((s,q)=>s+q.y,0)/a.length;
              return [f.left+(x-o1.x)*100/(o2.x-o1.x),f.top+(y-o1.y)*100/(o2.y-o1.y)]}})""", i)
        def drag(a, b):
            pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, steps=3); pg.mouse.move(*b, steps=3); pg.mouse.up()
        def load():
            pg.evaluate(f"m => {KJ}.setMolecule(m)", ACETONE_ION); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 4")
        def ids(): return pg.evaluate(f"[...{KJ}.editor.struct().atoms.keys()]")
        def half_head():
            load()
            for b in ("#mech-arrow", "#mech-lone", "#mech-ion", "#mech-key"):
                box = pg.locator(b).bounding_box(); assert box["width"] >= 24 and box["height"] >= 24, (b, box)
            pg.check("input[name=mech-kind][value=single]"); pg.click("#mech-arrow")
            assert pg.get_attribute("#mech-arrow", "aria-pressed") == "true"
            drag(px([2]), px([0, 1]))
            wait_for_js(pg, f"/1\\. un elettrone \\(mezza punta\\): da il radicale di O3 a il legame C1–C2 — ben formata/.test({ROWS})", timeout=15000)
            wait_for_js(pg, f"{OVER} && {OVER}.querySelectorAll('path[data-mech=single]').length === 1")
            assert pg.evaluate(f"{OVER}.querySelector('path[data-mech]').getAttribute('stroke-dasharray')") is None
            assert "ben formata" in pg.inner_text("#mech-note") or "Ogni freccia" in pg.inner_text("#mech-note"), pg.inner_text("#mech-note")
        step("a half-head arrow from the O radical to the C1–C2 bond is well formed and drawn over the canvas", half_head)
        def malformed():
            pg.check("input[name=mech-kind][value=pair]")
            drag(px([0]), px([1]))                                         # from the methyl C1 (no lone pair) to C2
            wait_for_js(pg, f"/2\\. coppia.*non ha doppietti liberi/.test({ROWS})", timeout=15000)
            wait_for_js(pg, f"{OVER}.querySelectorAll('path[data-mech=pair][stroke-dasharray]').length === 1")
            assert "1 freccia da rivedere" in pg.inner_text("#mech-note").lower() or "Una freccia da rivedere" in pg.inner_text("#mech-note"), pg.inner_text("#mech-note")
            pg.keyboard.press("Escape"); assert pg.get_attribute("#mech-arrow", "aria-pressed") == "false"
        step("a full-head arrow from a methyl is named (no lone pair) and dashed", malformed)
        def keyboard():
            a = ids()
            pg.evaluate(f"id => {KJ}.editor.selection({{atoms: [id]}})", a[2]); pg.click("#mech-key")
            assert "Fine dalla selezione" in pg.inner_text("#mech-key"), pg.inner_text("#mech-key")
            bond = pg.evaluate(f"[...{KJ}.editor.struct().bonds.entries()].find(([k,b])=>b.type===2)[0]")
            pg.evaluate(f"b => {KJ}.editor.selection({{bonds: [b]}})", bond); pg.focus("#mech-key"); pg.keyboard.press("Enter")
            wait_for_js(pg, f"/3\\. coppia \\(punta piena\\): da un doppietto di O3 a il legame C2–O3 — ben formata/.test({ROWS})", timeout=15000)
        step("from the keyboard: start = the selected O, end = the selected C=O bond, a well-formed row", keyboard)
        def lone_pairs():
            a = ids(); pg.evaluate(f"id => {KJ}.editor.selection({{atoms: [id]}})", a[2])
            pg.click("#mech-lone"); wait_for_js(pg, f"{OVER}.querySelectorAll('circle').length === 2")
            pg.evaluate(f"id => {KJ}.editor.selection({{atoms: [id]}})", a[2]); pg.click("#mech-lone")
            wait_for_js(pg, f"{OVER}.querySelectorAll('circle').length === 4")
            wait_for_js(pg, "/O3: 2 doppietti disegnati, ma l'atomo ne può avere 1/.test(document.querySelector('#mech-note').textContent)", timeout=15000)
        step("two lone pairs on the O of M+• are more than the one it holds: the status says so", lone_pairs)
        def ion():
            pg.evaluate(f"{KJ}.setMolecule('CCN')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 3")
            n = pg.evaluate(f"[...{KJ}.editor.struct().atoms.entries()].find(([k,a])=>a.label==='N')[0]")
            pg.evaluate(f"id => {KJ}.editor.selection({{atoms: [id]}})", n); pg.click("#mech-ion")
            wait_for_js(pg, f"(a => a.charge === 1 && a.radical === 2)({KJ}.editor.struct().atoms.get({n}))")
        step("«•+» on the selected N puts the charge + and the radical", ion)
        def round_trip():
            load(); pg.check("input[name=mech-kind][value=single]"); pg.click("#mech-arrow"); drag(px([2]), px([0, 1])); pg.keyboard.press("Escape")
            wait_for_js(pg, f"{OVER}.querySelectorAll('path[data-mech]').length === 1", timeout=15000)
            ket = pg.evaluate(f"{KJ}.getKet()"); assert "MZLAB_MECH" in ket and "single R" in ket, ket[:300]
            pg.evaluate(f"{KJ}.setMolecule('C')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 1")
            pg.evaluate(f"k => {KJ}.setMolecule(k)", ket); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 4")
            wait_for_js(pg, f"/1\\. un elettrone.*radicale di O3/.test({ROWS})", timeout=15000)
            wait_for_js(pg, "(t => t.length > 0 && t.every(x => x.style.visibility === 'hidden'))([...document.querySelector('#kframe').contentDocument.querySelectorAll('svg text')].filter(t => /^single /.test(t.textContent.trim())))", timeout=10000)
        step("the arrow survives a .ket round trip and Ketcher's S-group text stays hidden", round_trip)
        def export():
            svg = pg.evaluate("async () => await (await TPDraw.image('svg')).text()")
            assert 'id="mz-meccanismo"' in svg and 'data-mech="single"' in svg, svg[:300]
            assert "MZLAB" not in svg and "single R" not in svg, "the S-group text is in the export"
            png = pg.evaluate("async () => (await TPDraw.image('png')).size"); assert png > 1000, png
        step("the SVG export draws the arrow (no S-group text) and the PNG export works", export)
        def remove():
            pg.click("#mech-list li button[aria-label='Togli la freccia 1']")
            wait_for_js(pg, f"document.querySelectorAll('#mech-list li').length === 0 && {OVER}.querySelectorAll('path').length === 0", timeout=15000)
            assert "MZLAB_MECH" not in pg.evaluate(f"{KJ}.getKet()")
        step("the row button removes the arrow", remove)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
