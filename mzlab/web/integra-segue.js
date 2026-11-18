// @ts-nocheck
// The windows drawn by hand on an ion or transition trace (SIM/MRM panel) follow the sample shown: on a trace of the same channel with no window of its own,
// the windows of the sample that holds the last one drawn on that channel are drawn again (same minutes and anchors) and measured on that trace. A follower is
// view state only: never in p.ints, the session or undo (the source window is, with its name `nm` and the names given to one sample alone `fn`); the table
// «Manual integrations» has a row for each (the engine applies the same windows: `follow` of manint.rs). The apex is searched in each sample by the engine
// (api/manfollow, MANINT.follow): a peak that drifted moves the window, none leaves it and says so. Drawing a window on that sample overrides it there.
const SG_C = new WeakMap();      // panel -> Map(follower id -> follower): the same object across redraws, so the engine's answers cached per record stay
// the last window drawn by hand on each channel (the automatic ones, `org`, hold a peak of their own in every file and apply to none)
const sgLast = p => { const last = new Map(); for (const it of p.ints) { if (it.org) continue; const c = last.get(it.ion); if (!c || it.id > c.id) last.set(it.ion, it); } return last; };
// the windows that apply to the other samples: those of the sample that holds the last one drawn on each channel
function sgSources(p) {
  if (!ESP.mrm(p) || !p.ints.length) return [];
  const last = sgLast(p);
  return p.ints.filter(i => !i.org && last.get(i.ion) && last.get(i.ion).key === i.key);
}
// the name of a follower: the one given to its sample alone, else the group's (the source's)
const sgName = (src, key) => (src.fn && src.fn[key]) || src.nm || "";
// the committed windows of the panel and, for every shown trace of a channel with no window of its own, the followers of the last windows drawn on that channel
function sgAll(p, sr) {
  if (!ESP.mrm(p) || !p.ints.length) return p.ints;
  const last = sgLast(p);
  const old = SG_C.get(p) || new Map(), now = new Map(), out = [...p.ints];
  for (const s of sr) {
    const top = last.get(s.ion || s.name); if (!top || p.ints.some(i => i.key === s.key)) continue;
    for (const src of p.ints.filter(i => !i.org && i.key === top.key)) {
      const a = Math.min(src.a, src.b), b = Math.max(src.a, src.b), id = "fol|" + s.key + "|" + src.id, an = src.snap ? JSON.stringify([src.snap, src.my || {}]) : "";
      const sig = [a, b, src.bm || "", src.zero ? 1 : 0, an].join("|");
      let f = old.get(id);
      if (!f || f.sig !== sig) f = { fol: true, sig, id, srcId: src.id, k: s.k, key: s.key, ion: src.ion, a, b, name: s.name, src: src.file, ...(src.zero ? { zero: true } : {}),
        ...(src.snap ? { snap: { ...src.snap }, ...(src.my ? { my: { ...src.my } } : {}) } : src.bm ? { bm: src.bm, ...(src.bm === "line" ? { bl: edgeLine(s, a, b, null) } : {}) } : {}) };   // anchors are placed again on this trace
      // where the engine put the window on this sample (its apex searched within the window's width), the source's edges until it answers
      const ans = window.MANINT && MANINT.follow ? MANINT.follow(p, src, s.k, sr) : undefined;
      f.a = ans ? ans.a : a; f.b = ans ? ans.b : b; f.org = ans ? ans.origin : ""; f.shift = ans ? ans.shift : 0; f.nm = sgName(src, s.key);
      if (f.bm === "line" && !f.snap) f.bl = edgeLine(s, f.a, f.b, null);
      let n = 0; for (const t of s.x) if (t >= f.a && t <= f.b && ++n > 1) break;
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
window.SEGUE = { all: sgAll, sources: sgSources, name: sgName, none: sgNone, dots: sgDots };
