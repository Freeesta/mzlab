// @ts-check
"use strict";
// ONE model of progress for every loading (.mzML, .raw, .wiff, .D, .mzidx): the readers tell it how far each file has got (event mzlab-index {name, frac, phase}, frac = 0..1 of the whole file),
// and every bar of the page (the loading card, the overlay of «Open», Tabui) shows what the model says. Rules: a file never goes back; the total never goes back (a file added in the middle
// of the batch widens the denominator, the shown value stays where it was); nothing is 100 % until `end()` of the last file; the weight of a file is its size.
// Classic script, loaded before conversione.js; the pure part is also run by node (tests/test_progresso.py).

/** the share of the bar of each phase for a reader whose phases tell their own fraction (sum 1): a folder converted to mzML (Agilent .D, Waters .raw, ...) and then indexed (the same shares as ConvUI, CONV_W).
 * The readers that tell the fraction of the whole file need none: an mzML tells its index (0..1), a maker's file the phases of nativo-fasi.js (W), the desktop app adds PROG_NET */
const PROG_W = { conversione: { convert: 0.75, index: 0.25 } };
/** the app of the desktop sends the file over the network before the engine reads it: that is the first part of the bar (in the browser the file is already there) */
const PROG_NET = { send: 0.3 };
const PROG_PHASE_KEY = { send: "load.ph.read", read: "load.ph.read", scans: "load.ph.scans", chrom: "load.ph.chrom", convert: "load.ph.convert" };

/** whole-file fraction 0..1 from the phase being run and how far it has got: the phases before it count full */
function progCompose(profile, phase, local) {
  const w = PROG_W[profile]; if (!w || !(phase in w)) return Math.max(0, Math.min(1, local || 0));
  let before = 0; for (const k of Object.keys(w)) { if (k === phase) break; before += w[k]; }
  return Math.min(1, before + w[phase] * Math.max(0, Math.min(1, local || 0)));
}

const PROGRESSO = (() => {
  /** @type {Map<string, { w: number, f: number, phase: string, end: boolean }>} */
  const items = new Map(), views = new Set();
  let shown = 0;
  const all = () => [...items.values()];
  const finished = () => items.size > 0 && all().every(i => i.end);
  const compute = () => { const a = all(), tot = a.reduce((s, i) => s + i.w, 0); return tot ? a.reduce((s, i) => s + i.w * i.f, 0) / tot : 0; };
  const tell = () => { for (const v of views) v(PROGRESSO.state()); };
  return {
    /** a file enters the batch; when the previous batch is over, a new one starts at 0 */
    begin(name, weight) {
      if (finished()) { items.clear(); shown = 0; }
      if (!items.has(name)) items.set(name, { w: Math.max(1, Number(weight) || 1), f: 0, phase: "", end: false });
      tell();
    },
    /** how far `name` has got (0..1 of the whole file): never back, never complete before `end` */
    set(name, frac, phase) {
      const i = items.get(name); if (!i || i.end) return;
      const f = Math.max(0, Math.min(0.99, Number(frac) || 0));
      if (f >= i.f) { i.f = f; if (phase) i.phase = phase; }
      tell();
    },
    /** the file is done (or failed): the only way to reach 100 % */
    end(name) { const i = items.get(name); if (!i) return; i.end = true; i.f = 1; tell(); },
    /** fraction 0..1 of one file (0 if unknown) */
    file(name) { const i = items.get(name); return i ? i.f : 0; },
    /** { pc: whole percent 0..100 (never back, 99 at most before the last end), phase of the slowest file, done, n } */
    state() {
      const a = all(), over = finished();
      shown = Math.max(shown, over ? 100 : Math.min(99, Math.floor(100 * compute())));
      const low = a.filter(i => !i.end).sort((x, y) => x.f - y.f)[0];
      return { pc: shown, phase: low ? low.phase : "", done: a.filter(i => i.end).length, n: a.length, over };
    },
    busy() { return items.size > 0 && !finished(); },
    subscribe(fn) { views.add(fn); return () => views.delete(fn); },
    /** for tests: forget everything */
    reset() { items.clear(); shown = 0; tell(); },
    /** draws THE bar in `box` (a div) and keeps it up to date; returns the function that removes it */
    mount(box) {
      const el = document.createElement("div"); el.className = "loadbar";
      el.innerHTML = `<div class="muted sm cp-num" data-r="txt" role="status" aria-live="polite"></div><div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax="100"><i></i></div>`;
      box.appendChild(el);
      const txt = el.querySelector("[data-r=txt]"), bar = el.querySelector(".prog"), fill = bar.querySelector("i");
      let last = -1;
      const draw = s => {
        const lab = I18N.t(PROG_PHASE_KEY[s.phase] || "load.ph.index"), t = s.n > 1 ? `${lab} · ${I18N.t("load.prog.count", { done: s.done, n: s.n })}` : lab;
        if (txt.textContent !== t) txt.textContent = t;
        if (s.pc !== last) { last = s.pc; bar.setAttribute("aria-valuenow", String(s.pc)); fill.style.width = s.pc + "%"; }
        bar.setAttribute("aria-label", lab);
      };
      draw(PROGRESSO.busy() ? PROGRESSO.state() : { pc: 0, phase: "", done: 0, n: 0, over: false });      // a finished batch is not shown again
      const off = PROGRESSO.subscribe(draw);
      return () => { off(); el.remove(); };
    },
  };
})();

if (typeof module !== "undefined") module.exports = { PROGRESSO, progCompose, PROG_W, PROG_NET };
