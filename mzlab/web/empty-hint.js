// @ts-check
// @ts-strict
// Empty state of a panel with nothing to draw: a centred prompt that names what to choose, and (when its control is on screen) a curved arrow to that
// control. The prompt is a button that clicks the same control; the arrow is decorative (aria-hidden), its start and end are read from the live rectangles of the
// prompt and the control, so a resize, a scroll, a wrapped toolbar or a re-rendered control redraws it. A control inside the panel (spec.target) is reached from
// above; one outside it (spec.out: the file list of the side bar, the «+ XIC» button) from above, left or right, with the arrow in a fixed layer because the panel
// clips its children; off screen it gives way to a plain line (spec.plain). grafici.js drawLines calls sync(p, spec) / clear(p).

/** @typedef {{ key: string, above?: string, target?: string, out?: string | (() => Element | null), plain?: string, act?: (t: Element | null) => void, alt?: string }} Spec  key: i18n key of the prompt; above: the same without the arrow;
 * target: selector (inside the panel) of the control; out: selector (in the page), or a function that returns the element, of a control outside the panel; plain: key of the line when that control is off screen or has no room;
 * act: what the prompt does (t = the control, null when it is off screen), default a click on the control; alt: text when the control is absent */
/** @typedef {{ svg: SVGSVGElement, ln: SVGPathElement, hd: SVGPathElement }} Fixed */
/** @typedef {{ spec: Spec, root: HTMLElement, box: HTMLElement, go: HTMLElement, tx: HTMLElement, ln: SVGPathElement, hd: SVGPathElement, fx: Fixed | null, stops: (() => void)[], mo: MutationObserver, raf: number }} State */

const NARROW = 320;      // a panel narrower than this (px; the CSS minimum is 300, so a 320 px phone) gets the plain «… above» line, no arrow
const GAP = 14;          // least vertical room (px) between the control and the prompt for an arrow
const T = (/** @type {string} */ k) => /** @type {any} */ (window).I18N.t(k);

/** the styles go in once, here, so the module stays a leaf */
function css() {
  if (document.getElementById("eh-css")) return;
  const s = document.createElement("style"); s.id = "eh-css";
  s.textContent = ".pnl>.eh{position:absolute;inset:0;pointer-events:none;z-index:4}.eh-ar{position:absolute;left:0;top:0;width:100%;height:100%;overflow:visible}" +
    ".eh-fx{position:fixed;left:0;top:0;width:100vw;height:100vh;overflow:visible;pointer-events:none;z-index:45}" +
    ".eh-ln,.eh-hd{fill:none;stroke:var(--accent);stroke-width:2;stroke-linecap:round;stroke-linejoin:round}" +
    ".eh-box{position:absolute;display:flex;align-items:center;justify-content:center;box-sizing:border-box;padding:6px 10px;overflow:hidden}" +
    ".eh-go{pointer-events:auto;font:inherit;font-size:15px;font-weight:600;line-height:1.3;color:var(--ink);background:var(--panel);border:1.5px dashed var(--accent);border-radius:10px;padding:8px 16px;min-height:36px;max-width:100%;cursor:pointer;text-align:center}" +
    ".eh-go:hover{background:var(--sel)}.eh-go:focus-visible{outline:2px solid var(--accent);outline-offset:2px}" +
    ".eh-go:disabled,.eh-go.eh-txt{pointer-events:none;cursor:default;border-color:transparent;background:none;color:var(--muted);font-weight:400}" +
    "@media (pointer:coarse){.eh-go{min-height:44px}}" +
    "@media (forced-colors:active){.eh-ln,.eh-hd{stroke:CanvasText}.eh-go{border-color:ButtonText;background:ButtonFace;color:ButtonText}.eh-go:disabled,.eh-go.eh-txt{color:CanvasText;background:none}}";
  document.head.appendChild(s);
}

