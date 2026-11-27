// @ts-check
// @ts-strict
// Empty state of a panel with nothing to draw: a centred prompt that names what to choose, and (when its control is on screen above it) a curved arrow to that
// control. The prompt is a button that clicks the same control; the arrow is decorative (aria-hidden), its start and end are read from the live rectangles of the
// prompt and the control, so a resize, a wrapped toolbar or a re-rendered control redraws it. grafici.js drawLines calls sync(p, spec) / clear(p).

/** @typedef {{ key: string, above?: string, target?: string, alt?: string }} Spec  key: i18n key of the prompt; above: the same without the arrow; target: selector (inside the panel) of the control; alt: text when that control is absent */
/** @typedef {{ spec: Spec, root: HTMLElement, box: HTMLElement, go: HTMLElement, tx: HTMLElement, ln: SVGPathElement, hd: SVGPathElement, stops: (() => void)[], mo: MutationObserver, raf: number }} State */

const NARROW = 320;      // a panel narrower than this (px; the CSS minimum is 300, so a 320 px phone) gets the plain «… above» line, no arrow
const GAP = 14;          // least vertical room (px) between the control and the prompt for an arrow
const T = (/** @type {string} */ k) => /** @type {any} */ (window).I18N.t(k);

/** the styles go in once, here, so the module stays a leaf */
function css() {
  if (document.getElementById("eh-css")) return;
  const s = document.createElement("style"); s.id = "eh-css";
  s.textContent = ".pnl>.eh{position:absolute;inset:0;pointer-events:none;z-index:4}.eh-ar{position:absolute;left:0;top:0;width:100%;height:100%;overflow:visible}" +
    ".eh-ln,.eh-hd{fill:none;stroke:var(--accent);stroke-width:2;stroke-linecap:round;stroke-linejoin:round}" +
    ".eh-box{position:absolute;display:flex;align-items:center;justify-content:center;box-sizing:border-box;padding:6px 10px;overflow:hidden}" +
    ".eh-go{pointer-events:auto;font:inherit;font-size:15px;font-weight:600;line-height:1.3;color:var(--ink);background:var(--panel);border:1.5px dashed var(--accent);border-radius:10px;padding:8px 16px;min-height:36px;max-width:100%;cursor:pointer;text-align:center}" +
    ".eh-go:hover{background:var(--sel)}.eh-go:focus-visible{outline:2px solid var(--accent);outline-offset:2px}" +
    ".eh-go:disabled,.eh-go.eh-txt{pointer-events:none;cursor:default;border-color:transparent;background:none;color:var(--muted);font-weight:400}" +
    "@media (pointer:coarse){.eh-go{min-height:44px}}" +
    "@media (forced-colors:active){.eh-ln,.eh-hd{stroke:CanvasText}.eh-go{border-color:ButtonText;background:ButtonFace;color:ButtonText}.eh-go:disabled,.eh-go.eh-txt{color:CanvasText;background:none}}";
  document.head.appendChild(s);
}

/** what an empty chromatogram asks for (drawLines): hidden = traces exist but are all hidden (a line on the canvas, the legend is there to click); else the channel picker of an MRM / SIM panel
 * with no channel, the file counter when files exist but none is shown, or the words of the panel (say) @param {any} p @param {CanvasRenderingContext2D} g @param {number} hidden @param {string} say @param {number} x */
export function empty(p, g, hidden, say, x) {
  if (hidden) { clear(p); g.fillStyle = ecss("--muted"); g.fillText(T("leg.allHidden"), x, 30); return; }
  const mrm = ESP.mrm(p), files = tabFiles(p.tab || E.tab);
  sync(p, mrm && Array.isArray(p.tr) && !p.tr.length ? { key: "empty.channel", above: "empty.channel.above", target: ".canb[data-o=canali]", alt: say }
    : !mrm && files.length && !files.some((/** @type {any} */ f) => f.vis) ? { key: "empty.file", above: "empty.file.above", target: "[data-o=fpop]", alt: say } : { key: "msg." + p.type, alt: say });
}

