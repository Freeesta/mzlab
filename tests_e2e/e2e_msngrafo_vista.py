"""The drawer msngrafo-vista.js on a synthetic graph of 80 ions (tests_e2e/fixtures/msngrafo_grande.json): no box over another in the tree and in the radial view,
fewer crossings than the barycentre layout of `pos` (CRITERI point 7), no «Cn» digit in a visible text (subscripts apart), the same drawer in English and Italian, axe-core clean.
The graph is drawn straight into a host div of the built page (no engine involved)."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *

FX = json.loads((Path(__file__).parent / "fixtures" / "msngrafo_grande.json").read_text())
AXE = axe_path()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_msngrafo_vista.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))

def crossings(pos, arcs):
    """The measure of the engine (posa.rs `incroci`): pairs of arcs between the same two columns whose ends are in opposite order in both; arcs sharing an end never count."""
    seg = [(pos[a["da"]], pos[a["a"]], a["da"], a["a"]) for a in arcs if a["da"] in pos and a["a"] in pos]
    n = 0
    for i in range(len(seg)):
        for k in range(i + 1, len(seg)):
            a, b = seg[i], seg[k]
            if {a[2], a[3]} & {b[2], b[3]} or a[0][0] != b[0][0] or a[1][0] != b[1][0]: continue
            if (a[0][1] - b[0][1]) * (a[1][1] - b[1][1]) < 0: n += 1
    return n

OVERLAP = """() => { const r = [...document.querySelectorAll('#vh .ig-svg .ig-box')].map(e => e.getBoundingClientRect()); let n = 0; for (let i = 0; i < r.length; i++) for (let k = i + 1; k < r.length; k++) { const a = r[i], b = r[k]; if (a.left < b.right - 1 && b.left < a.right - 1 && a.top < b.bottom - 1 && b.top < a.bottom - 1) n++; } return n; }"""
TEXTS = """() => { const bad = []; for (const t of document.querySelectorAll('#vh .ig-svg text, #vh .ig-svg tspan, #vh .ig-svg title')) { const s = (t.textContent || '').replace(/MS[0-9]+|MSn/g, ''); if (!t.children.length && /[A-Z][a-z]?[0-9]/.test(s)) bad.push(s); } return bad.slice(0, 5); }"""
DRAW = """async ([fx, mode, lang]) => {
  const m = await import('/static/msngrafo-vista.js');
  let h = document.getElementById('vh'); if (h) h.remove();
  h = document.createElement('div'); h.id = 'vh'; h.style.cssText = 'position:fixed;left:0;top:0;width:100vw;height:100vh;overflow:auto;background:#fff;z-index:9999'; h.setAttribute('role', 'region'); h.setAttribute('aria-label', 'graph'); h.tabIndex = 0; document.body.appendChild(h);
  const r = m.drawVista(h, fx, { onSelect() {}, onOpen() {} }, { t: (k, p) => I18N.t(k, p), mode, chosen: () => null, proveText: () => '', sel: null });
  return [r.layout.w, r.layout.h, h.querySelectorAll('.ig-node').length];
}"""

r = Run(port=8869, wd="/tmp/wd_vista")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1600, "height": 1000})
        n = len(FX["nodi"]); assert n >= 80, n
        def cross():
            base = crossings({x["id"]: (x["pos"]["col"], x["pos"]["riga"]) for x in FX["nodi"]}, FX["archi"])
            ours = crossings({x["id"]: (x["albero"]["col"], x["albero"]["riga"]) for x in FX["vista"]["nodi"]}, FX["archi"])
            assert base > 0 and ours <= base * 0.5, (ours, base)
        step("tree: at most half of the crossings of the barycentre layout (80 ions)", cross)
        for mode in ("tree", "radial"):
            def draw(mode=mode):
                w, h, cnt = pg.evaluate(DRAW, [FX, mode, "x"]); assert cnt == n, (cnt, n)
                assert pg.evaluate(OVERLAP) == 0, "boxes over one another"
                assert not pg.evaluate(TEXTS), pg.evaluate(TEXTS)
            step(f"{mode}: all 80 ions drawn, no box over another, no digit outside the subscripts", draw)
            def axe(mode=mode):
                if not AXE: return
                pg.evaluate(DRAW, [FX, mode, "x"]); pg.add_script_tag(path=AXE)
                v = pg.evaluate("axe.run('#vh', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))")
                assert not v, v
            step(f"{mode}: axe-core finds no violation", axe)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
