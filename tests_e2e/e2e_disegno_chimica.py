"""Disegno, chemistry of the selection: the SMILES of selected atoms keeps isotopes ([13CH4], [2H]), radicals and stereo (@/@@ of L-alanine, as the canvas SMILES),
because the piece molfile carries M  ISO, M  RAD, wedge/hash bonds and atom parity. The labels under the structures and above the arrows come from the engine
(`etich/1`): isotopes, every element (Pt), «massa nominale»/«massa esatta», «isotopologo» across an arrow, the copied formula of a dianion in brackets.
Run from the repo root after tools/build_site.py and tools/build_feat_wasm.sh site/static."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8994, wd="/tmp/wd_disegno_chimica")
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        def selection_smiles(mol, want):                                # draw, select every atom, wait for the SMILES box of the selection to match `want` (a JS test on s)
            pg.evaluate(f"{KJ}.setMolecule({json.dumps(mol)})")
            pg.evaluate(f"(() => {{ const K = {KJ}; K.editor.selection({{ atoms: [...K.editor.struct().atoms.keys()] }}); }})()")
            try: pg.wait_for_function("(() => { const c = document.querySelector('#sel-body code'), s = c ? c.textContent.trim() : ''; return " + want + "; })()", timeout=30000)
            except Exception: raise AssertionError(pg.inner_text("#sel-body"))
            return pg.inner_text("#sel-body code").strip()
        def ala():
            pg.evaluate(f"{KJ}.setMolecule('C[C@H](N)C(=O)O')"); canvas = pg.evaluate(f"{KJ}.getSmiles()").split(" ")[0]
            assert "@" in canvas, canvas
            selection_smiles("C[C@H](N)C(=O)O", "s === " + json.dumps(canvas))
        step("the selection SMILES of L-alanine keeps @ (same as the canvas)", ala)
        step("the selection SMILES of [13CH4] keeps the isotope", lambda: selection_smiles("[13CH4]", "s === '[13CH4]'"))
        step("the selection SMILES of CD3OH keeps the three deuteria", lambda: selection_smiles("[2H]C([2H])([2H])O", "s.split('[2H]').length === 4"))
        step("the selection SMILES of trans-1,4-dimethylcyclohexane keeps both centres", lambda: selection_smiles("C[C@H]1CC[C@@H](C)CC1", "s.split('[C@').length === 3"))
        mf = "\n  test\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\nM  RAD  1   2   2\nM  END"
        step("the selection SMILES of the ethyl radical keeps the radical", lambda: selection_smiles(mf, "['C[CH2]', '[CH2]C'].includes(s)"))
        LBL = f"[...{KJ}.editor.render.paper.canvas.querySelectorAll('#mzlab-labels text')].map(t => t.textContent)"
        def labels(mol, want, dec="0"):                                 # draw, pick the decimals, wait for exactly these label lines
            pg.select_option("#lb-dec", dec); pg.evaluate(f"{KJ}.setMolecule({json.dumps(mol)})")
            try: pg.wait_for_function(f"JSON.stringify({LBL}) === {json.dumps(json.dumps(want, ensure_ascii=False, separators=(',', ':')))}", timeout=30000)
            except Exception: raise AssertionError(pg.evaluate(LBL))
        def methane():
            labels("[13CH4]", ["13CH4", "massa nominale 17"])
            labels("[13CH4]", ["13CH4", "massa esatta 17.0347"], "4")
        step("[13CH4]: label 13CH4, massa nominale 17, massa esatta 17.0347", methane)
        step("CD3OH: massa esatta 35.0450", lambda: labels("[2H]C([2H])([2H])O", ["CHD3O", "massa esatta 35.0450"], "4"))
        step("cisplatin: H4Cl2N2Pt 296.9399 (an element outside the old table)", lambda: labels("N[Pt](N)(Cl)Cl", ["H4Cl2N2Pt", "massa esatta 296.9399"], "4"))
        def isotopologue():
            pg.check("#lb-a"); pg.select_option("#lb-dec", "0"); pg.evaluate(f"{KJ}.setMolecule('c1ccccc1>>[13cH]1ccccc1')")
            try: pg.wait_for_function(f"{LBL}.includes('+13C \u2212C isotopologo \u0394m +1')", timeout=30000)
            except Exception: raise AssertionError(pg.evaluate(LBL))
            pg.uncheck("#lb-a")
        step("benzene -> 13C-benzene: +13C -C isotopologo, Δm +1", isotopologue)
        def dianion():
            labels("[O-]C(=O)CC(=O)[O-]", ["C3H2O42\u2212", "m/z 51"])
            xy = pg.evaluate(f"(() => {{ const r = document.getElementById('kframe').getBoundingClientRect(), t = {KJ}.editor.render.paper.canvas.querySelector('#mzlab-labels text').getBoundingClientRect(); return [r.left + t.x + t.width / 2, r.top + t.y + t.height / 2]; }})()")
            pg.mouse.click(xy[0], xy[1], button="right"); pg.wait_for_function("!document.getElementById('ctx').hidden", timeout=10000)
            t = pg.inner_text("#ctx"); pg.keyboard.press("Escape")
            assert "([C3H2O4]2-)" in t, t
        step("malonate: the copied formula is [C3H2O4]2-", dianion)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