const FILES = "#flst", XIC_BTN = "#np-xic";
/** the file list of the side bar: the prompt takes the focus to its first unticked file, or opens the side bar when it is shut */
const pickFile = (/** @type {Element | null} */ t) => {
  if (!t) { const b = /** @type {any} */ (window).BARRA; if (b) b.show("file"); return; }
  const c = /** @type {HTMLElement | null} */ (t.querySelector("input[type=checkbox]:not(:checked)")); if (c) c.focus();
};

/** what an empty chromatogram asks for (drawLines): hidden = traces exist but are all hidden (a line on the canvas, the legend is there to click); else files exist but none is ticked (the file list
 * of the side bar), the channel picker of an MRM / SIM panel with no channel, the «+ XIC» button for an XIC panel with no trace, or the words of the panel (say)
 * @param {any} p @param {CanvasRenderingContext2D} g @param {number} hidden @param {string} say @param {number} x */
export function empty(p, g, hidden, say, x) {
  if (hidden) { clear(p); g.fillStyle = ecss("--muted"); g.fillText(T("leg.allHidden"), x, 30); return; }
  const mrm = ESP.mrm(p), files = tabFiles(p.tab || E.tab);
  sync(p, files.length && !files.some((/** @type {any} */ f) => f.vis) ? { key: "empty.file", plain: "empty.file.list", out: FILES, act: pickFile, alt: say }
    : mrm && Array.isArray(p.tr) && !p.tr.length ? { key: "empty.channel", above: "empty.channel.above", target: ".canb[data-o=canali]", alt: say }
    : p.type === "xic" && !(p.traces || []).length ? { key: "empty.xic", plain: "msg.xic", out: XIC_BTN, act: (/** @type {Element | null} */ t) => /** @type {HTMLElement | null} */ (t || document.querySelector(XIC_BTN))?.click(), alt: say } : { key: "msg." + p.type, alt: say });
}

/** the chromatogram panel that holds the choice of a spectrum panel: the one it is linked to, else the first one of its tab (a TIC / BPC, XIC or MRM panel that is drawn)
 * @param {any} p @returns {any} */
function chromOf(p) {
  const ok = (/** @type {any} */ q) => q && q.el && q.el.isConnected && q.cv && !q._exp && isLine(q);
  const l = p.link != null ? E.panels.find((/** @type {any} */ q) => q.id === p.link) : null;
  return ok(l) ? l : tabPanels(p.tab ?? E.tab).find(ok) || null;
}

/** empty state of a spectrum or map panel that has nothing to show; say = its words (already translated), pick = no time range chosen yet: the prompt then points at the chromatogram
 * panel that holds the choice (its live rectangle), or is text only when there is none; any other message is text only
 * @param {any} p @param {string} say @param {boolean} [pick] */
export function note(p, say, pick) {
  const ch = pick ? chromOf(p) : null, cv = () => (chromOf(p) || ch)?.cv || null;
  sync(p, ch ? { key: "empty.spec", plain: "spec.pickRange", out: cv, alt: say, act: () => { const c = chromOf(p); if (c) { c.el.scrollIntoView({ block: "nearest" }); c.cv.focus?.(); } } } : { key: "empty.note", alt: say });
}

/** @param {any} p */
export function clear(p) {
  const s = /** @type {State | undefined} */ (p._eh); if (!s) return;
  s.stops.forEach(f => f()); s.mo.disconnect(); FRAME.cancel(s.raf); s.root.remove(); if (s.fx) s.fx.svg.remove(); p._eh = undefined;
}

