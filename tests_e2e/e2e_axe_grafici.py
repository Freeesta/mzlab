"""axe-core on the charts of the Dati view (panels, table of the data): no serious or critical violation. axe comes from the python package
axe-playwright-python (pip install axe-playwright-python) or from $AXE_JS; without either the test says SKIP and passes."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *


AXE = axe_path()
if not AXE:
    print("SKIP axe-core not available (pip install axe-playwright-python)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_axe_grafici.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))
r = Run(port=8867, wd="/tmp/wd_axe")
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(1500)
        pg.add_script_tag(path=AXE)
        def panels():
            v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
        step("axe: the chart panels have no serious or critical violation", panels)
        def tabledlg():
            pg.evaluate("E.panels.find(p=>p.type==='chrom').cv.focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            v = pg.evaluate(RUN % "'#tdati'"); assert not v, v
            pg.keyboard.press("Escape")
        step("axe: the data table dialog has no serious or critical violation", tabledlg)
        MEAS = """(coarse) => [...document.querySelectorAll('#dpanels button, #dpanels select, #dpanels input, #dpanels [role=slider], #dpanels [role=separator], #dpanels [tabindex="0"]')]
              .filter(e => e.offsetParent && !e.closest('[hidden]') && !e.closest('.hmore:not([hidden])') && (coarse || !e.classList.contains('pgrip')))
              .map(e => [e, (e.closest('label') && e.matches('input[type=checkbox],input[type=radio]') ? e.closest('label') : e).getBoundingClientRect()])
              .filter(([e, r]) => (r.width < 24 || r.height < 24) && getComputedStyle(e).display !== 'inline')
              .map(([e, r]) => (e.dataset.a || e.dataset.o || e.className || e.tagName) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height))"""
        def targets():
            small = pg.evaluate("(" + MEAS + ")(false)"); assert not small, small
        step("WCAG 2.5.8: the controls of the panels are at least 24 x 24 px", targets)
        def keys():
            CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
            pg.evaluate(f"{CH}.cv.focus()"); w0 = pg.evaluate(f"{CH}._a.x1-{CH}._a.x0")
            pg.keyboard.press("+"); pg.wait_for_timeout(500); w1 = pg.evaluate(f"{CH}._a.x1-{CH}._a.x0"); assert w1 < w0 * 0.6, (w0, w1)
            x0 = pg.evaluate(f"{CH}._a.x0"); pg.keyboard.press("]"); pg.wait_for_timeout(500); assert pg.evaluate(f"{CH}._a.x0") > x0
            pg.keyboard.press("0"); pg.wait_for_timeout(500); assert pg.evaluate(f"{CH}.zoom") is None
        step("WCAG 2.5.7: zoom and pan without dragging (+ - [ ] 0 on the focused chart)", keys)
        def touch():
            ctx = r.b.new_context(viewport={"width": 1024, "height": 1366}, has_touch=True, is_mobile=True, locale="it-IT")
            q = ctx.new_page(); q.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
            q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_timeout(1500)
            try:
                q.evaluate("1"); ready(q)
                small = q.evaluate("(" + MEAS + ")(true)"); assert not small, small
            finally: ctx.close()
        step("touch device: grips and controls at least 24 x 24 px", touch)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
