// @ts-strict
// Markers of the DDA trio (dda.js): the flags ▼ and open triangles ▽ on the Full Scan, the isolation band and the precursor line on the MS2, and the
// reason an MS2 panel is empty (averaged Full Scan, no MS2 from this scan, outside the m/z zoom, no MS2 in the file). Loaded on the first drawing
// of a DDA Full Scan (carica in DDA.decorate); `I` is the inside of DDA (DDA._in). Nothing here names a compound: an MS2 is shown as acquired.
const MARK = 6;
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);

/** The Full Scan: a flag over every peak that started an MS2 (only when the panel shows ONE scan), the isolation band of the chosen MS2.
 * @param {Bag} I @param {Bag} p @param {CanvasRenderingContext2D} g @param {(v: number) => number} X @param {(v: number) => number} Y @param {number} W @param {Bag} c */
export function full(I, p, g, X, Y, W, c) {
  const s2 = I.byId(p.duo), d0 = c.d0, f = c.files[0], D = I.ready(f.k);
  if (!D && f && f.dda) I.preload(f.k).then(() => { if (I.ready(f.k)) drawSoon(p); });
  p._a.flags = []; p._a.ddaWhy = why0(I, p, D, d0, f);
  if (s2 && s2.sid == null && s2.ion == null && !(s2.avg && s2.avg.length) && s2._why !== p._a.ddaWhy) drawSoon(s2);        // the empty MS2 panel says the reason of THIS view
  if (D && D.sid.length && p.r0 != null) {                      // the «N MS²» button: the MS2 of the scan(s) shown, from the engine
    const wk = `${f.k}|${p.r0}|${p.r1}`;
    if (p._dwk !== wk) { p._dwk = wk; p._dw = null; I.win(f.k, p.r0, p.r1).then((/** @type {Bag} */ w) => { if (p._dwk === wk) { p._dw = w; drawSoon(p); } }, () => {}); }
    if (window.DDAELENCO) window.DDAELENCO.sync(I, p, p._dw, p._a.ddaWhy); else carica("m:dda-elenco.js").then(() => drawSoon(p), () => {});
  }
  if (!s2 || !D) return;
  if (d0.scans !== 1 || d0.sid == null) { if (p._dw) averaged(I, p, g, X, Y, D, d0, f, s2); return; }
  const kids = D.byParent.get(d0.sid) || [], q2 = HR.prof(E.files[s2.k], 2);
  const ac = css("--accent"), tol = (/** @type {number} */ mz) => I.tolOf(f, mz), here = s2.sid != null ? D.idx.get(s2.sid) : null;
  if (here != null && D.parent[here] === d0.sid && D.lo[here] != null) {          // band of the isolation window of the chosen MS2 when its parent is this scan
    const a = Math.max(M.l, X(D.lo[here])), b = Math.min(W - M.r, X(D.hi[here]));
    if (b > a) {
      g.save(); g.globalAlpha = 0.10; g.fillStyle = ac; g.fillRect(a, M.t, b - a, p._a.H - M.t - M.b); g.globalAlpha = 1;
      g.strokeStyle = ac; g.lineWidth = 1; g.setLineDash([4, 3]); g.beginPath(); g.moveTo(X(D.lo[here]), M.t); g.lineTo(X(D.lo[here]), p._a.H - M.b); g.moveTo(X(D.hi[here]), M.t); g.lineTo(X(D.hi[here]), p._a.H - M.b); g.stroke();
      const t = D.tgt[here] ?? D.prec[here]; if (t != null) { g.setLineDash([]); g.beginPath(); g.moveTo(X(t), M.t); g.lineTo(X(t), p._a.H - M.b); g.stroke(); }
      g.restore();
    }
  }
  const near = nearby(I, p, D, d0, f, kids);
  p._a.markN = kids.length + near.size;
  const all = markOn(p), seen = (/** @type {number | string} */ k) => all || p._mk === k;           // the triangles of the peak under the pointer only, unless the switch is on
  g.save(); g.fillStyle = ac; g.font = fpx(9.5); g.textAlign = "center";
  const muted = css("--muted"), rt0 = D.ms1.rt[D.ms1pos.get(d0.sid)];
  near.forEach((idxs, jj) => {
    const px = X(d0.mz[jj]), py = Math.max(M.t + MARK + 10, Math.min(Y(Math.min(d0.y[jj], p._a.ymax)), p._a.H - M.b)) - 4;
    idxs.sort((/** @type {number} */ u, /** @type {number} */ v) => Math.abs(D.rt[u] - rt0) - Math.abs(D.rt[v] - rt0));
    if (seen("n" + idxs[0])) { g.save(); g.strokeStyle = muted; g.lineWidth = 1.4; g.beginPath(); g.moveTo(px - MARK, py - MARK - 3); g.lineTo(px + MARK, py - MARK - 3); g.lineTo(px, py); g.closePath(); g.stroke(); g.restore(); }
    const tip = `<b>${T("dda.near.title")}</b><div class="sm">${T("dda.near.line", { n: idxs.length, min: I.NEAR_MIN, prec: (D.prec[idxs[0]]).toFixed(q2.dec) })}</div><div class="sm">${T("dda.near.hint")}</div>`;
    p._a.lbls.push({ nb: idxs, ion: D.prec[idxs[0]], x: px - MARK - 3, y: py - MARK - 16, w: 2 * MARK + 6, h: MARK + 18, tip });
  });
  kids.forEach((/** @type {number} */ j, /** @type {number} */ n) => {
    const m = D.tgt[j] ?? D.prec[j]; if (m == null || m < p._a.x0 || m > p._a.x1) return;
    const px = X(m), tm = tol(m); let y0 = 0;
    for (let i = I.lowerBound(d0.mz, m - tm * 1.001 - 1e-9); i < d0.mz.length && d0.mz[i] <= m + tm * 1.001 + 1e-9; i++) if (Math.abs(d0.mz[i] - m) <= tm && d0.y[i] > y0) y0 = d0.y[i];      // the peaks inside the tolerance (m/z is ascending), the exact test kept
    const py = Math.max(M.t + MARK + 10, Math.min(Y(Math.min(y0, p._a.ymax)), p._a.H - M.b)) - 4;                // above the peak, at the top edge when the peak is cut
    const chosen = j === here;
    if (seen(j) || chosen) { g.beginPath(); g.moveTo(px - MARK, py - MARK - 3); g.lineTo(px + MARK, py - MARK - 3); g.lineTo(px, py); g.closePath(); g.fill(); g.fillText(String(n + 1), px, py - MARK - 6); }
    if (chosen) { g.strokeStyle = ac; g.lineWidth = 1.5; g.strokeRect(px - MARK - 2, py - MARK - 5, 2 * MARK + 4, MARK + 7); }
    const tip = `<b>${T("dda.kid.title", { i: n + 1, n: kids.length })}</b><div class="sm">${T("dda.kid.prec", { prec: (D.prec[j] ?? m).toFixed(q2.dec) })}${D.act[j] ? " · " + D.act[j] : ""}${D.ce[j] != null ? " · " + (D.nce ? "NCE " + I.fmt1(D.ce[j]) : "CE " + I.fmt1(D.ce[j]) + " eV") : ""}</div><div class="sm">${T("dda.kid.hint")}</div>`;
    p._a.flags.push({ j, px, py });
    p._a.lbls.push({ fl: j, ion: D.prec[j], x: px - MARK - 3, y: py - MARK - 16, w: 2 * MARK + 6, h: MARK + 18, tip });
  });
  g.restore();
}
/** An averaged Full Scan: a flag over every precursor fragmented from its scans (the groups of /api/dda_window); a click shows the most intense MS2
 * of the precursor and the average stays. @param {Bag} I @param {Bag} p @param {CanvasRenderingContext2D} g @param {(v: number) => number} X
 * @param {(v: number) => number} Y @param {Bag} D @param {Bag} d0 @param {Bag} f @param {Bag} s2 */
