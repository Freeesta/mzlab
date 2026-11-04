"""Disegno, chemistry of the selection: the SMILES of selected atoms keeps isotopes ([13CH4], [2H]), radicals and stereo (@/@@ of L-alanine, as the canvas SMILES),
because the piece molfile carries M  ISO, M  RAD, wedge/hash bonds and atom parity. Run from the repo root after tools/build_site.py."""
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
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
