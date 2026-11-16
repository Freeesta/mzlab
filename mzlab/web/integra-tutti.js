// @ts-strict
// «Integrate every peak shown» (the ▾ next to the automatic tool, and the menu of a trace): every visible trace of the panel, every peak with its apex in
// the stretch shown, in one undoable step. Low resolution: the engine's lr_auto_integrate (peakq_lrauto.rs, the bytes of `mzlab-cli peak-auto-lr`), which
// finds the peaks on a smoothed copy, splits touching ones at the valley and integrates the raw trace above its baseline (ends never raised to zero, never above
// the trace); high resolution keeps autoAll (peakq_auto.rs). Every choice the engine made goes on each record (`auto`, areas in counts x s) and in the
// notice; the baseline is always the engine's line, never the dialog's baseline setting. Loaded by picchi.js (intAll).
/** @typedef {Record<string, any>} Bag */
const G = /** @type {any} */ (window);
const T = (/** @type {string} */ k, /** @type {Record<string, any>} */ v = {}) => I18N.t(k, v);

/** The engine numbers of the dialog: lr_half is half the window (0 = from the peaks). @param {Bag} pk */
export const engineParams = pk => ({ lrHalf: pk.lrSmooth ? (pk.lrSmooth - 1) / 2 : 0, lrValley: pk.lrValley, lrSn: pk.lrSn, lrSplit: pk.lrSplit });

/** One record per peak of the engine's answer `r` ({ params, peaks }) on the trace s; the baseline is the engine's line (on SIM/MRM panels two manual anchors on it).
 * @param {Bag} p @param {Bag} s @param {Bag} r */
function records(p, s, r) {
  const mrm = ESP.mrm(p), q = r.params;
  return r.peaks.map((/** @type {Bag} */ k) => {
    const it = intRec(p, s, k.t0, k.t1);
    if (mrm) { it.snap = { a: "man", b: "man" }; it.my = { a: k.b0, b: k.b1 }; snapBase(s, it); }
    else { delete it.zero; delete it.base; it.bm = "line"; it.bl = [k.t0, k.b0, k.t1, k.b1]; }
    it.auto = { half: q.half, half_auto: q.half_auto, valley: q.valley, sn_min: q.sn, split: q.split, baseline: q.baseline, noise: q.noise, sn: k.sn, end0: k.end0, end1: k.end1,
      b0: k.b0, b1: k.b1, clamp0: k.clamp0, clamp1: k.clamp1, area: k.area * 60, below: k.below * 60, area_zero: k.area_zero * 60 };       // the engine's areas, counts x min -> counts x s
    return it;
  });
}

/** Integrates every peak of the visible traces of p (or of the one chosen in the panel). @param {Bag} p */
export async function run(p) {
  if (!guardInt(p)) return;
  const a = p._a; if (!a || !a.sr) return;
  if (!G.QUALITA) { nearMsg(T("int.autoOff")); return; }
  const shown = p.intf && p.intf !== "all" ? a.sr.filter((/** @type {Bag} */ s) => s.key === p.intf) : a.sr.filter((/** @type {Bag} */ s) => !isHid(p, s));
  const lr = shown.filter((/** @type {Bag} */ s) => !hrTrace(s)), x0 = a.x0, x1 = a.x1, ep = engineParams(PK);
  for (const s of shown) if (hrTrace(s)) await autoAll(p, s);
  if (!lr.length) return;
  /** @type {Bag[]} */ const add = [];
  let skip = 0, last = null;
  for (const s of lr) {
    const txt = await QUALITA.lrAuto(s.x, s.y, x0, x1, ep);
    if (txt == null) { nearMsg(T("int.autoOff")); return; }
    const r = JSON.parse(txt); last = r.params;
    records(p, s, r).forEach(it => { if (intClash(p, it.key, iLo(it), iHi(it))) skip++; else add.push(it); });
  }
  if (!add.length) { nearMsg(T(skip ? "int.clash" : "int.autoNone")); return; }
  pushIh(p);
  p.ints.push(...add);
  draw(p); uiSave();
  const n = (/** @type {number} */ v) => I18N.sciE(v), au = add.map(it => it.auto);
  const clamped = au.reduce((c, q) => c + (q.clamp0 !== "none" ? 1 : 0) + (q.clamp1 !== "none" ? 1 : 0), 0), below = au.reduce((c, q) => c + q.below, 0);
  nearMsg(T("int.lrAll.done", { n: add.length, w: 2 * last.half + 1, a: last.half_auto ? T("int.lrAll.auto") : "", v: Math.round(last.valley * 100), sn: last.sn, noise: n(last.noise),
    split: T(last.split === "valley" ? "pk.split.1" : "pk.split.0"), c: clamped, e: 2 * add.length, below: n(below) }) + (skip ? ". " + T("int.lrAll.skip", { n: skip }) : ""));
}

G.INTTUTTI = { run, engineParams };
