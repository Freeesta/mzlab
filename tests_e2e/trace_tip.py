"""Shared checks of the colour key (e2e_trace_key.py, e2e_trace_key_panels.py): the highlighted row of the chromatogram hover tooltip, and the contrast and focus of the picker checkbox.
Everything is read as the browser computes it; the drawing is observed through the canvas stroke calls, so «the trace is exactly 1 px thicker and only that trace» is measured, not assumed."""
import json
from lib import frames, wait_for_js

# the page helpers: a colour as the browser computes it (rgb, composited over a background), the WCAG contrast of two of them
PAGE = """(() => {
  if (window.__tk) return;
  const cv = document.createElement('canvas'); cv.width = cv.height = 1; const cx = cv.getContext('2d', { willReadFrequently: true });
  const rgb = (c, under) => { cx.clearRect(0, 0, 1, 1); cx.fillStyle = under || '#ffffff'; cx.fillRect(0, 0, 1, 1); cx.fillStyle = c; cx.fillRect(0, 0, 1, 1); const d = cx.getImageData(0, 0, 1, 1).data; return [d[0], d[1], d[2]]; };
  const lum = a => { const f = v => { v /= 255; return v <= .03928 ? v / 12.92 : Math.pow((v + .055) / 1.055, 2.4); }; return .2126 * f(a[0]) + .7152 * f(a[1]) + .0722 * f(a[2]); };
  const ratio = (a, b) => { const x = lum(a), y = lum(b); return (Math.max(x, y) + .05) / (Math.min(x, y) + .05); };
  const norm = c => { const e = document.createElement('i'); e.style.color = c; document.body.appendChild(e); const v = getComputedStyle(e).color; e.remove(); return v; };
  const panel = () => getComputedStyle(document.documentElement).getPropertyValue('--panel').trim() || '#ffffff';
  window.__tk = { rgb, lum, ratio, norm, panel };
  window.__strokes = null;
  const st = CanvasRenderingContext2D.prototype.stroke;
  CanvasRenderingContext2D.prototype.stroke = function (...a) { if (window.__strokes) window.__strokes.push({ w: this.lineWidth, c: String(this.strokeStyle), d: this.getLineDash().join(','), a: this.globalAlpha }); return st.apply(this, a); };
})()"""


def need(c, m):
    c or (_ for _ in ()).throw(AssertionError(m))


def _record(pg, P, act):
    """the stroke calls of one redraw of the panel (axes and grid included, in a fixed order)"""
    pg.evaluate("window.__strokes = []"); act()
    frames(pg, 4)
    return pg.evaluate("(() => { const s = window.__strokes; window.__strokes = null; return s; })()")


