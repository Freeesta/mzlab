"""axe-core on the charts of the Dati view (panels, table of the data): no serious or critical violation. axe comes from the python package
axe-playwright-python (pip install axe-playwright-python) or from $AXE_JS; without either the test says SKIP and passes."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

def axe_path():
    p = os.environ.get("AXE_JS")
    if p and os.path.exists(p): return p
    try:
        import axe_playwright_python
        q = os.path.join(os.path.dirname(axe_playwright_python.__file__), "axe.min.js")
        return q if os.path.exists(q) else None
    except ImportError:
        return None

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
        def targets():
            small = pg.evaluate("""(() => [...document.querySelectorAll('#dpanels button, #dpanels select, #dpanels input, #dpanels [role=slider], #dpanels [tabindex="0"]')]
              .filter(e => e.offsetParent && !e.closest('[hidden]') && !e.closest('.hmore:not([hidden])'))
              .map(e => [e, e.getBoundingClientRect()]).filter(([e, r]) => (r.width < 24 || r.height < 24) && getComputedStyle(e).display !== 'inline')
              .map(([e, r]) => (e.dataset.a || e.dataset.o || e.className || e.tagName) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height)))()""")
            assert not small, small
        step("WCAG 2.5.8: the controls of the panels are at least 24 x 24 px", targets)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
