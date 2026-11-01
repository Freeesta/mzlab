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
        pg.click("text=Carica dati"); ready(pg)
        def opaque():
            for b in ("#np-chrom", "#np-map"):
                if pg.locator(b).count(): pg.click(b); ready(pg)
            pg.evaluate("setVisOf(E.files, true); window.scrollTo(0, 400)"); frames(pg, 3)
            for sel in ("header", "#tools", "#stbar"):
                a = pg.evaluate("s => { const e = document.querySelector(s); if (!e) return 1; const m = getComputedStyle(e).backgroundColor.match(/[\\d.]+/g); return m.length > 3 ? +m[3] : 1 }", sel)
                assert a == 1 or sel != "header", (sel, a)
            assert pg.evaluate("parseInt(getComputedStyle(document.querySelector('header')).zIndex) >= 6000")
            bad = pg.evaluate("""() => [...document.querySelectorAll('header button, header select')].filter(b => b.offsetParent).map(b => { const r = b.getBoundingClientRect(); const e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return e && (e === b || b.contains(e)) ? null : (b.id || b.className || b.textContent).toString().slice(0, 30) + ' <- ' + (e && (e.id || e.className || e.tagName)) }).filter(Boolean)""")
            assert not bad, bad
        step("header opaque and above scrolled panels", opaque)
        def inside():
            for w in (1500, 1200, 1000, 800, 700, 400):
                pg.set_viewport_size({"width": w, "height": 900}); frames(pg, 3)
                r = pg.evaluate("""() => { const h = document.querySelector('header').getBoundingClientRect(); return [...document.querySelectorAll('header button, header select, header #stbar')].filter(b => b.offsetParent).map(b => { const r = b.getBoundingClientRect(); return r.bottom > h.bottom + 1 || r.top < h.top - 1 ? (b.id || b.className || b.textContent).toString().slice(0, 24) + ' ' + Math.round(r.top) + '-' + Math.round(r.bottom) + ' vs ' + Math.round(h.top) + '-' + Math.round(h.bottom) : null }).filter(Boolean) }""")
                assert not r, (w, r)
        step("every header control lies inside the header box (its opaque background covers them) at all widths", inside)
        def paints():
            bad = pg.evaluate("""() => [...document.querySelectorAll('header button, header select')].filter(b => b.offsetParent).map(b => { for (let e = b.parentElement; e && e !== document.documentElement; e = e.parentElement) { const m = getComputedStyle(e).backgroundColor.match(/[\\d.]+/g); if (m && (m.length < 4 || +m[3] > 0)) return m.length < 4 || +m[3] === 1 ? null : (b.id || b.className).toString().slice(0, 24) + ' <- ' + e.tagName + ' alpha ' + m[3]; } return (b.id || b.className).toString().slice(0, 24) + ' has no painted ancestor' }).filter(Boolean)""")
            assert not bad, bad
        step("the first painted ancestor behind every header control is fully opaque", paints)
finally:
    r.close()
print("\nSTEPS"); [print(" ", s) for s in steps]
print([e for e in r.errs if "ERR_ABORTED" not in str(e)])
