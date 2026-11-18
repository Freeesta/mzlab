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
    return { sub: null, err: T("map.sub.fail", { file: EH(rf.label), why: EH(String(e && e.message || e)) }), rf: null };
  }
}

/** What the subtraction did, in words: the kept ranges, the scale, the shift and one line per warning code of the engine. @param {any} S @param {string} a @param {string} b */
export function note(S, a, b) {
  const r = S.sub, nm1 = S.mz0 + S.nmz * S.dmz;
  const sc = r.scale === "none" ? T("map.sub.scale.none") : T("map.sub.scale.used", { name: T(`map.sub.scale.${r.scale}`), f: +(+r.factor).toPrecision(4) });
  const head = T("map.sub.note", { a, b, rt0: f2(S.rt0), rt1: f2(S.rt1), nm0: f2(S.mz0), nm1: f2(nm1), rows: r.rows[0], rowsA: r.rows[1], cols: r.cols[0], colsA: r.cols[1], scale: sc, shift: f2(r.shift_min) });
  const warn = r.warnings.map((/** @type {string} */ c) => T(`map.sub.w.${c}`, { a, b, sa: g3(r.rt_step_a), sb: g3(r.rt_step_b), na: g3(r.nm_step_a), nb: g3(r.nm_step_b) }));
  return { head, warn };
}

/** Writes the note (summary and legend) or the reason of a failure on the panel. @param {any} p @param {any} S @param {string} err @param {any} f @param {any} rf */
export function show(p, S, err, f, rf) {
  if (err) p.leg.insertAdjacentHTML("beforeend", `<span class="sm subnote" role="status">${err}</span>`);
  if (!S) return;
  const n = note(S, f.label, rf.label), full = [n.head, ...n.warn];
  (p.corrbar || p.leg).insertAdjacentHTML("beforeend", `<div class="sm subnote" role="status">${full.map(EH).join("<br>")}</div>`);
  p.leg.insertAdjacentHTML("beforeend", `<span class="sm subnote" title="${EH(full.join(" "))}">${EH(T("map.sub.short", { rows: S.sub.rows[0], rowsA: S.sub.rows[1], cols: S.sub.cols[0], colsA: S.sub.cols[1], n: n.warn.length }))}</span>`);
}
