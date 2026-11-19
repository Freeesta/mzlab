// @ts-strict
// DAD map, «MS» mode of the two strips: the MS1 spectrum at the cursor RT + the DAD→MS offset (right) and the TIC / BPC / EIC of the same file (bottom). The offset is
// signed, typed by the user (default 0: no reader exposes one in the file metadata) or taken from the engine estimate (`/api/delay`, evidence shown, applied only on request).
// Interface only; loaded with mappa-dad.js. Classic globals by name: E, I18N, getSpec, getChrom, getXic.
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);
const f2 = (/** @type {number} */ v) => (+v).toFixed(2);
/** The EIC window the strip asks the engine for: ± m/z and MS level; shown in the strip title and logged, not a hidden constant. */
const XIC_TOL = 0.5, XIC_LEVEL = 1;
/** A failed read: the strip shows the reason (a lang key) instead of waiting for ever. */
const fail = (/** @type {string} */ err) => ({ err });

/** The signed offset in minutes and where it comes from (default, typed, or the accepted estimate). @param {any} p */
export const offset = p => p.msOffset != null && isFinite(p.msOffset) ? { v: +p.msOffset, src: p._msUsed != null && +p.msOffset === p._msUsed ? "estimated" : "user" } : { v: 0, src: "default" };
/** The file that supplies the MS: the first file, or the reference of a subtracted map when asked. @param {any} p */
export const file = p => { const a = p._a; return p.msFrom === "b" && a.ref ? a.ref : a.f; };
const fname = (/** @type {any} */ f) => E.files[f.k] ? E.files[f.k].label : f.label || "";

/** Asks the engine for the delay of the file (cross-correlation of the DAD total trace and the TIC); stores the evidence on the panel, applies nothing. @param {any} p */
export async function estimate(p) {
  const f = file(p); p._msEst = { k: f.k, busy: true };
  try { const j = await J(`api/delay?k=${f.k}`); p._msEst = { k: f.k, d: j.delay || null }; } catch (e) { p._msEst = { k: f.k, d: null }; }
}
/** The user accepts the estimate: it becomes the offset and the choice is written on the panel. @param {any} p */
export function use(p) {
  const e = p._msEst && p._msEst.d; if (!e) return;
  p.msOffset = +f2(e.delay); p._msUsed = p.msOffset;
  (p._msLog || (p._msLog = [])).push(T("ctl.map.ms.used", { v: f2(e.delay), n: e.peaks, c: f2(e.corr) }));
}

/** A signed number of minutes for the strip titles. */
const sgn = (/** @type {number} */ v) => (v < 0 ? "−" : "+") + " " + f2(Math.abs(v));

/** Draws both strips in MS mode for the cursor `pt` and returns true. `K` carries the drawing helpers of mappa.js (canvasOf, M, ecss, fpx, XICDRAW, MAPMARGINI, st). @param {any} p @param {any} pt @param {any} K */
export function ms(p, pt, K) {
  const s = K.st(p), a = p._a, f = file(p), o = offset(p), rt = pt.rt + o.v, key = `${f.k}|${rt.toFixed(3)}`;
  s.shown = { pt, uv: null, ms: { k: f.k, rt, off: o.v } };
  const tr = p.msTrace || "tic", mz = tr === "xic" ? +p.msMz : NaN;
  const trace = tr === "xic" ? (isFinite(mz) && mz > 0 ? getXic(f.k, mz, XIC_TOL, XIC_LEVEL) : Promise.resolve(fail("mappa.ms.noMz"))) : getChrom(f.k, tr, 1);
  if (tr === "xic" && isFinite(mz) && mz > 0 && p._xicLog !== mz) { p._xicLog = mz; (p._msLog || (p._msLog = [])).push(T("ctl.map.ms.xic.log", { mz: mz.toFixed(2), tol: XIC_TOL, lv: XIC_LEVEL })); }
  s.msq = key;                                                              // a late answer for an older cursor is dropped
  Promise.all([scan(K, f, rt).catch(() => fail("mappa.ms.fail")), trace.catch(() => fail("mappa.ms.fail"))]).then(([sp, ch]) => { if (s.msq === key && p._a === a) { side(p, pt, K, f, o, rt, sp); bottom(p, pt, K, f, o, tr, ch); } });
  side(p, pt, K, f, o, rt, s.msSp && s.msSp.k === f.k ? s.msSp : null); bottom(p, pt, K, f, o, tr, s.msCh && s.msCh.key === f.k + tr ? s.msCh : null);
  return true;
}

/** The MS1 scan nearest to the MS time `rt` (the scan list of the file comes from mappa.js). */
async function scan(/** @type {any} */ K, /** @type {any} */ f, /** @type {number} */ rt) {
  const ax = await K.axisWait(f); if (!ax) return fail("mappa.ms.noAxis");
  return K.getScan(f, ax, K.MIRINO.nearest(ax.rt, rt));
}

