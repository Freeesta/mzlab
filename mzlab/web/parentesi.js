// @ts-check
// Brackets of a partial structure (a piece with a change whose position is unknown, «+O -H2»): square brackets around a set of points, the delta at the upper
// right (the lower right is where the editors write formula and mass), and optional dashed bonds (an attachment to «any atom of the ring»). Pure SVG drawing,
// no editor behind it: the Disegno tab draws it over Ketcher (disegno-parziali.js) and mzFinder can draw the same on its own structures. Coordinates are in the
// units of the SVG that receives the group.

/** @typedef {{ x: number, y: number }} Pt */
/** @typedef {{ x1: number, y1: number, x2: number, y2: number }} Seg */
/** @typedef {{ text?: string, dashed?: Seg[], color?: string, size?: number, pad?: number }} BracketOpts */

const NS = "http://www.w3.org/2000/svg";

/** the box of the brackets: the points plus `pad` on every side */
export function bracketBox(/** @type {Pt[]} */ pts, pad = 16) {
  const xs = pts.map(p => p.x), ys = pts.map(p => p.y);
  return { x0: Math.min(...xs) - pad, y0: Math.min(...ys) - pad, x1: Math.max(...xs) + pad, y1: Math.max(...ys) + pad };
}

/**
 * Appends to `g` the brackets around `pts`, the text and the dashed bonds; returns the group made (class «mz-parentesi»). Nothing for an empty `pts`.
 * @returns {SVGGElement | null}
 */
export function drawBrackets(/** @type {Element} */ g, /** @type {Pt[]} */ pts, /** @type {BracketOpts} */ o = {}) {
  if (!pts.length) return null;
  const doc = g.ownerDocument, color = o.color || "#3b3b3b", size = o.size || 13, b = bracketBox(pts, o.pad ?? size * 1.2), arm = Math.max(5, size * 0.45);
  const el = (/** @type {string} */ tag, /** @type {Record<string, string | number>} */ at) => {
    const e = doc.createElementNS(NS, tag); for (const [k, v] of Object.entries(at)) e.setAttribute(k, String(v)); return e;
  };
  const grp = /** @type {SVGGElement} */ (el("g", { class: "mz-parentesi", fill: "none", stroke: color, "stroke-width": 1.4 }));
  const f = (/** @type {number} */ v) => v.toFixed(1);
  grp.appendChild(el("path", { d: `M${f(b.x0 + arm)},${f(b.y0)}H${f(b.x0)}V${f(b.y1)}H${f(b.x0 + arm)}M${f(b.x1 - arm)},${f(b.y0)}H${f(b.x1)}V${f(b.y1)}H${f(b.x1 - arm)}` }));
  for (const s of o.dashed || []) grp.appendChild(el("line", { x1: f(s.x1), y1: f(s.y1), x2: f(s.x2), y2: f(s.y2), "stroke-dasharray": "4 3" }));
  if (o.text) {
    const t = el("text", { x: f(b.x1 + 3), y: f(b.y0 + size * 0.8), fill: color, stroke: "none", "font-size": size, "font-family": "Arial, sans-serif" });
    t.textContent = o.text; grp.appendChild(t);
  }
  g.appendChild(grp);
  return grp;
}

/**
 * A multicentre attachment (the substituent on «any of these atoms», V3000 ENDPTS): a dashed bond from the substituent atom `sub` to the centre of the
 * candidate atoms `ends`, and a small open circle on each candidate. Returns the group made (class «mz-multicentro»); nothing with fewer than two candidates.
 * @returns {SVGGElement | null}
 */
export function drawMulticenter(/** @type {Element} */ g, /** @type {Pt} */ sub, /** @type {Pt[]} */ ends, /** @type {{ color?: string, r?: number }} */ o = {}) {
  if (ends.length < 2) return null;
  const doc = g.ownerDocument, color = o.color || "#3b3b3b", r = o.r || 4, f = (/** @type {number} */ v) => v.toFixed(1);
  const cx = ends.reduce((s, p) => s + p.x, 0) / ends.length, cy = ends.reduce((s, p) => s + p.y, 0) / ends.length;
  const grp = /** @type {SVGGElement} */ (doc.createElementNS(NS, "g"));
  grp.setAttribute("class", "mz-multicentro"); grp.setAttribute("fill", "none"); grp.setAttribute("stroke", color); grp.setAttribute("stroke-width", "1.4");
  const add = (/** @type {string} */ tag, /** @type {Record<string, string>} */ at) => { const e = doc.createElementNS(NS, tag); for (const [k, v] of Object.entries(at)) e.setAttribute(k, v); grp.appendChild(e); };
  add("line", { x1: f(sub.x), y1: f(sub.y), x2: f(cx), y2: f(cy), "stroke-dasharray": "4 3" });
  for (const p of ends) add("circle", { cx: f(p.x), cy: f(p.y), r: String(r) });
  g.appendChild(grp);
  return grp;
}
