# e2e_header_opaco: the top header is fully opaque and above the panels (scrolled content never shows through or catches the clicks of its buttons);
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8884, wd="/tmp/wd_header_opaco")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_MRM-t0")])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(1500)
        def opaque():
            pg.evaluate("setVisOf(E.files, true); window.scrollTo(0, 400)"); pg.wait_for_timeout(400)
            for sel in ("header", "#tools", "#stbar"):
                a = pg.evaluate("s => { const e = document.querySelector(s); if (!e) return 1; const m = getComputedStyle(e).backgroundColor.match(/[\\d.]+/g); return m.length > 3 ? +m[3] : 1 }", sel)
                assert a == 1 or sel != "header", (sel, a)
            assert pg.evaluate("parseInt(getComputedStyle(document.querySelector('header')).zIndex) >= 6000")
            bad = pg.evaluate("""() => [...document.querySelectorAll('header button, header select')].filter(b => b.offsetParent).map(b => { const r = b.getBoundingClientRect(); const e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return e && (e === b || b.contains(e)) ? null : (b.id || b.className || b.textContent).toString().slice(0, 30) + ' <- ' + (e && (e.id || e.className || e.tagName)) }).filter(Boolean)""")
            assert not bad, bad
        step("header opaque and above scrolled panels", opaque)
finally:
    r.close()
print("\nSTEPS"); [print(" ", s) for s in steps]
print([e for e in r.errs if "ERR_ABORTED" not in str(e)])