function side(/** @type {any} */ p, /** @type {any} */ pt, /** @type {any} */ K, /** @type {any} */ f, /** @type {any} */ o, /** @type {number} */ rt, /** @type {any} */ sp) {
  const s = K.st(p), a = p._a; if (s.sp.hidden) return;
  s.errSp = sp && sp.err || null;                                           // the reason shown, for the tests
  if (sp && !sp.err) s.msSp = { ...sp, k: f.k };
  const { g, W, H } = K.canvasOf(s.sp), ink = K.ecss("--muted"), r = K.MAPMARGINI.rel(p.cv, s.sp), top = r.y + K.M.t, bot = r.y + p.cv.clientHeight - K.M.b;
  g.font = K.fpx(11); g.fillStyle = ink; g.strokeStyle = ink; g.beginPath(); g.moveTo(6.5, top); g.lineTo(6.5, bot); g.stroke();
  if (sp && sp.err) { g.fillText(T(sp.err, { file: fname(f) }), 10, Math.max(10, top + 10)); return; }
  if (!sp || !sp.mz || !sp.mz.length) { g.fillText(T(sp === null ? "mappa.ms.wait" : "mappa.ms.sideNone", { rt: f2(rt), file: fname(f) }), 10, Math.max(10, top + 10)); return; }
  const lo = sp.mz[0], hi = sp.mz[sp.mz.length - 1], span = hi - lo || 1, Y = (/** @type {number} */ v) => bot - (v - lo) / span * (bot - top), x1 = W - 8;
  let ymax = 0; for (let i = 0; i < sp.y.length; i++) if (sp.y[i] > ymax) ymax = sp.y[i];
  g.strokeStyle = f.color || K.ecss("--accent"); g.beginPath();
  for (let i = 0; i < sp.mz.length; i++) { const y = Math.round(Y(sp.mz[i])) + .5; g.moveTo(6, y); g.lineTo(6 + (x1 - 6) * sp.y[i] / (ymax || 1), y); }
  g.stroke(); g.fillStyle = ink; g.textAlign = "left";
  const top3 = [...sp.y.keys()].sort((x, y) => sp.y[y] - sp.y[x]).slice(0, 5);
  for (const i of top3) g.fillText(sp.mz[i].toFixed(2), Math.min(x1 - 28, 10 + (x1 - 6) * sp.y[i] / (ymax || 1)), Y(sp.mz[i]) + 4);
  g.fillText(T("mappa.ms.side", { file: fname(f), rt: f2(rt), dad: f2(pt.rt), off: sgn(o.v) }), 10, Math.min(H - 4, bot + 14));
  if (window.TDATI) TDATI.addButton(s.sp, () => ({ title: T("mappa.ms.side", { file: fname(f), rt: f2(rt), dad: f2(pt.rt), off: sgn(o.v) }), head: [T("mappa.ms.mz"), T("tdati.col.int")], rows: [...sp.mz.keys()].slice(0, 300).map(i => [TDATI.num(sp.mz[i], 4), TDATI.num(sp.y[i], 1)]) }));
}