function averaged(I, p, g, X, Y, D, d0, f, s2) {
  const gr = p._dw.groups, ac = css("--accent"), all = markOn(p), dec = HR.prof(E.files[s2.k], 2).dec, here = s2.sid != null ? D.idx.get(s2.sid) : null;
  p._a.markN = gr.length;
  g.save(); g.fillStyle = ac; g.font = fpx(9.5); g.textAlign = "center";
  gr.forEach((/** @type {Bag} */ q, /** @type {number} */ n) => {
    const m = q.prec; if (m < p._a.x0 || m > p._a.x1) return;
    const best = q.sid[q.pint.reduce((/** @type {number} */ b, /** @type {number | null} */ v, /** @type {number} */ i) => ((v ?? -1) > (q.pint[b] ?? -1) ? i : b), 0)], j = D.idx.get(best);
    if (j == null) return;
    const tm = I.tolOf(f, m); let y0 = 0;
    for (let i = I.lowerBound(d0.mz, m - tm); i < d0.mz.length && d0.mz[i] <= m + tm; i++) if (d0.y[i] > y0) y0 = d0.y[i];
    const px = X(m), py = Math.max(M.t + MARK + 10, Math.min(Y(Math.min(y0, p._a.ymax)), p._a.H - M.b)) - 4, chosen = q.sid.some((/** @type {number} */ x) => D.idx.get(x) === here);
    if (all || p._mk === j || chosen) { g.beginPath(); g.moveTo(px - MARK, py - MARK - 3); g.lineTo(px + MARK, py - MARK - 3); g.lineTo(px, py); g.closePath(); g.fill(); g.fillText(String(n + 1), px, py - MARK - 6); }
    if (chosen) { g.strokeStyle = ac; g.lineWidth = 1.5; g.strokeRect(px - MARK - 2, py - MARK - 5, 2 * MARK + 4, MARK + 7); }
    const tip = `<b>${T("dda.grp.title", { i: n + 1, n: gr.length })}</b><div class="sm">${T("dda.grp.line", { prec: m.toFixed(dec), n: q.sid.length })}</div><div class="sm">${T("dda.grp.hint")}</div>`;
    p._a.flags.push({ j, px, py });
    p._a.lbls.push({ fl: j, keep: true, ion: m, x: px - MARK - 3, y: py - MARK - 16, w: 2 * MARK + 6, h: MARK + 18, tip });
  });
  g.restore();
}
/** Nearby Precursors (Thermo FreeStyle): peaks of this scan fragmented in ANOTHER scan within +-NEAR_MIN min, peak index -> MS2 indices.
 * @param {Bag} I @param {Bag} p @param {Bag} D @param {Bag} d0 @param {Bag} f @param {number[]} kids @returns {Map<number, number[]>} */
