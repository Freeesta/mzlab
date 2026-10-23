// @ts-check
// UV spectrum of the DAD / PDA (absorbance against wavelength), opened by a double click on the PDA map (mappa.js) or on the PDA trace (eventi.js).
// A "spec" panel with `uv: true`: the data come from /api/uvspec (mean of the optical rows between r0 and r1, or the nearest row), drawn here as a line
// with the same axes machinery (zoom, hover, export, table of the data) as the other graphs. Only a .wiff with the detector has the matrix.
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
/** @param {any} j */
const reason = j => (j && j.error_key ? I18N.t(j.error_key, j.params || {}) : T("uv.error"));

/**
 * A new UV spectrum under the panel `from`: the mean spectrum of the times r0..r1 (minutes of the DAD) of file `k`; a message and no panel when the file has no matrix.
 * @param {any} from @param {number} k @param {number} r0 @param {number} r1
 */
export async function open(from, k, r0, r1) {
  try { await load(k, r0, r1); } catch (e) { if (window.toast) toast(/** @type {any} */ (e)?.error_key === "err.pda.none" ? T("uv.none") : reason(e)); return null; }
  return newSpec(from, r0, r1, k, null, { uv: true, title: "@uv" });
}

/**
 * A new PDA trace at the wavelengths nm0..nm1 (the mean of the DAD channels inside) under panel `from`; a message and no panel when the file has no matrix.
 * @param {any} from @param {number} nm0 @param {number} nm1 @param {number} [k]
 */
export async function openWl(from, nm0, nm1, k) {
  const fk = k ?? E.files.findIndex(f => f.pda), q = `api/chrom?k=${Math.max(0, fk)}&kind=pda&level=1&mz0=${nm0}&mz1=${nm1}`;
  try { const j = await (await fetch(q)).json(); if (j.error) throw j; } catch (e) { if (window.toast) toast(/** @type {any} */ (e)?.error_key === "err.pda.none" ? T("uv.none") : reason(e)); return null; }
  const np = addPanel("chrom", { kind: "pda", mz0: nm0, mz1: nm1 });
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
  /** @type {any} */ let d;
  try { d = await load(p.k, p.r0, p.r1); } catch (e) { return say(/** @type {any} */ (e)?.error_key === "err.pda.none" ? T("uv.none") : reason(e)); }
  if (tok !== p._tok) return false;
  /** @type {number[]} */ const nm = d.nm, y = d.mau;
  if (!nm.length) return say(T("uv.none"));
  const full = [nm[0], nm[nm.length - 1]], x0 = p.zoom ? p.zoom[0] : full[0], x1 = p.zoom ? p.zoom[1] : full[1];
  let lo = 0, hi = 0; for (const v of y) { if (v < lo) lo = v; if (v > hi) hi = v; }
  const pad = (hi - lo || 1) * 0.06, yfull = [lo < 0 ? lo - pad : 0, hi + pad], y0 = p.zoomY ? p.zoomY[0] : yfull[0], y1 = p.zoomY ? p.zoomY[1] : yfull[1];
  const pw = W - M.l - M.r, ph = H - M.t - M.b, X = (/** @type {number} */ v) => M.l + (v - x0) / (x1 - x0) * pw, Y = (/** @type {number} */ v) => H - M.b - (v - y0) / (y1 - y0) * ph;
  g.clearRect(0, 0, W, H);
  frame(g, W, H, nice(x0, x1, 8).map(t => [X(t), String(+t.toFixed(2))]), nice(y0, y1, 6).map(t => [Y(t), String(+t.toFixed(2))]), T("uv.axis.x"), T("uv.axis.y"));
  g.save(); g.beginPath(); g.rect(M.l, M.t, pw, ph); g.clip();
  g.strokeStyle = f.color || ecss("--ink"); g.lineWidth = 1.6; g.beginPath();
  let pen = false; nm.forEach((w, i) => { if (w < x0 - 1 || w > x1 + 1) return; const px = X(w), py = Y(y[i]); if (pen) g.lineTo(px, py); else { g.moveTo(px, py); pen = true; } });
  g.stroke(); g.restore();
  const near = (/** @type {number} */ w) => Math.max(0, Math.min(nm.length - 1, Math.round((w - nm[0]) / ((nm[nm.length - 1] - nm[0]) / Math.max(1, nm.length - 1)))));
  const rt = d.n > 1 ? `${d.rt0.toFixed(2)}–${d.rt1.toFixed(2)}` : d.rt0.toFixed(2);
  p._a = { x0, x1, y0, y1, ymin: y0, ymax: y1, yfull, full, W, H, X, Y, f, uvd: { nm, y, rt: d.rt0, rt1: d.rt1, n: d.n },
    yinv: (/** @type {number} */ py) => y1 - (py - M.t) / ph * (y1 - y0),
    hov: (/** @type {number} */ px) => { const w = x0 + (px - M.l) / pw * (x1 - x0), i = near(w); return { px: X(nm[i]), html: `<b>${nm[i].toFixed(0)} nm</b><div>${y[i].toFixed(2)} mAU</div>` }; } };
  p.leg.innerHTML = h`<span><i style="background:${f.color}"></i>${f.label}</span><span>${T(d.n > 1 ? "uv.leg.mean" : "uv.leg.one", { rt, n: d.n })}</span>`;
  if (p.leg2) p.leg2.innerHTML = "";
  p.r0 = d.rt0; p.r1 = d.rt1;
}
