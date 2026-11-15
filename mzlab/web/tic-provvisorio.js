// @ts-check
// The provisional TIC of the first file of a loading, drawn in the work area while its index is still being built: the MS1 rt/tic of the scan table (event mzlab-index
// with early_tic, idx-worker.js) arrives seconds before the first panel can open. It shows only while nothing is open yet, and goes as soon as a file opens or the loading
// ends; a TIC is the raw trace the file declares, not a result. Lazy ES module, loaded by area-lavoro.js at the first early_tic; syncWork and every mzlab-index event call TICPROV.sync.
// The RT axis can be zoomed and panned meanwhile (Ctrl/Cmd + wheel, drag, + - arrows 0, double click: reset), drawn with the display thinning of decima.js; the window the user chose
// is handed to the first panels (addPanel asks TICPROV.handover). Nothing else is interactive before the index: no spectrum, no XIC.
// static import, not a top-level await: carica() resolves when the script is loaded, before an awaiting module has run (TICPROV would still be undefined)
import * as DC from "./decima.js";
const W = /** @type {any} */ (window);
const T = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
/** @type {Map<string, { rt: number[], y: number[] }>} */ const tics = new Map();
/** @type {HTMLElement | null} */ let box = null;
let shownKey = "";

const css = (/** @type {Element} */ el, /** @type {string} */ v) => getComputedStyle(el).getPropertyValue(v).trim();
// round steps for the axis ticks (1, 2, 5 × 10^n)
function step(/** @type {number} */ span, /** @type {number} */ n) {
  const raw = span / Math.max(1, n), p = Math.pow(10, Math.floor(Math.log10(raw || 1))), m = raw / p;
  return (m < 1.5 ? 1 : m < 3.5 ? 2 : m < 7.5 ? 5 : 10) * p;
}
/** @type {[number, number] | null} */ let view = null;     // RT window the user chose (null = the whole trace)
/** @type {[number, number]} */ let full = [0, 1];            // RT extent of the trace shown
const PAD = { L: 56, R: 10, T: 8, B: 30 }, H = 180;
const plotW = (/** @type {HTMLCanvasElement} */ cv) => Math.max(200, cv.clientWidth || 600) - PAD.L - PAD.R;
function draw(/** @type {HTMLCanvasElement} */ cv, /** @type {{ rt: number[], y: number[] }} */ t) {
  const w = Math.max(200, cv.clientWidth || 600), h = H, R = devicePixelRatio || 1;
  cv.width = Math.round(w * R); cv.height = Math.round(h * R);
  const cx = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d")); cx.setTransform(R, 0, 0, R, 0, 0); cx.clearRect(0, 0, w, h);
  const { L, R: Rm, T: Tp, B } = PAD, n = t.rt.length;
  let x0 = Infinity, x1 = -Infinity;
  if (view) { x0 = view[0]; x1 = view[1]; } else for (let i = 0; i < n; i++) { const x = t.rt[i]; if (x < x0) x0 = x; if (x > x1) x1 = x; }
  if (!(x1 > x0)) x1 = x0 + 1;
  // the height follows the window (as a zoom of the panels does not, here the trace is a preview: the peaks in view fill the plot)
  const y1 = Math.max(DC.extent(t.rt, t.y, x0, x1)[0], 0) || 1;
  const X = (/** @type {number} */ v) => L + (v - x0) / (x1 - x0) * (w - L - Rm), Y = (/** @type {number} */ v) => h - B - v / y1 * (h - B - Tp);
  const ink = css(cv, "--ink"), muted = css(cv, "--muted"), line = css(cv, "--line");
  cx.font = "12px system-ui"; cx.fillStyle = muted; cx.strokeStyle = line; cx.lineWidth = 1;
  cx.beginPath(); cx.moveTo(L, Tp); cx.lineTo(L, h - B); cx.lineTo(w - Rm, h - B); cx.stroke();
  const sx = step(x1 - x0, 6);
  cx.textAlign = "center"; cx.textBaseline = "top";
  for (let v = Math.ceil(x0 / sx) * sx; v <= x1 + 1e-9; v += sx) { const px = X(v); cx.fillText(String(+v.toFixed(3)), px, h - B + 5); }
  const sy = step(y1, 3);
  cx.textAlign = "right"; cx.textBaseline = "middle";
  for (let v = 0; v <= y1 + 1e-9; v += sy) cx.fillText(v.toExponential(1), L - 5, Y(v));
  cx.strokeStyle = css(cv, "--lv1") || ink; cx.lineWidth = 1.5;
  cx.save(); cx.beginPath(); cx.rect(L, Tp - 1, w - L - Rm, h - B - Tp + 2); cx.clip();
  // four vertices per pixel column (decima.js), a little beyond the window so the line reaches both edges
  const pad = (x1 - x0) * 0.01, v = DC.polyline(t.rt, t.y, x0 - pad, x1 + pad, X, Y);
  cx.beginPath();
  for (let i = 0; i < v.length; i += 2) { if (i) cx.lineTo(v[i], v[i + 1]); else cx.moveTo(v[0], v[1]); }
  cx.stroke(); cx.restore();
}
// ---- zoom and pan of the RT axis: one drawing per animation frame whatever the number of events
const cvEl = () => /** @type {HTMLCanvasElement | null} */ (box && box.querySelector("#ticprov-cv"));
let frame = 0, said = 0;
function redraw() {
  if (frame) return;
  frame = FRAME.next(() => { frame = 0; const cv = cvEl(), t = tics.get(shownKey); if (cv && t && !(box && box.hidden)) draw(cv, t); });
}
/** the window of the same rules as the panels (grafici.js clampView): the whole trace gives null, never narrower than 0.2 % of it */
function setView(/** @type {number} */ z0, /** @type {number} */ z1) {
  const [f0, f1] = full, tot = f1 - f0;
  let span = z1 - z0;
  if (!(tot > 0)) return;
  if (span >= tot * 0.999) view = null;
  else {
    span = Math.max(span, tot * 0.002);
    if (z0 < f0) { z1 += f0 - z0; z0 = f0; } if (z1 > f1) { z0 -= z1 - f1; z1 = f1; }
    view = [Math.max(f0, z0), Math.min(f1, z1)];
  }
  redraw(); announce();
}
// the window said to screen readers once the gesture rests (a live region updated on every wheel event would read them all)
function announce() {
  clearTimeout(said);
  said = /** @type {any} */ (setTimeout(() => {
    const el = box && box.querySelector("#ticprov-w"); if (!el) return;
    const [a, b] = view || full;
    el.textContent = view ? T("load.tprov.win", { a: I18N.fix(a, 2), b: I18N.fix(b, 2) }) : "";
  }, 350));
}
const cur = () => view || full;
/** zoom by `f` (< 1 closer) around the RT `at` */
const zoomAt = (/** @type {number} */ f, /** @type {number} */ at) => { const [a, b] = cur(); setView(at - (at - a) * f, at + (b - at) * f); };
const rtAt = (/** @type {HTMLCanvasElement} */ cv, /** @type {number} */ clientX) => {
  const [a, b] = cur(), r = cv.getBoundingClientRect(), k = Math.max(0, Math.min(1, (clientX - r.left - PAD.L) / plotW(cv)));
  return a + k * (b - a);
};
function wire(/** @type {HTMLCanvasElement} */ cv) {
  cv.tabIndex = 0; cv.setAttribute("aria-keyshortcuts", "+ - ArrowLeft ArrowRight 0");
  cv.addEventListener("wheel", e => {
    if (!(e.ctrlKey || e.metaKey)) return;                      // a plain wheel scrolls the page, as over the panels
    e.preventDefault(); zoomAt(Math.pow(1.0015, e.deltaY), rtAt(cv, e.clientX));
  }, { passive: false });
  /** @type {null | { x: number, a: number, b: number, moved: boolean }} */ let drag = null;
  cv.addEventListener("pointerdown", e => { if (e.button) return; drag = { x: e.clientX, a: cur()[0], b: cur()[1], moved: false }; cv.setPointerCapture(e.pointerId); });
  cv.addEventListener("pointermove", e => {
    if (!drag) return;
    const dx = e.clientX - drag.x; if (Math.abs(dx) > 3) drag.moved = true;
    if (drag.moved && view) { const d = -dx / plotW(cv) * (drag.b - drag.a); setView(drag.a + d, drag.b + d); }
  });
  const end = () => { const d = drag; drag = null; if (d && !d.moved) { const n = box && box.querySelector("#ticprov-n"); if (n) { n.textContent = ""; update(); } } };   // a click says again where the index is
  cv.addEventListener("pointerup", end); cv.addEventListener("pointercancel", () => { drag = null; });
  cv.addEventListener("dblclick", () => setView(full[0], full[1]));
  cv.addEventListener("keydown", e => {
    const [a, b] = cur(), c = (a + b) / 2;
    if (e.key === "+" || e.key === "=") zoomAt(0.7, c);
    else if (e.key === "-") zoomAt(1 / 0.7, c);
    else if (e.key === "0") setView(full[0], full[1]);
    else if (e.key === "ArrowLeft" || e.key === "ArrowRight") { if (!view) return; const d = (e.key === "ArrowLeft" ? -1 : 1) * (b - a) * 0.2; setView(a + d, b + d); }
    else return;
    e.preventDefault();
  });
}
/** the window to open the first panels on, once: the one the user chose here (null = nothing chosen) */
function handover() {
  const v = view; if (!v) return null;
  view = null;
  if (typeof toast === "function") toast(T("load.tprov.hand", { a: I18N.fix(v[0], 2), b: I18N.fix(v[1], 2) }));
  return /** @type {[number, number]} */ ([...v]);
}
// the file whose TIC is shown: the first row of the loading, in order of arrival, that is still loading and has its TIC
function pick() {
  const rows = CONVROWS.rows.filter((/** @type {ConvRow} */ r) => r.state !== "fail" && r.state !== "cancel" && tics.has(r.name))
    .sort((/** @type {ConvRow} */ a, /** @type {ConvRow} */ b) => CONVROWS.rank(a.name) - CONVROWS.rank(b.name));
  return rows[0] || null;
}
let lastPc = 0;
function update() { const n = box && box.querySelector("#ticprov-n"); if (n) n.textContent = T("load.tprov.note", { pc: I18N.fix(lastPc, 0) }); }
function hide() { if (box) box.hidden = true; shownKey = ""; }
function sync() {
  const loading = (typeof UPBUSY !== "undefined" && UPBUSY > 0) || OPENNOW.busy() > 0;
  if (!loading) tics.clear();
  // it goes with the landing (area-lavoro.js syncWork), not when the first file enters E.files: the landing stays some frames more and the table under it would move up
  const land = /** @type {HTMLElement} */ (document.getElementById("v-workspace")).dataset.work === "landing";
  // its file just read and about to open (row done, session begun): the box shown stays as it is until the panels take the landing's place
  const stay = loading && land && !!shownKey && !!box && !box.hidden, r = loading && land && !S.sess ? pick() : null;
  if (!r) { if (!stay) hide(); return; }
  if (!box) {
    const up = document.getElementById("start");
    if (!up) return;
    box = document.createElement("section"); box.id = "ticprov"; box.className = "card";
    box.innerHTML = '<h3 id="ticprov-h"></h3><p class="sm muted" id="ticprov-n"></p><canvas id="ticprov-cv"></canvas><p class="sm muted" id="ticprov-w" role="status" aria-live="polite"></p>';
    box.setAttribute("aria-labelledby", "ticprov-h");
    wire(/** @type {HTMLCanvasElement} */ (box.querySelector("#ticprov-cv")));
    up.insertAdjacentElement("afterend", box); // below the drop area, so the drop target never moves under the pointer
  }
  const t = /** @type {{ rt: number[], y: number[] }} */ (tics.get(r.name)), pc = r.state === "done" ? 100 : Math.round(r.pc || 0);
  lastPc = pc; update();
  box.hidden = false;
  if (shownKey === r.name) return;
  shownKey = r.name; view = null;
  { let a = Infinity, b = -Infinity; for (const x of t.rt) { if (x < a) a = x; if (x > b) b = x; } full = [a, b]; }
  /** @type {HTMLElement} */ (box.querySelector("#ticprov-w")).textContent = "";
  const cv = /** @type {HTMLCanvasElement} */ (box.querySelector("#ticprov-cv")), title = T("load.tprov.title", { file: r.name });
  /** @type {HTMLElement} */ (box.querySelector("#ticprov-h")).textContent = title;
  cv.setAttribute("aria-label", title); cv.setAttribute("aria-description", T("load.tprov.keys"));
  draw(cv, t);
  TDATI.series(cv, { title, xl: T("load.tprov.xl"), yl: T("load.tprov.yl"), series: [{ x: t.rt, y: t.y, name: r.name }] });
}
function onIndex(/** @type {any} */ d) {
  if (d && d.name && d.early_tic && d.early_tic.rt && d.early_tic.rt.length > 1) tics.set(d.name, { rt: d.early_tic.rt, y: d.early_tic.y || d.early_tic.tic });
  if (d && d.name && (d.early_tic || shownKey === d.name)) sync();
}
addEventListener("mzlab-index", (/** @type {any} */ e) => onIndex(e && e.detail));
addEventListener("resize", () => { if (shownKey && box && !box.hidden) redraw(); });
// feed: the events that arrived before this module was loaded (area-lavoro.js loads it at the first early_tic)
W.TICPROV = { sync, handover, window: () => (view || full).slice(), feed: (/** @type {any[]} */ ds) => ds.forEach(onIndex) };