function nearby(I, p, D, d0, f, kids) {
  const rt0 = D.ms1.rt[D.ms1pos.get(d0.sid)], flagged = new Set(), near = new Map(), tol = (/** @type {number} */ mz) => I.tolOf(f, mz);
  kids.forEach((/** @type {number} */ j) => { const m = D.tgt[j] ?? D.prec[j]; if (m != null) { const jj = I.nearest(d0.mz, m); if (jj >= 0 && Math.abs(d0.mz[jj] - m) <= tol(m)) flagged.add(jj); } });
  if (rt0 == null) return near;
  const a = I.lowerBound(D.rt, rt0 - I.NEAR_MIN), b = I.lowerBound(D.rt, rt0 + I.NEAR_MIN + 1e-9);
  for (let i = a; i < b; i++) {
    if (D.parent[i] === d0.sid) continue;
    const m = D.tgt[i] ?? D.prec[i]; if (m == null || m < p._a.x0 || m > p._a.x1) continue;
    const jj = I.nearest(d0.mz, m); if (jj < 0 || Math.abs(d0.mz[jj] - m) > tol(m) || !(d0.y[jj] > 0) || flagged.has(jj)) continue;
    let l = near.get(jj); if (!l) near.set(jj, l = []); l.push(i);
  }
  return near;
}
/** Why the MS2 panel of this Full Scan has nothing to show yet: the text of the empty panel (a pure reading of the data already loaded).
 * @param {Bag} I @param {Bag} p @param {Bag | null} D @param {Bag} d0 @param {Bag} f @returns {string} */
function why0(I, p, D, d0, f) {
  if (!D || !d0) return T("dda.click");
  if (!D.sid.length) return T("dda.why.file");
  if (d0.scans !== 1 || d0.sid == null) {                    // an average: how many MS2 started from the Full Scans of its window
    const lo = I.lowerBound(D.ms1.rt, p.r0 - 1e-9), hi = I.lowerBound(D.ms1.rt, p.r1 + 1e-9); let m = 0;
    for (let i = lo; i < hi; i++) m += (D.byParent.get(D.ms1.sid[i]) || []).length;
    return T(m ? "dda.why.avg" : "dda.why.avg0", { n: d0.scans, m });
  }
  const kids = D.byParent.get(d0.sid) || [];
  if (kids.length) {
    const out = kids.filter((/** @type {number} */ j) => { const m = D.tgt[j] ?? D.prec[j]; return m != null && (m < p._a.x0 || m > p._a.x1); }).length;
    return out === kids.length ? T("dda.why.zoom", { n: out }) : T("dda.click");
  }
  return nearby(I, p, D, d0, f, []).size ? T("dda.why.near") : T("dda.why.none", { min: I.NEAR_MIN });
}
/** The text of an empty MS2 panel: the reason its Full Scan gave at its last drawing. @param {Bag} I @param {Bag} s2 @returns {string} */
export function why(I, s2) {
  const s1 = I.byId(s2.dda), t = (s1 && s1._a && s1._a.ddaWhy) || T("dda.click");
  s2._why = t; return t;
}
/** The MS2: a dashed line on the precursor.
 * @param {Bag} I @param {Bag} p @param {CanvasRenderingContext2D} g @param {(v: number) => number} X @param {(v: number) => number} Y @param {number} W */
export function ms2(I, p, g, X, Y, W) {
  const j = p._scan; if (!j || j.prec == null || p.avg) return;
  const ac = css("--accent"), D = I.ready(p.k), i = D && D.idx.get(p.sid), m = (D && D.prec[i]) ?? j.prec;
  if (m < p._a.x0 || m > p._a.x1) return;
  g.save(); g.strokeStyle = ac; g.fillStyle = ac; g.lineWidth = 1; g.setLineDash([4, 3]); g.beginPath(); g.moveTo(X(m), p._a.H - M.b); g.lineTo(X(m), M.t + 14); g.stroke(); g.setLineDash([]);
  g.font = fpx(9.5); g.textAlign = "center"; g.fillText(T("dda.precursor"), Math.max(M.l + 26, Math.min(W - M.r - 26, X(m))), M.t + 11); g.restore();
}
window.DDAMARK = { full, ms2, why };
