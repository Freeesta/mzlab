// @ts-strict
// UV spectrum of the DAD / PDA (absorbance against wavelength), opened by a double click on the PDA map (mappa.js) or on the PDA trace (eventi.js).
// A "spec" panel with `uv: true`: the matrix of each file (api/map?kind=pda, read once and held in MAT) is sliced in memory at r0..r1, one line per
// PDA sample shown in the source chromatogram; /api/uvspec only when a reader gives the spectrum without the matrix. Same axes machinery as the other graphs.
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);
/** @type {Map<string, Promise<any>>} */
const memo = new Map();

/** @param {number} k @param {number} r0 @param {number} r1 */
function load(k, r0, r1) {
  const key = `${k}|${r0}|${r1}`;
  if (!memo.has(key)) {
    const q = fetch(`api/uvspec?k=${k}&rt0=${r0}&rt1=${r1}`).then(async r => { const j = await r.json(); if (j.error) throw j; return j; });
    q.catch(() => memo.delete(key));
    memo.set(key, q);
  }
  return memo.get(key);
}
/** PDA matrices already resolved per file: once present, any (r0, r1) is sliced in memory in the same frame (no request while a line is dragged). @type {Record<number, any>} */
const MAT = {};
/**
 * Mean UV spectrum of the rows with r0 <= t <= r1 of a PDA matrix (the nearest row when none falls inside); null for an empty matrix.
 * @param {any} A the object of getMap(k, "pda", "") @param {number} r0 @param {number} r1
 */
export function sliceUv(A, r0, r1) {
  if (!(A && A.nrt > 0 && A.nmz > 0 && A.m && A.m.length >= A.nrt * A.nmz)) return null;
  if (!A._nm) A._nm = Array.from({ length: A.nmz }, (_, j) => +(A.mz0 + j * A.dmz).toFixed(2));
  const lo = Math.min(r0, r1), hi = Math.max(r0, r1), t = (/** @type {number} */ i) => (A.nrt > 1 ? A.rt0 + i * (A.rt1 - A.rt0) / (A.nrt - 1) : A.rt0);
  let i0 = -1, i1 = -1;
  for (let i = 0; i < A.nrt; i++) if (t(i) >= lo && t(i) <= hi) { if (i0 < 0) i0 = i; i1 = i; }
  if (i0 < 0) { let b = 0; for (let i = 1; i < A.nrt; i++) if (Math.abs(t(i) - (lo + hi) / 2) < Math.abs(t(b) - (lo + hi) / 2)) b = i; i0 = i1 = b; }
  const n = i1 - i0 + 1, mau = new Array(A.nmz).fill(0);
  for (let i = i0; i <= i1; i++) for (let j = 0; j < A.nmz; j++) mau[j] += A.m[i * A.nmz + j];
  for (let j = 0; j < A.nmz; j++) mau[j] /= n;
  return { nm: A._nm, mau, rt0: t(i0), rt1: t(i1), n };
}
/** Loads the matrix of file k into MAT once; false when the file has none (a lone 1D trace, a non-wiff file). @param {number} k */
async function ensureMat(k) {
  if (MAT[k]) return true;
  try { const A = await getMap(k, "pda", ""); if (A && A.nrt > 0 && A.nmz > 0) { MAT[k] = A; return true; } } catch (e) { /* the uvspec request below gives the structured error */ }
  return false;
}
/** @param {any} j */
const reason = j => (j && j.error_key ? I18N.t(j.error_key, j.params || {}) : T("uv.error"));

/**
 * A new UV spectrum under the panel `from`: the mean spectrum of the times r0..r1 (minutes of the DAD) of file `k`; a message and no panel when the file has no matrix.
 * @param {any} from @param {number} k @param {number} r0 @param {number} r1
 */
