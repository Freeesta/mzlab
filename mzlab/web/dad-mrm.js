// @ts-strict
// The DAD on the MRM panel, loaded by sim-ui.js (mark) when a file of transitions has a DAD matrix: the UV spectrum at a time (menu, double click), the TWC on
// the right axis (`p.overlay = "twc"`, one of the right-axis overlays of gradiente.js, whose hooks it wraps) with its column in the data table, and «Allinea».
// Interface only: the TWC is api/chrom?kind=pda, the spectrum uv-spettro.js. Classic globals by name: E, I18N, AZIONI, COMANDI, ESP, DISP, GRADIENTE, TDATI_P.
const g = /** @type {any} */ (window), T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);
/** the DAD files an MRM panel shows @param {any} p @returns {any[]} */
const dadOf = p => (typeof shownFor === "function" ? shownFor(p) : []).filter((/** @type {any} */ f) => f && f.pda && !f.gone);
const here = (/** @type {any} */ c) => ESP.mrm(c.p) && dadOf(c.p).length > 0;
/** «Allinea DAD e massa»: the DAD sees a compound earlier by the delay of the detectors in series @param {any} p */
const shift = p => (g.DISP ? DISP.pdaShift(p) : 0);
const fz1 = () => (typeof fz === "function" ? fz() : 1);
/** the TWC of each file, read once: k|file -> {rt, y} | null while it is read @type {Map<string, any>} */
const TWC = new Map();
const keyOf = (/** @type {any} */ f) => `${f.k}|${f.file}`;

/** @param {any} p */
function ensure(p) {
  dadOf(p).forEach(f => {
    const k = keyOf(f); if (TWC.has(k)) return;
    TWC.set(k, null);
    getChrom(f.k, "pda", 1).then((/** @type {any} */ d) => { TWC.set(k, d && d.rt && d.rt.length ? d : false); drawSoon(p); }, () => TWC.set(k, false));
  });
}
/** @param {any} p @returns {{ f: any, d: any }[]} */
const curves = p => dadOf(p).map(f => ({ f, d: TWC.get(keyOf(f)) })).filter(c => c.d);
/** @param {{ d: any }[]} cs */
const topOf = cs => { let m = 0; cs.forEach(c => c.d.y.forEach((/** @type {number} */ v) => { if (v > m) m = v; })); return m > 0 ? m * 1.08 : 1; };
const ticks = (/** @type {number} */ top) => nice(0, top, 5).map((/** @type {number} */ v) => String(+v.toFixed(1)));
/** the TWC of curve d at time t of the MRM axis (nearest sample), null outside it @param {any} d @param {number} t @param {number} sh */
function at(d, t, sh) {
  const x = t - sh, r = d.rt; if (!r.length || x < r[0] || x > r[r.length - 1]) return null;
  let lo = 0, hi = r.length - 1; while (hi - lo > 1) { const m = (lo + hi) >> 1; if (r[m] <= x) lo = m; else hi = m; }
  return d.y[Math.abs(r[lo] - x) <= Math.abs(r[hi] - x) ? lo : hi];
}

/** the right axis and one dashed curve per file in its colour, like the pump pressure @param {any} p @param {CanvasRenderingContext2D} c @param {any} a */
function draw(p, c, a) {
  const cs = curves(p); if (!cs.length) return;
  const { W, H, X } = a, ph = H - M.t - M.b, top = topOf(cs), Y = (/** @type {number} */ v) => H - M.b - v / top * ph, z = fz1(), ink = css("--muted"), sh = shift(p);
  c.save(); c.strokeStyle = ink; c.fillStyle = ink; c.lineWidth = 1; c.font = fpx(12); c.textAlign = "left";
  c.beginPath(); c.moveTo(W - M.r + .5, M.t); c.lineTo(W - M.r + .5, H - M.b); c.stroke();
  for (const v of nice(0, top, 5)) { c.beginPath(); c.moveTo(W - M.r, Math.round(Y(v)) + .5); c.lineTo(W - M.r + 4, Math.round(Y(v)) + .5); c.stroke(); c.fillText(String(+v.toFixed(1)), W - M.r + 7, Y(v) + 3.5 * z); }
  { const G = GRADIENTE, fp = 12 * z, ttl = G.rightLayout(ticks(top), fp); c.save(); c.translate(W - M.r + ttl.titleX, (M.t + H - M.b) / 2); c.rotate(Math.PI / 2); c.textAlign = "center"; c.fillText(G.fitTitle(T("tdati.col.twc"), ph, fp), 0, 0); c.restore(); }
  c.beginPath(); c.rect(M.l, M.t, W - M.l - M.r, ph); c.clip();
  cs.forEach(({ f, d }) => { c.strokeStyle = f.color; c.lineWidth = 1.6; c.setLineDash([2, 3]); c.beginPath(); d.rt.forEach((/** @type {number} */ t, /** @type {number} */ i) => (i ? c.lineTo(X(t + sh), Y(d.y[i])) : c.moveTo(X(t + sh), Y(d.y[i])))); c.stroke(); });
  c.restore();
  a.twc = true;
}
/** the box near the curve (8 px) @param {any} p @param {number} t @param {number} py */
function hover(p, t, py) {
  const a = p._a, cs = curves(p); if (!a || !cs.length) return "";
  const ph = a.H - M.t - M.b, top = topOf(cs), sh = shift(p);
  return cs.map(({ f, d }) => { const v = at(d, t, sh); return v != null && Math.abs(a.H - M.b - v / top * ph - py) <= 8 ? `<div><i style="background:${f.color}"></i>${T("twc.read", { t: t.toFixed(2), v: v.toFixed(1) })}</div>` : ""; }).join("");
}
/** the data table of the panel with the TWC at the time of each row (the file of the row's trace) @param {any} p @param {any} d */
function table(p, d) {
  if (!d || !p._a || !p._a.sr) return d;
  const sh = shift(p), sr = p._a.sr, byK = new Map(curves(p).map(c => [c.f.k, c.d]));
  d.head = [...d.head, T("tdati.col.twc")];
  d.rows = d.rows.map((/** @type {any[]} */ r) => {
    const s = sr.find((/** @type {any} */ s) => r[0] === s.name || String(r[0]).startsWith(s.name + " · ")), c = s && (byK.get(s.k) || byK.values().next().value);
    const v = c ? at(c, parseFloat(r[1]), sh) : null;
    return [...r, v == null ? "" : TDATI.num(v, 1)];
  });
  return d;
}

