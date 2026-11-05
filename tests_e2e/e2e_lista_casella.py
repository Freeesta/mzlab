"""Side list: the tick boxes of a file, of a precursor row and of a group header are one control (same size, border, checked fill and focus ring from the CSS variables), in light and dark."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np, dati_sintetici as ds
tmp = Path(tempfile.mkdtemp(prefix="casella_"))
FILES = []
for i in range(2):
    p = tmp / f"P_{i}.mzML"; ds.ms2(p, np.random.default_rng(i), 15); FILES.append(str(p))
LOOK = """() => { const out = {}; const pick = { file: '#flst .fl>input[data-k]', prec: '#flst .fl.pr>input[data-pr]', group: '#flst .fgh>input.gall' };
  for (const [n, q] of Object.entries(pick)) { const e = document.querySelector(q); if (!e) { out[n] = null; continue; }
    const b = getComputedStyle(e, '::before'), r = e.getBoundingClientRect(); e.focus({ focusVisible: true });
    out[n] = { w: Math.round(r.width), h: Math.round(r.height), app: getComputedStyle(e).appearance, border: b.borderTopColor, bw: b.borderTopWidth, bg: b.backgroundColor, on: [e.checked, getComputedStyle(e, '::before').backgroundColor], ring: getComputedStyle(e).outlineColor + getComputedStyle(e).outlineWidth }; }
  return out; }"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
r = Run(port=8896, wd="/tmp/wd_casella")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1300, "height": 800})
        attendi(pg, "window.qqReady === true", 60000)
        pg.set_input_files("#pick", FILES); open_first(pg); ready(pg)
        pg.locator("#flst .fgh").first.click(position={"x": 60, "y": 12}); ready(pg)
        pg.evaluate("document.querySelectorAll('#flst .pr-container').forEach(c => { c.style.display = ''; const i = c.querySelector('input'); if (i) i.checked = true })")
        def theme(t):
            pg.evaluate("t => { document.documentElement.dataset.theme = t }", t); ready(pg)
        for t in ("light", "dark"):
            def one(t=t):
                theme(t); pg.evaluate("document.querySelectorAll('#flst input[type=checkbox]').forEach(i => { i.checked = false })"); look = pg.evaluate(LOOK)
                for n in ("file", "prec", "group"): assert look[n], f"no {n} box in the list"
                for n, v in look.items():
                    assert v["app"] == "none", f"{n}: browser default appearance"
                    assert v["w"] >= 24 and v["h"] >= 24, f"{n}: target {v['w']}x{v['h']} under 24 px"
                f, pr, gr = look["file"], look["prec"], look["group"]
                for n, v in (("prec", pr), ("group", gr)):
                    for k in ("w", "h", "border", "bw", "bg", "ring"): assert v[k] == f[k], f"{t}: {n} differs from the file box in {k}: {v[k]} / {f[k]}"
                # a ticked box of the precursor row has the fill of the file one
                pg.evaluate("document.querySelectorAll('#flst input[type=checkbox]').forEach(i => { i.checked = true })")
                on = pg.evaluate("[...document.querySelectorAll('#flst .fl>input[data-k], #flst .fl.pr>input[data-pr]')].map(e => getComputedStyle(e, '::before').backgroundColor)")
                assert len(set(on)) == 1, f"{t}: ticked fills differ: {set(on)}"
            step(f"{t}: file, precursor and group boxes look the same", one)
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
