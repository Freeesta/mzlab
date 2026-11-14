"""Disegno: partial structures. Atoms selected + right click «Brackets with Δ…» put them in brackets with a delta (a data S-group «MZLAB» of Ketcher), drawn by
parentesi.js over the canvas; on the brackets the menu gives formula, mass and m/z with the delta (Rust `partial`, parz/1); an unreadable delta is refused;
the label under the structure counts the delta too (formula and nominal mass of structure + Δ, the same values as the bracket's menu line); the delta shows once,
also after a selection change; a click on the bracket line picks it, Delete removes it and undo brings it back; right click on the line offers «Remove the brackets»;
TPDraw.smiles() has no CXSMILES «|…|» and TPDraw.partial() gives {smiles, delta, level: 3, info: formula and m/z with the delta}; «Remove the brackets» takes them away. Multicentre: ring atoms +
the substituent atom selected, «Attached to one of these atoms» = dashed bond to the centre and circles on the candidates, and TPDraw.partial() gives
positionUnknown and a V3000 with a «*» atom and ENDPTS ... ATTACH=ANY.
Needs site/static/wasm-feat/grafo (tools/build_feat_wasm.sh); SKIP without it."""
# covers: mzlab/web/disegno-frammenti.js, mzlab/web/disegno-parziali.js, mzlab/web/disegno-molfile.js, mzlab/web/parentesi.js
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
        FD = "document.querySelector('#kframe').contentDocument"
        LABELS = f"[...{FD}.querySelectorAll('svg g[id$=labels] text')].map(t=>t.textContent).join(' | ')"
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
        def label():
            pg.keyboard.press("Escape")
            wait_for_js(pg, f"/C8H10O2/.test({LABELS}) && /138/.test({LABELS})", timeout=8000)   # C8H10O + O, nominal 122 + 16
            for i in range(20):
                menu_on_atom(2)
                try: pg.locator("#ctx div", has_text="Con +O").first.wait_for(state="visible", timeout=1500); break
                except Exception: pg.keyboard.press("Escape")
            row = pg.locator("#ctx div", has_text="Con +O").first.inner_text(); assert "C8H10O2" in row and " 138" in row, row   # the menu line agrees with the label
            pg.keyboard.press("Escape")
        step("the label under the structure has the formula and nominal mass with the delta, like the menu line", label)
        VIS = f"[...{FD}.querySelectorAll('svg text')].filter(t => t.textContent.trim() === '+O' && t.getBoundingClientRect().width > 0 && getComputedStyle(t).visibility !== 'hidden' && getComputedStyle(t).display !== 'none').length"
        FRAMES = "new Promise(r => { const w = document.querySelector('#kframe').contentWindow; w.requestAnimationFrame(() => w.requestAnimationFrame(() => w.requestAnimationFrame(r))); })"
        def once():
            assert pg.evaluate(VIS) == 1, pg.evaluate(VIS)
            for a in ("{atoms:[0,1]}", "null"):                          # Ketcher redraws on the next frames: two frames of the canvas, then count
                pg.evaluate(f"{KJ}.editor.selection({a})"); pg.evaluate(FRAMES)
            assert pg.evaluate(VIS) == 1, pg.evaluate(VIS)                 # Ketcher draws its DAT text again on a selection change
        step("the delta is drawn once, also after a selection change", once)
        def on_line():                                                    # page point on the left line of the brackets
            b = pg.evaluate(f"(()=>{{const r={FD}.querySelector('#mz-parziali .mz-parentesi path').getBoundingClientRect(),f=document.querySelector('#kframe').getBoundingClientRect();return [r.left,r.top,r.height,f.left,f.top]}})()")
            return b[0] + 1, b[1] + b[2] / 2, b[3], b[4]
        def ctx_line():
            x, y, fx, fy = on_line()
            for i in range(20):
                pg.evaluate("""([x,y])=>{const d=document.querySelector('#kframe').contentDocument;(d.elementFromPoint(x,y)||d.body).dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:x,clientY:y}))}""", [x, y])
                try: pg.locator("#ctx div", has_text="Togli la parentesi").first.wait_for(state="visible", timeout=1500); break
                except Exception: pg.keyboard.press("Escape")
            pg.locator("#ctx div", has_text="Togli la parentesi").first.wait_for(state="visible", timeout=5000)
            assert "Con +O: C8H10O2 · 138" in pg.inner_text("#ctx")                        # the same line as on an atom
            pg.keyboard.press("Escape")
        step("right click on the bracket line offers «Togli la parentesi»", ctx_line)
        NS = f"(()=>{{let n=0;{KJ}.editor.struct().sgroups.forEach(()=>n++);return n}})()"
        def pick_delete():
            x, y, fx, fy = on_line(); pg.mouse.click(fx + x, fy + y)
            wait_for_js(pg, f"!!{FD}.querySelector('#mz-parziali .mz-scelta')", timeout=5000)
            pg.keyboard.press("Delete"); wait_for_js(pg, f"{NS} === 0", timeout=8000)
            assert pg.evaluate(f"{KJ}.editor.struct().atoms.size") == 9                # the atoms stay, only the brackets go
            pg.evaluate("TPDraw.undo()"); wait_for_js(pg, f"{NS} === 1", timeout=8000)
            wait_for_js(pg, f"!!{FD}.querySelector('#mz-parziali .mz-parentesi path')", timeout=5000)
            pg.evaluate("TPDraw.undo(true)"); wait_for_js(pg, f"{NS} === 0", timeout=8000)
            pg.evaluate("TPDraw.undo()"); wait_for_js(pg, f"{NS} === 1", timeout=8000)
        step("a click on the bracket line picks it, Delete removes it, undo/redo", pick_delete)
        def call(js):                                                     # Ketcher's structure service drops a request that overlaps the ones of the menu: ask again
            for i in range(6):
                v = pg.evaluate(f"async () => {{ const t = new Promise(r => setTimeout(() => r('TIMEOUT'), 5000)); return await Promise.race([({js}).then(x => JSON.parse(JSON.stringify(x))), t]) }}")
                if v != "TIMEOUT": return v
            raise AssertionError("no answer: " + js)
        def export():
            pg.keyboard.press("Escape"); s = call("TPDraw.smiles()"); assert "|" not in s and s, s
            o = call("TPDraw.partial()"); assert o["delta"] == "+O" and o["level"] == 3 and "|" not in o["smiles"], o
            assert o["info"]["formula"] == "C8H10O2" and round(o["info"]["mz"]) == 139, o["info"]               # formula and m/z of the drawing with the delta (parz/1)
        step("TPDraw.smiles() without «|…|», TPDraw.partial() = {smiles, delta, level: 3}", export)
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
            assert o["positionUnknown"] is True and o["level"] == 3 and o["delta"] == "", o
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