// the hooks of the right axis (grafici.js, scroll.js, tdati-pannelli.js call them through their window objects): "twc" here, the rest as before
{
  const G = GRADIENTE, o = { ensure: G.ensure, draw: G.draw, hover: G.hover, needR: G.needR, cursorText: G.cursorText }, P = g.TDATI_P, up = P && P.update;
  G.ensure = (/** @type {any} */ p) => { if (p.overlay === "twc") ensure(p); return o.ensure(p); };
  G.draw = (/** @type {any} */ p, /** @type {any} */ c, /** @type {any} */ a) => (p.overlay === "twc" ? draw(p, c, a) : o.draw(p, c, a));
  G.hover = (/** @type {any} */ p, /** @type {number} */ t, /** @type {number} */ py) => (p.overlay === "twc" ? hover(p, t, py) : o.hover(p, t, py));
  G.needR = (/** @type {any} */ p) => { if (p.overlay !== "twc") return o.needR(p); const cs = curves(p); return cs.length ? G.rightLayout(ticks(topOf(cs)), 12 * fz1()).r : 0; };
  G.cursorText = (/** @type {any} */ p) => { if (p.overlay !== "twc") return o.cursorText(p); const c = curves(p)[0], v = c && p.cur != null ? at(c.d, p.cur, shift(p)) : null; return v != null ? " · " + T("twc.read.cursor", { v: v.toFixed(1) }) : ""; };
  if (up) P.update = (/** @type {any} */ p) => { up(p); if (p.overlay === "twc" && p.cv && !p._exp) TDATI.attach(p.cv, () => table(p, P.data(p))); };
}

AZIONI.registra({ id: "mrm.uvAtRt", gruppo: "extract", viste: ["mrm"], visibile: here, nome: () => T("ctx.uv.atRt"), doppioOrd: 3,
  doppio: (/** @type {any} */ c) => !!c.p._a && !c.p.el.classList.contains("max"),
  esegui: async (/** @type {any} */ c) => {
    const p = c.p, k = E.files[c.k] && E.files[c.k].pda ? c.k : dadOf(p)[0].k, t = c.x - shift(p);
    if (c.dbl) { p.sel = null; p.cur = c.x; draw0(p); }
    return (g.UVSPEC ||= await import("./uv-spettro.js")).open(p, k, t, t);
  } });
/** a check of the view group, undoable like the other display options @param {string} id @param {string} nome @param {string} aria @param {(p: any) => boolean} on @param {(p: any) => void} flip */
const check = (id, nome, aria, on, flip) => AZIONI.registra({ id, gruppo: "view", viste: ["mrm"], visibile: here, attiva: (/** @type {any} */ c) => on(c.p), nome: () => T(nome), aria: () => T(aria),
  esegui: (/** @type {any} */ c) => { COMANDI.fai("spec.display", c.p, () => flip(c.p)); drawSoon(c.p); uiSave(); } });
check("mrm.twc", "ctx.mrm.twc", "ctx.mrm.twc.aria", p => p.overlay === "twc", p => { p.overlay = p.overlay === "twc" ? null : "twc"; });
check("mrm.pdaAlign", "disp.pdaAlign", "disp.pdaAlign.title", p => !!p.pdaAlign, p => { p.pdaAlign = !p.pdaAlign; });
/** @param {any} p */
function draw0(p) { if (typeof g.draw === "function") g.draw(p); }
if (typeof redrawSoon === "function") redrawSoon();      // a restored panel with the TWC drawn before this module came
export { ensure, curves, at, table };
