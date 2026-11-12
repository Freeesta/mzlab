// @ts-nocheck
// A window drawn by hand on an ion or transition trace (SIM/MRM panel) follows the sample shown: on a trace of the same channel with no window of its own,
// the last window drawn on that channel is drawn again (same minutes and baseline mode) and measured on that trace. A follower is view state only: never in
// p.ints, the session, undo or the «Manual integrations» table; drawing a window on that sample commits it as today. Classic script, loaded by grafici.js.
const SG_C = new WeakMap();      // panel -> Map(trace key -> follower): the same object across redraws, so the engine's answers cached per record stay
// the committed windows of the panel and, for every shown trace of a channel with no window of its own, the follower of the last one drawn on that channel
function sgAll(p, sr) {
  if (!ESP.mrm(p) || !p.ints.length) return p.ints;
  const last = new Map(); for (const it of p.ints) { const c = last.get(it.ion); if (!c || it.id > c.id) last.set(it.ion, it); }
  const old = SG_C.get(p) || new Map(), now = new Map(), out = [...p.ints];
  for (const s of sr) {
    const src = last.get(s.ion || s.name); if (!src || p.ints.some(i => i.key === s.key)) continue;
    const a = Math.min(src.a, src.b), b = Math.max(src.a, src.b), sig = [src.id, a, b, src.bm || "", src.zero ? 1 : 0].join("|");
    let f = old.get(s.key);
    if (!f || f.sig !== sig) f = { fol: true, sig, id: "fol|" + s.key, k: s.k, key: s.key, ion: src.ion, a, b, name: s.name, src: src.file, ...(src.zero ? { zero: true } : {}),
      ...(src.bm ? { bm: src.bm } : {}), ...(src.bm === "line" ? { bl: edgeLine(s, a, b, null) } : {}) };       // a line baseline starts at this trace's edges, as a new record does
    let n = 0; for (const t of s.x) if (t >= a && t <= b && ++n > 1) break;
    f.none = n < 2;                                                  // no points of this sample in the window: an empty state, never a number
    now.set(s.key, f); out.push(f);
  }
  SG_C.set(p, now); return out;
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
window.SEGUE = { all: sgAll, none: sgNone, tip: sgTip };