export async function open(from, k, r0, r1) {
  // the held matrix answers at once; without it one request reads it (the engine keeps it), and api/uvspec only words a file that has none
  const slow = MAT[k] || !from || typeof stepMsg !== "function" ? null : setTimeout(() => { stepMsg(from, T("uv.reading"), false, true); from.el.setAttribute("aria-busy", "true"); }, 150);
  const has = await ensureMat(k);
  if (slow) { clearTimeout(slow); if (from.el.getAttribute("aria-busy")) { stepMsg(from, "", false, false); from.el.removeAttribute("aria-busy"); } }
  if (!has) try { await load(k, r0, r1); } catch (e) { if (window.toast) toast(/** @type {any} */ (e)?.error_key === "err.pda.none" ? T("uv.none") : reason(e)); return null; }
  const link = from && ["chrom", "xic", "mrm"].includes(from.type) ? from.id : null;
  return newSpec(from, r0, r1, k, link, { uv: true, title: "@uv", ks: pdaKs(from, k), wl: from && from.kind === "pda" && from.mz0 != null ? [from.mz0, from.mz1 ?? from.mz0] : null });
}
/** The PDA files the panel `src` shows (its overlaid samples), `k` first; just [k] for a map or a lone file. @param {any} src @param {number} k @returns {number[]} */
function pdaKs(src, k) {
  const ks = src && src.type === "chrom" && typeof shownFor === "function" ? shownFor(src).filter((/** @type {any} */ f) => f && f.pda && !f.gone).map((/** @type {any} */ f) => f.k) : [];
  return [k, ...ks.filter((/** @type {number} */ x) => x !== k)];
}
/** The files of the UV panel `p`: while live, the PDA samples its chromatogram shows now; frozen, the ones it had. @param {any} p @returns {number[]} */
function filesOf(p) {
  const src = p.link != null ? E.panels.find((/** @type {any} */ q) => q.id === p.link) : null;
  if (src) p.ks = pdaKs(src, p.k);
  return (p.ks && p.ks.length ? p.ks : [p.k]).filter((/** @type {number} */ k) => E.files[k]);
}

/**
 * A new PDA trace at the wavelengths nm0..nm1 (the mean of the DAD channels inside) under panel `from`; a message and no panel when the file has no matrix.
 * @param {any} from @param {number} nm0 @param {number} nm1 @param {number} [k]
 */
export async function openWl(from, nm0, nm1, k) {
  const fk = k ?? E.files.findIndex(f => f.pda), q = `api/chrom?k=${Math.max(0, fk)}&kind=pda&level=1&mz0=${nm0}&mz1=${nm1}`;
  try { const j = await (await fetch(q)).json(); if (j.error) throw j; } catch (e) { if (window.toast) toast(/** @type {any} */ (e)?.error_key === "err.pda.none" ? T("uv.none") : reason(e)); return null; }
  const np = apriAnnullabile(addPanel("chrom", { kind: "pda", mz0: nm0, mz1: nm1 }));
  if (from && E.panels.includes(from) && from.type === "chrom") {
    np.el.classList.add("drag"); np.y = from.y + from.h; np.el.style.top = np.y + "px"; void np.el.offsetHeight; np.el.classList.remove("drag");
    stackAfter(np, from); relayout(); fitHost();
  } else reveal(np);
  return np;
}

