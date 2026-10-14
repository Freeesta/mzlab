# e2e_xss: HTML in a file name or in the instrument / run metadata of an mzML is shown as text, runs nothing, and the CSP is not violated in the basic flow
import os, sys, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tools"))
from pathlib import Path
from lib import *
import numpy as np, dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
PAY = '<img src=x onerror="window.__xss=1">'
ESC = PAY.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")
tmp = Path(tempfile.mkdtemp(prefix="xss_"))
src = tmp / "src.mzML"; ds.full_scan(src, 0, np.random.default_rng(1))
text = src.read_text(encoding="utf-8").replace('name="3200 QTRAP"', f'name="{ESC}"').replace('<run id="src"', f'<run id="{ESC}"')
bad = tmp / "x<img src=x onerror=window.__xss=1>.mzML"; bad.write_text(text, encoding="utf-8"); src.unlink()
r = Run(port=8847, wd="/tmp/wd_xss")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("window.__xss = 0")
        dialogs = []; pg.on("dialog", lambda d: (dialogs.append(d.message), d.dismiss()))
        pg.set_input_files("#pick", [str(bad)])
        attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length>=1")
        pg.click("text=Carica dati"); ready(pg)
        def nothing_ran():
            assert pg.evaluate("window.__xss") == 0, "the payload ran"
            assert not dialogs, dialogs
            assert pg.evaluate("document.querySelectorAll('img[src=x]').length") == 0, "an <img src=x> element was created from the data"
        step("file name and metadata with HTML: nothing created, nothing run", nothing_ran)
        def as_text():
            assert "onerror=window.__xss=1" in pg.evaluate("document.body.innerText"), "the name is shown as text"
        step("the name is visible as plain text", as_text)
        def csp():
            v = [e for e in r.errs if "Content Security Policy" in str(e)]
            assert not v, v
        step("no CSP violation in the basic flow", csp)
        def meta():
            c = pg.evaluate("document.querySelector('meta[http-equiv=Content-Security-Policy]').content")
            for need in ("object-src 'none'", "base-uri 'none'", "form-action 'self'"): assert need in c, need
        step("CSP forbids plugins, base and form targets", meta)
finally:
    r.close(); r.report(); import shutil; shutil.rmtree(tmp, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
