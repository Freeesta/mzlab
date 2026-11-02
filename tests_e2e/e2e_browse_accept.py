"""Browse (the #pick input of the start screen) keeps a short accept list of plain extensions (a long or MIME-based one makes the OS file dialog
open slowly): the input stays multiple, still takes the intended file kinds, and an unknown file that gets through anyway is turned down by the intake
(synthetic mzML only, dev server)."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
D = Path(tempfile.mkdtemp(prefix="browse_accept_"))
synth.write_full(D / "campione.mzML", 5, seed=1)
(D / "appunti.xyz").write_text("x")
r = Run(port=8830 + 150, wd="/tmp/wd_browse_accept")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#pick", state="attached", timeout=60000)
        def attr():
            acc = pg.get_attribute("#pick", "accept") or ""
            es = acc.split(",")
            assert 0 < len(es) <= 8 and len(set(es)) == len(es) and all(e == e.lower() and e.startswith(".") for e in es), acc
            assert {".mzml", ".mzxml", ".raw", ".wiff", ".scan"} <= set(es), acc
            assert pg.get_attribute("#pick", "multiple") is not None and pg.get_attribute("#pickdir", "webkitdirectory") is not None
        step("the Browse input has a short lowercase-extension accept list and keeps multiple selection (folder input unchanged)", attr)
        def known():
            pg.set_input_files("#pick", str(D / "campione.mzML"))
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=90000)
        step("an mzML chosen with Browse loads", known)
        def unknown():
            pg.set_input_files("#pick", str(D / "appunti.xyz"))
            pg.wait_for_function("document.getElementById('unopen') && document.getElementById('unopen').textContent.includes('appunti.xyz')", timeout=30000)
            assert pg.evaluate("document.querySelectorAll('#flist input[data-k=use]').length") == 1
        step("an unknown file is turned down with a message, not loaded", unknown)
        r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
for s in steps: print("  ", s)
r.report()
bad = [s for s in steps if s[1] != "ok"]
sys.exit(1 if bad or any(e for e in r.errs if e[0] in ("pageerror",)) else 0)