/** @param {any} p */
export async function draw(p) {
  const tok = p._tok = (p._tok || 0) + 1;
  const { g, W, H } = setup(p.cv);
  /** @param {string} t */
  const say = t => { g.clearRect(0, 0, W, H); g.fillStyle = ecss("--muted"); g.fillText(t, M.l, 30); p.leg.innerHTML = ""; if (p.leg2) p.leg2.innerHTML = ""; p._a = null; };
  const f = E.files[p.k];
  if (!f || p.r0 == null) return say(T("spec.pickRange"));
  /** @param {number} k @returns {Promise<any>} the slice of file k at r0..r1, or the answer of api/uvspec, or the error */
  const one = async k => {
    if (MAT[k] || await ensureMat(k)) return sliceUv(MAT[k], p.r0, p.r1);
    try { return await load(k, p.r0, p.r1); } catch (e) { return { err: e }; }
  };
  const ks = filesOf(p), got = await Promise.all(ks.map(one));
  if (tok !== p._tok) return false;
  /** @type {{ f: any, nm: number[], y: number[], d: any }[]} */
  const ser = []; got.forEach((d, i) => { if (d && !d.err && d.nm && d.nm.length) ser.push({ f: E.files[ks[i]], nm: d.nm, y: d.mau, d }); });
  if (!ser.length) { const e = got[0] && got[0].err; return say(e && e.error_key !== "err.pda.none" ? reason(e) : T("uv.none")); }
  const d = ser[0].d, nm = ser[0].nm, y = ser[0].y;
  const full = [Math.min(...ser.map(s => s.nm[0])), Math.max(...ser.map(s => s.nm[s.nm.length - 1]))], x0 = p.zoom ? p.zoom[0] : full[0], x1 = p.zoom ? p.zoom[1] : full[1];
  let lo = 0, hi = 0; for (const s of ser) for (const v of s.y) { if (v < lo) lo = v; if (v > hi) hi = v; }
  const pad = (hi - lo || 1) * (window.ETPK && ETPK.on(p, 1) ? 0.16 : 0.06), yfull = [lo < 0 ? lo - pad : 0, hi + pad];
  if (p.lock && p.lock.pending) { p.lock.y = yfull[1]; p.lock.pending = false; }
  const y0 = p.zoomY ? p.zoomY[0] : yfull[0], y1 = p.zoomY ? p.zoomY[1] : p.lock && p.lock.y != null ? p.lock.y : yfull[1];
  const pw = W - M.l - M.r, ph = H - M.t - M.b, X = (/** @type {number} */ v) => M.l + (v - x0) / (x1 - x0) * pw, Y = (/** @type {number} */ v) => H - M.b - (v - y0) / (y1 - y0) * ph;
  g.clearRect(0, 0, W, H);
  frame(g, W, H, nice(x0, x1, 8).map(t => [X(t), String(+t.toFixed(2))]), nice(y0, y1, 6).map(t => [Y(t), String(+t.toFixed(2))]), T("uv.axis.x"), T("uv.axis.y"));
  g.save(); g.beginPath(); g.rect(M.l, M.t, pw, ph); g.clip();
  if (p.wl) {                                        // the wavelength of the PDA trace the spectrum comes from (a band, at least 2 px)
    const a = X(Math.min(p.wl[0], p.wl[1])), b = X(Math.max(p.wl[0], p.wl[1]));
    g.globalAlpha = 0.18; g.fillStyle = ecss("--accent"); g.fillRect(Math.min(a, (a + b) / 2 - 1), M.t, Math.max(2, b - a), ph); g.globalAlpha = 1;
  }
  for (const s of ser.slice().reverse()) {          // the sample of the click on top
    g.strokeStyle = s.f.color || ecss("--ink"); g.lineWidth = s === ser[0] ? 1.6 : 1.2; g.beginPath();
    let pen = false; s.nm.forEach((w, i) => { if (w < x0 - 1 || w > x1 + 1) return; const px = X(w), py = Y(s.y[i]); if (pen) g.lineTo(px, py); else { g.moveTo(px, py); pen = true; } });
    g.stroke();
  }
  g.restore();
  const near = (/** @type {number[]} */ a, /** @type {number} */ w) => Math.max(0, Math.min(a.length - 1, Math.round((w - a[0]) / ((a[a.length - 1] - a[0]) / Math.max(1, a.length - 1)))));
  const rt = d.n > 1 ? `${d.rt0.toFixed(2)}–${d.rt1.toFixed(2)}` : d.rt0.toFixed(2);
  p._a = { x0, x1, y0, y1, ymin: y0, ymax: y1, yfull, full, W, H, X, Y, f, uvd: { nm, y, rt: d.rt0, rt1: d.rt1, n: d.n, ser: ser.map(s => ({ k: s.f.k, label: s.f.label, nm: s.nm, y: s.y, rt: s.d.rt0 })) },
    yinv: (/** @type {number} */ py) => y1 - (py - M.t) / ph * (y1 - y0),
    hov: (/** @type {number} */ px) => { const w = x0 + (px - M.l) / pw * (x1 - x0), i = near(nm, w);
      const rows = ser.length > 1 ? ser.map(s => `<div><i style="background:${s.f.color}"></i>${EH(s.f.label)} <b>${T("hov.mau", { v: s.y[near(s.nm, w)].toFixed(2) })}</b></div>`).join("") : `<div>${T("map.hov.mean")}: <b>${T("hov.mau", { v: y[i].toFixed(2) })}</b></div>`;
      return { px: X(nm[i]), html: `<b>${T("hov.nm", { v: nm[i].toFixed(0) })}</b>${rows}` }; } };
  if (window.ETPK) ETPK.drawUv(p, g, { nm, y, X, Y, x0, x1, W });       // wavelength of the absorbance maxima of the first sample (etichette-picchi.js)
  p.leg.innerHTML = ser.map(s => h`<span><i style="background:${s.f.color}"></i>${s.f.label}</span>`).join(" ") + h` · <span>${T(d.n > 1 ? "uv.leg.mean" : "uv.leg.one", { rt, n: d.n })}</span>`;
  if (p.leg2) p.leg2.innerHTML = "";
  p.r0 = d.rt0; p.r1 = d.rt1;
}
