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


# fixtures: a picker row of every palette colour, ticked and not, with and without a dashed trace, inside a real .can-pop so the page CSS applies.
# STATE reads one row as the browser computes it: the box border, the fill, the tick and the background the row has in that state
FIXTURE = """(() => {
  document.getElementById('tk-fix')?.remove();
  const box = document.createElement('div'); box.id = 'tk-fix'; box.className = 'can-pop'; box.style.cssText = 'position:fixed;left:8px;top:8px;z-index:2147483000;width:300px;max-height:90vh;background:var(--panel)';
  for (const c of PAL) for (const on of [false, true]) for (const d of ['', '4,3']) box.insertAdjacentHTML('beforeend',
    `<div class="can-row" role="treeitem" tabindex="-1" aria-selected="${on}" data-fix="1"><span class="can-in"><span class="can-ck"${SIMUI.ckLook({ _look: { k: { c, d: d ? [4, 3] : [] } } }, 'k')}></span><span class="can-et">x</span></span></div>`);
  document.body.appendChild(box); return box.querySelectorAll('.can-row').length;
})()"""
STATE = """(i => {
  const T = window.__tk, row = document.querySelectorAll('#tk-fix .can-row')[i], ck = row.querySelector('.can-ck'), on = row.getAttribute('aria-selected') === 'true', cs = getComputedStyle(ck), a = getComputedStyle(ck, '::after'), b = getComputedStyle(ck, '::before');
  const rowBg = T.rgb(getComputedStyle(row).backgroundColor, T.panel()), panel = T.rgb(T.panel()), border = T.rgb(cs.borderTopColor, T.panel()), key = T.rgb(ck.dataset.tc), ink = T.rgb(getComputedStyle(document.documentElement).getPropertyValue('--ink').trim() || '#000');
  return { c: ck.dataset.tc, d: ck.dataset.td, on, bstyle: cs.borderTopStyle, bw: parseFloat(cs.borderTopWidth), border, key, panel, shadow: cs.boxShadow, fill: T.rgb(cs.backgroundColor, T.panel()), rowBg,
    keyVsPanel: T.ratio(key, panel), borderVsRow: T.ratio(border, rowBg), borderVsPanel: T.ratio(border, panel), tick: on ? T.ratio(T.rgb(a.borderRightColor), T.rgb(cs.backgroundColor, T.panel())) : null,
    ringOn: on && ck.dataset.td ? { style: b.borderTopStyle, content: b.content, ratio: T.ratio(T.rgb(b.borderTopColor), T.rgb(cs.backgroundColor, T.panel())) } : null, ink };
})"""


def _lum(c):
    f = lambda v: v / 255 / 12.92 if v / 255 <= 0.03928 else ((v / 255 + 0.055) / 1.055) ** 2.4
    return 0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])


def _ratio(a, b):
    x, y = _lum(a), _lum(b)
    return (max(x, y) + 0.05) / (min(x, y) + 0.05)


def rgb_of(css):
    """'rgb(1, 2, 3)' as the browser serialises a colour -> [1, 2, 3]"""
    return [int(v) for v in css[css.index("(") + 1:css.index(")")].split(",")[:3]]


def expected_border(key, ink, panel):
    """what the border of a key box must be, worked out here independently of the page: the key colour itself when it holds 3:1 against the popup, else the key moved toward the text colour in
    5% steps (channels rounded half up) to the FIRST step that holds 3:1, so the key colour is kept as far as the contrast allows"""
    for t in range(0, 101, 5):
        m = [int(k + (i - k) * t / 100 + 0.5) for k, i in zip(key, ink)]
        if t == 100 or _ratio(m, panel) >= 3: return m
    return key


def border_rule(label, state, r):
    """the box border is the key colour of the trace in every state: exactly expected_border (the key itself, or, only for a colour under 3:1 against the popup such as a pale yellow, the key moved
    toward the text colour to the first 5% step that holds 3:1); never black or grey (unless the key is black), and no outer ring"""
    where = "%s [%s] %s" % (label, state, r["c"])
    need(r["bw"] >= 2 and r["bstyle"] in ("solid", "dashed"), "%s: the box border must be at least 2 px: %s" % (where, r))
    need(r["shadow"] == "none", "%s: no outer ring is allowed on the key box (it read as a dark border): %s" % (where, r))
    need(r["borderVsPanel"] >= 3, "%s: the border has contrast %.2f against the popup, under 3: %s" % (where, r["borderVsPanel"], r))
    want = expected_border(r["key"], r["ink"], r["panel"])
    need(r["border"] == want, "%s: the border %s is not the expected %s (key %s, text %s, popup %s)" % (where, r["border"], want, r["key"], r["ink"], r["panel"]))
    if r["border"] != r["key"]: need(r["keyVsPanel"] < 3, "%s: the border %s differs from the key %s though the key already holds 3:1: %s" % (where, r["border"], r["key"], r))
    if state in ("on", "off"):
        if r["on"]: need(r["fill"] == r["key"], "%s: the ticked box is filled with the exact key colour: %s" % (where, r))
        else: need(r["fill"] == r["rowBg"], "%s: an unticked box is empty: %s" % (where, r))


