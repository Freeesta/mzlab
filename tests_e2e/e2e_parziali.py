"""Disegno: partial structures. Atoms selected + right click «Brackets with Δ…» put them in brackets with a delta (a data S-group «MZLAB» of Ketcher), drawn by
parentesi.js over the canvas; on the brackets the menu gives formula, mass and m/z with the delta (Rust `partial`, parz/1); an unreadable delta is refused;
TPDraw.smiles() has no CXSMILES «|…|» and TPDraw.partial() gives {smiles, delta, level: "3b"}; «Remove the brackets» takes them away. Multicentre: ring atoms +
the substituent atom selected, «Attached to one of these atoms» = dashed bond to the centre and circles on the candidates, and TPDraw.partial() gives
positionUnknown and a V3000 with a «*» atom and ENDPTS ... ATTACH=ANY.
Needs site/static/wasm-feat/grafo (tools/build_feat_wasm.sh); SKIP without it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
if not os.path.exists(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8992, wd="/tmp/wd_parziali")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000); pg.wait_for_timeout(800)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.setMolecule('c1ccccc1CCO')"); pg.wait_for_timeout(1000)
        def menu_on_atom(i):                                              # right click on atom number i (Ketcher id)
            pt = pg.evaluate(f"""(()=>{{const K={KJ},a=K.editor.struct().atoms.get({i}),R=K.editor.render,o1=R.page2obj({{clientX:0,clientY:0}}),o2=R.page2obj({{clientX:100,clientY:100}});
              return [(a.pp.x-o1.x)*100/(o2.x-o1.x),(a.pp.y-o1.y)*100/(o2.y-o1.y)]}})()""")
            pg.evaluate("""([x,y])=>{const d=document.querySelector('#kframe').contentDocument;const e=d.elementFromPoint(x,y)||d.body;e.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:x,clientY:y}))}""", pt)
        def bracket(delta):
            pg.evaluate(f"{KJ}.editor.selection({{atoms:[0,1,2,3,4,5]}})")
            menu_on_atom(0); pg.locator("#ctx div", has_text="Parentesi con").first.click()
            pg.wait_for_selector("#askdlg[open]", timeout=5000); pg.fill("#askin", delta); pg.click("#askok")
            pg.evaluate(f"{KJ}.editor.selection(null)")
        def add():
            bracket("+O")
            pg.wait_for_function(f"(()=>{{let n=0;{KJ}.editor.struct().sgroups.forEach(s=>{{if(s.type==='DAT'&&s.data.fieldName==='MZLAB')n++}});return n===1}})()", timeout=8000)
            pg.wait_for_function("!!document.querySelector('#kframe').contentDocument.querySelector('#mz-parziali .mz-parentesi path')", timeout=5000)
            t = pg.evaluate("document.querySelector('#kframe').contentDocument.querySelector('#mz-parziali text').textContent"); assert t == "+O", t
        step("selected atoms + «Parentesi con Δ…» put them in brackets with the delta", add)
        def line():
            for i in range(20):
                pg.keyboard.press("Escape"); menu_on_atom(2)
                if pg.locator("#ctx div", has_text="Con +O").count(): break
                pg.wait_for_timeout(300)
            t = pg.locator("#ctx div", has_text="Con +O").first.inner_text()
            assert t.startswith("Con +O: C8H10O2 ") and "m/z 139" in t, t                        # C8H10O + O; [M+H]+ 139 at unit decimals
            assert pg.locator("#ctx div", has_text="Parentesi con").count() == 0                    # atoms already in brackets
        step("on the brackets: formula, mass and m/z with the delta", line)
        def call(js):                                                     # Ketcher's structure service drops a request that overlaps the ones of the menu: ask again
            for i in range(6):
                v = pg.evaluate(f"async () => {{ const t = new Promise(r => setTimeout(() => r('TIMEOUT'), 5000)); return await Promise.race([({js}).then(x => JSON.parse(JSON.stringify(x))), t]) }}")
                if v != "TIMEOUT": return v
            raise AssertionError("no answer: " + js)
        def export():
            pg.keyboard.press("Escape"); s = call("TPDraw.smiles()"); assert "|" not in s and s, s
            o = call("TPDraw.partial()"); assert o["delta"] == "+O" and o["level"] == "3b" and "|" not in o["smiles"], o
        step("TPDraw.smiles() without «|…|», TPDraw.partial() = {smiles, delta, level: "3b"}", export)
        def svg_text():                                                   # the exported SVG, as text
            return call("TPDraw.image('svg').then(b => b.text())")
        def export_svg():
            t = svg_text(); assert 'class="mz-parentesi"' in t and ">+O</text>" in t, t[:300]
            assert t.count("MZLAB") == 0, "DAT text of Ketcher left in the image"
        step("exported SVG has the brackets and the delta", export_svg)
        def remove():
            menu_on_atom(2); pg.locator("#ctx div", has_text="Togli la parentesi").first.click()
            pg.wait_for_function(f"(()=>{{let n=0;{KJ}.editor.struct().sgroups.forEach(()=>n++);return n===0}})()", timeout=8000)
            pg.wait_for_function("!document.querySelector('#kframe').contentDocument.querySelector('#mz-parziali .mz-parentesi')", timeout=5000)
            o = call("TPDraw.partial()"); assert o["delta"] == "" and o["level"] is None, o
        step("«Togli la parentesi» removes them", remove)
        def bad():
            bracket("+Qq"); pg.wait_for_timeout(800)
            n = pg.evaluate(f"(()=>{{let n=0;{KJ}.editor.struct().sgroups.forEach(()=>n++);return n}})()"); assert n == 0, n
        step("an unreadable delta is refused", bad)
        def multi():
            pg.evaluate(f"{KJ}.editor.selection({{atoms:[6,3,4,5]}})")                # CH2 of the chain + three ring atoms, one of them the drawn attachment
            menu_on_atom(6); pg.locator("#ctx div", has_text="Attacco su uno di questi").first.click()
            pg.evaluate(f"{KJ}.editor.selection(null)")
            pg.wait_for_function("document.querySelector('#kframe').contentDocument.querySelectorAll('#mz-parziali .mz-multicentro circle').length === 3", timeout=8000)
            assert pg.evaluate("!!document.querySelector('#kframe').contentDocument.querySelector('#mz-parziali .mz-multicentro line[stroke-dasharray]')"), "no dashed bond"
            o = call("TPDraw.partial()"); mf = o["molfile"]
            assert o["positionUnknown"] is True and o["level"] == "3b" and o["delta"] == "", o
            lines = mf.split("\n")
            assert "M  V30 COUNTS 10 9 0 0 0" in mf and any(l.split()[3:4] == ["*"] for l in lines if l.startswith("M  V30 ")), mf
            assert any("ENDPTS=(3 " in l and l.endswith("ATTACH=ANY") for l in lines), mf
        step("multicentre: dashed bond, circles on the candidates, V3000 with ENDPTS", multi)
        def multi_svg():
            t = svg_text(); assert 'class="mz-multicentro"' in t and t.count("<circle") >= 3 and "stroke-dasharray" in t, t[:300]
        step("exported SVG has the dashed attachment", multi_svg)
        def multi_off():
            for i in range(10):
                pg.keyboard.press("Escape"); menu_on_atom(6)
                if pg.locator("#ctx div", has_text="Togli l'attacco").count(): break
                pg.wait_for_timeout(300)
            pg.locator("#ctx div", has_text="Togli l'attacco").first.click()
            pg.wait_for_function("!document.querySelector('#kframe').contentDocument.querySelector('#mz-parziali .mz-multicentro')", timeout=8000)
            o = call("TPDraw.partial()"); assert o["positionUnknown"] is False and o["level"] is None and o["molfile"] == "", o
        step("«Togli l'attacco variabile» removes it", multi_off)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
