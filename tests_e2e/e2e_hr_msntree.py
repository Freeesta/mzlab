"""H4: «Albero MSn del file…» on the direct-infusion MSn file of the private data repository (skipped without it): the tree, the formulas, the peaks of a node, a wrong formula."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
import glob
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_msntree.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
F = None
for c in (os.environ.get("MZLAB_DATI"), os.environ.get("QQQ_DATI"), "/home/user/mzlab-dati", "/home/user/QqQ-lab-dati", str(ROOT.parent / "QqQ-lab-dati")):
    if c and glob.glob(c + "/HRMS/*/*direct-infusion_MSn.mzML"): F = glob.glob(c + "/HRMS/*/*direct-infusion_MSn.mzML")[0]; break
if not F:
    print("SKIP e2e_hr_msntree: infusion MSn file not available"); sys.exit(0)
r = Run(port=8971, wd="/tmp/wd_hrmsn")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [F], 8000, 1)
        def menu():
            k = pg.evaluate("E.files.findIndex(f=>f.kind==='infusion')"); assert k >= 0
            pg.evaluate(f"COMP.tree({k})"); pg.wait_for_selector("#mt-tbl tr[data-n]", timeout=60000)
            assert "non identificazioni" in pg.inner_text("#bigbody") or "candidati" in pg.inner_text("#bigbody")
            assert pg.locator("#mt-tbl tr[data-n]").count() >= 19, pg.locator("#mt-tbl tr[data-n]").count()
            assert "Senza la formula" in pg.inner_text("#mt-out")
        step("the tree opens with 19+ nodes and offers candidates for the first stage", menu)
        def fixed():
            pg.fill("#mt-f", "C13H25N4O3S"); pg.click("#mt-go"); pg.wait_for_function("!document.querySelector('#mt-out').innerText.includes('Senza la formula')", timeout=60000)
            t = pg.inner_text("#mt-tbl").replace("\t", " ")
            for f in ("C9H17N4O3S", "C9H14N3O3S", "C5H9N2OS"): assert f in t.replace(" ", ""), f
            assert "quasi vuoto" in t
        step("with the formula the nodes are sub-formulas (261, 244, 145)", fixed)
        def xl():
            with pg.expect_download() as d: pg.click("#mt-xlsx")
            assert d.value.suggested_filename.startswith("albero_MSn") and os.path.getsize(d.value.path()) > 1500
        step("Excel of the MSn tree (paths and peaks)", xl)
        def node():
            pg.click("#mt-tbl tr[data-n='1']"); pg.wait_for_selector("#mt-pk tr[data-mz]", timeout=10000)
            t = pg.inner_text("#mt-pk"); assert "244.07" in t and "C9H14N3O3S" in t.replace(" ", ""), t[:300]
            pg.click("#mt-pk tr[data-mz]"); pg.wait_for_selector("#cmp-mz", timeout=10000)
            assert pg.input_value("#cmp-par") == "C9H17N4O3S", pg.input_value("#cmp-par")
        step("a node shows its peaks; a peak opens «Formule compatibili» with the sub-formula of the node", node)
        def wrong():
            pg.evaluate("document.querySelector('#bigdlg').close()")
            pg.evaluate("COMP.tree(E.files.findIndex(f=>f.kind==='infusion'), 'Xx9')"); pg.wait_for_selector("#mt-out .fail", timeout=60000)
        step("a wrong formula is said", wrong)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