def top_row(pg, P, label):
    """hover the middle of the chart: exactly the first tooltip row carries the highlight, it is the series with the tallest value, its tint is that series' colour with readable text,
    and while the pointer is there ONLY that series is stroked 1 px thicker (every other stroke is unchanged); leaving restores the drawing"""
    pg.evaluate(PAGE)
    pg.mouse.move(5, 5); wait_for_js(pg, "%s._top == null" % P, timeout=10000); frames(pg, 3)
    rest = _record(pg, P, lambda: pg.evaluate("draw(%s)" % P))
    need(len(rest) > 2, "%s: no stroke recorded at rest" % label)
    pg.evaluate("%s.cv.scrollIntoView({ block: 'center' })" % P); frames(pg, 2)
    pos = pg.evaluate("(() => { const p = %s, r = p.cv.getBoundingClientRect(), a = p._a; const s = a.sr[0], k = Math.max(0, Math.min(s.x.length - 2, s.x.findIndex(v => v >= (a.x0 + a.x1) / 2))); return { x: r.left + a.X(s.x[k] + 0.25 * (s.x[k + 1] - s.x[k])), y: r.top + (a.H - 10) / 2 }; })()" % P)   # a quarter of a step past a sample: no tie between two samples, whatever the grid
    pg.evaluate("(() => { if (window.putHover.__w) return; const o = window.putHover; window.putHover = function (p, h, px, py) { window.__hv = { p, px, py, rt: h.rt, html: h.html }; return o.apply(this, arguments); }; window.putHover.__w = 1; })()")   # the product's own call: the very point and tooltip it showed
    pg.evaluate("window.__hv = null")
    def hover():
        pg.mouse.move(pos["x"], pos["y"])      # one move: the tooltip read below is that of this very point
        wait_for_js(pg, "!!%s.tip && !%s.tip.hidden && !!%s.tip.querySelector('div.top')" % (P, P, P), timeout=10000)
    lit = _record(pg, P, hover)
    need(len(lit) >= len(rest), "%s: the hover did not redraw (%d strokes, %d at rest)" % (label, len(lit), len(rest)))
    lit = lit[-len(rest):]
    diff = [i for i, (a, b) in enumerate(zip(rest, lit)) if a != b]
    need(len(diff) == 1, "%s: exactly one stroke must change while hovering, got %d: %s" % (label, len(diff), [(rest[i], lit[i]) for i in diff][:4]))
    a, b = rest[diff[0]], lit[diff[0]]
    need(abs(b["w"] - a["w"] - 1) < 1e-6 and a["c"] == b["c"] and a["d"] == b["d"], "%s: the thick stroke must be the same line 1 px wider: %s -> %s" % (label, a, b))
    d = pg.evaluate("""(() => { const p = %s, a = p._a, T = window.__tk, top = p.tip.querySelector('div.top'), i = +top.dataset.i, rows = [...p.tip.querySelectorAll('div')].filter(d => d.querySelector('i'));
      const h = window.__hv && window.__hv.p === p ? window.__hv : null; if (!h) return { none: true }; const at = s => s.ys[nearIdx(s.x, h.rt)];
      const hi = +(/data-i="([0-9]+)"/.exec(h.html) || [0, -1])[1], v = a.sr.map(at), tall = Math.max(...v), cs = getComputedStyle(top), under = T.rgb(T.panel()), tint = T.rgb(cs.backgroundColor, T.panel());
      return { rt: h.rt, vals: v, n: a.sr.length, rows: rows.length, tops: p.tip.querySelectorAll('div.top').length, first: rows[0] === top, dom: i, i: hi, vi: v[hi], tall, ties: v.filter(x => x === tall).length, tc: T.norm(top.style.getPropertyValue('--tc')), color: T.norm(a.sr[i].color),
        alpha: cs.backgroundColor, weight: +cs.fontWeight, text: T.ratio(T.rgb(cs.color), tint), stroke: %s, thick: T.rgb(a.sr[i].color).join() === T.rgb(%s).join() }; })()""" % (P, json.dumps(b["c"]), json.dumps(b["c"])))
    need(not d.get("none"), "%s: the product showed no tooltip for this panel" % label)
    need(d["tops"] == 1 and d["first"], "%s: exactly the first row is the highlighted one: %s" % (label, d))
    need(d["dom"] == d["i"], "%s: the tooltip on screen (series %d) is not the one of this point (series %d)" % (label, d["dom"], d["i"]))
    need(d["vi"] == d["tall"], "%s: the highlighted row (series %d, value %s) is not the tallest (%s): rt %s, values %s" % (label, d["i"], d["vi"], d["tall"], d["rt"], d["vals"]))
    need(d["tc"] == d["color"], "%s: the tint colour %s is not the colour of series %d (%s)" % (label, d["tc"], d["i"], d["color"]))
    need(d["alpha"] not in ("rgba(0, 0, 0, 0)", "transparent") and d["weight"] >= 600, "%s: the highlighted row has no tint or weight: %s" % (label, d))
    need(d["text"] >= 4.5, "%s: the text on the tint has contrast %.2f" % (label, d["text"]))
    need(d["thick"], "%s: the thick stroke is %s, the colour of series %d is %s" % (label, d["stroke"], d["i"], d["color"]))
    pg.mouse.move(5, 5); wait_for_js(pg, "%s._top == null" % P, timeout=10000)
    back = _record(pg, P, lambda: pg.evaluate("draw(%s)" % P))
    need(back == rest, "%s: the drawing did not come back after the mouse left" % label)
    return d


