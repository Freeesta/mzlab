// @ts-check
// The provisional TIC of the first file of a loading, drawn in the work area while its index is still being built: the MS1 rt/tic of the scan table (event mzlab-index
// with early_tic, idx-worker.js) arrives seconds before the first panel can open. It shows only while nothing is open yet, and goes as soon as a file opens or the loading
// ends; a TIC is the raw trace the file declares, not a result. ES module; syncWork (area-lavoro.js) and every mzlab-index event call TICPROV.sync.
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
function draw(/** @type {HTMLCanvasElement} */ cv, /** @type {{ rt: number[], y: number[] }} */ t) {
  const w = Math.max(200, cv.clientWidth || 600), h = 180, R = devicePixelRatio || 1;
  cv.width = Math.round(w * R); cv.height = Math.round(h * R);
  const cx = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d")); cx.setTransform(R, 0, 0, R, 0, 0); cx.clearRect(0, 0, w, h);
  const L = 56, Rm = 10, Tp = 8, B = 30, n = t.rt.length;
  let x0 = Infinity, x1 = -Infinity, y1 = 0;
  for (let i = 0; i < n; i++) { const x = t.rt[i], y = t.y[i]; if (x < x0) x0 = x; if (x > x1) x1 = x; if (y > y1) y1 = y; }
  if (!(x1 > x0)) x1 = x0 + 1;
  if (!(y1 > 0)) y1 = 1;
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
  cx.strokeStyle = css(cv, "--lv1") || ink; cx.lineWidth = 1.5; cx.beginPath();
  for (let i = 0; i < n; i++) { const px = X(t.rt[i]), py = Y(t.y[i]); if (i) cx.lineTo(px, py); else cx.moveTo(px, py); }
  cx.stroke();
}
// the file whose TIC is shown: the first row of the loading, in order of arrival, that is still loading and has its TIC
function pick() {
  const rows = CONVROWS.rows.filter((/** @type {ConvRow} */ r) => r.state !== "fail" && r.state !== "cancel" && tics.has(r.name))
    .sort((/** @type {ConvRow} */ a, /** @type {ConvRow} */ b) => CONVROWS.rank(a.name) - CONVROWS.rank(b.name));
  return rows[0] || null;
}
function hide() { if (box) box.hidden = true; shownKey = ""; }
function sync() {
  const loading = (typeof UPBUSY !== "undefined" && UPBUSY > 0) || OPENNOW.busy() > 0;
  if (!loading) tics.clear();
  const open = E.files.some((/** @type {FileAperto} */ f) => !f.gone);
  const r = loading && !open && !S.sess ? pick() : null;
  if (!r) { hide(); return; }
  if (!box) {
    const up = document.getElementById("start");
    if (!up) return;
    box = document.createElement("section"); box.id = "ticprov"; box.className = "card";
    box.innerHTML = '<h3 id="ticprov-h"></h3><p class="sm muted" id="ticprov-n"></p><canvas id="ticprov-cv"></canvas>';
    box.setAttribute("aria-labelledby", "ticprov-h");
    up.insertAdjacentElement("afterend", box); // below the drop area, so the drop target never moves under the pointer
  }
  const t = /** @type {{ rt: number[], y: number[] }} */ (tics.get(r.name)), pc = r.state === "done" ? 100 : Math.round(r.pc || 0);
  /** @type {HTMLElement} */ (box.querySelector("#ticprov-n")).textContent = T("load.tprov.note", { pc: I18N.fix(pc, 0) });
  box.hidden = false;
  if (shownKey === r.name) return;
  shownKey = r.name;
  const cv = /** @type {HTMLCanvasElement} */ (box.querySelector("#ticprov-cv")), title = T("load.tprov.title", { file: r.name });
  /** @type {HTMLElement} */ (box.querySelector("#ticprov-h")).textContent = title;
  cv.setAttribute("aria-label", title);
  draw(cv, t);
  TDATI.series(cv, { title, xl: T("load.tprov.xl"), yl: T("load.tprov.yl"), series: [{ x: t.rt, y: t.y, name: r.name }] });
}
addEventListener("mzlab-index", (/** @type {any} */ e) => {
  const d = e && e.detail;
  if (d && d.name && d.early_tic && d.early_tic.rt && d.early_tic.rt.length > 1) tics.set(d.name, { rt: d.early_tic.rt, y: d.early_tic.y || d.early_tic.tic });
  if (d && d.name && (d.early_tic || shownKey === d.name)) sync();
});
addEventListener("resize", () => { if (shownKey && box && !box.hidden) { const t = tics.get(shownKey); if (t) draw(/** @type {HTMLCanvasElement} */ (box.querySelector("#ticprov-cv")), t); } });
W.TICPROV = { sync };