function bottom(/** @type {any} */ p, /** @type {any} */ pt, /** @type {any} */ K, /** @type {any} */ f, /** @type {any} */ o, /** @type {string} */ tr, /** @type {any} */ ch) {
  const s = K.st(p), a = p._a; if (s.xc.hidden) return;
  s.errCh = ch && ch.err || null;
  if (ch && !ch.err && ch.rt) s.msCh = { key: f.k + tr, rt: ch.rt, y: ch.y };
  const { g, W, H } = K.canvasOf(s.xc), ink = K.ecss("--muted"), mt = 10, mb = 22, r = K.MAPMARGINI.rel(p.cv, s.xc), ml = r.x + K.M.l, mr = r.x + p.cv.clientWidth - K.M.r;
  const X = (/** @type {number} */ v) => ml + (v - a.x0) / (a.x1 - a.x0) * (mr - ml);
  g.font = K.fpx(11); g.fillStyle = ink; g.strokeStyle = ink; g.beginPath(); g.moveTo(ml, H - mb + .5); g.lineTo(mr, H - mb + .5); g.stroke();
  K.XICDRAW.axis(g, X, a.x0, a.x1, { base: H - mb, labelY: H - mb + 13, lo: 1, hi: W - 1, color: ink, font: K.fpx(11) });
  if (ch && ch.err) { g.textAlign = "left"; g.fillText(T(ch.err, { file: fname(f) }), ml + 4, mt + 8); return; }
  const d = s.msCh && s.msCh.key === f.k + tr ? s.msCh : null;
  if (!d) { g.textAlign = "left"; g.fillText(T("mappa.ms.wait"), ml + 4, mt + 8); return; }
  const t = Array.from(d.rt, (/** @type {number} */ v) => v - o.v);           // the MS time axis put on the DAD time axis of the map
  let ymax = 0; for (let i = 0; i < t.length; i++) if (t[i] >= a.x0 && t[i] <= a.x1 && d.y[i] > ymax) ymax = d.y[i];
  const Y = (/** @type {number} */ v) => H - mb - v / ((ymax * 1.08) || 1) * (H - mt - mb);
  g.strokeStyle = f.color || K.ecss("--accent"); g.beginPath();
  for (let i = 0; i < t.length; i++) { const x = X(t[i]); if (x < ml - 1 || x > mr + 1) continue; g.lineTo(x, Y(d.y[i])); if (i === 0) g.moveTo(x, Y(d.y[i])); }
  g.stroke();
  const x = Math.round(X(pt.rt)) + .5; g.strokeStyle = K.ecss("--ink"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(x, mt); g.lineTo(x, H - mb); g.stroke(); g.setLineDash([]);
  g.fillStyle = ink; g.textAlign = "left"; g.fillText(T("mappa.ms.bot", { kind: tr === "xic" ? `${T("ctl.map.ms.xic")} ${(+p.msMz).toFixed(2)} ± ${XIC_TOL}` : T(tr === "bpc" ? "ctl.map.ms.bpc" : "ctl.map.ms.tic"), file: fname(f), off: f2(o.v) }), ml + 4, mt + 8);
}

/** The options of the popover of a DAD map: what the strips show, the signed DAD→MS offset with its source and the evidence of the estimate. */
export function msControls(/** @type {any} */ p, /** @type {Function} */ num, /** @type {Function} */ opt) {
  const SRC = { default: "ctl.map.ms.src.default", user: "ctl.map.ms.src.user", estimated: "ctl.map.ms.src.estimated" }, ms = p.stripMs === "ms";
  const v = p.msOffset != null && isFinite(p.msOffset) ? +p.msOffset : 0, src = p.msOffset == null || !isFinite(p.msOffset) ? "default" : p._msUsed != null && +p.msOffset === p._msUsed ? "estimated" : "user", e = p._msEst, d = e && e.d;
  const bt = (/** @type {string} */ k, /** @type {string} */ lab, /** @type {string} */ tip, /** @type {boolean} */ off) => `<button class="bt" type="button" data-o="${k}" title="${tip}"${off ? " disabled" : ""}>${lab}</button>`;
  return `<label class="muted" title="${I18N.t("ctl.map.ms.mode.title")}">${I18N.t("ctl.map.ms.mode")} <select data-o="stripMs">${opt("uv", p.stripMs || "uv", I18N.t("ctl.map.ms.uv"))}${opt("ms", p.stripMs || "uv", I18N.t("ctl.map.ms.ms"))}</select></label>` + (!ms ? "" :
    `<label class="muted" title="${I18N.t("ctl.map.ms.off.title")}">${I18N.t("ctl.map.ms.off.lab", { input: num("msOffset", p.msOffset ?? "", -2, 2, 0.01, true) })}</label>` +
    `<select data-o="msTrace" title="${I18N.t("ctl.map.ms.trace.title")}">${opt("tic", p.msTrace || "tic", I18N.t("ctl.map.ms.tic"))}${opt("bpc", p.msTrace, I18N.t("ctl.map.ms.bpc"))}${opt("xic", p.msTrace, I18N.t("ctl.map.ms.xic"))}</select>` +
    (p.msTrace === "xic" ? `<input data-o="msMz" class="mzf" data-num="0:" value="${p.msMz ?? ""}" style="width:5em" title="${I18N.t("ctl.map.ms.mz.title")}" aria-label="m/z">` : "") +
    (p.ref || (window.SUBREF && SUBREF.eff(p)) ? `<select data-o="msFrom" title="${I18N.t("ctl.map.ms.from.title")}">${opt("a", p.msFrom || "a", I18N.t("ctl.map.ms.from.a"))}${opt("b", p.msFrom, I18N.t("ctl.map.ms.from.b"))}</select>` : "") +
    bt("msEst", I18N.t("ctl.map.ms.est"), I18N.t("ctl.map.ms.est.title"), false) + bt("msUse", I18N.t("ctl.map.ms.use"), I18N.t("ctl.map.ms.use.title"), !d) +
    `<span class="muted subnote" role="status">${I18N.t(SRC[/** @type {"default"} */ (src)], { v: v.toFixed(2) })}${e ? " · " + (e.busy ? I18N.t("ctl.map.ms.ev.busy") : d ? I18N.t("ctl.map.ms.ev", { v: d.delay.toFixed(2), n: d.peaks, c: d.corr.toFixed(2), file: (E.files[e.k] || {}).label || "" }) : I18N.t("ctl.map.ms.ev.none")) : ""}${(p._msLog || []).slice(-1).map((/** @type {string} */ l) => " · " + EH(l)).join("")}</span>`);
}

/** The Estimate / Use estimate buttons of the popover: the estimate shows its evidence, using it sets the offset (logged). @param {any} p @param {string} k @param {Function} ctl @param {Function} draw */
export async function act(p, k, ctl, draw) {
  if (k === "msUse") use(p); else { const q = estimate(p); ctl(p); await q; }
  ctl(p); draw(p);
}

// the buttons of the popover are plain markup: one listener finds the panel they belong to
document.addEventListener("click", e => { const b = /** @type {any} */ (e.target).closest && /** @type {any} */ (e.target).closest("[data-o=msEst],[data-o=msUse]"), p = b && E.panels.find((/** @type {any} */ q) => q.el && q.el.contains(b)); if (p) act(p, b.dataset.o, ctl, draw); });
