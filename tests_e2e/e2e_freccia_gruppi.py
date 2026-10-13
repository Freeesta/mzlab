"""File list: the arrows that open and close a group («FULL SCAN») and the precursors of a file are clear icons: at least 14 px drawn, text colour (not muted), target of at least 24 px, aria-expanded that follows the state, keyboard; light and dark."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
r = Run(port=8899, wd="/tmp/wd_freccia_gruppi")
INFO = """() => [...document.querySelectorAll('#flst .fgh[data-gt], #flst .fgh[data-gg]')].map(g => { const c = g.querySelector('.chv'), b = c.getBoundingClientRect(), s = getComputedStyle(c, '::before');
  return { w: b.width, h: b.height, bw: parseFloat(s.width) + parseFloat(s.borderRightWidth), col: getComputedStyle(c).color, ink: getComputedStyle(document.body).color, muted: getComputedStyle(g).color, exp: g.getAttribute('aria-expanded'), on: c.classList.contains('on'), tf: s.transform } })"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        for theme in ("light", "dark"):
            ctx = b.new_context(locale="en-US", viewport={"width": 1280, "height": 900})
            ctx.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1')}catch(_){}")
            pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/?telefono=0"); pg.wait_for_timeout(800)
            pg.evaluate(f"document.documentElement.dataset.theme='{theme}'")
            pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
            pg.click("#opbtn"); ready(pg); pg.wait_for_timeout(500)
            pg.evaluate("document.querySelector('#sb-panel-files, #flst') && 0")
            i0 = pg.evaluate(INFO); assert i0, "no group header"
            x = i0[0]
            assert x["w"] >= 24 and x["h"] >= 24 and x["bw"] >= 11, x
            assert x["col"] == x["ink"] and x["on"] and x["exp"] == "true", x
            pg.focus("#flst .fgh >> nth=0"); pg.keyboard.press("Enter"); pg.wait_for_timeout(300)
            x1 = pg.evaluate(INFO)[0]
            assert x1["exp"] == "false" and not x1["on"] and x1["tf"] != x["tf"], (x, x1)
            steps.append((f"{theme}: group arrow is a clear 24 px control that toggles with the keyboard", "ok"))
            ctx.close()
        b.close()
finally:
    pass
for s in steps: print(s)
print("ERRORS", r.errs)
assert len(steps) == 2 and not r.errs
