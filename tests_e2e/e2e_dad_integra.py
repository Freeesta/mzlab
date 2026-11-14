"""Integrating DAD traces on the built site with the Rust engine, on the synthetic MRM `.wiff` of tools/wiff_dad_mrm.py (no laboratory data): an XWC
integrates like an XIC (no question, its area in the integration table); its area is in mAU·s (table header, tooltip); the TWC asks `int.guard.uv` (not the ion text of `int.guard`) and «yes» opens
the XWC the engine proposes (274 ± 2 nm) under it; «no» opens nothing. The DAD names are on screen: «DAD (TWC)» in the picker, «DAD TWC» and «XWC» titles.
Run from the repo root: python3 tests_e2e/e2e_dad_integra.py"""
# covers: tools/wiff_dad_mrm.py, mzlab/web/xwc.js
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="dadint_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "dadmrm.json").read_text())
MRM = "E.panels.find(q => q.type === 'mrm')"
TWC = "E.panels.find(q => q.type === 'chrom' && q.kind === 'pda' && q.mz0 == null)"
XWCS = "E.panels.filter(q => q.type === 'chrom' && q.kind === 'pda' && q.mz0 != null)"
ASK = "(() => { const d = document.querySelector('#askdlg'); return d && d.open ? document.querySelector('#asktxt').innerHTML : ''; })()"
r = Run(port=8882, wd="/tmp/wd_dadint")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "dadmrm.wiff"), str(D / "dadmrm.wiff.scan")])
        wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=90000); pg.click("text=Carica dati"); ready(pg)
        wait_for_js(pg, "E.files.length >= 1", timeout=120000)
        def twc():
            open_kind(pg, "mrm"); wait_for_js(pg, f"!!{MRM} && !!{MRM}.el.querySelector('[data-o=canali]')", timeout=30000)
            pg.evaluate(f"{MRM}.el.querySelector('[data-o=canali]').click()")
            wait_for_js(pg, f"!!{MRM}.el.querySelector('.can-pop:not([hidden]) [data-kind=pda]')")
            assert pg.evaluate(f"{MRM}.el.querySelector('.can-pop [data-kind=pda]').textContent.trim()").startswith("DAD")
            pg.evaluate(f"{MRM}.el.querySelector('.can-pop [data-kind=pda]').click()")
            wait_for_js(pg, f"!!{TWC} && !!{TWC}._a && !!{TWC}.el.querySelector('[data-a=xwcl]:not([hidden])')", timeout=60000)
            assert pg.evaluate(f"TITOLI.di({TWC})") == "DAD TWC", pg.evaluate(f"TITOLI.di({TWC})")
        step("the DAD trace of an MRM file is the «DAD TWC» panel", twc)
        def no():
            assert pg.evaluate(f"guardInt({TWC})") is False
            wait_for_js(pg, f"{ASK}.length > 0")
            t = pg.evaluate(ASK)
            assert t == pg.evaluate("I18N.t('int.guard.uv')") and "XWC" in t and "XIC" not in t, t
            pg.click("#askno"); wait_for_js(pg, f"!{ASK}")
            assert pg.evaluate(f"{XWCS}.length") == 0
        step("integrating the TWC asks int.guard.uv (no ion text); «no» opens nothing", no)
        def yes():
            assert pg.evaluate(f"guardInt({TWC})") is False
            wait_for_js(pg, f"{ASK}.length > 0"); pg.click("#askok")
            wait_for_js(pg, f"{XWCS}.length === 1 && !!{XWCS}[0]._a && {XWCS}[0]._a.sr.length === 1", timeout=60000)
            q = pg.evaluate(f"(() => {{ const q = {XWCS}[0], st = stackOrder(); return {{ nm0: q.mz0, nm1: q.mz1, ttl: TITOLI.di(q), under: st.indexOf(q) === st.indexOf({TWC}) + 1 }}; }})()")
            assert abs((q["nm0"] + q["nm1"]) / 2 - 274) <= 2 and q["under"] and q["ttl"].startswith("XWC "), q
        step("«yes» opens the XWC the engine proposes (274 ± 2 nm) under the TWC", yes)
        def integrate():
            assert pg.evaluate(f"guardInt({XWCS}[0])") is True and not pg.evaluate(ASK)
            pg.evaluate(f"(() => {{ const p = {XWCS}[0]; p.ints = []; autoInt(p, p._a.sr[0], {TRUE['a']['rt']}); draw(p); }})()")
            wait_for_js(pg, f"{XWCS}[0].ints.length === 1 && Number.isFinite({XWCS}[0].ints[0].area)", timeout=30000)
            i = pg.evaluate(f"(() => {{ const i = {XWCS}[0].ints[0]; return {{ area: i.area, rt: i.rt }}; }})()")
            assert i["area"] > 0 and abs(i["rt"] - TRUE["a"]["rt"]) < 0.05, i
            pg.evaluate("showInts()"); wait_for_js(pg, "document.querySelector('#bigdlg') && document.querySelector('#bigdlg').open")
            wait_for_js(pg, "document.querySelector('#bigbody').innerText.includes('XWC')", timeout=30000)
            hs = pg.evaluate("[...document.querySelectorAll('#bigbody th')].map(e => e.textContent)")
            assert pg.evaluate("I18N.t('int.tbl.areaUv')") in hs and pg.evaluate("I18N.t('int.tbl.area')") not in hs, hs
            pg.evaluate("document.querySelector('#bigdlg').close()")
            tip = pg.evaluate(f"(() => {{ const l = ({XWCS}[0]._a.lbls || []).find(l => l.int); return l ? l.tip : ''; }})()")
            assert "mAU·s" in tip, tip
        step("an XWC integrates like an XIC: no question, the peak of compound A has its area in mAU·s in the integration table and its tooltip", integrate)
        def unit():
            t = pg.evaluate(f"JSON.stringify(plotSheets({XWCS}[0]))")
            assert pg.evaluate("I18N.t('export.col.pdaSignal')") in t and "mAU" in t and "PDA" not in t, t[max(0, t.find("PDA") - 300):t.find("PDA") + 100]
        step("the export of an XWC says absorbance in mAU (the y axis uses the same text)", unit)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
