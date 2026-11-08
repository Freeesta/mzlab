"""File list rows: checkbox, colour swatch, name, time and × share one vertical centre (within 1 px) in every row type (single, grouped, user group
header, plain and hovered). Runs in the browser chosen by MZLAB_BROWSER (chromium, firefox, webkit). Synthetic files only."""
import sys, os, importlib.util; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
_s = importlib.util.spec_from_file_location("misura_lista_molti", str(Path(__file__).with_name("misura_lista_molti.py"))); _m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
# vertical centre of every part of every row of the given selector; the name is the text line box, not the (taller) flex item
CENTRES = """sel => [...document.querySelectorAll(sel)].map(r => {
  const c = e => { if (!e) return null; const b = e.getBoundingClientRect(); return b.top + b.height / 2; };
  const nm = r.querySelector('.nm,.fgh > span:not(.chv)'), t = r.querySelector('small'), sw = r.querySelector(':scope > i'), x = r.querySelector('.fl-x');
  const tx = nm && (() => { const g = document.createRange(); g.selectNodeContents(nm); const b = g.getBoundingClientRect(); return b.top + b.height / 2; })();
  return { row: (nm && nm.textContent || '').slice(0, 20), h: r.getBoundingClientRect().height, box: c(r.querySelector(':scope > input[type=checkbox]')), sw: c(sw), nm: tx, tm: t && t.textContent.trim() ? c(t) : null, x: x && getComputedStyle(x).opacity > 0 ? c(x) : null };
})"""
def check(pg, sel, what):
    rows = pg.evaluate(CENTRES, sel); assert rows, "no rows for " + what
    for r in rows:
        vs = [v for k, v in r.items() if k not in ("row", "h") and v is not None]
        assert len(vs) >= 2 and max(vs) - min(vs) <= 1, what + ": " + str(r)
        assert r["h"] <= 31, "row too tall: " + str(r)
r = Run(port=8904, wd="/tmp/wd_lista_allineo_e2e")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", _m.small_files(Path("/tmp/wd_lista_allineo_e2e/dati"), 6)); loaded(pg); ready(pg)
        step("grouped file rows (box, swatch, name, time)", lambda: check(pg, "#flst .fl[data-k]:not(.pr)", "file row"))
        step("automatic group headers", lambda: check(pg, "#flst .fgh", "group header"))
        def hover():
            pg.hover("#flst .fl[data-k]:not(.pr) >> nth=1"); check(pg, "#flst .fl[data-k]:not(.pr):hover", "hovered row with ×")
        step("hovered row with the × button", hover)
        def user():
            pg.evaluate("carica('gruppi.js').then(() => GRUPPI.set(E.files.filter(f => f && !f.gone).slice(0, 2).map(f => fileBase(f.file)), 'Group 1', 'new'))"); ready(pg)      # two loaded files: ugBases() holds only the shown ones (#2716)
            check(pg, "#flst .fgh[data-ug]", "user group header"); check(pg, "#flst .fl[data-k]:not(.pr)", "row under a user group")
        step("user group header and its rows", user)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
