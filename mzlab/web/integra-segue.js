// @ts-nocheck
// The windows drawn by hand on an ion or transition trace (SIM/MRM panel) follow the sample shown: on a trace of the same channel with no window of its own,
// the windows of the sample that holds the last one drawn on that channel are drawn again (same minutes and anchors) and measured on that trace. A follower is
// view state only: never in p.ints, the session, undo or the «Manual integrations» table; drawing a window on that sample commits it. Loaded by grafici.js.
const SG_C = new WeakMap();      // panel -> Map(follower id -> follower): the same object across redraws, so the engine's answers cached per record stay
// the committed windows of the panel and, for every shown trace of a channel with no window of its own, the followers of the last windows drawn on that channel
function sgAll(p, sr) {
  if (!ESP.mrm(p) || !p.ints.length) return p.ints;
  const last = new Map(); for (const it of p.ints) { const c = last.get(it.ion); if (!c || it.id > c.id) last.set(it.ion, it); }
  const old = SG_C.get(p) || new Map(), now = new Map(), out = [...p.ints];
  for (const s of sr) {
    const top = last.get(s.ion || s.name); if (!top || p.ints.some(i => i.key === s.key)) continue;
    for (const src of p.ints.filter(i => i.key === top.key)) {
      const a = Math.min(src.a, src.b), b = Math.max(src.a, src.b), id = "fol|" + s.key + "|" + src.id, an = src.snap ? JSON.stringify([src.snap, src.my || {}]) : "";
      const sig = [a, b, src.bm || "", src.zero ? 1 : 0, an].join("|");
      let f = old.get(id);
      if (!f || f.sig !== sig) f = { fol: true, sig, id, k: s.k, key: s.key, ion: src.ion, a, b, name: s.name, src: src.file, ...(src.zero ? { zero: true } : {}),
        ...(src.snap ? { snap: { ...src.snap }, ...(src.my ? { my: { ...src.my } } : {}) } : src.bm ? { bm: src.bm, ...(src.bm === "line" ? { bl: edgeLine(s, a, b, null) } : {}) } : {}) };   // anchors are placed again on this trace
      let n = 0; for (const t of s.x) if (t >= a && t <= b && ++n > 1) break;
      f.none = n < 2;                                                // no points of this sample in the window: an empty state, never a number
      now.set(id, f); out.push(f);
    }
  }
  SG_C.set(p, now); return out;
}
// the two anchors of a follower: dashed rings on its baseline (not buttons: a follower is committed by drawing it)
function sgDots(g, X, Y, U, s, it) {
  const y = t => it.bm === "zero" || it.zero ? 0 : it.bm === "line" && it.bl ? INTBASE.lineAt(it, t) : edge3(s.y, nearIdx(s.x, t));
  g.save(); g.strokeStyle = s.color; g.lineWidth = 1.5; g.setLineDash([2, 2]);
  for (const t of [it.a, it.b]) { g.beginPath(); g.arc(X(t), Y(U(s, y(t))), 5, 0, 2 * Math.PI); g.stroke(); }
  g.restore();
}
// the measure shown for a window with no points of the sample
const sgNone = it => ({ area: NaN, height: NaN, rt: (it.a + it.b) / 2, below: NaN });
// the hover lines of a follower's label: its S/N (by the engine, as a table row has it), the baseline mode, the sample it comes from and how to commit it
function sgTip(p, it) {
  const bm = it.bm ? `<div class="sm">${I18N.t("int.base.tip", { b: I18N.t("int.base.tag." + it.bm) })}</div>` : "";
  if (it.none) return `<div>${I18N.t("int.follow.none")}</div>` + bm;
  const sn = window.MANINT && MANINT.snr ? MANINT.snr(p, it) : undefined;
  return `<div>${I18N.t("int.follow.sn", { sn: sn === undefined ? "…" : sn || "—" })}</div>` + bm + `<div class="sm">${I18N.t("int.follow.note", { f: EH(it.src || "") })}</div>`;
}
window.SEGUE = { all: sgAll, none: sgNone, tip: sgTip, dots: sgDots };
