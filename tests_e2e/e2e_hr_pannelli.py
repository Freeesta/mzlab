"""HR_PANNELLO 1: the panels of the drawer in the high-resolution world are compact: no clipped text at 1280, 1440 and 1680 px, one title (the one of the drawer),
m/z, ppm and search in compact rows, the two filters of the reference lists as small segmented controls."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_pannelli.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
TABS = ("lists", "hdr", "lst", "fil", "cmp", "iso", "idn")
# text clipped by its own box (overflow hidden or ellipsis) or sticking out of the drawer, outside the containers that scroll on purpose
CLIP = """() => { const root = document.querySelector('#dfiles'), R = root.getBoundingClientRect(), out = [];
  root.querySelectorAll('*').forEach(e => {
    if (!e.offsetParent || e.closest('[hidden]') || e.closest('#lst-table-wrap,#hri-list')) return;
    const own = [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim()); if (!own) return;
    const cs = getComputedStyle(e);
    if (e.scrollWidth > e.clientWidth + 1 && (cs.overflowX !== 'visible' || cs.textOverflow === 'ellipsis')) out.push(['clip', e.id || e.className || e.tagName, e.textContent.trim().slice(0, 30)]);
    const b = e.getBoundingClientRect(); if (b.width && b.right > R.right + 1) out.push(['out', e.id || e.className || e.tagName, e.textContent.trim().slice(0, 30), Math.round(b.right - R.right)]);
  }); return out; }"""
D = synth("hrpannelli")
r = Run(port=8992, wd="/tmp/wd_hrpannelli")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 1000})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); ready(pg)
        def clip(w):
            pg.set_viewport_size({"width": w, "height": 1000}); pg.wait_for_timeout(400)
            for t in TABS:
                drawer(pg, t); ready(pg)
                bad = pg.evaluate(CLIP); assert not bad, f"{t} at {w}: {bad[:3]}"
                if t != "lists":
                    shots = os.path.join(os.path.dirname(os.path.abspath(__file__)), "shots"); os.makedirs(shots, exist_ok=True)
                pg.locator("#dfiles").screenshot(path=os.path.join(os.path.dirname(os.path.abspath(__file__)), "shots", f"hr_pannello_{t}_{w}.png"))
        for w in (1280, 1440, 1680): step(f"no clipped text in the seven drawer panels at {w} px", lambda w=w: clip(w))
        def one_title():
            for t in ("fil", "iso", "cmp"):
                drawer(pg, t); ready(pg)
                assert not pg.is_visible("#hri-ttl"), "the title of the panel is the one of the drawer, not repeated"
                assert pg.is_visible("#sb-title")
        step("one title per panel", one_title)
        def lists():
            drawer(pg, "lists"); ready(pg)
            rows = pg.evaluate("[...document.querySelectorAll('#sb-panel-lists .lb-row')].map(r => [r.querySelectorAll('input').length, r.querySelectorAll('.seg').length])")
            assert rows == [[2, 0], [1, 0], [0, 2]], rows
            h = pg.evaluate("document.querySelector('#lst-mode-seg').getBoundingClientRect().height"); assert h < 26, h
            assert pg.inner_text("#lst-mode-seg") .replace("\n", " ").strip() in ("Nominale Alta ris.",), pg.inner_text("#lst-mode-seg")
            rh = pg.evaluate("document.querySelector('#lst-tbody tr').getBoundingClientRect().height"); assert rh < 34, rh
        step("reference lists: m/z and ppm on one row, search on one, two small segmented controls, compact rows", lists)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
