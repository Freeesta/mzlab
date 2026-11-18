// @ts-strict
// DAD map minus the DAD map of another file (the «Correction» file of the RT-λ map): asks the engine (`/api/pdasub`, `dad_subtract`: the reference is resampled onto the
// grid of the first file) and writes what it did on the panel. Interface only: every number comes from the engine. Loaded on first use by grafici.js `drawMap`.
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);
const f2 = (/** @type {number} */ v) => +(+v).toFixed(2), g3 = (/** @type {number} */ v) => +(+v).toPrecision(3);

/** The scale asked by the panel: a typed factor, else the normalization menu (abs = none, the default). @param {any} p */
const scaleOf = p => p.subFactor != null && isFinite(p.subFactor) ? String(p.subFactor) : ({ max: "max", tic: "tic", fit: "fit" }[/** @type {"max"|"tic"|"fit"} */ (p.norm)] || "none");

/** File k minus the DAD of `ref`: {map JSON, m: matrix, sub: report}, or an error text. Kept per request (memo of explore.js). @param {any} p @param {any} f @param {any} rf */
export async function sub(p, f, rf) {
  const sc = scaleOf(p), sh = p.subShift || 0;
  try {
    const j = await memo(`ps${f.k}|${rf.k}|${sc}|${sh}`, () => J(`api/pdasub?k=${f.k}&ref=${rf.k}&view=map&scale=${encodeURIComponent(sc)}&shift=${sh}`).then((/** @type {any} */ j) => ({ ...j, m: b64F32(j.data, Number.isInteger(j.nrt) && Number.isInteger(j.nmz) ? j.nrt * j.nmz : null) })));
    return { sub: j, err: "", rf };
  } catch (e_) {
    const e = /** @type {any} */ (e_);
    return { sub: null, err: T("map.sub.fail", { file: rf.label, why: String(e && e.message || e) }), rf: null };
  }
}

// literal keys (the i18n check finds a key by its text)
const SCALE = { max: "map.sub.scale.max", tic: "map.sub.scale.tic", fit: "map.sub.scale.fit", factor: "map.sub.scale.factor" };
const WARN = { rt_range_partial: "map.sub.w.rtRange", nm_range_partial: "map.sub.w.nmRange", rt_interpolated: "map.sub.w.rtInterp", nm_interpolated: "map.sub.w.nmInterp",
  rt_step_differs: "map.sub.w.rtStep", nm_step_differs: "map.sub.w.nmStep", shift_applied: "map.sub.w.shift", scale_applied: "map.sub.w.scaled" };

/** What the subtraction did, in words: the kept ranges, the scale, the shift and one line per warning code of the engine. @param {any} S @param {string} a @param {string} b */
export function note(S, a, b) {
  const r = S.sub, nm1 = S.mz0 + S.nmz * S.dmz;
  const sc = r.scale === "none" ? T("map.sub.scale.none") : T("map.sub.scale.used", { name: T(/** @type {any} */ (SCALE)[r.scale]), f: +(+r.factor).toPrecision(4) });
  const head = T("map.sub.note", { a, b, rt0: f2(S.rt0), rt1: f2(S.rt1), nm0: f2(S.mz0), nm1: f2(nm1), rows: r.rows[0], rowsA: r.rows[1], cols: r.cols[0], colsA: r.cols[1], scale: sc, shift: f2(r.shift_min) });
  const warn = r.warnings.map((/** @type {string} */ c) => T(/** @type {any} */ (WARN)[c], { a, b, sa: g3(r.rt_step_a), sb: g3(r.rt_step_b), na: g3(r.nm_step_a), nb: g3(r.nm_step_b) }));
  return { head, warn };
}

/** Writes the note (summary and legend) or the reason of a failure on the panel. @param {any} p @param {any} S @param {string} err @param {any} f @param {any} rf */
export function show(p, S, err, f, rf) {
  if (err) p.leg.insertAdjacentHTML("beforeend", h`<span class="sm subnote" role="status">${err}</span>`);
  if (!S) return;
  const n = note(S, f.label, rf.label), full = [n.head, ...n.warn];
  (p.corrbar || p.leg).insertAdjacentHTML("beforeend", h`<div class="sm subnote" role="status">${raw(full.map(t => h`${t}`).join("<br>"))}</div>`);
  p.leg.insertAdjacentHTML("beforeend", h`<span class="sm subnote" title="${full.join(" ")}">${n.head}${n.warn.length ? ` (${n.warn.length})` : ""}</span>`);
}

/**
 * The matrix of the DAD map (plain, or minus the reference of the 3D viewer) cut to the window of the viewer, in the shape the 3D grid uses (nrt, nmz, mz0, dmz, row-major
 * intensities). The window is snapped to the bins; the time axis is the one of the map (uniform). @param {any} V the state of the viewer @param {any} p the map panel
 */
export async function grid(V, p) {
  let a;
  if (V.ref != null) { const r = await sub(p, { k: V.k }, { k: V.ref, label: "" }); if (!r.sub) throw new Error(r.err); a = r.sub; } else a = await getMap(V.k, "pda", "");
  const [r0, r1] = V.rt, [w0, w1] = V.mz, dr = (a.rt1 - a.rt0) / a.nrt, cl = (/** @type {number} */ v, /** @type {number} */ lo, /** @type {number} */ hi) => Math.max(lo, Math.min(hi, v));
  const i0 = cl(Math.floor((r0 - a.rt0) / dr), 0, a.nrt - 1), i1 = cl(Math.ceil((r1 - a.rt0) / dr), i0 + 1, a.nrt), j0 = cl(Math.floor((w0 - a.mz0) / a.dmz), 0, a.nmz - 1), j1 = cl(Math.ceil((w1 - a.mz0) / a.dmz), j0 + 1, a.nmz);
  const m = new Float32Array((i1 - i0) * (j1 - j0));
  for (let i = i0; i < i1; i++) m.set(a.m.subarray(i * a.nmz + j0, i * a.nmz + j1), (i - i0) * (j1 - j0));
  V.rt = [a.rt0 + i0 * dr, a.rt0 + i1 * dr]; V.mz = [a.mz0 + j0 * a.dmz, a.mz0 + j1 * a.dmz];
  return { nrt: i1 - i0, nmz: j1 - j0, mz0: V.mz[0], dmz: a.dmz, log: false, m };
}
