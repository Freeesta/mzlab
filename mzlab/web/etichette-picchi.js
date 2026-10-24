// @ts-check
// Labels of the main peaks of a chromatogram, as in Xcalibur: the RT of the apex and the quantity that defines the peak ("14.26 · 301.14" = base peak m/z of
// the apex scan, "14.26 · 254 nm" = absorbance maximum of the DAD at the apex; XIC and MRM: the extracted ion). Which peaks and which maximum is worked out by
// the Rust engine (QUALITA.labels / QUALITA.mainPeak, qualita.js); this file only asks at rest, caches, places the labels and draws them (grafici.js drawLines).
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);
const MAXC = 12, REL = 0.05, SNR = 5, GAP = 4;           // candidates asked of the engine, relative height, S/N, pixels between two labels
/** @type {Map<string, { list: { apex: number, rt: number, height: number, snr: number, q: { kind: string, v: string } | null }[] } | "wait">} */
const cache = new Map();

/** Labels are on by default for one visible trace; a student's choice (p.etpk) wins. @param {any} p @param {number} n visible traces */
const on = (p, n) => !isAnalog(p.kind) && (typeof p.etpk === "boolean" ? p.etpk : n <= 1);
const mzText = (/** @type {number} */ v) => v.toFixed(Math.abs(v * 100 - Math.round(v * 100)) < 1e-6 ? 2 : 4);

/** The defining quantity of the peak at `rt`, or null (RT only). @param {any} p @param {any} s @param {number} rt */
async function quantity(p, s, rt) {
  if (p.type === "xic" && s.mz != null) return { kind: "xic", v: mzText(+s.mz) };
  if (!ESP.spettri(p)) return s.ion ? { kind: "mrm", v: String(s.ion) } : null;
  if (p.type !== "chrom") return null;
  if (p.kind === "pda") {
    if (p.mz0 != null && p.mz1 != null) return { kind: "nm", v: String(Math.round((p.mz0 + p.mz1) / 2)) };
    const t = rt - (window.DISP ? DISP.pdaShift(p) : 0), d = await (await fetch(`api/uvspec?k=${s.k}&rt0=${t}&rt1=${t}`)).json();
    const m = d && !d.error && d.nm ? await QUALITA.mainPeak(d.nm, d.mau, true) : null;
    return m ? { kind: "nm", v: String(Math.round(m.x)) } : null;
  }
  const f = E.files[s.k]; if (!f || !scanFiles([f]).length) return null;
  const lv = p.flv ?? f.lv, n = await nearScan(s.k, rt), dt = scanStep();
  const d = await getSpec(s.k, n ? n.rt - NEAR : rt - dt / 2, n ? n.rt + NEAR : rt + dt / 2, lv, prk(lv > 1 ? p.prec : null, p.filt), null);
  if (!d || !d.mz || !d.mz.length) return null;
  const lo = p.mz0 ?? -Infinity, hi = p.mz1 ?? Infinity, ix = d.mz.map((/** @type {number} */ _, /** @type {number} */ i) => i).filter((/** @type {number} */ i) => d.mz[i] >= lo && d.mz[i] <= hi);
  const m = await QUALITA.mainPeak(ix.map((/** @type {number} */ i) => d.mz[i]), ix.map((/** @type {number} */ i) => d.y[i]), false);
  return m ? { kind: "mz", v: mzText(m.x) } : null;
}

/** @param {any} s */
const keyOf = (p, s) => { let sum = 0; for (let i = 0; i < s.y.length; i++) sum += s.y[i]; return [p.type, p.kind, s.key, s.x.length, +sum.toFixed(1), p.flv, p.mz0, p.mz1, p.prec, p.filt, p.pdaAlign ? DISP.pdaShift(p) : 0].join("|"); };

/** The labels of a trace, or null while the engine is still working (the panel is drawn again when they are ready). @param {any} p @param {any} s */
function labelsOf(p, s) {
  const key = keyOf(p, s), c = cache.get(key);
  if (c && c !== "wait") return c.list;
  if (c === "wait") return null;
  if (cache.size > 200) cache.clear();
  cache.set(key, "wait");
  (async () => {
    let list = [];
    try {
      const found = (window.QUALITA ? await QUALITA.labels(s.x, s.y, MAXC, REL, SNR) : null) || [];
      list = await Promise.all(found.map(async l => ({ ...l, rt: s.x[l.apex], q: await quantity(p, s, s.x[l.apex]).catch(() => null) })));
    } catch (e) { list = []; }
    cache.set(key, { list });
    if (window.drawSoon) drawSoon(p);
  })();
  return null;
}

