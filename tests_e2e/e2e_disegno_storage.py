"""Disegno with a full or blocked localStorage: Storage.setItem throws QuotaExceededError in every frame, yet the editor starts, draws «CCO»
with the label C2H6O and no error box appears (Ketcher's settings fall back to memory)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
FULL = """(() => { const full = function () { throw new DOMException('The quota has been exceeded.', 'QuotaExceededError'); };
  const real = Storage.prototype.setItem; Storage.prototype.setItem = function (k, v) { if (this === window.localStorage) full(); return real.call(this, k, v); }; })();"""
r = Run(port=8996, wd="/tmp/wd_disegno_storage")
fail = []
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.add_init_script(FULL)
        pg.goto(f"http://127.0.0.1:{r.port}/#/draw"); pg.reload()       # a real navigation: the init script runs in the page and in the Ketcher frame
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        KJ = "document.querySelector('#kframe').contentWindow.ketcher"
        pg.evaluate(f"{KJ}.addFragment('CCO')")
        attendi(pg, f"!{KJ}.editor.struct().isBlank()")
        label = "() => { const d = document.querySelector('#kframe').contentDocument; return [...d.querySelectorAll('svg text')].some(t => t.textContent.replace(/\\s/g, '') === 'C2H6O'); }"
        try: attendi(pg, f"({label})()", timeout=15000)
        except Exception: fail.append("no C2H6O label under the drawing")
        if pg.evaluate("() => !!document.getElementById('errbox') && !document.getElementById('errbox').hidden"): fail.append("an error box is shown: " + pg.inner_text("#errbox")[:200])
finally:
    r.close()
print("FAIL: " + "; ".join(fail) if fail else "OK")
sys.exit(1 if fail else 0)
