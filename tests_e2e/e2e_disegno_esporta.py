"""Disegno with the trimmed Ketcher bundle (no macromolecules mode, empty monomer library): a structure is drawn from SMILES, a bond is added with the editor,
and the result exports as SMILES, molfile V2000 and .ket, and through the MOL / SDF buttons (Rust writer, sdf/1) as one .mol and one SDF record
per structure with its data items; the macromolecules switch is not in the page."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:260]))
def poll(cond, n=40):
    for _ in range(n):
        if cond(): return
        frames(pg, 15)
    raise AssertionError("state not reached")
r = Run(port=8865, wd="/tmp/wd_desporta")
try:
    with sync_playwright() as p:
        global pg
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=90000)
        K = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate("TPDraw.load('CC(=O)Nc1ccc(O)cc1')"); poll(lambda: len(pg.evaluate("TPDraw.smiles()")) > 5)       # paracetamol: 11 heavy atoms
        def smiles():
            smi = pg.evaluate("TPDraw.smiles()")
            assert smi.count("c") == 6 and smi.count("N") == 1 and smi.count("O") == 2, smi
        step("SMILES export of a loaded structure", smiles)
        def molfile():
            mol = pg.evaluate(f"{K}.getMolfile()")
            assert "V2000" in mol, mol[:120]
            n = int(mol.split("\n")[3][:3]); assert n == 11, ("heavy atoms", n)
        step("molfile export", molfile)
        def draw_more():
            pg.evaluate(f"{K}.addFragment('CCO')"); poll(lambda: "." in pg.evaluate("TPDraw.smiles()"))
            smi = pg.evaluate("TPDraw.smiles()")
            assert len(smi.split(".")) == 2, smi
            ket = pg.evaluate(f"{K}.getKet()")
            assert '"mol1"' in ket and "monomer" not in ket.lower(), ket[:200]
        step("a second structure is added and the .ket has no monomers", draw_more)
        def mol_sdf():
            pg.select_option("#ex-ion", "[M+H]+")
            with pg.expect_download(timeout=60000) as d: pg.click("#ex-sdf")
            sdf = open(d.value.path(), encoding="utf-8").read()
            assert d.value.suggested_filename.endswith(".sdf"), d.value.suggested_filename
            recs = [x for x in sdf.split("$$$$\n") if x.strip()]
            assert len(recs) == 2, sdf[:400]
            forms = sorted(x.split("> <MZLAB_FORMULA>\n")[1].split("\n")[0] for x in recs)
            assert forms == ["C2H6O", "C8H9NO2"], forms
            assert all("> <MZLAB_ION>\n[M+H]+\n" in x and "> <MZLAB_HASH>\nmol1:" in x for x in recs), sdf[:600]
            assert "152.0706" in sdf, "paracetamol [M+H]+"
            with pg.expect_download(timeout=60000) as d: pg.click("#ex-mol")
            mol = open(d.value.path(), encoding="utf-8").read()
            assert d.value.suggested_filename.endswith(".mol") and mol.endswith("M  END\n") and "$$$$" not in mol and "> <" not in mol, mol[-200:]
            assert int(mol.split("\n")[3][:3]) == 14, mol.split("\n")[3]
            assert pg.evaluate("document.querySelector('#ex-warn').hidden"), pg.inner_text("#ex-warn")
        step("MOL and SDF are written by the engine: one record per structure with formula, ion, m/z and hash", mol_sdf)
        def no_macro():
            assert not pg.evaluate("!!document.querySelector('#kframe').contentDocument.querySelector('[data-testid=polymer-toggler]')")
        step("the macromolecules switch is not in the editor", no_macro)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
