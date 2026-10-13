"""Disegno: right click on a bond shows the two pieces it would leave (formula, mass, m/z of [F+H]+) from the Rust graph, a ring bond says it does not split alone,
and «All cuts…» opens the table of every cut. Needs site/static/wasm-feat/grafo (tools/build_feat_wasm.sh); SKIP without it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
if not os.path.exists(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8993, wd="/tmp/wd_frammenti")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000); pg.wait_for_timeout(800)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.setMolecule('CCO')"); pg.wait_for_timeout(1000)
        def menu_at(kind, idx):                                           # right click on the centre of bond/atom number idx
            pt = pg.evaluate(f"""(()=>{{const K={KJ},st=K.editor.struct(),ids=[...st.bonds.keys()],b=st.bonds.get(ids[{idx}]),a=st.atoms.get(b.begin),c=st.atoms.get(b.end);
              const R=K.editor.render,o1=R.page2obj({{clientX:0,clientY:0}}),o2=R.page2obj({{clientX:100,clientY:100}}),m=(a.pp.x+c.pp.x)/2,n=(a.pp.y+c.pp.y)/2;return [(m-o1.x)*100/(o2.x-o1.x),(n-o1.y)*100/(o2.y-o1.y)]}})()""")
            pg.evaluate("""([x,y])=>{const d=document.querySelector('#kframe').contentDocument;const e=d.elementFromPoint(x,y)||d.body;e.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:x,clientY:y}))}""", pt)
        def two():
            for i in range(20):
                menu_at("bond", 0)
                if pg.locator("#ctx div", has_text="m/z").count() >= 2: break
                pg.wait_for_timeout(400)
            t = pg.locator("#ctx div", has_text="m/z").all_inner_texts(); assert len(t) == 2, t
            assert any(x.startswith("CH4 ") and "m/z 17" in x for x in t) and any(x.startswith("CH4O ") and "m/z 33" in x for x in t), t       # C-C of ethanol; nominal values, as the label decimals say
        step("a right click on a bond gives its two pieces with formula, mass and m/z", two)
        def table():
            pg.locator("#ctx div", has_text="Tutte le rotture").first.click(); pg.wait_for_selector("#tdati table tbody tr", timeout=5000)
            assert pg.locator("#tdati tbody tr").count() == 2, pg.locator("#tdati tbody tr").count()
            pg.keyboard.press("Escape")
        step("«All cuts…» opens the table with one row per bond", table)
        def ring():
            pg.evaluate(f"{KJ}.setMolecule('C1CCCCC1')"); pg.wait_for_timeout(1000)
            menu_at("bond", 0); pg.wait_for_selector("#ctx div:has-text('Legame di un anello')", timeout=5000)
        step("a ring bond says it does not split alone", ring)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
