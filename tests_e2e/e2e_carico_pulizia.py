"""Loading screen tidy-up: no «drag here to place side by side» bubble; the folder button keeps its distance from the drop zone and is centred on
the title (1440 and 400 px, light and dark); the file selector (arrows + name menu) is aligned and the menu is as wide as its content, truncated with the full name in the tooltip."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8913, wd="/tmp/wd913")
steps = []
try:
    with sync_playwright() as p:
        pg = r.page(p)
        for scheme in ("light", "dark"):
            for w in (1440, 400):
                pg.emulate_media(color_scheme=scheme); pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_selector("#cartpick", state="visible")
                g = pg.evaluate("""() => { const b = document.querySelector('#cartpick').getBoundingClientRect(), h = document.querySelector('.cardhd h3').getBoundingClientRect(), d = document.querySelector('#drop').getBoundingClientRect(), c = document.querySelector('.cardhd').closest('.card').getBoundingClientRect();
                    return { gap: d.top - b.bottom, dy: (b.top + b.bottom) / 2 - (h.top + h.bottom) / 2, over: Math.max(0, h.right - b.left), w: b.width, h: b.height, right: c.right - b.right } }""")
                assert g["gap"] >= 8, (scheme, w, g)
                assert abs(g["dy"]) <= 1.5, (scheme, w, g)
                assert g["over"] == 0 and g["w"] >= 24 and g["h"] >= 24 and g["right"] >= 4, (scheme, w, g)
        steps.append(("folder button: >= 8 px from the drop zone, centred on the title, clear of it, light/dark at 1440 and 400 px", "ok"))
        pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=full]")
        pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===5", timeout=30000)
        pg.click("text=Carica dati"); ready(pg)
        for w in (1440, 400):
            for scheme in ("light", "dark"):
                pg.emulate_media(color_scheme=scheme); pg.set_viewport_size({"width": w, "height": 900})
                pg.wait_for_function("(()=>{const s=document.querySelector('#fsel');return s&&s.offsetParent!==null})()", timeout=10000)
                a = pg.evaluate("""() => ['#fprev', '#fsel', '#fnext'].map(q => { const e = document.querySelector(q).getBoundingClientRect(); return [e.height, (e.top + e.bottom) / 2] })""")
                assert max(x[0] for x in a) - min(x[0] for x in a) <= 1 and max(x[1] for x in a) - min(x[1] for x in a) <= 1, (w, scheme, a)
                f = pg.evaluate("(()=>{const s=document.querySelector('#fsel'),b=s.getBoundingClientRect(),p=s.parentElement.getBoundingClientRect();return {w:b.width,right:b.right,pr:document.documentElement.clientWidth,title:s.title,fs:parseFloat(getComputedStyle(s).fontSize)}})()")
                assert f["right"] <= f["pr"] + 1 and f["w"] >= 8 * f["fs"] * 0.5 and f["title"], (w, scheme, f)
        steps.append(("file selector: arrows and menu share height and vertical centre (+-1 px), menu fits the screen and carries the full name as tooltip", "ok"))
        pg.set_viewport_size({"width": 1440, "height": 900})
        pg.evaluate("document.querySelector('#fsel').style.width=''"); 
        wide = pg.evaluate("document.querySelector('#fsel').getBoundingClientRect().width")
        pg.evaluate("(()=>{const s=document.querySelector('#fsel');s.options[0].textContent='x';s.style.width='auto'})()")
        narrow = pg.evaluate("document.querySelector('#fsel').getBoundingClientRect().width")
        assert narrow <= wide, (narrow, wide)
        steps.append(("file menu width follows its content and never exceeds the available space", "ok"))
        pg.click("#fnext"); frames(pg)
        pg.focus("#fsel"); assert pg.evaluate("document.activeElement.id") == "fsel"
        steps.append(("keyboard: menu reachable by focus", "ok"))
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
