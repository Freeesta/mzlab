"""Gradient and analog traces: the Method window offers the gradient section when the file has none (mzML files never do), the course method and the editor store
it for the file, the file lists its pressure trace; the answers of api/analog and api/gradient."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gradiente.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "gradiente_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "GRAD_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(3), pressure=True)
r = Run(port=8865, wd="/tmp/wd_gradiente")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def listed():
            a = pg.evaluate("E.files[0].analog"); assert len(a) == 1 and a[0]["kind"] == "pressure" and a[0]["unit"] == "pascal" and a[0]["n"] == 6000, a
            j = pg.evaluate("(async () => await (await fetch('api/analog?k=0&i=%d&n=300')).json())()" % a[0]["i"])
            assert j["unit"] == "bar" and j["unit_note"]["key"] == "analog.unit.tenfold" and abs(np.interp(1.0, j["t"], j["y"]) - 278) < 12, j["unit_note"]
        step("the file lists its pressure trace; the unit is corrected", listed)
        def section():
            pg.click("#np-method"); pg.wait_for_selector("#grad-sec"); 
            assert pg.is_visible("#grad-course") and pg.is_visible("#grad-raw") and pg.is_visible("#grad-csv") and pg.is_visible("#grad-new")
            assert pg.evaluate("(async () => (await GRADIENTE.get(E.files[0])))()") is None
        step("Method window: gradient section, no gradient yet", section)
        def course():
            pg.click("#grad-course"); pg.wait_for_selector("#grad-sec .lct"); ready(pg)
            g = pg.evaluate("(async () => (await (await fetch('api/gradient?k=0')).json()).gradient)()")
            assert g["source"] == "course" and [p["b"] for p in g["points"]] == [10, 10, 60, 70, 100, 100, 10, 10], g
            assert pg.evaluate("Object.keys(NB.gradienti||{}).length") == 1
            assert pg.evaluate("!!localStorage.getItem('qqq.gradiente.ultimo')")
        step("«Metodo del corso» stores the gradient for the file (and the last used)", course)
        def editor():
            pg.click("#grad-rm"); pg.wait_for_selector("#grad-new"); ready(pg)
            assert pg.evaluate("(async () => (await (await fetch('api/gradient?k=0')).json()).gradient)()") is None
            pg.click("#grad-new"); pg.wait_for_selector("#grad-ok")
            pg.fill('#grad-ed input[data-c=t][data-i="0"]', "0"); pg.fill('#grad-ed input[data-c=b][data-i="0"]', "5")
            pg.fill('#grad-ed input[data-c=t][data-i="1"]', "10"); pg.fill('#grad-ed input[data-c=b][data-i="1"]', "95")
            pg.click("#grad-ok"); pg.wait_for_selector("#grad-sec .lct"); ready(pg)
            g = pg.evaluate("(async () => (await (await fetch('api/gradient?k=0')).json()).gradient)()")
            assert g["source"] == "manual" and [(p["t"], p["b"]) for p in g["points"]] == [(0, 5), (10, 95)], g
        step("the table editor stores a typed gradient; «Togli» removes it", editor)
        def paste():
            j = pg.evaluate("(async () => await (await fetch('api/gradient_parse', {method:'POST', body: JSON.stringify({fmt:'csv', text:'0\\t10\\n5,5\\t60\\n'})})).json())()")
            assert j["gradient"]["points"][1] == {"t": 5.5, "b": 60.0} or j["gradient"]["points"][1]["t"] in (5, 5.5), j
        step("pasted text from Excel becomes a gradient", paste)
        TIC = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full'&&p.kind==='tic')"
        def ctx_items():
            pg.evaluate(f"(()=>{{const p={TIC};p.el.scrollIntoView({{block:'center'}})}})()"); pg.wait_for_timeout(300)
            b = pg.evaluate(f"(()=>{{const p={TIC},r=p.cv.getBoundingClientRect();return [r.left+p._a.X(10),r.top+r.height*0.5]}})()")
            pg.mouse.click(b[0], b[1], button="right"); pg.wait_for_selector("#ctx:not([hidden])")
            labs = pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(e=>e.textContent)")
            pg.keyboard.press("Escape"); return labs
        def no_entry():
            pg.evaluate("{const d=Q('#bigdlg'); if (d.open) d.close()}"); pg.wait_for_timeout(300)
            pg.evaluate("(async () => { await GRADIENTE.set(E.files[0], null); redrawAll(); })()"); pg.wait_for_timeout(1200); ready(pg)
            assert not any("gradiente" in l.lower() for l in ctx_items())
        step("right click on a chromatogram: no gradient entry without a gradient", no_entry)
        def entry():
            pg.evaluate("(async () => { const g = (await (await fetch('api/gradient_parse', {method:'POST', body: JSON.stringify({fmt:'course'})})).json()).gradient; await GRADIENTE.set(E.files[0], g); })()")
            pg.evaluate("redrawAll()"); pg.wait_for_timeout(1200); ready(pg)
            labs = ctx_items(); assert any("Mostra il gradiente" in l for l in labs), labs
            pg.evaluate(f"(()=>{{const p={TIC};p.cv.dispatchEvent(new MouseEvent('contextmenu',{{bubbles:true,clientX:300,clientY:300}}))}})()")
            pg.click("#ctx [role=menuitemcheckbox]:has-text('gradiente')"); pg.wait_for_timeout(800); ready(pg)
            assert pg.evaluate(f"{TIC}.overlay") == "grad"
            assert pg.evaluate("palette = BARRA.commands.find(c=>c.id==='gradient'); !!palette && palette.visible()")
        step("the entry appears with a gradient and turns the overlay on (and the palette has it)", entry)
        def reading():
            b = pg.evaluate(f"GRADIENTE.reading({TIC}, 11.5).b"); assert abs(b - 57.5) < 1e-6, b
            assert pg.evaluate("window.MARG_R") >= 40 and pg.evaluate(f"{TIC}._a.grad") is True
            pg.evaluate(f"(()=>{{{TIC}.cur=11.5;cursorLine({TIC})}})()")
            assert "57.5" in pg.evaluate(f"{TIC}.ch.getAttribute('aria-valuetext')")
        step("right axis, %B at 11.5 min and in the cursor reading", reading)
        def aligned():
            pg.evaluate("addPanel('chrom',{tab:'full',kind:'pda'})"); pg.wait_for_timeout(1500); ready(pg)
            LN = "E.panels.filter(p=>p.type==='chrom'&&p.tab==='full')"
            for w, h in ((1280, 2200), (1440, 2200), (1680, 2200), (1024, 1366)):
                pg.set_viewport_size({"width": w, "height": h}); pg.wait_for_timeout(1300); ready(pg)
                e = pg.evaluate(f"{LN}.map(p=>{{const r=p.cv.getBoundingClientRect(),a=p._a;return [r.left+a.X(a.x0),r.left+a.X(a.x1)]}})")
                assert len(e) == 2 and abs(e[0][0] - e[1][0]) <= 1 and abs(e[0][1] - e[1][1]) <= 1, (w, e)
        step("TIC with %B and PDA without keep the same plot edges (1280/1440/1680/iPad)", aligned)
        def delay():
            pg.evaluate(f"{TIC}.gdelay = 1; draw({TIC})"); ready(pg)
            r = pg.evaluate(f"GRADIENTE.reading({TIC}, 12.5)"); assert abs(r["b"] - 65) < 1e-6 and abs(r["bcol"] - 57.5) < 1e-6, r
        step("a delay of 1.0 min moves the reading at the column", delay)
        def saved():
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(300)
            u = pg.evaluate("NB.ui.panels.filter(q=>q.overlay).map(q=>[q.overlay,q.gdelay])"); assert u == [["grad", 1]], u
            assert pg.evaluate("!!GRADIENTE.exportState()")
        step("overlay and delay are saved with the session; the gradient goes to the .mzflow", saved)
        def pdadelay():
            j = pg.evaluate("(async () => (await (await fetch('api/delay?k=0')).json()).delay)()")
            assert j and abs(j["delay"]) < 0.03 and j["peaks"] >= 1, j
        step("the PDA-mass delay is measured by the server", pdadelay)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
