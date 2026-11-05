// @ts-strict
// @ts-noany
// LC gradient (%B) of a file, read-only: from the .dam (the server) or from the file's own method document (metodo/1); nothing can be typed or imported.
// The model is {source: dam|raw|file, points: [{t, b, flow?, curve?}], run_time?, solvents?}. Classic globals are used as they are (E, J, I18N, EH).
const cache = new Map();                                     // file name -> model | null (what the server answered)
const nameOf = (/** @type {FileAperto} */ f) => f.file.split("#")[0];
/** @param {string} k @param {Bag} [p] */ const T = (k, p) => I18N.t(k, p);


// The pump program that the file's own method document (metodo/1: .raw, .wiff, .D, Waters) carries, as a gradient model; null when it has fewer than two %B points.
async function fromMethod(/** @type {FileAperto} */ f) {
  let lc = null;
  try { lc = (((await J("api/metodo?k=" + f.k)) || {}).metodo || {}).lc; } catch (_) { return null; }
  const pts = ((lc && lc.gradiente) || []).filter((/** @type {Bag} */ q) => q.t_min != null && q.B_pct != null).map((/** @type {Bag} */ q) => ({ t: +q.t_min, b: +q.B_pct, ...(q.flusso_mL_min != null ? { flow: +q.flusso_mL_min } : {}) }));
  return pts.length > 1 ? { source: "file", points: pts, run_time: lc.durata_min != null ? +lc.durata_min : pts[pts.length - 1].t } : null;
}
// The gradient of file f: the .dam first (the server), then the file's own method.
async function get(/** @type {FileAperto} */ f) {
  const n = nameOf(f);
  if (cache.has(n)) return cache.get(n);
  let g = null;
  try { g = (await J("api/gradient?k=" + f.k)).gradient; } catch (_) { /* no gradient */ }
  if (!g) g = await fromMethod(f);
  cache.set(n, g);
  return g;
}
const any = () => [...cache.values()].some(Boolean);
const forget = () => cache.clear();
document.addEventListener("nbreset", () => { forget(); });

// ---- the read-only section in the Method window (files without a gradient in the .dam): the gradient comes only from the file's own metadata
const SRC = { dam: "grad.src.dam", raw: "grad.src.raw", file: "grad.src.file" };

async function html(/** @type {FileAperto} */ f) {
  const g = await get(f);
  if (!g) return "";
  let h = `<section class="msec" id="grad-sec"><h4>${T("grad.title")}</h4>`;
  h += `<div class="sm" style="margin:0 0 6px">${T((/** @type {Record<string, string>} */ (SRC))[g.source] || "grad.src.file")}${g.nonlinear ? " · " + T("grad.nonlinear") : ""}</div>`;
  h += typeof lcHtml === "function" ? lcHtml({ gradient: g.points, run_time: g.run_time }, () => "") : "";
  return h + "</section>";
}


