// @ts-strict
"use strict";
// The Kendrick plot of a large feature table (kmd-vista.js draws it): above CLOUD ions one focusable element per point froze the panel, so the points become
// one path per series, the click and the hover find the nearest ion (the same 12-unit target), and the keyboard goes through the tables. Also the frame of
// the plot (scales and positions in viewBox units), computed once per result.
const KMDCLOUD = (() => {
  const W = 320, H = 250, L = 52, R = 10, T = 10, B = 34;
  const SHAPES = ["circle", "square", "triangle", "diamond"];
  const CLOUD = 2000;
  const isCloud = res => res.rows.length > CLOUD;
  const geoms = new WeakMap();
  function geom(res) {
    let g = geoms.get(res);
    if (g) return g;
    const xs = res.rows.map(r => r.km), ys = res.rows.map(r => r.kmd);
    let x0 = minOf(xs), x1 = maxOf(xs), y0 = minOf(ys), y1 = maxOf(ys);
    const px = (x1 - x0 || 1) * 0.06, py = (y1 - y0 || 0.01) * 0.12;
    x0 -= px; x1 += px; y0 -= py; y1 += py;
    const X = v => L + (v - x0) / (x1 - x0) * (W - L - R), Y = v => T + (1 - (v - y0) / (y1 - y0)) * (H - T - B);
    g = { x0, x1, y0, y1, px: Float64Array.from(xs, X), py: Float64Array.from(ys, Y), paths: null };
    geoms.set(res, g);
    return g;
  }
  // the outline of one mark (the shapes of kmd-vista.js) as a subpath
  function sub(shape, x, y, r) {
    const f = v => v.toFixed(1);
    if (shape === "square") return `M${f(x - r)} ${f(y - r)}h${2 * r}v${2 * r}h${-2 * r}z`;
    if (shape === "triangle") return `M${f(x)} ${f(y - r * 1.2)}L${f(x + r * 1.1)} ${f(y + r * 0.9)}L${f(x - r * 1.1)} ${f(y + r * 0.9)}z`;
    if (shape === "diamond") return `M${f(x)} ${f(y - r * 1.3)}L${f(x + r * 1.3)} ${f(y)}L${f(x)} ${f(y + r * 1.3)}L${f(x - r * 1.3)} ${f(y)}z`;
    return `M${f(x - r)} ${f(y)}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
  }
  // { series (null = no series), d } per series; points on the same tenth of a unit draw the same pixels and are written once
  function paths(res) {
    const g = geom(res);
    if (g.paths) return g.paths;
    const by = new Map();
    res.rows.forEach((r, k) => {
      const key = r.series || 0, shape = key ? SHAPES[(key - 1) % SHAPES.length] : "circle";
      let c = by.get(key); if (!c) by.set(key, c = { seen: new Set(), d: [] });
      const at = g.px[k].toFixed(1) + " " + g.py[k].toFixed(1);
      if (!c.seen.has(at)) { c.seen.add(at); c.d.push(sub(shape, g.px[k], g.py[k], 3.8)); }
    });
    g.paths = [...by.keys()].sort((a, b) => a - b).map(key => ({ series: key || null, d: by.get(key).d.join("") }));
    return g.paths;
  }
  // the ion nearest to a point of the plot (viewBox units) within the 12-unit target, or null
  function nearest(res, vx, vy) {
    const g = geom(res);
    let best = -1, bd = 144;
    for (let k = 0; k < g.px.length; k++) { const dx = g.px[k] - vx, dy = g.py[k] - vy, d = dx * dx + dy * dy; if (d < bd) { bd = d; best = k; } }
    return best < 0 ? null : { row: res.rows[best], x: g.px[best], y: g.py[best] };
  }
  // the ion under the pointer: client coordinates to viewBox units, then the nearest point
  function hit(res, e) {
    const svg = e.target && e.target.closest && e.target.closest("#kmd-plot[data-cloud]"), m = svg && svg.getScreenCTM();
    if (!m || !res) return null;
    const pt = new DOMPoint(e.clientX, e.clientY).matrixTransform(m.inverse());
    return nearest(res, pt.x, pt.y);
  }
  // the result in #kmd-res: summary, plot, the rest (tables). A new selection on a dense plot of the same result keeps the plot in place, only the dimming
  // and the selected ion change: taking it out of the page, or parsing its paths again, costs more than the whole rest of the panel. The summary, a live
  // region that depends only on the result, is then left alone (not announced again)
  function paint(el, res, selSer, selRow, sum, rest) {
    const svg = /** @type {any} */ (el.querySelector("#kmd-plot[data-cloud]")), restEl = el.querySelector(".kmd-rest");
    if (svg && svg._kmdRes === res && restEl) {
      svg.querySelectorAll("path.kmd-cloud").forEach(p => { const n = +(p.getAttribute("data-s") || 0) || null; p.setAttribute("opacity", selSer != null && n !== selSer ? "0.3" : "1"); });
      svg.querySelectorAll("g[data-i]").forEach(g => g.remove());
      const g = geom(res), k = selRow == null ? -1 : res.rows.findIndex(r => r.i === selRow);
      if (k >= 0) svg.insertAdjacentHTML("beforeend", KMDVIEW.pointG(res.rows[k], g.px[k], g.py[k], true, selSer != null && res.rows[k].series !== selSer));
      restEl.innerHTML = rest;
      return;
    }
    el.innerHTML = `${sum}${KMDVIEW.plotSvg(res, selSer, selRow)}<div class="kmd-rest">${rest}</div>`;
    const nsvg = /** @type {any} */ (el.querySelector("#kmd-plot[data-cloud]")); if (nsvg) nsvg._kmdRes = res;
  }
  // a pointer click takes the nearest ion, also inside the target of the selected one (Enter on that one, detail 0, keeps it); the hover target (#kmd-hov, a transparent circle with its tooltip) follows the nearest ion, at most once a frame
  function bind(box, getRes, pick) {
    if (box._kmdCloud) return;
    box._kmdCloud = true;
    box.addEventListener("click", e => { const near = e.detail > 0 && hit(getRes(), e); if (near) { e.stopPropagation(); pick(near.row.i); } }, true);
    let ev = null, queued = false;
    box.onmousemove = e => {
      const res = getRes();
      if (!res || !isCloud(res)) return;
      ev = e; if (queued) return; queued = true;
      FRAME.next(() => {
        queued = false;
        const hov = box.querySelector("#kmd-hov"), res = getRes(); if (!hov || !res) return;
        const near = hit(res, ev);
        if (!near) { hov.setAttribute("visibility", "hidden"); return; }
        hov.setAttribute("transform", `translate(${near.x.toFixed(1)} ${near.y.toFixed(1)})`); hov.setAttribute("visibility", "visible");
        const tip = KMDVIEW.tipOf(near.row), tt = hov.querySelector("title"); if (tt && tt.textContent !== tip) tt.textContent = tip;
      });
    };
  }
  return { W, H, L, R, T, B, SHAPES, CLOUD, isCloud, geom, paths, nearest, hit, paint, bind, at: (res, k) => [geom(res).px[k], geom(res).py[k]] };
})();
window.KMDCLOUD = KMDCLOUD;
