"""A method (.dam) that does not match the open data (MRM method with a Full Scan file) gives a visible notice, not a toast; a matching one gives none."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_dam_incoerente.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
DAMDIR = DAM.parent
MRM_DAM = DAMDIR / "Lab_inq_MRM_Carba.dam"
r = Run(port=8964, wd="/tmp/wd_dam")
try:
    if not (DAM.exists() and MRM_DAM.exists()):
        steps.append(("SKIP no .dam files", "ok")); raise SystemExit
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0")]); pg.wait_for_timeout(1000)
        pg.set_input_files("#pickdam", [str(DAM), str(MRM_DAM)]); pg.wait_for_timeout(1200)
        pg.click("text=Load data"); ready(pg); pg.wait_for_timeout(800)
        def warn():
            assert pg.is_visible("#methwarn"), "visible notice"
            t = pg.inner_text("#methwarn"); print(t)
            assert "non corrisponde" in t and "Lab_inq_MRM_Carba.dam" in t and "MRM" in t and "Full Scan" in t, t
            assert "FullMass_pos_max480" not in t, t                  # the matching method is not mentioned
        step("an MRM method with a Full Scan file: a visible notice naming both", warn)
        def dismiss():
            pg.click("#methwarn-x"); pg.wait_for_timeout(200); assert not pg.is_visible("#methwarn")
        step("the notice can be dismissed", dismiss)
    r.close()
except SystemExit:
    try: r.close()
    except Exception: pass
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
