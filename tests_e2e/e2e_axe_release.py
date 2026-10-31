"""axe-core on the areas not covered by e2e_axe_grafici / e2e_a11y_dati: the HR bench, Disegno (Ketcher's own iframe excluded) and the guided tour.
Same rule as e2e_axe_grafici: no serious or critical violation (wcag2a/2aa/21a/21aa/22aa). axe comes from axe-playwright-python or $AXE_JS; without either: SKIP."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
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
TMP = "/tmp/Esempio_FullScan_t15.mzML"; shutil.copy(str(ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML"), TMP)
r = Run(port=8868, wd="/tmp/wd_axe_rel")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 1100})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        step("axe: the HR bench (panels, toolbar, status bar)", lambda: axe(pg, "'#dpanels'"))
        def grips():
            small = pg.evaluate("[...document.querySelectorAll('.aff-grip')].map(e => e.getBoundingClientRect()).filter(r => r.width < 24 || r.height < 24).length")
            assert pg.locator(".aff-grip").count() and not small, small
        step("WCAG 2.5.8: the panel grips (.aff-grip) are at least 24 x 24 px", grips)
        def hrinfo():
            pg.evaluate("BARRA.setTab('hdr')"); attendi(pg, "(document.querySelector('#hri-body')?.innerText || '').length > 0"); axe(pg, "'#hrinfo'")
        step("axe: the information panel of the HR bench", hrinfo)
        step("axe: the whole page with the HR bench open", lambda: axe(pg))
        def draw():
            pg.click("#nav [data-v=draw]")
            attendi(pg, "!!(window.TPDraw && TPDraw.ready())", 60000)
            attendi(pg, "!!document.querySelector('#kframe')?.contentDocument?.querySelector('#tp-bar')", 60000); axe(pg, "'body'", ["#kframe"])
        step("axe: Disegno, outside the Ketcher iframe", draw)
        def tour():
            q = r.b.new_page(viewport={"width": 1500, "height": 1100}, reduced_motion="reduce")
            try:
                q.goto(f"http://127.0.0.1:{r.port}/"); q.wait_for_selector("#pick", state="attached")
                q.evaluate("try{localStorage.removeItem('mzlab.tour.lr')}catch(_){}")
                stage(q, [TMP])
                q.click("text=Carica dati"); attendi(q, "typeof E !== 'undefined' && E.files.length >= 1")
                q.wait_for_selector("#tour-invito", timeout=5000); axe(q, "'#tour-invito'")
                q.click("#tour-inv-no"); q.click("#np-set"); q.wait_for_selector("#uipset"); q.evaluate("document.getElementById('uip-g-app').open = true"); q.click("#uip-tour")
                q.wait_for_selector("#tour-dialog", timeout=3000)
                for i in range(15):
                    axe(q, "'#tour-dialog'")
                    if not q.is_visible("#tour-btn-next") or q.get_attribute("#tour-btn-next", "disabled") is not None: break
                    hd = q.inner_text("#tour-dialog .tour-hd")
                    q.click("#tour-btn-next"); attendi(q, "document.querySelector('#tour-dialog .tour-hd')?.textContent !== %r" % hd)
            finally: q.close()
        step("axe: the invitation and the steps of the tour", tour)
        def landing_states():
            # the landing, files arriving, the sample table and the first panel, at a wide and at a narrow window
            import tempfile, numpy as np
            sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
            d = Path(tempfile.mkdtemp()); rng = np.random.default_rng(3)
            files = []
            for t in (0, 15, 30, 45, 60):
                ds.full_scan(d / f"B_FullMass-t{t}.mzML", t, rng); files.append(str(d / f"B_FullMass-t{t}.mzML"))
            init = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}"
            drop = """(sel) => { const i = document.querySelector('#__tmpdrop'), t = document.querySelector(sel), dt = new DataTransfer(); for (const f of i.files) dt.items.add(f);
              for (const ty of ['dragenter', 'dragover', 'drop']) t.dispatchEvent(new DragEvent(ty, {bubbles: true, cancelable: true, dataTransfer: dt})); i.remove(); }"""
            for w in (1440, 820):
                q = r.b.new_page(viewport={"width": w, "height": 900}, reduced_motion="reduce", locale="it-IT")
                q.add_init_script(init)
                try:
                    q.goto(f"http://127.0.0.1:{r.port}/?telefono=0"); q.wait_for_selector("#drop", state="attached"); attendi(q, "!document.querySelector('#v-data').hidden && document.querySelector('#sb-toggle').innerText.trim().length > 0")
                    axe(q)
                    small = q.evaluate("[...document.querySelectorAll('#stbar button:not([disabled]), #sb-toggle, #drop')].filter(e => { const b = e.getBoundingClientRect(); return b.width && (b.width < 24 || b.height < 24); }).length")
                    assert not small, ("targets below 24 px on the landing", w, small)
                    dis = q.evaluate("[...document.querySelectorAll('#stbar button[disabled], #np-chrom[disabled], #np-spec[disabled]')].filter(e => e.getAttribute('aria-disabled') !== 'true').length")
                    assert not dis, ("disabled toolbar buttons without aria-disabled", w, dis)
                    q.evaluate("() => { const i = document.createElement('input'); i.type = 'file'; i.multiple = true; i.id = '__tmpdrop'; i.style.display = 'none'; document.body.appendChild(i); }")
                    lib._real_sif(q, "#__tmpdrop", files); q.evaluate(drop, "#drop")
                    q.wait_for_function("document.querySelectorAll('#flist tbody tr, #flst .fl').length >= 1", timeout=60000)
                    axe(q)
                    live = q.evaluate("[...document.querySelectorAll('[role=status],[aria-live]:not([aria-live=off])')].filter(e => !e.hidden && e.getBoundingClientRect().width > 0 && (e.textContent || '').trim()).length")
                    assert live <= 1, ("more than one live region speaks", w, live)
                    q.wait_for_function("document.querySelectorAll('#flist tbody tr').length >= 5", timeout=90000); ready(q)
                    if w < 900:
                        q.click("#sb-toggle"); attendi(q, "document.querySelector('#v-data').classList.contains('sb-open')")
                    q.click("#samptab"); q.wait_for_function("document.querySelector('#v-workspace').dataset.work === 'table'"); attendi(q, "document.querySelector('#v-workspace').dataset.work === 'table'")
                    axe(q)
                    q.locator("#flist .ldtab button[data-open]").nth(1).click(); q.wait_for_selector(".pnl", timeout=60000); ready(q)
                    axe(q)
                finally: q.close()
        step("axe: landing, files arriving, sample table and first panel at 1440 and 820 px", landing_states)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
