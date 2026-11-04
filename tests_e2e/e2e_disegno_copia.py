"""Disegno: when the browser refuses the clipboard, the copy actions say «Copia non riuscita» (never «Copiato») and raise no error box; when it accepts, they say «Copiato»."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8994, wd="/tmp/wd_disegno_copia")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.addFragment('CCO')"); attendi(pg, f"!{KJ}.editor.struct().isBlank()")
        pg.evaluate("window.__t = []; const o = window.toast; window.toast = m => { window.__t.push(m); o(m); }")
        OPEN = """(()=>{const d=document.querySelector('#kframe').contentDocument;d.body.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:200,clientY:200}))})()"""
        def copy_first(mode):
            pg.evaluate("window.__t.length = 0; document.getElementById('errbox') && document.getElementById('errbox').remove()")       # an earlier error (e.g. a wasm not built) must not hide this check
            pg.evaluate("(m) => Object.defineProperty(navigator.clipboard, 'writeText', { configurable: true, value: () => m ? Promise.reject(new DOMException('no', 'NotAllowedError')) : Promise.resolve() })", mode == "refuse")
            for _ in range(200):
                pg.evaluate(OPEN)
                if pg.locator("#ctx:not([hidden]) div[role=menuitem]").count(): break
                pg.evaluate("new Promise(r => requestAnimationFrame(r))")
            pg.locator("#ctx div[role=menuitem]").first.click()
            attendi(pg, "window.__t.length > 0")
            return pg.evaluate("window.__t.join('|')")
        def refused():
            t = copy_first("refuse"); assert "Copia non riuscita" in t and "Copiato" not in t, t
            assert pg.locator("#errbox").count() == 0, "error box shown"
        step("a refused clipboard says «Copia non riuscita», no «Copiato», no error box", refused)
        def accepted():
            t = copy_first("accept"); assert "Copiato" in t and "non riuscita" not in t, t
        step("an accepted clipboard says «Copiato»", accepted)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
