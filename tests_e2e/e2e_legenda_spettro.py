# e2e_legenda_spettro: the legend of a mass spectrum panel with several files overlaid looks like the chromatogram one: one row per file, a coloured swatch (the colour of
# the bars drawn for that file) and the name next to it, in the light and in the dark theme
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
r = Run(port=8873, wd="/tmp/wd_legspec")
PROBE = """() => { const p = E.panels.filter(q => q.type === 'spec' && q.all)[0], c = document.createElement('canvas').getContext('2d');
  const rgb = v => { c.fillStyle = '#000'; c.fillStyle = v; return c.fillStyle; };
  const rows = [...p.leg2.children].map(s => { const i = s.querySelector('i'), b = i.getBoundingClientRect(), t = s.getBoundingClientRect();
    return { text: s.textContent.trim(), w: b.width, h: b.height, top: t.top, bg: getComputedStyle(i).backgroundColor, left: b.left, textLeft: t.left }; });
  const shown = E.files.filter(f => f.vis && !f.gone).map(f => ({ label: f.label, color: rgb(f.color) }));
  return { rows, shown, mode: p.mode || null }; }"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t10")])
        frames(pg, 3); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setVisOf(E.files.filter(f => !f.gone), true); addPanel('spec', {tab: E.tab, k: E.cur, level: 1, all: true, r0: 14, r1: 15})")
        attendi(pg, "(() => { const p = E.panels.filter(q => q.type === 'spec' && q.all)[0]; return p && p.leg2 && p.leg2.children.length >= 2; })()", 30000)
        def entries():
            g = pg.evaluate(PROBE)
            assert len(g["rows"]) == 2 and len(g["shown"]) == 2, g
            tops = [x["top"] for x in g["rows"]]; assert tops[1] - tops[0] > 8, ("one row per file", tops)
            for x in g["rows"]:
                assert x["w"] >= 10 and x["h"] >= 2, ("the swatch has no size", x)
                assert x["textLeft"] <= x["left"] + 1 and x["text"], x
            cols = [x["bg"] for x in g["rows"]]; assert len(set(cols)) == 2, ("two different colours", cols)
        step("two files overlaid: two rows, each with a swatch", entries)
        def same_colour():
            g = pg.evaluate(PROBE)
            want = [x["color"] for x in g["shown"]]; got = [x["bg"] for x in g["rows"]]
            hexof = lambda c: c if c.startswith("#") else "#%02x%02x%02x" % tuple(int(v) for v in c[c.index("(") + 1:c.index(")")].split(",")[:3])
            assert [hexof(c) for c in got] == [hexof(c) for c in want], (got, want)
        step("the swatch colours are the colours of the files (the bars)", same_colour)
        def dark():
            pg.evaluate("document.documentElement.setAttribute('data-theme', 'dark')"); frames(pg, 4)
            g = pg.evaluate(PROBE)
            assert len(g["rows"]) == 2 and all(x["w"] >= 10 and x["h"] >= 2 for x in g["rows"]), g
            pg.evaluate("document.documentElement.removeAttribute('data-theme')")
        step("dark theme: same entries and swatches", dark)
        def chrom_alike():
            a = pg.evaluate("() => { const i = document.querySelector('.pnl.spec .leg2 i'), s = getComputedStyle(i); return [s.width, s.height]; }")
            pg.evaluate("addPanel('chrom', {tab: E.tab, k: E.cur, kind: 'tic'})"); attendi(pg, "!!document.querySelector('.pnl.chrom .leg i')", 30000)
            b = pg.evaluate("() => { const i = document.querySelector('.pnl.chrom .leg i'), s = getComputedStyle(i); return [s.width, s.height]; }")
            assert a == b, ("same swatch as the chromatogram", a, b)
        step("the swatch is the one of the chromatogram legend", chrom_alike)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
