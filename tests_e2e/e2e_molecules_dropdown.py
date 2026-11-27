"""The compound search box over the built-in catalogue (molecules-field.js), at 1400 px and at 390 px: typing «dimeth» lists dimethomorph first (a name that starts with
the text comes before one that only contains it), typing «sd» lists names that do not break inside a word, no row is wider than the list, and the page does not scroll
sideways. MZLAB_SHOTS=DIR also writes the screenshots (device scale factor 2). Needs the built site (tools/prepara_sito.py). Run from the repo root."""
# covers: mzlab/web/molecules-field.js, mzlab/web/molecules/catalogue.csv
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

SHOTS = os.environ.get("MZLAB_SHOTS")
HOST = """() => { const h = document.createElement('div'); h.id = 'molc-host'; h.style.cssText = 'position:fixed;left:16px;top:60px;right:16px;max-width:460px;z-index:5000;background:var(--panel)';
  document.body.appendChild(h); window.__box = MOLDB.field(h, { texts: { label: 'Compound', placeholder: 'Name, formula or mass', none: 'No compound found', count: n => n + ' found' }, limit: 8, manage: null, onPick: () => {} }); }"""
# what the list shows: per row the name element, whether it fits on one line, and whether the row is inside the list
ROWS = """() => { const ul = document.querySelector('#molc-host ul').getBoundingClientRect();
  return [...document.querySelectorAll('#molc-host li')].map(li => { const n = li.querySelector('.molc-n'), b = li.getBoundingClientRect();
    return { name: n.textContent, oneLine: n.getBoundingClientRect().height < 2 * parseFloat(getComputedStyle(n).lineHeight || 24) - 1, fits: li.scrollWidth <= li.clientWidth + 1,
      inside: b.left >= ul.left - 1 && b.right <= ul.right + 1, whole: n.scrollWidth <= n.clientWidth + 1 }; }); }"""

r = Run(port=8852, wd="/tmp/wd_molecules_dropdown")
try:
    with sync_playwright() as p:
        for width, height in ((1400, 900), (390, 844)):
            pg = r.page(p, viewport={"width": width, "height": height}, **({"device_scale_factor": 2} if SHOTS else {}))
            pg.wait_for_selector("#drop", timeout=120000)
            pg.evaluate("loadScript('m:molecules-ui.js')")
            pg.wait_for_function("window.MOLDB", timeout=60000)
            pg.evaluate(HOST)
            i = "#molc-host input"
            for text, first in (("dimeth", "dimethomorph"), ("sd", None)):
                pg.fill(i, ""); pg.click(i); pg.keyboard.type(text)
                pg.wait_for_selector("#molc-host [role=option]", timeout=20000)
                rows = pg.evaluate(ROWS)
                assert rows, (width, text)
                assert all(x["oneLine"] and x["fits"] and x["inside"] for x in rows), (width, text, [x for x in rows if not (x["oneLine"] and x["fits"] and x["inside"])])
                if first:
                    assert rows[0]["name"] == first, (width, rows[:3])
                    assert rows[0]["whole"], (width, "the first name is cut", rows[0])
                else:
                    starts = [x["name"] for x in rows if x["name"].lower().startswith("sd")]
                    assert [x["name"] for x in rows[:len(starts)]] == starts, (width, "names that start with the text come first", [x["name"] for x in rows])
                assert not pg.evaluate("document.documentElement.scrollWidth > document.documentElement.clientWidth"), (width, text, "the page scrolls sideways")
                assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
                if SHOTS:
                    os.makedirs(SHOTS, exist_ok=True)
                    pg.screenshot(path=f"{SHOTS}/compound-list-{text}-{width}.png", clip={"x": 0, "y": 0, "width": width, "height": 460})
                print(f"OK  {width} px, «{text}»: {len(rows)} rows, one line each, inside the list")
            r.b.close()
        assert not [e for e in r.errs if e[0] in ("pageerror", "console.error")], r.errs[:3]
finally:
    r.close()
