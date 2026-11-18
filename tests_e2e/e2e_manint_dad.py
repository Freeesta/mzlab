"""The card «Manual integrations» lists a DAD (XWC) window with its area in mAU·s, and its numbers (area, height, apex, below the line, correction) are those of the
toolbar dialog table. Synthetic MRM .wiff of tools/wiff_dad_mrm.py (no laboratory data). Step 1 of one integration table.
Run from the repo root: python3 tests_e2e/e2e_manint_dad.py"""
# covers: mzlab/web/integrazioni-tracce, mzlab/web/integrazioni-manuali
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="manintdad_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dad_mrm.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "dadmrm.json").read_text())
MRM = "E.panels.find(q => q.type === 'mrm')"
TWC = "E.panels.find(q => q.type === 'chrom' && q.kind === 'pda' && q.mz0 == null)"
XWCS = "E.panels.filter(q => q.type === 'chrom' && q.kind === 'pda' && q.mz0 != null)"
r = Run(port=8883, wd="/tmp/wd_manint_dad")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "dadmrm.wiff"), str(D / "dadmrm.wiff.scan")])
        wait_for_js(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=90000); pg.click("text=Load data"); ready(pg)
        wait_for_js(pg, "E.files.length >= 1", timeout=120000)
        open_kind(pg, "mrm"); wait_for_js(pg, f"!!{MRM} && !!{MRM}.el.querySelector('[data-o=canali]')", timeout=30000)
        pg.evaluate(f"{MRM}.el.querySelector('[data-o=canali]').click()")
        wait_for_js(pg, f"!!{MRM}.el.querySelector('.can-pop:not([hidden]) [data-kind=pda]')")
        pg.evaluate(f"{MRM}.el.querySelector('.can-pop [data-kind=pda]').click()")
        wait_for_js(pg, f"!!{TWC} && !!{TWC}._a", timeout=60000)
        pg.evaluate(f"guardInt({TWC})"); wait_for_js(pg, "document.querySelector('#askdlg') && document.querySelector('#askdlg').open"); pg.click("#askok")
        wait_for_js(pg, f"{XWCS}.length === 1 && !!{XWCS}[0]._a && {XWCS}[0]._a.sr.length === 1", timeout=60000)
        pg.evaluate(f"(() => {{ const p = {XWCS}[0]; p.ints = []; autoInt(p, p._a.sr[0], {TRUE['a']['rt']}); draw(p); }})()")
        wait_for_js(pg, f"{XWCS}[0].ints.length === 1 && Number.isFinite({XWCS}[0].ints[0].area)", timeout=30000)
        wait_for_js(pg, "MANTR.count() > 0", timeout=60000)
        def card():
            wait_for_js(pg, "!!document.querySelector('#manint-x tbody tr')", timeout=60000)
            hs = pg.locator("#manint-x th").all_inner_texts()
            assert pg.evaluate("I18N.t('canali.manint.h.areaUv')") in hs, hs
            assert pg.evaluate("MANINT.rows(MANTR.csv())[1][MANINT.rows(MANTR.csv())[0].indexOf('area_unit')]") == "mAU·s"
        step("the card row of an XWC window has its area in mAU·s", card)
        def parity():
            d = pg.evaluate("allInts().map(r => ({panel: r.panel, a: r.a, b: r.b, area: r.area, height: r.height, rt: r.rt, below: r.below || 0, corr: r.corr || ''}))")
            c = pg.evaluate("(() => { const [h, ...b] = MANINT.rows(MANTR.csv()); const k = n => h.indexOf(n); return b.map(r => ({panel: r[k('panel')], a: +r[k('start_min')], area: +r[k('area')], height: +r[k('height')], rt: +r[k('apex_rt_min')], below: +r[k('below')] || 0, corr: r[k('correction')]})); })()")
            assert len(d) == len(c) == 1, (d, c)
            d, c = d[0], c[0]
            for k in ("area", "height", "below"): assert abs(c[k] - d[k]) <= 1e-4 * max(abs(d[k]), 1), (k, d, c)
            assert abs(c["rt"] - d["rt"]) < 6e-4 and abs(c["a"] - min(d["a"], d["b"])) < 6e-4 and c["corr"] == d["corr"], (d, c)
        step("area, height, apex, below and correction of the XWC card row are those of the dialog table", parity)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] in ("pageerror", "console.error")]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