/** @param {any} p */
export function clear(p) {
  const s = /** @type {State | undefined} */ (p._eh); if (!s) return;
  s.stops.forEach(f => f()); s.mo.disconnect(); FRAME.cancel(s.raf); s.root.remove(); p._eh = undefined;
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
  go.addEventListener("click", () => { const t = target(p); if (t) /** @type {HTMLElement} */ (t).click(); });
  p.el.appendChild(root);
  const redo = () => { if (s) { FRAME.cancel(s.raf); s.raf = FRAME.next(() => layout(p)); } };
  const mo = new MutationObserver(redo);
  s = p._eh = { spec, root, box: q(".eh-box"), go, tx: q(".eh-tx"), ln: q(".eh-ln"), hd: q(".eh-hd"), stops: [], mo, raf: 0 };
  s.stops = [p.el, p.cv, go, ...[".hd", ".ttl", ".ctl"].map(q => p.el.querySelector(q))].map(e => FRAME.ro(e, redo));
  const hd = p.el.querySelector(".hd"); if (hd) mo.observe(hd, { childList: true, subtree: true, attributes: true });      // the toolbar is rebuilt as a whole, and its fit moves controls with a class or a style: its control is not where it was
  layout(p);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(() => layout(p));
}

/** the control the prompt points at, if it is there @param {any} p */
function target(p) {
  const s = /** @type {State | undefined} */ (p._eh), t = s && s.spec.target ? p.el.querySelector(s.spec.target) : null;
  if (!t) return null;
  const r = t.getBoundingClientRect(); return r.width > 0 && r.height > 0 ? t : null;
}

/** place the prompt over the canvas, choose its words and draw the arrow @param {any} p */
function layout(p) {
  const s = /** @type {State | undefined} */ (p._eh); if (!s || !p.el.isConnected) return;
  const pr = p.el.getBoundingClientRect(); if (!pr.width) return;            // hidden (another tab): nothing to measure
  const ox = pr.left + p.el.clientLeft, oy = pr.top + p.el.clientTop, cr = p.cv.getBoundingClientRect();
  Object.assign(s.box.style, { left: cr.left - ox + "px", top: cr.top - oy + "px", width: cr.width + "px", height: cr.height + "px" });
  const t = target(p), sp = s.spec, narrow = pr.width < NARROW;
  const go = /** @type {HTMLButtonElement} */ (s.go);
  go.disabled = !t; go.classList.toggle("eh-txt", !t);
  s.tx.textContent = !t && sp.alt ? sp.alt : T(!t ? sp.key : narrow && sp.above ? sp.above : sp.key);      // alt is words already translated (msgOf), the rest are keys
  const clearArrow = () => { s.ln.removeAttribute("d"); s.hd.removeAttribute("d"); };
  if (!t || narrow) return clearArrow();
  const tb = t.getBoundingClientRect(), gr = go.getBoundingClientRect();
  if (tb.bottom > gr.top - GAP) { s.tx.textContent = T(sp.above || sp.key); return clearArrow(); }     // no room above the prompt: the line says «above» and that is all
  const tcx = tb.left + tb.width / 2 - ox, sx = Math.min(Math.max(tcx, gr.left - ox + 16), gr.right - ox - 16), sy = gr.top - oy - 5;
  const ex = tcx, ey = tb.bottom - oy + 6, dy = sy - ey, dx = ex - sx, sgn = dx >= 0 ? 1 : -1, bow = Math.min(Math.max(Math.abs(dx) * 0.25, 26), 90);
  // leaves the prompt bending away from the control and comes back up into it: a hand-drawn curl, not a ruled line
  const c1x = sx - sgn * bow, c1y = sy - dy * 0.45, c2x = ex, c2y = ey + Math.min(dy * 0.4, 70);
  s.ln.setAttribute("d", `M${sx.toFixed(1)} ${sy.toFixed(1)}C${c1x.toFixed(1)} ${c1y.toFixed(1)} ${c2x.toFixed(1)} ${c2y.toFixed(1)} ${ex.toFixed(1)} ${ey.toFixed(1)}`);
  const a = Math.atan2(ey - c2y, ex - c2x) + Math.PI, hl = 10, w = 0.5;
  s.hd.setAttribute("d", `M${(ex + hl * Math.cos(a - w)).toFixed(1)} ${(ey + hl * Math.sin(a - w)).toFixed(1)}L${ex.toFixed(1)} ${ey.toFixed(1)}L${(ex + hl * Math.cos(a + w)).toFixed(1)} ${(ey + hl * Math.sin(a + w)).toFixed(1)}`);
}