def picker_fixture(pg, label):
    """every palette colour, ticked and not, solid and dashed, in each state: unticked, ticked, hover (real mouse), keyboard focus, selected row. The box border is the key colour in all of them
    (border_rule); the tick and the inner ring of a dashed trace keep 3:1 against the fill; a dashed trace shows its dash unticked (dashed border) and ticked (inner dashed ring)"""
    pg.evaluate(PAGE)
    n = pg.evaluate(FIXTURE)
    need(n >= 16, "%s: fixture rows %d" % (label, n))
    pg.mouse.move(700, 5)
    states = {}
    for i in range(n):
        r = pg.evaluate(STATE + "(%d)" % i); border_rule(label, "on" if r["on"] else "off", r); states[i] = r
        if r["d"]: need(r["bstyle"] == "dashed", "%s: a dashed trace needs a dashed border: %s" % (label, r))
        else: need(r["bstyle"] == "solid", "%s: a solid trace needs a solid border: %s" % (label, r))
        if r["on"]:
            need(r["tick"] >= 3, "%s: tick on %s has contrast %.2f < 3" % (label, r["c"], r["tick"]))
            if r["d"]: need(r["ringOn"]["style"] == "dashed" and r["ringOn"]["content"] not in ("none", "normal") and r["ringOn"]["ratio"] >= 3, "%s: ticked dashed trace shows no dash cue: %s" % (label, r))
    pg.keyboard.press("Shift")                       # keyboard modality, so the focus below is a :focus-visible one
    for i in range(n):
        bb = pg.evaluate("(i => { const e = document.querySelectorAll('#tk-fix .can-row')[i]; e.scrollIntoView({ block: 'nearest' }); const b = e.getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })(%d)" % i)
        pg.mouse.move(bb[0], bb[1])                  # the real pointer over the row: :hover applies without any actionability wait
        need(pg.evaluate("document.querySelectorAll('#tk-fix .can-row')[%d].matches(':hover')" % i), "%s: the pointer is not over fixture row %d" % (label, i))
        r = pg.evaluate(STATE + "(%d)" % i); border_rule(label, "hover", r)
        need(r["border"] == states[i]["border"], "%s: hover changed the border of row %d: %s -> %s" % (label, i, states[i]["border"], r["border"]))
        pg.evaluate("document.querySelectorAll('#tk-fix .can-row')[%d].focus()" % i)
        r = pg.evaluate(STATE + "(%d)" % i); border_rule(label, "focus", r)
        need(r["border"] == states[i]["border"], "%s: focus changed the border of row %d: %s -> %s" % (label, i, states[i]["border"], r["border"]))
    pg.mouse.move(700, 5); pg.evaluate("document.getElementById('tk-fix').remove()")
    return n


def picker_focus(pg, P, label):
    """a row of the open picker reached with the keyboard shows a focus ring of at least 2 px, 3:1 against its background"""
    pg.evaluate(PAGE)
    pg.evaluate("(() => { const r = %s.el.querySelector('.can-pop .can-row[data-group]'); r.focus(); })()" % P); pg.keyboard.press("ArrowDown"); frames(pg, 2)
    d = pg.evaluate("""(() => { const T = window.__tk, e = document.activeElement, cs = getComputedStyle(e), bg = T.rgb(cs.backgroundColor, T.panel());
      return { row: e.classList.contains('can-row'), vis: e.matches(':focus-visible'), style: cs.outlineStyle, w: parseFloat(cs.outlineWidth), ratio: T.ratio(T.rgb(cs.outlineColor), bg) }; })()""")
    need(d["row"] and d["vis"], "%s: the keyboard did not reach a picker row: %s" % (label, d))
    need(d["style"] != "none" and d["w"] >= 2 and d["ratio"] >= 3, "%s: focus ring on the row too weak: %s" % (label, d))
    return d