/** show (or update) the empty state of panel p; spec = null removes it @param {any} p @param {Spec | null} spec */
export function sync(p, spec) {
  if (!spec) return clear(p);
  let s = /** @type {State | undefined} */ (p._eh);
  if (s && s.spec.key !== spec.key) { clear(p); s = undefined; }
  if (s) { s.spec = spec; return layout(p); }
  css();
  const root = document.createElement("div"); root.className = "eh";
  root.innerHTML = '<svg class="eh-ar" aria-hidden="true" focusable="false"><path class="eh-ln"/><path class="eh-hd"/></svg><div class="eh-box"><button type="button" class="eh-go"><span class="eh-tx"></span></button></div>';
  const q = (/** @type {string} */ sel) => /** @type {any} */ (root.querySelector(sel));
  const go = /** @type {HTMLElement} */ (q(".eh-go"));
  go.addEventListener("click", () => { const t = target(p), a = /** @type {State} */ (p._eh).spec.act; if (a) a(t); else if (t) /** @type {HTMLElement} */ (t).click(); });
  p.el.appendChild(root);
  const redo = () => { if (s) { FRAME.cancel(s.raf); s.raf = FRAME.next(() => layout(p)); } };
  const mo = new MutationObserver(redo);
  s = p._eh = { spec, root, box: q(".eh-box"), go, tx: q(".eh-tx"), ln: q(".eh-ln"), hd: q(".eh-hd"), fx: null, stops: [], mo, raf: 0 };
  s.stops = [p.el, p.cv, go, ...[".hd", ".ttl", ".ctl"].map(q => p.el.querySelector(q))].map(e => FRAME.ro(e, redo));
  if (spec.out) {      // a control outside the panel moves with the page (scroll, side bar opened or resized, list re-rendered), not with the panel
    s.stops.push(FRAME.ro(outEl(spec), redo));
    for (const ev of ["resize", "scroll"]) { window.addEventListener(ev, redo, { capture: true, passive: true }); s.stops.push(() => window.removeEventListener(ev, redo, { capture: true })); }
  }
  const hd = p.el.querySelector(".hd"); if (hd) mo.observe(hd, { childList: true, subtree: true, attributes: true });      // the toolbar is rebuilt as a whole, and its fit moves controls with a class or a style: its control is not where it was
  layout(p);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => layout(p));
}

/** the control outside the panel that a spec names @param {Spec} sp */
const outEl = sp => typeof sp.out === "function" ? sp.out() : sp.out ? document.querySelector(sp.out) : null;

/** the control the prompt points at, if it is there (and, outside the panel, on screen) @param {any} p */
function target(p) {
  const s = /** @type {State | undefined} */ (p._eh), sp = s && s.spec, t = sp ? sp.out ? outEl(sp) : sp.target ? p.el.querySelector(sp.target) : null : null;
  if (!t) return null;
  const r = t.getBoundingClientRect(); if (!(r.width > 0 && r.height > 0)) return null;
  return sp && sp.out && !(r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth) ? null : t;
}

/** the layer an outside control's arrow is drawn in: the panel clips its children, this one is fixed on the page @param {State} s */
function fixed(s) {
  if (s.fx) return s.fx;
  const svg = /** @type {SVGSVGElement} */ (document.createElementNS("http://www.w3.org/2000/svg", "svg")); svg.setAttribute("class", "eh-fx"); svg.setAttribute("aria-hidden", "true"); svg.setAttribute("focusable", "false");
  const ln = /** @type {SVGPathElement} */ (document.createElementNS(svg.namespaceURI, "path")), hd = /** @type {SVGPathElement} */ (document.createElementNS(svg.namespaceURI, "path")); ln.setAttribute("class", "eh-ln"); hd.setAttribute("class", "eh-hd");
  svg.append(ln, hd); document.body.appendChild(svg); return s.fx = { svg, ln, hd };
}

/** a hand-drawn curl from (sx, sy) to (ex, ey): it leaves along F bending away from the control (L is the lateral axis) and comes back into it; the head is on the end
 * @param {SVGPathElement} ln @param {SVGPathElement} hd @param {number} sx @param {number} sy @param {number} ex @param {number} ey @param {number} fx @param {number} fy */
