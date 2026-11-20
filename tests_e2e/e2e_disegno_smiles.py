"""Draw, the «Draw» button on a SMILES: the layout comes from the Rust engine (`draw` of the molkit .wasm, `draw/1`), Ketcher only adds the molfile. Needs the molkit .wasm.
With the .wasm blocked «Draw» still draws (Ketcher's own layout); with it, the structure on the canvas has the engine's shape (same distances between atoms,
whatever Ketcher does to position and scale), a second «Draw» adds a second molecule, and an invalid SMILES shows the notice and changes nothing."""
# covers: mzlab/web/draw.js, mzlab/web/disegno-smiles.js
import sys, os, math; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
def poll(cond, n=60):
    for _ in range(n):
        if cond(): return
        frames(pg, 15)
    raise AssertionError("state not reached")
K = "document.querySelector('#kframe').contentWindow.ketcher"
SMI = "C[C@@H](O)c1ccc2ccccc2c1"          # a fused ring system and one stereo centre (synthetic)
def parse(mol):
    """elements, atom positions and bonds (a, b, order, stereo) of a V2000 molfile"""
    ln = mol.split("\n"); na, nb = int(ln[3][:3]), int(ln[3][3:6])
    at = [(float(l[0:10]), float(l[10:20]), l[31:34].strip()) for l in ln[4:4 + na]]
    bo = [(int(l[0:3]) - 1, int(l[3:6]) - 1, int(l[6:9]), int(l[9:12] or 0)) for l in ln[4 + na:4 + na + nb]]
    return at, bo
def shape(mol):
    """distances between all atom pairs in units of the mean bond length: the same for a molfile moved, turned, mirrored or scaled"""
    at, bo = parse(mol)
    d = lambda i, j: math.hypot(at[i][0] - at[j][0], at[i][1] - at[j][1])
    mean = sum(d(a, b) for a, b, _, _ in bo) / len(bo)
    return [at[i][2] for i in range(len(at))], [[d(i, j) / mean for j in range(len(at))] for i in range(len(at))], bo
def atoms(): return pg.evaluate(f"{K}.editor.struct().atoms.size")
def note(): return pg.evaluate("(() => { const w = document.getElementById('smi-warn'); return w.hidden ? '' : w.textContent; })()")
def draw(text):
    pg.fill("#ex-smi", text); pg.click("#ex-load")
r = Run(port=8971, wd="/tmp/wd_dsmiles")
try:
    with sync_playwright() as p:
        global pg
        pg = r.page(p, service_workers="block"); pg.set_viewport_size({"width": 1440, "height": 900})
        def fallback():
            pg.route("**/wasm-feat/**", lambda route: route.fulfill(status=404, body="not found"))      # the engine is not there
            pg.click("#nav [data-v=draw]"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=90000)
            draw(SMI); poll(lambda: atoms() == 13)
            assert note() == "", note()
            assert pg.input_value("#ex-smi") == "", "the field is emptied after a drawing"
            assert pg.evaluate("async s => (await import('/static/disegno-smiles.js')).layoutSmiles(s)", SMI) == {}, "the engine answered: the block did not work"
        step("the engine not there: «Draw» still draws, with Ketcher's own layout", fallback)
        pg.unroute("**/wasm-feat/**"); pg.reload()                      # a new page: the manifest of the engine is asked again
        pg.click("#nav [data-v=draw]"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=90000)
        engine = pg.evaluate("async s => (await import('/static/disegno-smiles.js')).layoutSmiles(s)", SMI)
        assert engine.get("molfile"), ("the engine is not there", engine)
        def laid_out():
            draw(SMI); poll(lambda: atoms() == 13)
            assert note() == "" and pg.input_value("#ex-smi") == "", note()
            got = pg.evaluate(f"{K}.getMolfile()")
            (e1, d1, b1), (e2, d2, b2) = shape(engine["molfile"]), shape(got)
            assert e1 == e2 and len(b1) == len(b2), (e1, e2)
            assert sorted((min(a, b), max(a, b), o) for a, b, o, _ in b1) == sorted((min(a, b), max(a, b), o) for a, b, o, _ in b2), "connectivity"
            worst = max(abs(d1[i][j] - d2[i][j]) for i in range(len(e1)) for j in range(len(e1)))
            assert worst < 0.05, ("the canvas is not the engine's layout", worst)
            assert sum(1 for *_, s in b1 if s in (1, 6)) == sum(1 for *_, s in b2 if s in (1, 6)) == 1, "one wedge (stereo centre)"
        step("«Draw» adds the molfile of the engine: same connectivity, same distances (position and scale are Ketcher's), one wedge", laid_out)
        def second():
            draw("CCO"); poll(lambda: atoms() == 16)
            assert "." in pg.evaluate("TPDraw.smiles()") and note() == ""
        step("a second «Draw» adds a second molecule", second)
        def invalid():
            draw("C(C"); wait_for_js(pg, "!document.getElementById('smi-warn').hidden", timeout=20000)
            assert note().startswith("Nothing was added."), note()
            frames(pg, 10); assert atoms() == 16, ("the canvas changed", atoms())
            assert pg.input_value("#ex-smi") == "C(C", "the typed text stays"
        step("an invalid SMILES shows its notice and changes nothing", invalid)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