// ---- the gradient drawn over a chromatogram: right axis 0-100 %B, ramps, area at 10 %, dashed (never colour alone)
const pending = new Set();
const filesOf = (/** @type {Bag} */ p) => (typeof shownFor === "function" ? shownFor(p) : []);
const key = (/** @type {GradModel} */ m) => JSON.stringify(m.points);
// the gradients of the files of a panel are fetched once; when one arrives the panels are drawn again
function ensure(/** @type {Bag} */ p) {
  if (p.overlay === "press") pressEnsure(p);
  filesOf(p).forEach((/** @type {FileAperto} */ f) => {
    const n = nameOf(f); if (cache.has(n) || pending.has(n)) return;
    pending.add(n);
    get(f).then(g => { pending.delete(n); if (g && typeof redrawSoon === "function") redrawSoon(); }, () => pending.delete(n));
  });
}
const have = (/** @type {Bag} */ p) => filesOf(p).some((/** @type {FileAperto} */ f) => cache.get(nameOf(f)));
// one curve if the gradients of the shown files are equal, else the one of the current file, with a note
function curve(/** @type {Bag} */ p) {
  const fs = filesOf(p), ms = fs.map((/** @type {FileAperto} */ f) => ({ f, m: cache.get(nameOf(f)) })).filter((/** @type {Bag} */ x) => x.m);
  if (!ms.length) return null;
  const same = ms.every((/** @type {Bag} */ x) => key(x.m) === key(ms[0].m));
  if (same && ms.length === fs.length) return { m: ms[0].m, note: null };
  const cur = ms.find((/** @type {Bag} */ x) => x.f === E.files[E.cur]) || ms[0];
  return { m: cur.m, note: fs.length > 1 ? T("grad.legend.one", { file: cur.f.label }) : null };
}
// {b, flow?} on the straight ramps; null outside the programmed time
function at(/** @type {GradModel} */ m, /** @type {number} */ t) {
  const pts = m.points; if (!pts.length || t < pts[0].t || t > pts[pts.length - 1].t) return null;
  /** @type {{ b: number, flow?: number } | null} */ let r = null;
  for (let i = 0; i < pts.length - 1; i++) {
    const a = pts[i], b = pts[i + 1]; if (t < a.t || t > b.t) continue;
    const w = b.t === a.t ? 1 : (t - a.t) / (b.t - a.t);
    r = { b: a.b + w * (b.b - a.b) }; if (a.flow != null && b.flow != null) r.flow = a.flow + w * (b.flow - a.flow);
  }
  return r;
}
const delayOf = (/** @type {Bag} */ p) => p.gdelay || 0;
function reading(/** @type {Bag} */ p, /** @type {number} */ t) {                                     // the values at time t (programmed and at the column)
  const c = curve(p); if (!c || p.overlay !== "grad") return null;
  const d = delayOf(p), prog = at(c.m, t), col = d ? at(c.m, t - d) : null;
  return prog || col ? { b: prog ? prog.b : null, flow: prog ? prog.flow : null, bcol: col ? col.b : null } : null;
}
function colorOf(/** @type {Bag} */ p) {
  const used = new Set(filesOf(p).map((/** @type {FileAperto} */ f) => String(f.color || "").toLowerCase()));
  const dark = document.documentElement.dataset.theme === "dark" || (document.documentElement.dataset.theme !== "light" && matchMedia("(prefers-color-scheme: dark)").matches);
  const first = css("--grad").toLowerCase();
  if (!used.has(first)) return css("--grad");
  const c = (dark ? OKABE_DARK : OKABE).find((/** @type {string} */ x) => !used.has(x.toLowerCase()));
  return c || css("--muted");
}
// Right axis box (px from the axis line): 7 px tick + tick labels + gap + the rotated title line + pad. Estimated widths (no canvas), so needR can run before drawing.
const TICK = 7, GAP = 5, PAD = 3, DIGIT = 0.6;
/** @param {string[]} labels @param {number} fontPx @returns {{ r: number, titleX: number }} r = right margin, titleX = title baseline from the axis line */
function rightLayout(labels, fontPx) {
  const tw = labels.reduce((m, s) => Math.max(m, Math.ceil(s.length * DIGIT * fontPx)), 0), left = TICK + tw + GAP;
  return { r: left + Math.ceil(fontPx) + PAD, titleX: left + fontPx * 0.2 };
}
// the rotated title never runs past the axis length: cut with an ellipsis
/** @param {string} s @param {number} maxPx @param {number} fontPx */
function fitTitle(s, maxPx, fontPx) { const n = Math.max(1, Math.floor(maxPx / (DIGIT * fontPx))); return s.length <= n ? s : s.slice(0, Math.max(1, n - 1)) + "\u2026"; }
const gradTicks = ["0", "25", "50", "75", "100"];
function needR(/** @type {Bag} */ p) {
  const z = typeof fz === "function" ? fz() : 1;
  if (p.overlay === "grad" && have(p)) return rightLayout(gradTicks, 12 * z).r;
  if (p.overlay === "press" && pressHave(p)) { const cs = pressCurves(p); return rightLayout(cs.length ? nice(0, pressMax(cs), 5).map((/** @type {number} */ v) => String(+v.toFixed(1))) : ["000"], 12 * z).r; }
  return 0;
}

