"""Periodic table: hovering any of the 118 elements never moves or resizes the grid, the info box or the text beside it
(constant size of the detail area). Light layout at narrow and wide widths, Italian and English."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
# boxes of the info area, the grid, the gap text and the first/last cell, rounded to 0.1 px
BOXES = """()=>{const q=s=>{const e=document.querySelector('#refbody '+s);if(!e)return null;const r=e.getBoundingClientRect();return [r.x,r.y,r.width,r.height].map(v=>Math.round(v*10)/10)};
 return {pt:q('.pt'),info:q('.pt-info'),gap:q('.pt-gap'),first:q('.pt-c'),last:q('.pt-c:last-of-type'),body:q(''),inner:document.querySelector('#refbody .pt-info').clientWidth}}"""
r = Run(port=8981, wd="/tmp/wd_tavola_fissa")
try:
    with sync_playwright() as p:
        pg = r.page(p); ready(pg)
        def run(lang, w, h):
            pg.set_viewport_size({"width": w, "height": h})
            pg.evaluate(f"document.querySelector('#refdlg') && document.querySelector('#refdlg').close()")
            if pg.evaluate("I18N.lang") != lang: pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); ready(pg)
            assert pg.evaluate("I18N.lang") == lang, lang
            pg.evaluate("document.querySelector('#np-pt').click()"); attendi(pg, "document.querySelectorAll('#refbody .pt-c').length >= 118")
            syms = pg.evaluate("[...document.querySelectorAll('#refbody .pt-c')].map(c=>c.dataset.s)")
            assert len(syms) >= 118, len(syms)
            base = pg.evaluate(BOXES); bad = []
            for s in syms:
                pg.locator(f'#refbody .pt-c[data-s="{s}"]').hover(); b = pg.evaluate(BOXES)
                if b != base: bad.append((s, {k: (base[k], b[k]) for k in b if b[k] != base[k]}))
            assert not bad, (lang, w, bad[:3], len(bad))
        for lang in ("it", "en"):
            for w, h in ((700, 800), (1024, 768), (1440, 900)):
                step(f"{lang} {w}px: 118 elements keep the layout", lambda lang=lang, w=w, h=h: run(lang, w, h))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any("FAIL" in s for _, s in steps) else 0)
