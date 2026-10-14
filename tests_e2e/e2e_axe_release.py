"""axe-core on the areas not covered by e2e_axe_grafici / e2e_a11y_dati: the HR bench, Disegno (Ketcher's own iframe excluded), Pratica and the guided tour.
Same rule as e2e_axe_grafici: no serious or critical violation (wcag2a/2aa/21a/21aa/22aa). axe comes from axe-playwright-python or $AXE_JS; without either: SKIP."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load

AXE = axe_path()
if not AXE:
    print("SKIP axe-core not available (pip install axe-playwright-python)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_axe_release.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))
RUN = ("axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations"
       ".filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,4).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))")
def axe(pg, ctx="document", exclude=None):
    if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=AXE)
    c = ctx if not exclude else "{include: [[%s]], exclude: [%s]}" % (ctx, ",".join("['%s']" % e for e in exclude))
    v = pg.evaluate(RUN % c); assert not v, v
D = synth("axe_rel")
r = Run(port=8868, wd="/tmp/wd_axe_rel")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 1100})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        step("axe: the HR bench (panels, toolbar, status bar)", lambda: axe(pg, "'#dpanels'"))
        def hrinfo():
            pg.evaluate("BARRA.setTab('hdr')"); pg.wait_for_timeout(900); axe(pg, "'#hrinfo'")
        step("axe: the information panel of the HR bench", hrinfo)
        step("axe: the whole page with the HR bench open", lambda: axe(pg))
        def draw():
            pg.click("#nav [data-v=draw]")
            for _ in range(300):
                if pg.evaluate("() => !!(window.TPDraw && TPDraw.ready())"): break
                pg.wait_for_timeout(100)
            pg.wait_for_timeout(800); axe(pg, "'body'", ["#kframe"])
        step("axe: Disegno, outside the Ketcher iframe", draw)
        base = f"http://127.0.0.1:{r.port}/static/teoria/"
        for pr in ("pratica-ei", "pratica-orale", "pratica-perdite", "pratica-isotopi", "pratica-formula", "pratica-strumento"):
            def pra(pr=pr):
                pg.goto(base + pr + ".html"); pg.wait_for_timeout(1500); axe(pg)
            step("axe: " + pr, pra)
        def tour():
            q = r.b.new_page(viewport={"width": 1500, "height": 1100}, reduced_motion="reduce")
            try:
                q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_timeout(1000)
                q.evaluate("try{localStorage.removeItem('qqq.tour.lr')}catch(_){}")
                q.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); q.wait_for_timeout(1000)
                q.click("text=Carica dati"); ready(q); q.wait_for_timeout(1500)
                q.wait_for_selector("#tour-invito", timeout=5000); axe(q, "'#tour-invito'")
                q.click("#tour-inv-no"); q.click("#np-set"); q.wait_for_selector("#uipset"); q.click("#uip-tour")
                q.wait_for_selector("#tour-dialog", timeout=3000)
                for i in range(15):
                    axe(q, "'#tour-dialog'")
                    if not q.is_visible("#tour-btn-next") or q.get_attribute("#tour-btn-next", "disabled") is not None: break
                    q.click("#tour-btn-next"); q.wait_for_timeout(300)
            finally: q.close()
        step("axe: the invitation and the steps of the tour", tour)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