// ---- the pump pressure over a chromatogram (same right axis as the gradient, one or the other); in bar, one curve per file in its colour
const PRESS = new Map();                                     // "k|i" -> the answer of api/analog
const pressTrace = (/** @type {FileAperto} */ f) => (f.analog || []).find((/** @type {Bag} */ a) => a.kind === "pressure");
const pressHave = (/** @type {Bag} */ p) => filesOf(p).some((/** @type {FileAperto} */ f) => pressTrace(f));
function pressEnsure(/** @type {Bag} */ p) {
  filesOf(p).forEach((/** @type {FileAperto} */ f) => {
    const a = pressTrace(f); if (!a) return; const k = f.k + "|" + a.i; if (PRESS.has(k)) return;
    PRESS.set(k, null);
    J(`api/analog?k=${f.k}&i=${a.i}&n=2000`).then((/** @type {Bag} */ d) => { PRESS.set(k, d); if (typeof redrawSoon === "function") redrawSoon(); }, () => {});
  });
}
/** The unit of the first pressure curve: "bar" when the file declares a pressure unit, otherwise the unit the file gives. */
const pressUnit = (/** @type {Bag[]} */ cs) => (cs[0] && cs[0].d.unit) || "";
const pressCurves = (/** @type {Bag} */ p) => filesOf(p).map((/** @type {FileAperto} */ f) => ({ f, d: PRESS.get(f.k + "|" + (pressTrace(f) || {}).i) })).filter((/** @type {Bag} */ x) => x.d && x.d.t.length);
const pressMax = (/** @type {Bag[]} */ cs) => { let m = 0; cs.forEach((/** @type {Bag} */ c) => c.d.y.forEach((/** @type {number} */ v) => { if (v > m) m = v; })); return m > 0 ? m * 1.08 : 1; };
function drawPress(/** @type {Bag} */ p, /** @type {CanvasRenderingContext2D} */ g, /** @type {Bag} */ a) {
  const cs = pressCurves(p); if (!cs.length) return;
  const { W, H, X } = a, ph = H - M.t - M.b, top = pressMax(cs), Y = (/** @type {number} */ v) => H - M.b - v / top * ph, z = typeof fz === "function" ? fz() : 1, ink = css("--muted");
  g.save(); g.strokeStyle = ink; g.fillStyle = ink; g.lineWidth = 1; g.font = fpx(12); g.textAlign = "left";
  g.beginPath(); g.moveTo(W - M.r + .5, M.t); g.lineTo(W - M.r + .5, H - M.b); g.stroke();
  for (const v of nice(0, top, 5)) { g.beginPath(); g.moveTo(W - M.r, Math.round(Y(v)) + .5); g.lineTo(W - M.r + 4, Math.round(Y(v)) + .5); g.stroke(); g.fillText(String(+v.toFixed(1)), W - M.r + 7, Y(v) + 3.5 * z); }
  { const fp = 12 * z, ttl = rightLayout(nice(0, top, 5).map((/** @type {number} */ v) => String(+v.toFixed(1))), fp); g.save(); g.translate(W - M.r + ttl.titleX, (M.t + H - M.b) / 2); g.rotate(Math.PI / 2); g.textAlign = "center"; g.fillText(fitTitle(pressUnit(cs), ph, fp), 0, 0); g.restore(); }
  g.beginPath(); g.rect(M.l, M.t, W - M.l - M.r, ph); g.clip();
  cs.forEach((/** @type {{ f: FileAperto, d: Bag }} */ { f, d }) => { g.strokeStyle = /** @type {string} */ (f.color); g.lineWidth = 1.6; g.setLineDash([6, 3]); g.beginPath(); d.t.forEach((/** @type {number} */ t, /** @type {number} */ i) => (i ? g.lineTo(X(t), Y(d.y[i])) : g.moveTo(X(t), Y(d.y[i])))); g.stroke(); });
  g.restore();
  a.press = true;
}
function pressAt(/** @type {Bag} */ d, /** @type {number} */ t) { if (!d.t.length || t < d.t[0] || t > d.t[d.t.length - 1]) return null; let lo = 0, hi = d.t.length - 1; while (hi - lo > 1) { const m = (lo + hi) >> 1; d.t[m] <= t ? lo = m : hi = m; } return d.y[Math.abs(d.t[lo] - t) <= Math.abs(d.t[hi] - t) ? lo : hi]; }
function hoverPress(/** @type {Bag} */ p, /** @type {number} */ t, /** @type {number} */ py) {
  const a = p._a, cs = pressCurves(p); if (!a || !cs.length) return "";
  const ph = a.H - M.t - M.b, top = pressMax(cs);
  return cs.map((/** @type {{ f: FileAperto, d: Bag }} */ { f, d }) => { const v = pressAt(d, t); return v != null && Math.abs(a.H - M.b - v / top * ph - py) <= 8 ? `<div><i style="background:${f.color}"></i>${T("press.read", { t: t.toFixed(2), p: v.toFixed(0), u: pressUnit(cs) })}</div>` : ""; }).join("");
}
function draw(/** @type {Bag} */ p, /** @type {CanvasRenderingContext2D} */ g, /** @type {Bag} */ a) {
  if (p.overlay === "press") return drawPress(p, g, a);
  const c = curve(p); if (!c) return;
  const { W, H, X, x0, x1 } = a, ph = H - M.t - M.b, Y = (/** @type {number} */ v) => H - M.b - v / 100 * ph, col = colorOf(p), d = delayOf(p), z = typeof fz === "function" ? fz() : 1;
  g.save();
  g.strokeStyle = col; g.fillStyle = col; g.lineWidth = 1; g.font = fpx(12);
  // the right axis
  g.beginPath(); g.moveTo(W - M.r + .5, M.t); g.lineTo(W - M.r + .5, H - M.b); g.stroke();
  g.textAlign = "left";
  for (const v of [0, 25, 50, 75, 100]) { g.beginPath(); g.moveTo(W - M.r, Math.round(Y(v)) + .5); g.lineTo(W - M.r + 4, Math.round(Y(v)) + .5); g.stroke(); g.fillText(String(v), W - M.r + 7, Y(v) + 3.5 * z); }
  { const fp = 12 * z, ttl = rightLayout(gradTicks, fp); g.save(); g.translate(W - M.r + ttl.titleX, (M.t + H - M.b) / 2); g.rotate(Math.PI / 2); g.textAlign = "center"; g.fillText(fitTitle("%B", ph, fp), 0, 0); g.restore(); }
  g.beginPath(); g.rect(M.l, M.t, W - M.l - M.r, ph); g.clip();
  const trace = (/** @type {number} */ shift, /** @type {number} */ w, /** @type {number[]} */ dash, /** @type {boolean} */ area) => {
    const pts = c.m.points.map((/** @type {GradPt} */ q) => [X(q.t + shift), Y(q.b)]);
    g.beginPath(); pts.forEach((/** @type {number[]} */ [x, y], /** @type {number} */ i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
    g.setLineDash(dash); g.lineWidth = w; g.stroke(); g.setLineDash([]);
    if (area) { g.lineTo(pts[pts.length - 1][0], Y(0)); g.lineTo(pts[0][0], Y(0)); g.closePath(); g.globalAlpha = 0.1; g.fill(); g.globalAlpha = 1; }
  };
  // programmed: thin dots; at the column: full
  if (d) { g.globalAlpha = 0.75; trace(0, 1, [2, 3], false); g.globalAlpha = 1; trace(d, 2, [], true); }
  else trace(0, 2, [6, 3], true);
  g.restore();
  if (c.note) { g.save(); g.font = fpx(11); g.fillStyle = css("--muted"); g.textAlign = "right"; g.fillText(c.note, W - M.r - 6, M.t + 14 * z); g.restore(); }
  a.grad = true;
}
// the box that follows the mouse near the curve (8 px)
function hover(/** @type {Bag} */ p, /** @type {number} */ t, /** @type {number} */ py) {
  if (p.overlay === "press") return hoverPress(p, t, py);
  const r = /** @type {any} */ (reading(p, t)), a = p._a; if (!r || !a) return "";
  const ph = a.H - M.t - M.b, Y = (/** @type {number} */ v) => a.H - M.b - v / 100 * ph, vals = [r.b, r.bcol].filter(v => v != null);
  if (!vals.some(v => Math.abs(Y(v) - py) <= 8)) return "";
  return `<div><i style="background:${colorOf(p)}"></i>${T("grad.read", { t: t.toFixed(2), b: vals.length === 2 ? `${r.b.toFixed(1)} / ${r.bcol.toFixed(1)}` : vals[0].toFixed(1) })}${r.flow != null ? " · " + r.flow.toFixed(2) + " mL/min" : ""}</div>`;
}
const cursorText = (/** @type {Bag} */ p) => { if (p.overlay === "press") { const c = pressCurves(p)[0], v = c && p.cur != null ? pressAt(c.d, p.cur) : null; return v != null ? " · " + T("press.read.cursor", { p: v.toFixed(0), u: pressUnit([c]) }) : ""; } const r = p.cur != null ? reading(p, p.cur) : null; return r ? " · " + T("grad.read.cursor", { b: /** @type {number} */ (r.bcol != null ? r.bcol : r.b).toFixed(1) }) : ""; };


// the overlays of the chromatogram menu (registry azioni/1): the pressure trace and the gradient over the graph
if (typeof AZIONI !== "undefined") {
  /** @param {string} id @param {string} nome @param {string} aria @param {(c: any) => boolean} have */
  const over = (id, nome, aria, have) => AZIONI.registra({ id: "chrom." + id, gruppo: "view", viste: ["chrom", "xic", "mrm"], visibile: have, attiva: (/** @type {Bag} */ c) => c.p.overlay === id,
    nome: () => T(nome), aria: () => T(aria), esegui: (/** @type {Bag} */ c) => { COMANDI.fai("spec.display", c.p, () => { c.p.overlay = c.p.overlay === id ? null : id; }); drawSoon(c.p); uiSave(); } });
  over("press", "ctx.chrom.press", "ctx.chrom.press.aria", c => pressHave(c.p) && !isAnalog(c.p.kind));
  over("grad", "ctx.chrom.grad", "ctx.chrom.grad.aria", c => have(c.p));
}
window.GRADIENTE = { pressHave, get, html, any, forget, ensure, have, curve, at, reading, draw, hover, needR, rightLayout, fitTitle, cursorText, colorOf };
