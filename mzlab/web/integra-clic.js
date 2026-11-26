// @ts-nocheck
// The click of the automatic tool: ONE trace (the one nearest the pointer on the plot, or the file chosen in the panel) and the peak whose apex is nearest the
// click, by the Rust engine (QUALITA.click = peakq_click.rs, the bytes of `mzlab-cli peak edges --set click=1`). A junk window (too few points, area under the noise,
// S/N too low) is refused with its evidence; Alt+click or the menu keeps it, and the record logs the numbers and the override. "All channels at this time"
// (`atRt`) is a separate command that leaves the channels without a peak out. Loaded by picchi.js (intClick, autoInt) on first use.
const G = window;
const T = (k, v = {}) => I18N.t(k, v);
const REACH = 0.25;                                           // min: how far from the click a trace is looked at (as the engine's `reach`)

/** The trace nearest the click: the file chosen in the panel, else the visible trace whose line passes closest to the pointer (pixels, within the reach). */
function nearest(p, x, py) {
  const a = p._a, vis = a.sr.filter(s => !isHid(p, s));
  if (p.intf && p.intf !== "all") return intTargets(p, x, py)[0] || null;
  if (a.stk || py == null || !a.Y || vis.length < 2) return intSeries(p, x, py) || vis[0] || null;
  const w = a.W - M.l - M.r, px = M.l + (x - a.x0) / (a.x1 - a.x0) * w;
  let best = null;
  for (const s of vis) {
    let d = Infinity;
    for (let j = nearIdx(s.x, x - REACH), h = nearIdx(s.x, x + REACH); j <= h; j++) {
      const dx = M.l + (s.x[j] - a.x0) / (a.x1 - a.x0) * w - px, dy = a.Y(a.U(s, s.ys[j])) - py;
      d = Math.min(d, dx * dx + dy * dy);
    }
    if (!best || d < best.d) best = { d, s };
  }
  return best && best.s;
}

/** The traces a click integrates: one, unless the panel says "all traces" on a free panel (the SIM/MRM panels keep "all" for the manual windows only). */
export function targets(p, x, py) {
  const a = p._a; if (!a || !a.sr || !a.sr.length) return [];
  if (p.intf === "all" && !p.intfFixed) return intTargets(p, x, py);
  const s = nearest(p, x, py); return s ? [s] : [];
}

/** The engine's answer for the click at x on the trace s ({ ok, edges, apex, points, area, height, noise, sn, refused, forced, params }); null without the engine. */
export async function ask(s, x, force) {
  const mrm = (s.key && s.key.startsWith("m|")) || s.tr != null;
  const t = window.QUALITA ? await QUALITA.click(s.x, s.y, x, mrm, PK, force) : null;
  try { return t ? JSON.parse(t) : null; } catch (_) { return null; }
}

// the trace of the click shows for a moment (the others fade, the line thickens: the highlight of a trace of the panel)
function mark(p, s) {
  if (s.tk == null) return;
  p._hl = s.tk; draw(p);
  setTimeout(() => { if (p._hl === s.tk) { p._hl = null; draw(p); } }, 2500);
}

const WHY = { no_peak: "int.why.noPeak", points: "int.why.points", area: "int.why.area", sn: "int.why.sn" };      // the engine's refusal codes (peakq_click.rs)
const why = r => r.refused.map(k => T(WHY[k], { n: r.points, min: r.params.min_points, sn: r.sn.toFixed(1), minSn: r.params.min_sn, reach: r.params.reach, area: I18N.sciE(r.area) })).join("; ");

/** The click is refused: which trace was looked at, and why, with how to keep it. */
export function refuse(p, s, r) { mark(p, s); nearMsg(T("int.refused", { why: why(r) })); }

/** A window was made from the click: it is the selected one, its record logs what the engine measured and the guard used (and what was forced). */
export function done(p, s, r) {
  const it = p.ints.find(i => i.id === p._intCur); if (!it) return;
  it.click = { apex: r.apex, points: r.points, area: r.area, noise: r.noise, sn: r.sn, forced: r.forced, refused: r.refused, params: r.params };
  p.isel = it.id; mark(p, s);
  if (r.forced) nearMsg(T("int.forced", { why: why(r) }));
}

/** The click of the page at time x and pixel row py: one trace, one peak; several traces only when the panel says "all traces" on a free panel. */
export async function go(p, x, py, force, mouse) {
  const t = targets(p, x, py); if (!t.length) return;
  if (t.length > 1) return atRt(p, x, t);
  const s = t[0]; mark(p, s);
  if (mouse && hrTrace(s)) return autoAll(p, s);              // a high-resolution trace: the click integrates every peak of the stretch shown
  return autoInt(p, s, x, force);
}

/** "Integrate all channels at this time": every visible trace (or `list`) that has a peak at x, in one undoable step; the others are named in the notice, never integrated. */
export async function atRt(p, x, list = null) {
  const a = p._a; if (!a || !a.sr) return;
  const all = list || a.sr.filter(s => !isHid(p, s)), res = await Promise.all(all.map(async s => [s, await ask(s, x, false)]));
  if (res.some(q => !q[1])) { nearMsg(T("int.autoOff")); return; }
  const adds = [], skipped = res.filter(q => !q[1].ok).map(q => q[0].name);
  for (const [s, r] of res.filter(q => q[1].ok)) { const w = clickWin(p, s, x, r.edges[0], r.edges[1]); if (w) adds.push([s, w, r]); else skipped.push(s.name); }
  const note = T("int.atrt.note", { n: adds.length, m: all.length, names: skipped.join(", ") });
  if (adds.length) { const run = () => adds.forEach(([s, w, r]) => { if (addInt(p, s, w[0], w[1])) done(p, s, r); }); if (HISTORY) HISTORY.group(run); else run(); }
  if (skipped.length || !adds.length) nearMsg(note);
}

G.INTCLICK = { go, ask, refuse, done, atRt, targets };