# fixtures: a picker row of every palette colour, ticked and not, with and without a dashed trace, inside a real .can-pop so the page CSS applies
FIXTURE = """(() => {
  document.getElementById('tk-fix')?.remove();
  const box = document.createElement('div'); box.id = 'tk-fix'; box.className = 'can-pop'; box.style.cssText = 'position:fixed;left:8px;top:8px;z-index:99;width:300px';
  for (const c of PAL) for (const on of [false, true]) for (const d of ['', '4,3']) box.insertAdjacentHTML('beforeend',
    `<div class="can-row" role="treeitem" aria-selected="${on}" data-fix="1"><span class="can-in"><span class="can-ck"${SIMUI.ckLook({ _look: { k: { c, d: d ? [4, 3] : [] } } }, 'k')}></span><span class="can-et">x</span></span></div>`);
  document.body.appendChild(box);
  const T = window.__tk, out = [];
  for (const row of box.querySelectorAll('.can-row')) {
    const ck = row.querySelector('.can-ck'), on = row.getAttribute('aria-selected') === 'true', cs = getComputedStyle(ck), a = getComputedStyle(ck, '::after'), b = getComputedStyle(ck, '::before');
    const under = T.rgb(T.panel()), rowBg = T.rgb(getComputedStyle(row).backgroundColor, T.panel()), ring = (cs.boxShadow.match(/rgba?\\([^)]*\\)/) || [''])[0];
    out.push({ c: ck.dataset.tc, d: ck.dataset.td, on, border: cs.borderTopStyle, fill: cs.backgroundColor, ringRatio: ring ? T.ratio(T.rgb(ring), rowBg) : 0, tick: on ? T.ratio(T.rgb(a.borderRightColor), T.rgb(cs.backgroundColor, T.panel())) : null,
      ringOn: on && ck.dataset.td ? { style: b.borderTopStyle, content: b.content, ratio: T.ratio(T.rgb(b.borderTopColor), T.rgb(cs.backgroundColor, T.panel())) } : null, off: on ? null : { ratio: T.ratio(T.rgb(cs.borderTopColor), rowBg) } });
  }
  box.remove(); return out;
})()"""


def picker_fixture(pg, label):
    """every palette colour: the box keeps a 3:1 ring against the row, the tick (and the inner ring of a dashed trace) 3:1 against the fill, a dashed trace shows its dash both unticked
    (dashed border) and ticked (inner dashed ring); ticked boxes are filled with the exact colour of the trace"""
    pg.evaluate(PAGE)
    rows = pg.evaluate(FIXTURE)
    need(len(rows) >= 16, "%s: fixture rows %d" % (label, len(rows)))
    for r in rows:
        need(r["ringRatio"] >= 3, "%s: %s ring contrast %.2f < 3: %s" % (label, r["c"], r["ringRatio"], r))
        if r["d"]: need(r["border"] == "dashed", "%s: a dashed trace needs a dashed border: %s" % (label, r))
        else: need(r["border"] == "solid", "%s: a solid trace needs a solid border: %s" % (label, r))
        if r["on"]:
            need(r["tick"] >= 3, "%s: tick on %s has contrast %.2f < 3" % (label, r["c"], r["tick"]))
            if r["d"]: need(r["ringOn"]["style"] == "dashed" and r["ringOn"]["content"] not in ("none", "normal") and r["ringOn"]["ratio"] >= 3, "%s: ticked dashed trace shows no dash cue: %s" % (label, r))
    return len(rows)


def picker_focus(pg, P, label):
    """a row of the open picker reached with the keyboard shows a focus ring of at least 2 px, 3:1 against its background"""
    pg.evaluate(PAGE)
    pg.evaluate("(() => { const r = %s.el.querySelector('.can-pop .can-row[data-group]'); r.focus(); })()" % P); pg.keyboard.press("ArrowDown"); frames(pg, 2)
    d = pg.evaluate("""(() => { const T = window.__tk, e = document.activeElement, cs = getComputedStyle(e), bg = T.rgb(cs.backgroundColor, T.panel());
      return { row: e.classList.contains('can-row'), vis: e.matches(':focus-visible'), style: cs.outlineStyle, w: parseFloat(cs.outlineWidth), ratio: T.ratio(T.rgb(cs.outlineColor), bg) }; })()""")
    need(d["row"] and d["vis"], "%s: the keyboard did not reach a picker row: %s" % (label, d))
    need(d["style"] != "none" and d["w"] >= 2 and d["ratio"] >= 3, "%s: focus ring on the row too weak: %s" % (label, d))
    return d
