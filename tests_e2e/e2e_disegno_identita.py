"""Disegno: «È nel catalogo?» looks the drawing up in the molecule catalogue through the Rust engine (molid .wasm): draw → identify is two clicks; the
enantiomer is a near match that says «other stereo», two structures get one answer each, an unknown structure and an empty canvas say so, and the button
works from the keyboard with a target of at least 24 px; the canvas menu has the same row. Needs site/static/wasm-feat with the molid module (tools/build_feat_wasm.sh); SKIP without it."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
MAN = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")
if not os.path.exists(MAN):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
assert "molid" in json.load(open(MAN)), "the manifest has no molid module: rebuild with tools/build_feat_wasm.sh"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8994, wd="/tmp/wd_identita")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        wait_for_js(pg, f"!!({KJ} && {KJ}.editor && {KJ}.editor.struct())")
        out = pg.locator("#id-out")
        def answer(text):
            try: pg.wait_for_function(f"document.querySelector('#id-out').textContent.includes({json.dumps(text)})", timeout=30000)
            except Exception: raise AssertionError(f"no «{text}» in: {out.inner_text()[:200]!r}")
            return out.inner_text()
        def two_clicks():
            pg.fill("#ex-smi", "COc1ccc2cc([C@H](C)C(O)=O)ccc2c1"); pg.click("#ex-load")                    # click 1: draw
            wait_for_js(pg, f"{KJ}.editor.struct().atoms.size > 10")
            pg.click("#ex-id")                                                                                # click 2: identify
            t = answer("naproxen")
            assert "Nel catalogo come" in t and "C14H14O3" in t and "230.0943" in t, t
            assert "non a un segnale misurato" in t, t
        step("draw → identify in two clicks: the drawing is catalogue entry naproxen, with formula and mass", two_clicks)
        def enantiomer():
            pg.evaluate(f"{KJ}.setMolecule('COc1ccc2cc([C@@H](C)C(O)=O)ccc2c1')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size > 10")
            pg.click("#ex-id"); t = answer("altra stereo")
            assert "Stesso scheletro" in t and "naproxen" in t and "Nel catalogo come" not in t, t
        step("the enantiomer is a near match labelled «other stereo», not an identity", enantiomer)
        def two_pieces():
            pg.evaluate(f"{KJ}.setMolecule('CC(=O)Nc1ccc(O)cc1.Cn1cnc2c1c(=O)n(C)c(=O)n2C')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size > 20")
            pg.click("#ex-id"); t = answer("Struttura 2")
            assert "acetaminophen" in t and "caffeine" in t.lower() and "Struttura 1" in t, t
        step("two structures on the canvas get one answer each", two_pieces)
        def unknown():
            pg.evaluate(f"{KJ}.setMolecule('CCCCCCCCCCCCCCCCCCCCCCCCCCCCCC')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size == 30")
            pg.click("#ex-id"); t = answer("Non è nel catalogo")
            assert "strutture" in t, t
        step("a structure that is not there says so, with how many were compared", unknown)
        def empty():
            pg.evaluate(f"{KJ}.setMolecule('')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size == 0")
            pg.click("#ex-id"); answer("Prima disegna una struttura")
        step("an empty canvas asks for a drawing", empty)
        def keyboard():
            box = pg.locator("#ex-id").bounding_box(); assert box["width"] >= 24 and box["height"] >= 24, box
            pg.evaluate(f"{KJ}.setMolecule('Cc1ncc([N+](=O)[O-])n1CCO')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size > 5")
            pg.focus("#ex-id"); pg.keyboard.press("Enter"); t = answer("metronidazole")
            assert out.get_attribute("role") == "status", out.get_attribute("role")
        step("the button works from the keyboard, is at least 24 px and the answer is a status", keyboard)
        def menu_row():
            pg.evaluate(f"{KJ}.setMolecule('Cn1cnc2c1c(=O)n(C)c(=O)n2C')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size > 10")
            pg.evaluate("""(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()""")
            wait_for_js(pg, "!document.querySelector('#ctx').hidden && !!document.querySelector('#ctx div[data-row=catalogue-id]')")
            row = pg.locator("#ctx div[data-row=catalogue-id]"); assert "catalogo" in row.inner_text(), row.inner_text()
            row.click(); t = answer("caffeine"); assert "Nel catalogo come" in t, t
        step("the canvas menu row «È nel catalogo?» (TPDraw.menu registry) gives the same answer", menu_row)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if any(not s[1].startswith("ok") for s in steps) else 0)
