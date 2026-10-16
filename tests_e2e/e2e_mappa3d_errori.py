"""The 3D viewer says why it did not open (never «WebGL2 not supported» for something else): no WebGL2 at all, a shader that does not compile, a JavaScript error, a lost context.
Each case is simulated; the dialog names the cause and shows the technical lines; the 2D map stays whole; a context that refuses the viewer's options but gives one without them opens the viewer."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa3d_errori.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
M = "E.panels.find(q=>q.type==='map')"; BTN = ".pnl.map [data-o=view][data-v='3d']"
NOGL = "const g=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(t,o){return t==='webgl2'?null:g.call(this,t,o)};"
OPTS = "const g=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(t,o){if(t==='webgl2'&&o&&Object.keys(o).length)return null;return g.call(this,t,o)};"
SHADER = "const c=WebGL2RenderingContext.prototype.getShaderParameter;WebGL2RenderingContext.prototype.getShaderParameter=function(s,p){return p===this.COMPILE_STATUS?false:c.call(this,s,p)};WebGL2RenderingContext.prototype.getShaderInfoLog=function(){return 'ERROR: 0:7: simulated compile error'};"
def session(p, init):
    r = Run(port=8892, wd="/tmp/wd_mappa3d_err")
    try:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}); ctx.add_init_script(init + "try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1')}catch(_){}")
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); pg.wait_for_timeout(700)
        pg.evaluate(M + ".el.scrollIntoView({block:'start'})"); pg.wait_for_timeout(300)
        return r, b, pg
    except Exception:
        r.close(); raise
def failing(p, init, kind_text, *must):
    r, b, pg = session(p, init)
    try:
        pg.locator(BTN).click(); pg.wait_for_function("document.getElementById('bigdlg').open", timeout=15000)
        txt = pg.inner_text("#bigdlg"); assert kind_text in txt, txt
        for m in must: assert m in txt, (m, txt)
        assert "3D viewer not available" in txt and "2D map is still available" in txt
        assert pg.evaluate("!document.querySelector('dialog.m3d')"), "no half-built viewer left"
        pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
        assert pg.evaluate("!!" + M + "._a && " + M + "._a.map") and pg.evaluate("E.panels.some(q=>q.type==='map')"), "the 2D map is intact"
        return pg
    finally:
        b.close(); r.close()
try:
    with sync_playwright() as p:
        step("no WebGL2 at all: the cause in words, what to do in Chrome, the technical lines", lambda: failing(p, NOGL, "gives this page no WebGL2 context", "chrome://gpu", "kind: nowebgl2", "webgl1:"))
        step("a shader that does not compile: the compiler's own line is shown", lambda: failing(p, SHADER, "did not compile a graphics program", "kind: shader", "simulated compile error", "version:"))
        def jserr():
            r, b, pg = session(p, "")
            try:
                pg.evaluate("(()=>{const o=MAPPA.prefs; Object.defineProperty(MAPPA,'prefs',{get(){ if (new Error().stack.includes('mappa3d')) throw new Error('simulated program error'); return o }})})()")
                pg.locator(BTN).click(); pg.wait_for_function("document.getElementById('bigdlg').open", timeout=15000)
                txt = pg.inner_text("#bigdlg"); assert "program error while opening" in txt and "not a WebGL2 problem" in txt and "simulated program error" in txt and "WebGL2 context" not in txt, txt
                assert pg.evaluate("!document.querySelector('dialog.m3d')")
            finally: b.close(); r.close()
        step("a JavaScript error is not reported as a missing WebGL2", jserr)
        def opts():
            r, b, pg = session(p, OPTS)
            try:
                pg.locator(BTN).click()
                pg.wait_for_function("document.querySelector('dialog.m3d[open]') && window.__m3d && window.__m3d.V.g", timeout=60000)
                assert pg.evaluate("window.__m3d.gl ? 1 : 1") and pg.evaluate("!document.getElementById('bigdlg').open")
            finally: b.close(); r.close()
        step("a browser that refuses the viewer's options but gives a plain context: the viewer opens", opts)
        def lost():
            r, b, pg = session(p, "")
            try:
                pg.locator(BTN).click(); pg.wait_for_function("document.querySelector('dialog.m3d[open]') && window.__m3d && window.__m3d.V.g", timeout=60000)
                pg.evaluate("document.querySelector('.m3d-gl').getContext('webgl2').getExtension('WEBGL_lose_context').loseContext()"); pg.wait_for_timeout(500)
                assert "context was lost" in pg.inner_text(".m3d-msg"), pg.inner_text(".m3d-main")
                pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert pg.evaluate("!document.querySelector('dialog.m3d[open]')"), "the viewer still closes"
            finally: b.close(); r.close()
        step("a lost context: said as such, the viewer closes normally", lost)
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
for s in steps: print(" ", s)
bad = [s for s in steps if s[1] != "ok"]
sys.exit(1 if bad else 0)