/** Text of the label and of its tooltip. @param {any} l */
function texts(l) {
  const rt = l.rt.toFixed(2), q = l.q;
  if (!q) return { label: rt, tip: T("etpk.tip.rt", { rt }) };
  const v = q.kind === "nm" ? T("etpk.nm", { v: q.v }) : q.v;
  const tip = { mz: "etpk.tip.mz", nm: "etpk.tip.nm", xic: "etpk.tip.xic", mrm: "etpk.tip.mrm" }[/** @type {"mz" | "nm" | "xic" | "mrm"} */ (q.kind)];
  return { label: `${rt} · ${v}`, tip: T(tip, { rt, v: q.v }) };
}

/**
 * Draws the labels of the main peaks of the visible traces of panel `p` (called by drawLines, inside the export too). They are placed above the apex, never
 * over another label (the area labels of the integrations included) and follow the zoom: the engine's candidates inside the view, tallest first.
 * @param {any} p @param {CanvasRenderingContext2D} g @param {any[]} sr @param {{ X: (v: number) => number, Y: (v: number) => number, U: (s: any, v: number) => number, x0: number, x1: number, W: number }} c
 */
function draw(p, g, sr, c) {
  if (!on(p, sr.length) || !sr.length || p.mode === "cas") return;
  const a = p._a, lbls = a.lbls = a.lbls || [], pw = c.W - M.l - M.r, room = Math.max(1, Math.min(8, Math.floor(pw / 120)));
  g.save(); g.font = fpx(11, "bold "); g.textAlign = "center"; g.lineJoin = "round"; g.lineWidth = 3;
  const h = 11 * fz() + 4;
  for (const s of sr.length > 1 ? [sr[0]] : sr) {
    const list = labelsOf(p, s); if (!list) continue;
    const seen = list.filter(l => l.rt >= c.x0 && l.rt <= c.x1).sort((u, v) => v.height - u.height).slice(0, room);
    for (const l of seen) {
      const { label, tip } = texts(l), w = g.measureText(label).width + 6, ys = s.ys[l.apex];
      if (ys == null) continue;
      const x = Math.max(M.l + w / 2 + 2, Math.min(c.W - M.r - w / 2 - 2, c.X(l.rt)));
      let top = Math.max(M.t + 2, c.Y(c.U(s, ys)) - 5 - h), box = null;
      for (let k = 0; k < 3 && !box; k++, top -= h + 2) {
        const b = { x: x - w / 2, y: top, w, h };
        if (top >= M.t && !lbls.some((/** @type {any} */ o) => b.x < o.x + o.w + GAP && o.x < b.x + b.w + GAP && b.y < o.y + o.h && o.y < b.y + b.h)) box = b;
      }
      if (!box) continue;
      g.strokeStyle = ecss("--panel"); g.strokeText(label, x, box.y + h - 4); g.fillStyle = ecss("--ink"); g.fillText(label, x, box.y + h - 4);
      lbls.push({ ...box, tip, text: label, etpk: true });
    }
  }
  g.restore();
}

/** @param {any} p */
const toggle = p => { COMANDI.fai("spec.display", p, () => { p.etpk = !on(p, p._a && p._a.sr ? p._a.sr.length : 1); }); drawSoon(p); uiSave(); };

/** The labels of trace `s` shown now (none when off, not ready or not the labelled trace): [{ rt, apex, text }] for the data table. @param {any} p @param {any} s */
function rows(p, s) {
  const a = p._a; if (!a || !a.sr || !on(p, a.sr.length) || (a.sr.length > 1 && a.sr[0] !== s)) return [];
  return (labelsOf(p, s) || []).filter(l => l.rt >= a.x0 && l.rt <= a.x1).map(l => ({ rt: l.rt, apex: l.apex, text: texts(l).label }));
}

if (typeof AZIONI !== "undefined") {
  AZIONI.registra({ id: "chrom.etpk", gruppo: "view", viste: ["chrom", "xic", "mrm"], visibile: (/** @type {any} */ c) => !isAnalog(c.p.kind), attiva: (/** @type {any} */ c) => on(c.p, c.p._a && c.p._a.sr ? c.p._a.sr.length : 1),
    nome: () => T("ctx.chrom.etpk"), aria: () => T("ctx.chrom.etpk.aria"), esegui: (/** @type {any} */ c) => toggle(c.p) });
}
window.ETPK = { on, draw, labelsOf, rows, toggle };