function curl(ln, hd, sx, sy, ex, ey, fx, fy) {
  const lx = -fy, ly = fx, dist = (ex - sx) * fx + (ey - sy) * fy, lat = (ex - sx) * lx + (ey - sy) * ly, sgn = lat >= 0 ? 1 : -1, bow = Math.min(Math.max(Math.abs(lat) * 0.25, 26), 90);
  const at = (/** @type {number} */ u, /** @type {number} */ v) => [sx + u * fx + v * lx, sy + u * fy + v * ly];
  const [c1x, c1y] = at(dist * 0.45, -sgn * bow), [c2x, c2y] = at(dist - Math.min(dist * 0.4, 70), lat);
  ln.setAttribute("d", `M${sx.toFixed(1)} ${sy.toFixed(1)}C${c1x.toFixed(1)} ${c1y.toFixed(1)} ${c2x.toFixed(1)} ${c2y.toFixed(1)} ${ex.toFixed(1)} ${ey.toFixed(1)}`);
  const a = Math.atan2(ey - c2y, ex - c2x) + Math.PI, hl = 10, w = 0.5;
  hd.setAttribute("d", `M${(ex + hl * Math.cos(a - w)).toFixed(1)} ${(ey + hl * Math.sin(a - w)).toFixed(1)}L${ex.toFixed(1)} ${ey.toFixed(1)}L${(ex + hl * Math.cos(a + w)).toFixed(1)} ${(ey + hl * Math.sin(a + w)).toFixed(1)}`);
}

/** place the prompt over the canvas, choose its words and draw the arrow @param {any} p */
function layout(p) {
  const s = /** @type {State | undefined} */ (p._eh); if (!s) return; if (!p.el.isConnected) return clear(p);      // a closed panel takes its fixed arrow layer with it
  const pr = p.el.getBoundingClientRect(); if (!pr.width) { if (s.fx) s.fx.svg.remove(), s.fx = null; return; }            // hidden (another tab): nothing to measure
  const ox = pr.left + p.el.clientLeft, oy = pr.top + p.el.clientTop, cr = p.cv.getBoundingClientRect();
  Object.assign(s.box.style, { left: cr.left - ox + "px", top: cr.top - oy + "px", width: cr.width + "px", height: cr.height + "px" });
  const t = target(p), sp = s.spec, out = !!sp.out, narrow = pr.width < NARROW, plain = T(sp.plain || sp.above || sp.key);
  const go = /** @type {HTMLButtonElement} */ (s.go);
  go.disabled = !t && !sp.act; go.classList.toggle("eh-txt", go.disabled);
  // alt is words already translated (msgOf), the rest are keys
  s.tx.textContent = !t ? (out ? plain : sp.alt || T(sp.key)) : narrow ? (out ? plain : T(sp.above || sp.key)) : T(sp.key);
  const fx = out && t ? fixed(s) : null, ln = fx ? fx.ln : s.ln, hd = fx ? fx.hd : s.hd;
  const clearArrow = () => { for (const e of [s.ln, s.hd, fx?.ln, fx?.hd]) e?.removeAttribute("d"); if (!fx && s.fx) s.fx.svg.remove(), s.fx = null; };
  if (!t || narrow) return clearArrow();
  const tb = t.getBoundingClientRect(), gr = go.getBoundingClientRect(), qx = fx ? 0 : ox, qy = fx ? 0 : oy;      // the fixed layer is in viewport coordinates, the panel's in its own
  const none = () => { s.tx.textContent = out ? plain : T(sp.above || sp.key); clearArrow(); };      // no room for an arrow: the line says «above» (or the plain words) and that is all
  if (tb.bottom <= gr.top - GAP) {
    const tcx = tb.left + tb.width / 2 - qx, sx = Math.min(Math.max(tcx, gr.left - qx + 16), gr.right - qx - 16);
    return curl(ln, hd, sx, gr.top - qy - 5, tcx, tb.bottom - qy + 6, 0, -1);
  }
  if (!out) return none();
  const cy = (gr.top + gr.bottom) / 2, ey = Math.min(Math.max(cy, Math.max(tb.top, 0) + 10), Math.min(tb.bottom, innerHeight) - 10);      // a tall control (the file list): the arrow lands level with the prompt, kept on screen
  if (tb.right <= gr.left - GAP) return curl(ln, hd, gr.left - 5, cy, tb.right + 6, ey, -1, 0);
  if (tb.left >= gr.right + GAP) return curl(ln, hd, gr.right + 5, cy, tb.left - 6, ey, 1, 0);
  none();
}
