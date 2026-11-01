// @ts-strict
// @ts-noany
// @ts-check
// Data view base: notices (toast), theme and colours, notebook save and sync, session boot and UI restore, file list (order, groups, colours, drag and drop).
// Data exploration starts from the total chromatogram; the student decides what to extract.
// Classic script: its functions are page globals used by the other Dati scripts (pannelli.js, grafici.js, ...).

window.toast = function(/** @type {string} */ msg) {
  let t = document.getElementById("toast-container");
  if (!t) {
    t = document.createElement("div");
    t.id = "toast-container";
    t.className = "toast-container";
    t.setAttribute("aria-live", "polite");
    document.body.appendChild(t);
  }
  const el = document.createElement("div");
  el.className = "toast-msg";
  el.textContent = msg;
  t.appendChild(el);
  setTimeout(() => el.classList.add("show"), 10);
  setTimeout(() => {
    el.classList.remove("show");
    setTimeout(() => el.remove(), 400);
  }, 3000);
};

/**
 * An informational notice that stays on the page until it is closed (the reset note, the release notice, the engine notices) goes away by itself after `ms` (20 s):
 * the timer stands still while the pointer is over it or the focus is inside it, and an X closes it at once (added unless `x` is false: the notice has its own).
 * Notices that ask for an action are not given this. Returns the function that closes it.
 * @param {HTMLElement} el the notice, already in the page
 * @param {{ ms?: number, x?: boolean, onClose?: (by: string) => void }} [o]
 */
window.avvisoAuto = function(el, o = {}) {
  let left = o.ms || 20000, t0 = 0, tm = 0, done = false, over = false, inside = false;
  /** @param {string} by */
  const close = by => { if (done) return; done = true; clearTimeout(tm); el.remove(); if (o.onClose) o.onClose(by); };
  const go = () => { if (done || tm || over || inside) return; t0 = Date.now(); tm = window.setTimeout(() => close("time"), left); };
  const stop = () => { if (!tm) return; clearTimeout(tm); tm = 0; left = Math.max(1000, left - (Date.now() - t0)); };
  el.addEventListener("mouseenter", () => { over = true; stop(); });
  el.addEventListener("mouseleave", () => { over = false; go(); });
  el.addEventListener("focusin", () => { inside = true; stop(); });
  el.addEventListener("focusout", e => { if (!el.contains(/** @type {Node | null} */ (e.relatedTarget))) { inside = false; go(); } });
  if (o.x !== false && !el.querySelector("button.x")) {
    // the shared close X (index.html)
    const b = document.createElement("button"); b.type = "button"; b.className = "x"; b.textContent = "×";
    b.setAttribute("aria-label", I18N.t("avviso.close")); b.onclick = () => close("x"); el.appendChild(b);
  }
  go();
  return () => close("api");
};

"use strict";
// user preferences (settings gear): text size, theme, chart colours, high resolution (hr: automatic | off, tolerance in ppm, decimals; see hr.js). Filled from localStorage by uipLoad() (settings.js).
// merge = join the centroids of the same nominal mass (centroid files only)
const UIP = { theme: "light", pal: "file", merge: true, beside: false, hrPpm: 5, hrDec: 4, xicLens: true };
// areas: at least 3 decimals in the label (2.243e+6), whole number in the tooltip
const fmtA = (/** @type {number} */ v) => !Number.isFinite(v) || v === 0 ? "0" : Math.abs(v) >= 1e4 || Math.abs(v) < 0.01 ? v.toExponential(3) : (+v.toPrecision(4)).toString();
const fmtFull = (/** @type {number} */ v) => Number.isFinite(v) ? Math.round(v).toString() : "";
let EXPORTING = false;          // true while a plot is drawn for a saved image
// ---- chart colours (settings: "Colori dei grafici"). PAL = categorical colours of the current palette (filled in place by setPal, so every user of PAL follows it);
// files of the Full Scan tab that have a time get a SEQUENTIAL scale ordered by time (seqColor), see paintFiles.
function isDark() {
  if (EXPORTING) return false;
  if (typeof UIP !== "undefined") {
    if (UIP.theme === "dark") return true;
    if (UIP.theme === "light") return false;
  }
  return !!(typeof window !== "undefined" && window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches);
}
window.isDark = isDark;

const VIRIDIS = ["#440154", "#472d7b", "#3b528b", "#2c728e", "#21918c", "#28ae80", "#5ec962", "#addc30", "#fde725"];
const VIRIDIS_DARK = ["#38bdf8", "#06b6d4", "#14b8a6", "#10b981", "#84cc16", "#eab308", "#fde047"];
const CIVIDIS = ["#00204d", "#00336f", "#39486b", "#575c6d", "#707173", "#8a8779", "#a69d75", "#c4b56c", "#e4cf5b", "#ffea46"];
const CIVIDIS_DARK = ["#7dd3fc", "#93c5fd", "#c7d2fe", "#e0e7ff", "#fef08a", "#fde047"];
const OKABE = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#F0E442", "#000000", "#7a3b9b", "#8a8a8a"];
const OKABE_DARK = ["#56B4E9", "#F0E442", "#34d399", "#fb923c", "#f472b6", "#38bdf8", "#fde047", "#f8fafc", "#c084fc", "#cbd5e1"];
const PALS = /** @type {Record<string, Palette>} */ ({
  // viridis cut at 72% (the yellow end is almost invisible on white); in dark mode uses luminous tones
  time: {
    seq: VIRIDIS, seqDark: VIRIDIS_DARK, a: 0, b: 0.72, aDark: 0, bDark: 1.0,
    cat: ["#1f77b4", "#e6550d", "#2ca02c", "#9467bd", "#d62728", "#17becf", "#bcbd22", "#e377c2", "#8c564b", "#0b6e4f", "#f2a900", "#5b5fc7"],
    catDark: ["#60a5fa", "#fb923c", "#4ade80", "#c084fc", "#f87171", "#38bdf8", "#facc15", "#f472b6", "#2dd4bf", "#a3e635", "#fbbf24", "#818cf8"],
    blank: "#8a8a8a", blankDark: "#9ca3af", std: "#4d4d4d", stdDark: "#cbd5e1"
  },
  // CVD-safe: cividis + Okabe-Ito (bright in dark mode)
  cb: {
    seq: CIVIDIS, seqDark: CIVIDIS_DARK, a: 0, b: 0.75, aDark: 0, bDark: 1.0,
    cat: OKABE, catDark: OKABE_DARK,
    dash: true, blank: "#8a8a8a", blankDark: "#9ca3af", std: "#4d4d4d", stdDark: "#cbd5e1"
  },
  // every colour >= 4.5:1 on white (dark half on white, bright and vibrant on dark)
  hc: {
    seq: CIVIDIS, seqDark: CIVIDIS_DARK, a: 0, b: 0.45, aDark: 0.2, bDark: 1.0,
    cat: ["#003f9e", "#b30000", "#006b2e", "#6a1b9a", "#8a4b00", "#00727f", "#a1006b", "#000000"],
    catDark: ["#60a5fa", "#f87171", "#4ade80", "#c084fc", "#fb923c", "#38bdf8", "#f472b6", "#ffffff"],
    dash: true, lw: 1, blank: "#6e6e6e", blankDark: "#9ca3af", std: "#333333", stdDark: "#cbd5e1"
  },
  // well separated hues; on time series the hue order still follows the time
  rainbow: {
    hue: (/** @type {number} */ u) => isDark() ? `hsl(${Math.round(265 - 250 * u)} 85% 65%)` : `hsl(${Math.round(265 - 250 * u)} 80% 40%)`,
    cat: ["#d62728", "#1f77b4", "#2ca02c", "#9467bd", "#ff7f0e", "#17becf", "#e377c2", "#bcbd22", "#8c564b", "#0b6e4f", "#f2a900", "#5b5fc7"],
    catDark: ["#f87171", "#60a5fa", "#4ade80", "#c084fc", "#fb923c", "#38bdf8", "#f472b6", "#facc15", "#2dd4bf", "#a3e635", "#fbbf24", "#818cf8"],
    blank: "#8a8a8a", blankDark: "#9ca3af", std: "#4d4d4d", stdDark: "#cbd5e1"
  },
  // Monochrome black & white with dashes for distinct traces
  bw: {
    seq: ["#000000", "#333333", "#666666", "#999999"],
    seqDark: ["#ffffff", "#cccccc", "#999999", "#666666"],
    a: 0, b: 1.0, aDark: 0, bDark: 1.0,
    cat: ["#000000", "#222222", "#444444", "#666666", "#111111", "#333333", "#555555", "#777777"],
    catDark: ["#ffffff", "#e0e0e0", "#b8b8b8", "#909090", "#f0f0f0", "#d0d0d0", "#a8a8a8", "#808080"],
    dash: true, blank: "#777777", blankDark: "#aaaaaa", std: "#333333", stdDark: "#dddddd"
  },
});
// default: one categorical colour per file, no time scale, no neutral greys, continuous lines
PALS.file = { ...PALS.time, plain: true };
const PAL = /** @type {string[]} */ ([]);
function setPal(/** @type {string} */ mode) {
  UIP.pal = PALS[mode] ? mode : "file";
  const P = PALS[UIP.pal], dark = isDark();
  PAL.length = 0; PAL.push(...(dark && P.catDark ? P.catDark : P.cat));
  document.documentElement.dataset.pal = UIP.pal;
}
setPal(UIP.pal);
// u in 0..1 -> colour of the current palette's sequential scale
function seqColor(/** @type {number} */ u) {
  const P = PALS[UIP.pal], dark = isDark();
  if (P.hue) return P.hue(u);
  const seq = /** @type {string[]} */ (dark && P.seqDark ? P.seqDark : P.seq);
  const a = /** @type {number} */ (dark && P.aDark != null ? P.aDark : P.a);
  const b = /** @type {number} */ (dark && P.bDark != null ? P.bDark : P.b);
  const x = (a + (b - a) * u) * (seq.length - 1), i = Math.min(Math.floor(x), seq.length - 2), f = x - i;
  const c = (/** @type {string} */ h) => [1, 3, 5].map(k => parseInt(h.slice(k, k + 2), 16)), A = c(seq[i]), B = c(seq[i + 1]);
  return "#" + A.map((v, k) => Math.round(v + (B[k] - v) * f).toString(16).padStart(2, "0")).join("");
}
const DASHES = [[], [6, 3], [2, 3], [10, 3, 2, 3]];
/** The one back / forward arrow of the program (the same drawing as the file arrows of the header): a chevron. @param {number} d -1 = back, 1 = forward @param {number} [n] size in px */
const chevron = (d, n = 16) => `<svg width="${n}" height="${n}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="${d < 0 ? "M10 3L5 8l5 5" : "M6 3l5 5-5 5"}"/></svg>`;
const LWX = () => PALS[UIP.pal].lw || 0;                      // extra line width (px) of the high contrast palette
// line style of a file: solid, so that blanks and standards read as plain traces (neutral grey carries the distinction); with the colour-blind /
// high-contrast palettes the files of the time scale are also told apart by line style (second code besides the colour)
// true while every file takes a categorical colour (palette "file", or "time" with no file that has a time)
let PLAIN = false;
function fdash(/** @type {FileAperto} */ f) {
  if (PLAIN) return [];
  return PALS[UIP.pal].dash && f.rank != null ? DASHES[f.rank % 4] : [];
}
/** @type {(s: string) => any} */
const Q = s => document.querySelector(s);
const EH = (/** @type {unknown} */ t) => String(t).replace(/[&<>"]/g, c => (/** @type {Record<string, string>} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }))[c]);
const fmtFormula = (/** @type {string | undefined} */ f) => !f ? "" : EH(f).replace(/([A-Z][a-z]?|\))(\d+)/g, "$1<sub>$2</sub>");
const fmtAdduct = (/** @type {string|undefined} */ a) => !a ? "" : fmtFormula(a).replace(/([+-]+)$/, "<sup>$1</sup>");
// formula in plain text for the canvas (subscript digits)
const subText = (/** @type {unknown} */ f) => String(f || "").replace(/\d/g, d => "₀₁₂₃₄₅₆₇₈₉"[+d]);
window.fmtFormula = fmtFormula; window.fmtAdduct = fmtAdduct;
const KIND = /** @type {Record<string, string>} */ ({ full: "Full Scan", ms2: "MS2 (Product Ion)", mrm: "MRM" });
const kindOf = (/** @type {FileAperto} */ f) => f.kind === "empty" ? I18N.t("kind.empty") : KIND[f.kind] || f.kind;
// the three kinds of experiment never share a graph: Dati has one block of panels for each (Full Scan, MS2, MRM); panels and files belong to one
// f.gone = removed from the session (the file on disk is untouched)
const tabFiles = (t = E.tab) => E.files.filter((/** @type {FileAperto} */ f) => f.kind === t && !f.gone);
// the blocks of every experiment type are on the same page (one block per type in the low-resolution world, the cells of the bench in the high-resolution one):
// tabPanels() without an argument is all the panels. A panel still holds ONE experiment type; E.tab is the type of the current file.
const lrTog = () => !(window.BANCO && BANCO.on());
/** @param {string} [t] */ const tabPanels = t => (t === undefined ? E.panels : E.panels.filter((/** @type {Bag} */ p) => p.tab === (t ?? E.tab)));
// group of a file in the list: a user group (lista-file.js) first; otherwise in LR strictly the experiment name (Full Scan, MS2, MRM), in HR bench sub-groups
const grpOf = (/** @type {FileAperto} */ f) => f.ugrp ? "u:" + f.ugrp : (window.BANCO && BANCO.on()) ? (f.kind === "mrm" ? (["standard", "sample", "blank"].includes(f.type) ? f.type : "sample") : "hr." + hrGroup(f)) : kindOf(f);
const GRP_HR = /** @type {Record<string, () => string>} */ ({ "hr.full": () => I18N.t("files.group.full"), "hr.ms2": () => I18N.t("files.group.ms2"), "hr.infusion": () => I18N.t("files.group.infusion") }); const grpLabel = (/** @type {string} */ g) => g.startsWith("u:") ? g.slice(2) : GRP_HR[g] ? GRP_HR[g]() : ["standard", "sample", "blank"].includes(g) ? I18N.t(`files.group.${g}`) : g;
// the entries of a physical file that is open together (the Full Scan and the MS2 part of a DDA file)
const sibsOf = (/** @type {FileAperto} */ f) => E.files.filter((/** @type {FileAperto} */ x) => fileBase(x.file) === fileBase(f.file));
const TOL0 = 1.0;                                   // strumento datato: finestra XIC di +-1 Da

// ------------------------------------------------------------------ schermata di caricamento
// small inline icons (no external files): download arrow and "fit to window" corners
const IC_DL = '<svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-1px;margin-right:4px"><path d="M6 1v7M3 5.5 6 8.5 9 5.5M1.5 11h9"/></svg>';
// peak with its area + sparkle = automatic
const IC_AUTO = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"><path d="M1 14h14"/><path d="M3 14C5 14 6 4.5 8 4.5S11 14 13 14z" fill="currentColor" fill-opacity=".3"/><path d="M13 1.2l.7 1.6 1.6.7-1.6.7-.7 1.6-.7-1.6-1.6-.7 1.6-.7z" fill="currentColor" stroke="none"/></svg>';
// peak between two edge bars = manual
const IC_MAN = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"><path d="M1 14h14"/><path d="M4.5 14C6 14 6.5 5 8 5s2 9 3.5 9" /><path d="M4 2.5v11.5M12 2.5v11.5" stroke-dasharray="2 1.6"/></svg>';
// peak with an X = delete integration
const IC_DEL_INT = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M1 14h14"/><path d="M3 14c1.5 0 2.5-9 5-9s3.5 9 5 9"/><path d="M10.5 2.5l4 4M14.5 2.5l-4 4"/></svg>';
// magnifier = zoom tool
const IC_ZOOM = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"><circle cx="6.5" cy="6.5" r="4.6"/><path d="M10 10l4.6 4.6M4.4 6.5h4.2M6.5 4.4v4.2"/></svg>';
const IC_TAB = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3"><rect x="1.5" y="2" width="11" height="10" rx="1.2"/><path d="M1.5 5.5h11M1.5 8.7h11M6 5.5V12"/></svg>';
// full screen: arrows pointing out
const IC_FS = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><path d="M8.5 1.5h4v4M12.5 1.5 8 6M5.5 12.5h-4v-4M1.5 12.5 6 8"/></svg>';
// leave full screen: arrows pointing in
const IC_FSX = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><path d="M12 5.5H8.5V2M8.5 5.5 13 1M2 8.5h3.5V12M5.5 8.5 1 13"/></svg>';
const IC_SMOOTH = '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M1 11L2.6 7.5 4 10.5 5.6 5.5 7 9.5 8.6 4.5 10 8.5 11.6 6 13 9.5 15 7" stroke-width="1" stroke-opacity=".6"/><path d="M1 11C4 11 5 4 8 4s4 7 7 7" stroke-width="2"/></svg>';
const IC_RANGE = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 2v10M12 2v10M2 7h10M4.5 4.7 2 7l2.5 2.3M9.5 4.7 12 7l-2.5 2.3"/></svg>';
const IC_FIT = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><path d="M1 5V1h4M9 1h4v4M13 9v4H9M5 13H1V9"/></svg>';
const IC_LOCK = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><rect x="2.5" y="6.2" width="9" height="6.3" rx="1.2"/><path d="M4.5 6.2V4.3a2.5 2.5 0 0 1 5 0v1.9"/></svg>';
const IC_UNLOCK = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><rect x="2.5" y="6.2" width="9" height="6.3" rx="1.2"/><path d="M4.5 6.2V4.3a2.5 2.5 0 0 1 4.8-1"/></svg>';
// the loading phrases (ldNext, ldShow, ldRefresh) are in avvio.js: the first one shows at once, then one every 5 s; a status text or loading(true) never swaps the phrase early
let ldSlow = 0;
/** @param {boolean} on @param {string} [msg] */ function loading(on, msg) {
  const L = Q("#loading");
  if (on) {
    ldSince = Date.now(); ldMsg = msg ? { text: msg, pct: null } : null; ldRefresh();
    clearTimeout(ldSlow); ldSlow = setTimeout(() => { if (!window.qqFailed && !Q("#loading").hidden && !ldMsg && !window.qqPhaseCur) { ldMsg = { text: I18N.t("load.slow"), pct: null }; ldRefresh(); } }, 20000);
    // engine start and data loading: the ions (one analyser per load)
    ldIon(true);
    // a phrase already turning (first start) goes on with its 5 s clock
    if (!ldTimer) { ldNext(); ldTimer = setInterval(ldNext, 5000); } L.hidden = false;
  } else {
    // a failed engine start keeps its message (and the «start from scratch» button) on screen
    if (window.qqFailed) return;
    if (window.ldClearBootTimer) window.ldClearBootTimer();
    const wait = Math.max(0, 900 - (Date.now() - ldSince));
    clearTimeout(ldSlow); setTimeout(() => {
      L.classList.add("fade-out"); ldMsg = null;
      // the button of a stuck start goes with its veil (it would come back with the next one)
      setTimeout(() => { L.hidden = true; L.classList.remove("fade-out"); ldIon(false); ldAction(""); clearInterval(ldTimer); clearInterval(ldDotTimer); ldTimer = null; }, 400);
    }, wait);
  }
}

// progress bar for any API call (the work runs in the worker) that lasts more than ~1 s; the loading screen already covers file opening
(() => {
  const bar = () => document.getElementById("busybar"); let pending = 0, timer = 0; const orig = window.fetch.bind(window);
  const update = () => { const b = bar(); if (b) b.hidden = !(pending > 0 && !timer && Q("#loading").hidden && window.qqReady); };
  window.fetch = (input, init) => {
    const url = typeof input === "string" ? input : (input && /** @type {any} */ (input).url) || "";
    if (!/^(\.\/)?api\//.test(url)) return orig(input, init);
    if (!pending++) timer = setTimeout(() => { timer = 0; update(); }, 1000);
    const done = () => { if (!--pending) { clearTimeout(timer); timer = 0; } update(); };
    const p = orig(input, init); p.then(done, done); return p;
  };
})();

// ------------------------------------------------------------------ taccuino (salvato accanto ai dati)
let nbTimer = /** @type {ReturnType<typeof setTimeout> | null} */ (null), nbLast = "", nbBase = /** @type {Bag} */ ({}), nbRemote = /** @type {Bag | null} */ (null);
// schema of the notebook: bump it when a field changes meaning and migrate in nbLoad (2: NB.session in the format campioni/2)
const NB_V = 2;
const nbPlain = (/** @type {unknown} */ v) => !!v && typeof v === "object" && !Array.isArray(v);
// what comes back from the storage can be old, empty or damaged: only a plain object is accepted and the fields the program iterates over must have the right type
function nbClean(/** @type {Bag} */ j) {
  if (!nbPlain(j) || j.error) return {};
  const o = { ...j };
  if ("session" in o && o.session != null && !Array.isArray(o.session)) delete o.session;
  if ("ui" in o && o.ui != null && !nbPlain(o.ui)) delete o.ui;
  if (o.session) o.session = o.session.filter((/** @type {Bag} */ f) => nbPlain(f) && typeof f.file === "string");
  if (o.ui && !Array.isArray(o.ui.panels)) o.ui.panels = [];
  if (o.ui && !Array.isArray(o.ui.files)) o.ui.files = [];
  return o;
}
// two tabs (or a stale tab after the browser was closed and reopened) must not overwrite each other: only the fields THIS tab changed since it read the notebook win over the stored copy
function nbMerge(/** @type {Bag} */ remote) {
  const out = { ...nbClean(remote) }, same = (/** @type {unknown} */ a, /** @type {unknown} */ b) => a === b || JSON.stringify(a) === JSON.stringify(b);
  for (const k of new Set([...Object.keys(NB), ...Object.keys(nbBase)])) {
    if (same(NB[k], nbBase[k])) continue;
    if (NB[k] === undefined) delete out[k]; else out[k] = NB[k];
  }
  out._v = NB_V; out._t = Date.now(); return out;
}
function nbSave(now = false) {
  clearTimeout(/** @type {number} */ (nbTimer));
  const go = async () => {
    // not read yet (a save would erase the stored notebook), or nothing changed: no request (fewer aborted saves on reload)
    if (!nbEverLoaded || JSON.stringify(NB) === nbLast) return;
    let remote = nbRemote;
    if (!now) { try { remote = await (await fetch("api/notebook")).json(); } catch (_) { /* keep the last copy read */ } }
    const out = nbMerge(remote || nbBase), body = JSON.stringify(out);
    const snap = JSON.stringify(NB); nbLast = snap; nbBase = JSON.parse(snap); nbRemote = out; if (now) NBP.keep(body);
    fetch("api/notebook", { method: "POST", body, keepalive: true }).then(r => { if (r && r.ok) NBP.done(body); }).catch(() => { nbLast = ""; });
  };
  if (now) go(); else nbTimer = setTimeout(go, 500);
}
let nbEverLoaded = false;
async function nbLoad() {
  let j = {};
  try { j = nbClean(await (await fetch("api/notebook")).json()); } catch (_) { /* first time, or the storage is not answering */ }
  const again = NBP.take(j, nbClean); if (again) j = again; Object.assign(NB, j); const snap = JSON.stringify(NB); nbBase = JSON.parse(snap); nbRemote = j; nbLast = snap;
  nbEverLoaded = true;
  if (again) { nbLast = ""; nbSave(true); }
  // notebook v1: roles of the Data view, QC and dark roles of qqq.qaqc -> campioni/2, without loss
  if (CDOC.isOld(NB.session)) {
    try { NB.session = await CDOC.migrate(NB.session, (window.QAQC && QAQC.state.roles) || {}); NB._v = NB_V; nbSave(); } catch (_) { /* the notebook stays as it is */ }
  }
  document.dispatchEvent(new Event("nbloaded"));
}
// back in the tab (after another tab or the browser was closed) the fields it did not change are refreshed from the stored notebook: the next save never undoes the other tab
async function nbSync() {
  if (!nbEverLoaded) return;
  try {
    const r = nbClean(await (await fetch("api/notebook")).json()); nbRemote = r;
    for (const k of Object.keys(r)) if (JSON.stringify(NB[k]) === JSON.stringify(nbBase[k]) && k !== "ui" && k !== "session") { NB[k] = r[k]; nbBase[k] = JSON.parse(JSON.stringify(r[k])); }
  } catch (_) { /* offline engine: nothing to refresh */ }
}
document.addEventListener("visibilitychange", () => { if (document.visibilityState === "visible") nbSync(); });
// the Disegno tab can be opened before any data file is loaded: the notebook (with the drawing) must be read first, otherwise the first save
// would overwrite the saved drawing with an empty one
window.nbEnsure = async () => { if (!nbEverLoaded) await nbLoad(); };
/** One panel as the notebook stores it (uiSave); the links are indices into E.panels. @param {any} p */
function panelRec(p) {
  return {
    type: p.type, tab: p.tab, title: p.title, x: p.x, y: p.y, w: p.w, h: p.h, full: !!p.full, riga: p.full ? p.riga : undefined, quota: p.quota ?? undefined, kind: p.kind, smooth: p.smooth, tol: p.tol,
    traces: (p.traces || []).map((/** @type {Bag} */ t) => ({ mz: t.mz, w: t.w, label: t.label, ion: t.ion || undefined, obs: t.obs || undefined, canale: t.canale || undefined })),
    canali: p.canali && p.canali.length ? p.canali : undefined,
    k: E.files[p.k]?.file ?? null, r0: p.r0, r1: p.r1, level: p.level, prec: p.prec, ks: p.ks ? p.ks.map((/** @type {number} */ k) => E.files[k]?.file).filter(Boolean) : undefined, ptol: p.ptol ?? undefined, filt: p.filt || undefined, flv: p.flv ?? undefined, pin: p.pin || undefined, ranges: p.ranges || undefined, norm100: p.norm100 || undefined, all: p.all, zoom: p.zoom, anns: p.anns, ints: p.ints, noise: p.noise || undefined, tr: p.tr,
    link: p.link ? E.panels.findIndex((/** @type {Bag} */ q) => q.id === p.link) : -1, src: p.src ? E.panels.findIndex((/** @type {Bag} */ q) => q.id === p.src) : -1,
    dda: p.dda != null ? E.panels.findIndex((/** @type {Bag} */ q) => q.id === p.dda) : undefined, dock: p.dock != null ? E.panels.findIndex((/** @type {Bag} */ q) => q.id === p.dock) : undefined, duo: p.duo != null ? E.panels.findIndex((/** @type {Bag} */ q) => q.id === p.duo) : undefined, extra: p.extra || undefined, sid: p.sid ?? undefined, ion: p.ion ?? undefined, avg: p.avg || undefined, imode: p.imode || null, intf: p.intf || "",
    uv: p.uv || undefined, lock: p.type === "spec" ? !!p.lock : undefined, rel: p.type === "spec" && typeof p.rel === "boolean" ? p.rel : undefined, sticks: p.type === "spec" && p.sticks ? true : undefined, dots: p.type === "spec" && p.dots ? true : undefined, mk: p.type === "spec" && p.mk != null ? !!p.mk : undefined, noCont: p.type === "spec" && p.noCont ? true : undefined, thr: p.type === "spec" ? p.thr : undefined, nlab: p.type === "spec" ? p.nlab : undefined, dec: p.type === "spec" ? p.dec : undefined, meas: p.type === "spec" && p.meas ? p.meas : undefined, tl: p.tl === undefined ? undefined : !!p.tl, overlay: p.overlay || undefined, etpk: typeof p.etpk === "boolean" ? p.etpk : undefined, pdaAlign: p.pdaAlign || undefined, pdaDelay: p.pdaDelay ?? undefined, gdelay: p.gdelay || undefined, ms2tri: p.ms2tri || undefined, iso: p.iso || null, fondo: p.fondo && p.fondo.mode !== "none" ? p.fondo : undefined, clipDraw: p.clipDraw || undefined, mz0: p.mz0 ?? null, mz1: p.mz1 ?? null, mode: p.mode, log: p.log, hid: p.hid, bk: p.bk === "" || p.bk == null ? "" : E.files[+p.bk]?.file ?? "", adduct: p.adduct, bg: p.bg === "" || p.bg == null ? "" : p.bg === "w" || p.bg === "x" ? p.bg : E.files[+p.bg]?.file ?? "", bw0: p.bw0, bw1: p.bw1, bwi: p.bwi || undefined, scale: p.scale, gamma: p.scale === "gamma" ? p.gamma : undefined, norm: p.norm, corr: p.type === "map" ? p.corr : undefined, bgr: p.type === "map" && p.bgr ? p.bgr : undefined, zoomY: p.zoomY, ref: p.ref === "" || p.ref == null ? "" : E.files[+p.ref]?.file ?? ""
  };
}
let uiTimer = /** @type {ReturnType<typeof setTimeout> | null} */ (null);
function uiSave(now = false) {
  clearTimeout(/** @type {number} */ (uiTimer));
  const run = () => {
    if (!E.files.length) return;
    if (typeof AVVIO !== "undefined" && AVVIO.segnaSessione) AVVIO.segnaSessione(true);
    NB.session = CDOC.session(E.files.filter((/** @type {FileAperto} */ f) => !f.gone));
    NB.ui = {
      contEx: window.LISTE ? LISTE.exclExport() : undefined,
      ref: { ...S.ref, k: S.ref.k === "auto" ? "auto" : E.files[S.ref.k]?.file ?? "auto" },
      tab: E.tab, cur: E.cur, msn: window.INF ? INF.state() : undefined, files: E.files.map((/** @type {FileAperto} */ f) => ({ file: f.file, label: f.label, vis: f.vis, color: f.colorSet ? f.color : undefined, gone: f.gone || undefined, ugrp: f.ugrp || undefined })), ugBy: E.ugBy, ugOrder: E.ugOrder,
      panels: E.panels.map(panelRec)
    };
    nbSave(now);
  };
  if (now) run(); else uiTimer = setTimeout(run, 800);
}
// when the page closes (or the tab goes to the background) save at once: the last change is not lost
// (nbSave too: with no file open uiSave has nothing to write, but the Disegno or Flow fields of NB may still be waiting in the 500 ms timer)
const flushAll = () => { uiSave(true); nbSave(true); };
addEventListener("pagehide", flushAll);
document.addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") flushAll(); });

// colours of the saved images: always the light theme, whatever the page theme is (so a PNG is the same in light and dark); css() everywhere else
const EXPORT_LIGHT = /** @type {Record<string, string>} */ ({ "--panel": "#ffffff", "--line": "#e2e5ea", "--ink": "#25282c", "--muted": "#5e6676", "--accent": "#2b5c8a", "--warn": "#92400e", "--sel": "#edf2f8", "--soft": "#f0f3f6", "--bg": "#ffffff", "--diff-pos": "#D55E00", "--diff-neg": "#0072B2" });
// opaque pale version of a trace colour (85% towards white): the area under a saved cascade trace
const lightTint = (/** @type {string} */ c) => { const m = /^#([0-9a-f]{6})$/i.exec(c); if (!m) return "#ffffff"; const v = parseInt(m[1], 16); return "#" + [v >> 16, v >> 8 & 255, v & 255].map(x => Math.round(x + (255 - x) * .85).toString(16).padStart(2, "0")).join(""); };
/** @param {string} n @returns {any} kept loose: picchi.js builds [sign, colour] pairs from it */ const ecss = n => EXPORTING && n in EXPORT_LIGHT ? EXPORT_LIGHT[n] : css(n);
const fz = () => 1;        // (the text size of the graphs follows the browser zoom: Ctrl/Cmd + and −)
const fpx = (/** @type {number} */ n, st = "") => `${st}${+(n * fz()).toFixed(1)}px system-ui`;
// ------------------------------------------------------------------ data (cached)
const J = (/** @type {string} */ u) => fetch(u).then(async r => { const j = await r.json(); if (j.error) throw I18N.err(j); return j; });
// precursor + scan filter (high-resolution bench: the type of scan chosen in a cell) travel together in the "precursor" slot of the caches and the requests
const prk = (/** @type {unknown} */ pr, /** @type {string | undefined} */ ft) => ft ? `${pr ?? ""}~${ft}` : pr;
const prSplit = (/** @type {unknown} */ pr) => { const t = String(pr ?? ""), i = t.indexOf("~"); return i < 0 ? [/** @type {string} */ (pr ?? ""), ""] : [t.slice(0, i), t.slice(i + 1)]; };
const FT = (/** @type {string | undefined} */ ft) => ft ? `&filt=${encodeURIComponent(ft)}` : "";
// precursor > fragment pairs of an MS2 file (unit resolution)
const getMs2Tr = (/** @type {string | number} */ k) => memo(`m2t${k}`, () => J(`api/ms2tr?k=${k}`)).then((/** @type {Bag} */ d) => d.files[0].transitions);
(() => {
  addEventListener("mzlab-index", (/** @type {any} */ e) => {
    const d = e && e.detail;
    if (!d || !d.name) return;
    // keyed by the base name: the parts of a multi-part file (x.mzML#full) share the early TIC of their file
    const base = fileBase(d.name), parts = E.files.filter((/** @type {FileAperto} */ x) => fileBase(x.file) === base);
    if (d.early_tic) {
      (E.earlyTic || (E.earlyTic = new Map())).set(base, d.early_tic);
      if (parts.length) {
        parts.forEach((/** @type {FileAperto} */ f) => { f._provisional = true; });
        redrawAll();
      }
    } else if (d.phase === "open" || d.frac >= 1) {
      if (E.earlyTic) E.earlyTic.delete(base);
      const shown = parts.filter((/** @type {FileAperto} */ f) => f._provisional);
      if (shown.length) {
        shown.forEach((/** @type {FileAperto} */ f) => { delete f._provisional; });
        redrawAll();
      }
    }
  });
})();
// the provisional TIC is the MS1 one: only a level 1 TIC of the whole file may show it
const earlyTicFor = (/** @type {string | null} */ name, /** @type {number} */ lv, /** @type {Map<string, any> | undefined} */ local, /** @type {((n: string) => any) | undefined} */ lr) =>
  lv === 1 && name ? (local && local.get(fileBase(name))) || (lr && lr(fileBase(name))) : null;
/** @param {string | number} k @param {string} kind @param {number} lv @param {number | string} [mz0] @param {number | string} [mz1] @param {unknown} [prec] @param {string} [filt] @param {number | null} [ptol] the precursor window in Da (default 0.6) */ const getChrom = (k, kind, lv, mz0, mz1, prec, filt, ptol) => {
  const key = `c${k}|${kind}|${lv}|${mz0 ?? ""}|${mz1 ?? ""}|${prec ?? ""}|${filt ?? ""}|${ptol ?? ""}`;
  const f = E.files[k], name = f ? f.file : null;
  const early = (kind === "tic" && mz0 == null && mz1 == null && prec == null && !filt && name) ? earlyTicFor(name, lv, E.earlyTic, (/** @type {any} */ (window).MZLAB_LR)?.earlyTic) : null;
  const url = `api/chrom?k=${k}&kind=${kind}&level=${lv}` + (mz0 != null ? `&mz0=${mz0}` : "") + (mz1 != null ? `&mz1=${mz1}` : "") + (prec ? `&prec=${prec}` + (ptol ? `&ptol=${ptol}` : "") : "") + FT(filt);
  if (early && !CACHE.has(key)) {
    CHROM1.once(key, () => J(url).then(res => {
      if (f) delete f._provisional;
      CACHE.set(key, Promise.resolve(res));
      redrawAll();
    }).catch(() => {}));
    return Promise.resolve({ rt: early.rt, y: early.y || early.tic, provvisorio: true });
  }
  return CHROM1.sent(memo(key, () => J(url)));
};
// api/xicbin: 4 bytes = length of the JSON header {mz, tol, traces:[{k, n}]}, the header, zeros up to a multiple of 8, then per trace rt and y as float64; the same {k, rt, y} as api/xic
async function xicBin(/** @type {string} */ url) {
  const r = await fetch(url);
  if (!r.ok) { let m = new Error(I18N.t("err.http", { status: r.status })); try { const j = await r.json(); if (j && j.error) m = I18N.err(j); } catch (e) { /* not JSON */ } throw m; }
  const buf = await r.arrayBuffer(), hl = new DataView(buf).getUint32(0, true), head = JSON.parse(new TextDecoder().decode(new Uint8Array(buf, 4, hl)));
  let off = (4 + hl + 7) & ~7;
  const traces = head.traces.map((/** @type {Bag} */ t) => { const rt = f64Arr(new Float64Array(buf, off, t.n)), y = f64Arr(new Float64Array(buf, off + 8 * t.n, t.n)); off += 16 * t.n; return { k: t.k, rt, y }; });
  return { mz: head.mz, tol: head.tol, traces };
}
// the caller in grafici.js spreads [mz, tol] from HR.xicArgs, so the middle arguments are a rest list for tsc
// canale: the XIC inside the scans of a channel (api/xic, canali.js), otherwise the whole level
const getXic = /** @type {(k: string | number, ...rest: (number | string | null | undefined)[]) => Promise<Bag>} */ ((k, mz, tol, lv, canale) => canale
  ? memo(`x${k}|${mz}|${tol}|${lv}|${canale}`, () => J(`api/xic?k=${k}&mz=${mz}&tol=${tol}&level=${lv}&canale=${encodeURIComponent(canale)}`).then(j => j.traces ? j.traces[0] : j))
  : memo(`x${k}|${mz}|${tol}|${lv}`, () => xicBin(`api/xicbin?k=${k}&mz=${mz}&tol=${tol}&level=${lv}`).then(j => j.traces[0])));
// + "&hr=0" when high resolution is switched off (hr.js)
const MERGE = () => (UIP.merge ? "&merge=1" : "") + (window.HR ? HR.q() : "");
const getSpec = (/** @type {string | number} */ k, /** @type {number | string} */ a, /** @type {number | string} */ b, /** @type {number} */ lv, /** @type {unknown} */ pr, /** @type {number[] | null | undefined} */ bg) => memo(`s${k}|${a}|${b}|${lv}|${pr}|${bg ? bg.join(",") : ""}|${UIP.merge ? 1 : 0}`, () => J(`api/spectrum?k=${k}&rt0=${a}&rt1=${b}&level=${lv}&precursor=${prSplit(pr)[0]}${FT(prSplit(pr)[1])}` + (bg ? `&bgk=${bg[0]}&bgrt0=${bg[1]}&bgrt1=${bg[2]}&bgf=${bg[3] ?? 1}&bgclip=${bg[4] ?? 1}` : "") + MERGE()));
const getFormula = (/** @type {string} */ f, /** @type {string | undefined} */ ad) => J(`api/formula?f=${encodeURIComponent(f)}&adduct=${encodeURIComponent(ad || "")}`);
// default adduct from the polarity of the visible files
// polarity of the files (read from the mzML): a small sign next to the name, the default adduct of a graph follows ITS files
const POLS = /** @type {Record<string, string[]>} */ ({ positive: ["+", "pol.positive"], negative: ["−", "pol.negative"], mixed: ["±", "pol.mixed"], unknown: ["?", "pol.unknown"] });
// the polarity as a small readable badge "ESI+" / "ESI−" (never a bare sign); in lists it appears ONLY when the files of the session do not all have the same polarity
const polKnown = (/** @type {FileAperto[] | null | undefined} */ list) => new Set((list || []).map((/** @type {FileAperto} */ f) => f && f.polarity).filter((/** @type {unknown} */ x) => x && x !== "unknown"));
const polAll = () => polKnown(E.files.filter((/** @type {FileAperto} */ f) => !f.gone));
const polLabel = (/** @type {Bag | null | undefined} */ f) => (/** @type {Record<string, string>} */ ({ positive: "ESI+", negative: "ESI\u2212", mixed: "ESI\u00b1" })[f && f.polarity] || "ESI?");
const polSign = /** @type {(f: Bag | null | undefined, list?: FileAperto[] | null) => string} */ ((f, list) => { const q = POLS[f && f.polarity] || POLS.unknown; if ((list ? polKnown(list) : polAll()).size < 2) return ""; return `<span class="pol" title="${I18N.t(q[1])}">${polLabel(f)}</span>`; });
// the polarity written once in the heading of a group when all its files share it: " · ESI+"
const polMixed = () => new Set(shown().map((/** @type {FileAperto} */ f) => f.polarity).filter((/** @type {unknown} */ x) => x && x !== "unknown")).size > 1;
// the legend shows the sign only when positive and negative files are drawn together
const legPol = (/** @type {FileAperto | null | undefined} */ f) => polMixed() && f ? " " + polSign(f, shown()) : "";
/** @param {FileAperto[]} [fs] */ const defAdduct = fs => { const l = fs || (shown().length ? shown() : E.files.filter((/** @type {FileAperto} */ f) => f.vis)); return l.some((/** @type {FileAperto} */ f) => f.polarity === "negative") && !l.some((/** @type {FileAperto} */ f) => f.polarity === "positive") ? "[M-H]-" : "[M+H]+"; };
const getMrm = (/** @type {string | number} */ k) => memo(`m${k}`, () => J(`api/mrm?k=${k}`).then(j => j.files[0].transitions));

// a method (.dam) of another experiment type than the open files (MRM against Full Scan...): a visible notice above the panels, not a toast
async function methodWarn() {
  const el = Q("#methwarn"); if (!el) return;
  try {
    const j = await J("api/method_warnings");
    if (!j.warnings.length) { el.hidden = true; el.innerHTML = ""; return; }
    el.innerHTML = I18N.t("methwarn.title") + " " + j.warnings.map((/** @type {Bag} */ w) => I18N.t("methwarn.item", { method: EH(w.method), expects: EH(w.expects), files: EH(w.files.length === 2 ? I18N.t("methwarn.and", { a: w.files[0], b: w.files[1] }) : w.files.join(", ")) })).join(" ") + ` <button type="button" id="methwarn-x" title="${I18N.t("methwarn.hide")}">&times;</button>`;
    el.hidden = false; Q("#methwarn-x").onclick = () => { el.hidden = true; };
  } catch (_) { /* no session or no method: nothing to say */ }
}
async function bootSession() { await bootSession0(); methodWarn(); window.TOUR && TOUR.offri(); }
const PEND = new WeakMap();      // object -> { key: { k: placeholder index, name: file name } }
/** A file index follows its file name; a file not back yet (list reopened file by file) keeps its name in PEND and is found at a later step.
 *  @param {any} o @param {string} key @param {any} none value while the file is missing @param {string[]} oldNames @param {{file: string}[]} files */
function remapFileIndex(o, key, none, oldNames, files) {
  const v = o[key], pend = (PEND.get(o) || {})[key];
  // the placeholder written at an earlier step (even "") is still in place: the file may be back now
  const pending = !!pend && String(pend.k) === String(v);
  if (!pending && (v == null || v === "" || isNaN(+v))) return;
  const name = pending ? pend.name : oldNames[+v], i = files.findIndex(f => f.file === name);
  const out = i >= 0 ? (typeof v === "string" ? String(i) : i) : none;
  if (i >= 0) { if (pend) delete PEND.get(o)[key]; } else if (name != null) PEND.set(o, { ...(PEND.get(o) || {}), [key]: { k: out, name } });
  o[key] = out;
}
async function bootSession0() {
  const j = await J("api/session");
  S.sess = j.session;
  if (typeof AVVIO !== "undefined" && AVVIO.segnaSessione) AVVIO.segnaSessione(!!j.session);
  if (!j.session) { E.files = []; E.ugBy = E.ugOrder = undefined; E.key = ""; E.pending = []; E._seq = false; E._mem = null; E.progTok = null; return; }
  const key = JSON.stringify(j.session.map((/** @type {Bag} */ s) => s.file));
  if (key === E.key) return;
  const oldNames = E.files.map((/** @type {FileAperto} */ f) => f.file), mem = E._mem || {};
  // a list reopened file by file: the files not back yet keep their state (visibility, label, role) until they return
  E.files.forEach((/** @type {FileAperto} */ f) => { mem[f.file] = f; });
  const oldVis = mem;
  E.key = key;
  CACHE.clear(); CHROM1.clear(); if (typeof SC !== "undefined") { SC.m.clear(); SC.n.clear(); }
  // seq: a list opened file by file, this is not its first step
  const seq = !!E._seq, waiting = !!(E.pending && E.pending.some((/** @type {Pendiente} */ x) => x.state === "wait"));
  E._seq = waiting; E._mem = waiting ? mem : null;
  // while a list is opened file by file the new ones follow the rules of the first load
  const isFirstLoad = !oldNames.length || seq || waiting, first = firstOpenK(j.session);
  E.files = j.session.map((/** @type {Bag} */ s, /** @type {number} */ k) => {
    // a file new to the session starts hidden, LR and HR alike (first load and additions): until a click the work area shows the sample table; a restore gives the saved visibility back (restoreUi)
    const vis = oldVis[s.file] ? oldVis[s.file].vis && !(isFirstLoad && isBlankFile(s) && fileBase(s.file) !== fileBase(j.session[first].file)) : false;
    const t = TIPOF.apply(s, s.file);
    return { ...t, ...CAMPIONI.keep(oldVis[s.file]), k, vis, label: noExt(noFullScan(oldVis[s.file] ? oldVis[s.file].label : s.label, s.dda)), lv: t.kind === "ms2" ? 2 : 1 };
  }); uniqueLabels(E.files);
  paintFiles();
  if (window.HR) HR.notice(E.files);
  // a file index follows its file name; a file not back yet (list reopened file by file) keeps its name in PEND and is found at a later step
  const remap = (/** @type {any} */ o, /** @type {string} */ key, /** @type {any} */ none) => remapFileIndex(o, key, none, oldNames, E.files);
  if (E.panels.length && oldNames.length) {                // later load: the panels stay
    E.panels.forEach((/** @type {Bag} */ p) => { remap(p, "k", 0); remap(p, "ref", 0); remap(p, "bk", ""); if (p.bg !== "w" && p.bg !== "x") remap(p, "bg", ""); });
    remap(E, "cur", 0);
    if (S.ref.k !== "auto") remap(S.ref, "k", 0);
    STATO.file(Math.min(E.cur, E.files.length - 1), "reload");
    if (isFirstLoad && E.files[E.cur] && isBlankFile(E.files[E.cur]) && !isBlankFile(E.files[first])) activeFile(first);
    renderFileList();
    // @ts-ignore toolbar() below is also window.toolbar (a BarProp) for lib.dom
    toolbar(); E.panels.forEach(ctl); redrawAll();
    const added = E.files.filter((/** @type {FileAperto} */ f) => !oldNames.includes(f.file)); if (added.length && window.setTab && !seq && !(typeof OPENNOW !== "undefined" && OPENNOW.batchShown()) && added[0].kind !== E.tab) setTab(added[0].kind); else if (added.length) { ensureLayout(); }
    if (added.length && window.INF && INF.autoOpen) INF.autoOpen(added);
    return;
  }
  if (first > 0 && isFirstLoad) STATO.file(first, "first"); renderFileList();
  STATO.pannelli([], "clear"); Q("#dpanels").innerHTML = ""; syncWorkEmpty();
  await nbLoad();
  CAMPIONI.restore(NB.session);
  if (!restoreUi()) STATO.scheda(pickTab(E.tab), "reload");
  ensureLayout();
  // @ts-ignore toolbar() is also window.toolbar (a BarProp) for lib.dom
  toolbar(); uiSave();
  if (window.INF && INF.autoOpen) INF.autoOpen(E.files);
}
// the spectrum starts at the retention time with the highest intensity (of all the visible full-scan/MS2 files)
async function apexSpectrum(/** @type {Bag} */ c, /** @type {Pannello} */ sp) {
  try {
    const all = (await rawSeries(c)).filter((/** @type {Bag} */ s) => E.files[s.k] && E.files[s.k].kind !== "mrm");
    let best = null;
    for (const s of all) for (let i = 0; i < s.y.length; i++) if (!best || s.y[i] > best.v) best = { v: s.y[i], rt: s.x[i], k: s.k };
    if (!best) return;
    const f = E.files[best.k], dt = Math.max((f.rt_max - f.rt_min) / Math.max(f.ms1 + f.ms2 - 1, 1), 0.005);
    c.cur = best.rt; Object.assign(sp, { k: best.k, level: f.lv, r0: best.rt - dt / 2, r1: best.rt + dt / 2, zoom: null });
    ctl(sp); draw(c); draw(sp); uiSave();
  } catch (_) { /* the spectrum simply waits for a click */ }
}
// width available for the panels (the panel area may still be hidden while the files are being opened)
function hostWidth() {
  const w = Q("#dpanels").clientWidth;
  return w > 100 ? w : Math.max(600, innerWidth - 28 - (window.BARRA ? BARRA.dockedWidth : 0));
}
// full-width panels follow the page: when the window or the file list changes size
function fitWidth() {
  const w = Q("#dpanels").clientWidth; if (w < 100) return;
  const st = stackOrder();
  E.panels.forEach((/** @type {Pannello} */ p) => { if (p.full && !dockedP(p) && !hasDocked(p) && !LAYOUT.inRiga(p, st) && Math.abs(p.w - w) > 1) { p.x = 0; p.w = w; apply(p); } });
  // panels side by side follow the width too (and stack when it is too narrow)
  const rows = st.some((/** @type {Bag} */ q) => LAYOUT.inRiga(q, st));
  // Full Scan and MS2 of a DDA file side by side only when the page is wide enough
  if (rows || window.DDA && E.panels.some((/** @type {Bag} */ q) => q.dock != null)) { relayout(); if (window.DDA) DDA.sync(); }
}
/** Opens a panel from its notebook record (restoreUi, and the Undo of a closed panel); the links to other panels are set by the caller. @param {any} sp */
function riapriPannello(sp) {
  const kof = (/** @type {string} */ n) => { const i = E.files.findIndex((/** @type {FileAperto} */ f) => f.file === n); return i < 0 ? 0 : i; };
  return addPanel(sp.type, {
    ...sp, ks: sp.ks ? sp.ks.map(kof) : undefined, tab: sp.tab || (sp.type === "mrm" ? "mrm" : E.files[kof(sp.k)]?.kind === "ms2" ? "ms2" : "full"), k: sp.k ? kof(sp.k) : undefined, ref: sp.ref ? kof(sp.ref) : "", bk: sp.bk ? kof(sp.bk) : "", bg: sp.bg === "w" || sp.bg === "x" ? sp.bg : sp.bg ? kof(sp.bg) : "", traces: (sp.traces || []).map((/** @type {any} */ t) => ({ id: E.seq++, ...t })), link: null, ints: sp.ints || [], anns: sp.anns || []
  });
}
function restoreUi() {
  const u = NB.ui; if (u) { E.ugBy = u.ugBy; E.ugOrder = u.ugOrder; }
  // the contaminant exclusions travel with the session
  if (u && u.contEx && window.LISTE) LISTE.exclImport(u.contEx);
  // the grouping of the MSn tree travels with the session
  if (u && u.msn && window.INF) INF.restore(u.msn);
  if (!u || !u.panels || !u.panels.length) return false;
  const names = new Set(E.files.map((/** @type {FileAperto} */ f) => f.file));
  // removed files are not in the restored session (their entry only records the removal): they must not make the whole restore fail
  const saved = u.files.filter((/** @type {Bag} */ sf) => !sf.gone || names.has(sf.file));
  if (!saved.every((/** @type {FileAperto} */ f) => names.has(f.file))) return false;
  saved.forEach((/** @type {Bag} */ sf) => { const f = E.files.find((/** @type {FileAperto} */ x) => x.file === sf.file); f.label = sf.label || f.label; if (sf.ugrp) f.ugrp = sf.ugrp; f.vis = sf.vis !== false; if (sf.color) { f.color = sf.color; f.colorSet = true; } if (sf.gone) { f.gone = true; f.vis = false; } });
  STATO.file(Math.min(u.cur || 0, E.files.length - 1), "notebook"); migrateBrowse(u); STATO.scheda(pickTab(u.tab), "notebook");
  const kof = (/** @type {string} */ n) => { const i = E.files.findIndex((/** @type {FileAperto} */ f) => f.file === n); return i < 0 ? 0 : i; };
  if (u.ref) { const r = u.ref, i = r.k === "auto" ? -1 : E.files.findIndex((/** @type {FileAperto} */ f) => f.file === r.k); S.ref = { ...S.ref, ...r, k: i < 0 ? "auto" : i, on: { ...(r.on || {}) } }; }
  const made = u.panels.map((/** @type {Bag} */ sp) => riapriPannello(sp));
  u.panels.forEach((/** @type {Bag} */ sp, /** @type {number} */ i) => { if (sp.link >= 0 && made[sp.link]) made[i].link = made[sp.link].id; if (sp.src >= 0 && made[sp.src]) made[i].src = made[sp.src].id; });
  // DDA trio: Full Scan / MS2 side by side (dda.js)
  u.panels.forEach((/** @type {Bag} */ sp, /** @type {number} */ i) => { for (const k of ["dda", "dock", "duo"]) if (sp[k] != null) made[i][k] = sp[k] >= 0 && made[sp[k]] ? made[sp[k]].id : null; });
  if (window.DDA) DDA.restored(made);
  // one live spectrum per chromatogram, even in an old notebook
  made.forEach((/** @type {Pannello} */ q) => { if (q.type === "spec" && q.link) ctl(q); });
  if (E.files[E.cur]?.kind !== E.tab) STATO.file((tabFiles()[0] || { k: 0 }).k, "notebook");
  // a session saved with one type at a time: the blocks go one under the other
  if (typeof stackBlocks === "function") stackBlocks();
  renderFileList(); return true;
}

// File colours. Full Scan files of type sample WITH a time: one sequential scale ordered by time, spread over the n files of the tab (rank, not the real
// time, so 7 files at 0-5-10-15-30-45-60 min are as far apart as 3 files; hiding a file does not change any colour, removing one does). Blank and
// Full Scan standards are outside the scale (neutral grey, dashed); Full Scan samples without a time use the categorical colours. MS2 and MRM files keep
// the categorical colours (MRM: transition, not time). A colour chosen by the student (colorSet) is never overwritten.
function paintFiles() {
  setPal(UIP.pal);
  const P = PALS[UIP.pal], dark = isDark();
  const blankCol = dark && P.blankDark ? P.blankDark : P.blank;
  const stdCol = dark && P.stdDark ? P.stdDark : P.std;
  const byTime = (/** @type {string} */ kind) => E.files.filter((/** @type {FileAperto} */ f) => !f.gone && f.kind === kind && f.type === "sample" && f.time != null && !(window.CDOC && CDOC.kindOf(f) === "dark")).sort((/** @type {FileAperto} */ a, /** @type {FileAperto} */ b) => a.time - b.time || a.k - b.k);
  const scales = [byTime("full"), byTime("mrm")], inScale = new Set(scales.flat());
  E.files.forEach((/** @type {FileAperto} */ f) => { f.rank = null; });
  const cal = E.files.filter((/** @type {FileAperto} */ f) => !f.gone && f.kind === "mrm" && f.type === "standard" && f.conc != null).sort((/** @type {FileAperto} */ a, /** @type {FileAperto} */ b) => a.conc - b.conc || a.k - b.k), inCal = new Set(cal);
  PLAIN = !!P.plain || (UIP.pal === "time" && !inScale.size && !cal.length);
  // nothing to draw on a time scale: a different colour per file, never grey or black
  if (PLAIN) {
    let m = 0;
    E.files.forEach((/** @type {FileAperto} */ f) => { if (f.type === "blank") { if (!f.colorSet) f.color = blankCol; } else if (!f.colorSet) f.color = PAL[m++ % PAL.length]; else m++; });
    return;
  }
  scales.forEach(scale => scale.forEach((/** @type {FileAperto} */ f, /** @type {number} */ i) => { f.rank = i; if (!f.colorSet) f.color = seqColor(scale.length > 1 ? i / (scale.length - 1) : 0.5); }));
  cal.forEach((/** @type {FileAperto} */ f, /** @type {number} */ i) => { if (!f.colorSet) f.color = stdColor(cal.length > 1 ? i / (cal.length - 1) : 0.6); });
  let n = 0;
  E.files.forEach((/** @type {FileAperto} */ f) => {
    if (inScale.has(f) || inCal.has(f)) return;
    const own = f.colorSet;
    if (f.type === "blank") { if (!own) f.color = blankCol; }
    else if (f.type === "standard") { if (f.kind === "full") { if (!own) f.color = stdCol; } else if (!own) f.color = PAL[(n++ + 3) % PAL.length]; }
    else if (!own) f.color = PAL[n++ % PAL.length]; else n++;
  });
}
// back to the automatic colours of the current palette (the student's own colours are dropped)
function resetColors() { E.files.forEach((/** @type {FileAperto} */ f) => { f.colorSet = false; }); paintFiles(); renderFileList(); redrawAll(); uiSave(); }
// a file of another experiment type: that type becomes the current one (toolbar, precursor list) and the page goes to its block
function goToFileTab(/** @type {FileAperto} */ f) {
  setTab(f.kind); STATO.file(f.k, "file"); renderFileList(); renderNav(); redrawAll(); uiSave(); scrollToBlock(f.kind);
}
const reorderFiles = (/** @type {number} */ fromK, /** @type {number} */ toK, /** @type {boolean | undefined} */ isBefore) => fileViewDo(() => reorderFilesRaw(fromK, toK, isBefore));
function reorderFilesRaw(/** @type {number} */ fromK, /** @type {number} */ toK, /** @type {boolean | undefined} */ isBefore) {
  if (fromK == null || toK == null || fromK === toK) return;
  const srcIdx = E.files.findIndex((/** @type {FileAperto} */ f) => f.k === fromK);
  const dstIdx = E.files.findIndex((/** @type {FileAperto} */ f) => f.k === toK);
  if (srcIdx < 0 || dstIdx < 0) return;
  const [src] = E.files.splice(srcIdx, 1);
  let insertAt = E.files.findIndex((/** @type {FileAperto} */ f) => f.k === toK);
  if (!isBefore) insertAt++;
  E.files.splice(insertAt, 0, src);

  const oldKToNewK = new Map();
  E.files.forEach((/** @type {FileAperto} */ f, /** @type {number} */ newIdx) => {
    oldKToNewK.set(f.k, newIdx);
    f.k = newIdx;
  });

  if (oldKToNewK.has(E.cur)) STATO.file(oldKToNewK.get(E.cur), "order");
  if (S.ref.k !== "auto") S.ref.k = oldKToNewK.has(S.ref.k) ? oldKToNewK.get(S.ref.k) : "auto";
  if (E.curBy) {
    for (const t in E.curBy) {
      if (oldKToNewK.has(E.curBy[t])) E.curBy[t] = oldKToNewK.get(E.curBy[t]);
    }
  }

  E.panels.forEach((/** @type {Bag} */ p) => {
    if (p.k != null && oldKToNewK.has(p.k)) p.k = oldKToNewK.get(p.k);
    if (p.ref !== "" && p.ref != null && oldKToNewK.has(+p.ref)) p.ref = oldKToNewK.get(+p.ref);
    if (p.bk !== "" && p.bk != null && oldKToNewK.has(+p.bk)) p.bk = oldKToNewK.get(+p.bk);
    if (p.bg !== "" && p.bg != null && p.bg !== "w" && p.bg !== "x" && oldKToNewK.has(+p.bg)) p.bg = oldKToNewK.get(+p.bg);
    if (Array.isArray(p.traces)) {
      p.traces.forEach((/** @type {Bag} */ tr) => {
        if (tr.k != null && oldKToNewK.has(tr.k)) tr.k = oldKToNewK.get(tr.k);
      });
    }
  });

  renderFileList();
  redrawAll();
  uiSave();
}

const flCompact = () => !!(/** @type {any} */ (window).matchMedia && matchMedia("(pointer:coarse), (max-width:1100px)").matches);
function flGrpGet() { return lsObj(NOMI.KEY("flgrp")); }
function flGrpSet(/** @type {Bag} */ m) { E._flgrp = m; try { localStorage.setItem(NOMI.KEY("flgrp"), JSON.stringify(m)); } catch (_) { /* no storage */ } }
// the rows of the list: a click selects the file (the active one), a double click on the name shows only that file (rename: right click), on the colour square it opens the colour picker (called by renderFileList)
function bindFileRows() {
  bindListaFile();
  // the WHOLE row selects the file (colour square, time, empty space); only the check box shows / hides. The list is not rebuilt on a click (only the classes change), so a double click on the name still reaches it.
  Q("#flst").querySelectorAll(".fl:not(.pr)").forEach((/** @type {ElementoDati} */ row) => {
    const nm = row.querySelector(".nm"); if (!nm) return;
    row.onclick = (/** @type {PointerEvent} */ e) => {
      if (/** @type {HTMLElement} */ (e.target).closest("input") || /** @type {HTMLElement} */ (e.target).closest(".pr-tog")) return;
      const f0 = E.files[+nm.dataset.k];
      // the first click (nothing shown yet, the work area holds the sample table): only this file, with the starting layout of its type
      if (f0 && !E.files.some((/** @type {FileAperto} */ f) => f.vis && !f.gone)) { soloFile(f0); return; }
      if (f0 && f0.kind !== "mrm") followSpectra(f0);
      // high resolution: a click on a file of the list that is not open opens it (it joins the open ones)
      if (window.BANCO && BANCO.on() && f0 && !f0.vis && f0.kind !== "mrm") {
        sibsOf(f0).forEach((/** @type {FileAperto} */ f) => { f.vis = true; }); STATO.file(f0.k, "file"); renderFileList(); renderNav(); redrawAll(); uiSave(); return;
      }
      // a hidden file (or an empty work area): the file opens with the starting layout of its type (MRM: its chromatograms; Full Scan: TIC + spectrum...); panels left without a visible file are closed, none stays empty
      if (lrTog() && f0 && (!E.panels.length || !f0.vis)) {
        const far = E.panels.length && f0.kind !== E.tab;
        sibsOf(f0).forEach((/** @type {FileAperto} */ f) => { f.vis = true; }); STATO.file(f0.k, "file"); setTab(f0.kind); ensureLayout(); cleanupOrphanPanels(); renderFileList(); renderNav(); redrawAll(); uiSave();
        if (far) scrollToBlock(f0.kind); return;
      }
      if (lrTog() && f0 && f0.kind !== E.tab) return goToFileTab(f0);
      STATO.file(+nm.dataset.k, "file"); Q("#flst").querySelectorAll(".fl").forEach((/** @type {ElementoDati} */ r) => { r.classList.toggle("cur", r === row); if (r === row) r.setAttribute("aria-current", "true"); else r.removeAttribute("aria-current"); }); renderNav(); if (E.browse) redrawAll();
    };
  });
}

// HTML5 drag and drop of one row of the file list
function bindDragDrop(/** @type {ElementoDati} */ el, /** @type {number} */ k) {
  el.ondragstart = (/** @type {DragEvent} */ e) => {
    /** @type {DataTransfer} */ (e.dataTransfer).setData("text/plain", String(k));
    /** @type {DataTransfer} */ (e.dataTransfer).effectAllowed = "move";
    el.classList.add("fl-dragging"); carica("gruppi-dnd.js").then(() => GDND.start(k));
  };
  el.ondragover = (/** @type {DragEvent} */ e) => {
    e.preventDefault();
    /** @type {DataTransfer} */ (e.dataTransfer).dropEffect = "move";
    const rect = el.getBoundingClientRect();
    if (e.clientY < rect.top + rect.height / 2) {
      el.classList.add("drop-before");
      el.classList.remove("drop-after");
    } else {
      el.classList.add("drop-after");
      el.classList.remove("drop-before");
    }
  };
  el.ondragleave = () => {
    el.classList.remove("drop-before", "drop-after");
  };
  el.ondrop = (/** @type {DragEvent} */ e) => {
    e.preventDefault();
    const fromK = +/** @type {DataTransfer} */ (e.dataTransfer).getData("text/plain");
    const isBefore = el.classList.contains("drop-before");
    el.classList.remove("drop-before", "drop-after");
    if (!isNaN(fromK) && fromK !== k) {
      reorderFiles(fromK, k, isBefore);
    }
  };
  el.ondragend = () => {
    document.querySelectorAll("#flst .fl").forEach(f => f.classList.remove("fl-dragging", "drop-before", "drop-after"));
  };
}

// drag and drop (and long press on touch screens) to reorder the files of the list (called by renderFileList)
function bindFileReorder() {
  // Reorder files via HTML5 drag & drop and touch long-press
  let touchTimer = /** @type {ReturnType<typeof setTimeout> | null} */ (null), touchDragging = /** @type {number | null} */ (null), touchStartX = 0, touchStartY = 0;
  Q("#flst").querySelectorAll(".fl[data-k]:not(.pr)").forEach((/** @type {ElementoDati} */ el) => {
    const k = +el.dataset.k;

    bindDragDrop(el, k);

    el.onpointerdown = (/** @type {PointerEvent} */ e) => {
      if (/** @type {HTMLElement} */ (e.target).closest("input") || /** @type {HTMLElement} */ (e.target).closest(".pr-tog") || e.button !== 0) return;
      touchStartX = e.clientX; touchStartY = e.clientY; carica("gruppi-dnd.js");
      touchTimer = setTimeout(() => {
        touchDragging = k; carica("gruppi-dnd.js").then(() => GDND.start(k, true));
        el.classList.add("fl-dragging");
        try { el.setPointerCapture(e.pointerId); } catch (_) { /* the pointer is already gone: dragging still works without capture */ }
      }, 350);
    };

    el.onpointermove = (/** @type {PointerEvent} */ e) => {
      if (!touchDragging && touchTimer) {
        if (Math.hypot(e.clientX - touchStartX, e.clientY - touchStartY) > 8) {
          clearTimeout(touchTimer);
          touchTimer = null;
        }
      }
      if (touchDragging != null) {
        if (window.GDND && GDND.move(e.clientY)) { document.querySelectorAll("#flst .fl").forEach(f => f.classList.remove("drop-before", "drop-after")); return; }
        const target = /** @type {HTMLElement | null | undefined} */ (document.elementFromPoint(e.clientX, e.clientY)?.closest(".fl[data-k]:not(.pr)"));
        document.querySelectorAll("#flst .fl").forEach(f => {
          if (f !== target) f.classList.remove("drop-before", "drop-after");
        });
        if (target && target !== el) {
          const rect = target.getBoundingClientRect();
          if (e.clientY < rect.top + rect.height / 2) {
            target.classList.add("drop-before");
            target.classList.remove("drop-after");
          } else {
            target.classList.add("drop-after");
            target.classList.remove("drop-before");
          }
        }
      }
    };

    const endTouch = (/** @type {PointerEvent} */ e) => {
      if (touchTimer) {
        clearTimeout(touchTimer);
        touchTimer = null;
      }
      if (touchDragging != null) {
        const fromK = touchDragging;
        touchDragging = null;
        try { el.releasePointerCapture(e.pointerId); } catch (_) { /* capture already released with the pointer */ }
        if (e.type === "pointerup" && window.GDND && GDND.end(e.clientY)) { document.querySelectorAll("#flst .fl").forEach(f => f.classList.remove("fl-dragging", "drop-before", "drop-after")); return; }
        const target = /** @type {HTMLElement | null | undefined} */ (document.elementFromPoint(e.clientX, e.clientY)?.closest(".fl[data-k]:not(.pr)"));
        const isBefore = target?.classList.contains("drop-before");
        document.querySelectorAll("#flst .fl").forEach(f => f.classList.remove("fl-dragging", "drop-before", "drop-after"));
        if (target && +(/** @type {string} */ (target.dataset.k)) !== fromK) {
          reorderFiles(fromK, +(/** @type {string} */ (target.dataset.k)), isBefore);
        }
      }
    };
    el.onpointerup = endTouch;
    el.onpointercancel = (/** @type {PointerEvent} */ e) => { if (touchDragging != null && window.GDND) GDND.cancel(); endTouch(e); };
  });
}

// short tag of a standard / blank file (3 letters, the full word in the tooltip)
const typeTag = (/** @type {boolean} */ blank) => `<abbr class="ftag" title="${blank ? I18N.t("files.type.blk.title") : I18N.t("files.type.std.title")}">${blank ? I18N.t("files.type.blk") : I18N.t("files.type.std")}</abbr>`;
function renderFileList() {
  if (window.GDND && GDND.busy()) return GDND.defer(); if (window.GRUPPI) GRUPPI.adopt();
  // files grouped by experiment type (Full scan, MS2, MRM...): the type is written once per group, not per file. ALL the files are listed, all of them live
  const groups = /** @type {{ g: string, fs: FileAperto[] }[]} */ ([]);
  // both worlds list the files of every experiment, all of them live
  const side = window.BANCO && BANCO.on() ? E.files.filter((/** @type {FileAperto} */ f) => !f.gone && f.kind !== "mrm") : E.files.filter((/** @type {FileAperto} */ f) => !f.gone);
  side.forEach((/** @type {FileAperto} */ f) => { const g = grpOf(f); let G = groups.find(x => x.g === g); if (!G) groups.push(G = { g, fs: [] }); G.fs.push(f); });
  // a DDA file is ONE row (its Full Scan and MS2 parts open together)
  groups.forEach(G => { G.fs = G.fs.filter((/** @type {FileAperto} */ f, /** @type {number} */ i, /** @type {FileAperto[]} */ a) => !f.dda || a.findIndex((/** @type {FileAperto} */ x) => x.dda && fileBase(x.file) === fileBase(f.file)) === i); });
  if (E.ugBy && E.ugBy.by !== "custom" && E.ugBy.by !== "flat") carica("gruppi.js").then(() => GRUPPI.follow());
  if (window.GRUPPI) GRUPPI.order(groups); ugArrange(groups);
  if (window.BANCO && BANCO.on() && E.tab === "mrm") groups.sort((a, b) => ["standard", "sample", "blank"].indexOf(a.g) - ["standard", "sample", "blank"].indexOf(b.g));
  const sub = (/** @type {FileAperto} */ f) => f.type === "sample" ? ((ESP.voce(f.kind) || {}).tempo ? fmtTime(f.time) : "") : (f.type === "blank" ? typeTag(true) : typeTag(false) + (f.conc != null && f.kind === "mrm" ? " " + f.conc + " " + (f.cunit || "") : ""));
  const row = (/** @type {FileAperto} */ f) => {
    const hasPr = lrTog() && f.kind === "ms2" && f.precursors && f.precursors.length;
    const prOpen = f._prOpen !== false;
    const prList = hasPr ? [...new Set(f.precursors)].sort((a, b) => a - b) : [];
    return `<div class="fl ${f.k === E.cur ? "cur" : ""}"${f.k === E.cur ? ' aria-current="true"' : ""} data-tip="${EH(f.label)}" data-k="${f.k}" draggable="true" style="flex-wrap:wrap">
      <input type="checkbox" data-k="${f.k}" ${f.vis ? "checked" : ""} title="${I18N.t("files.showHide")}">
      <i style="background:${f.color}"></i><div class="fi"><b class="nm" data-k="${f.k}">${EH(f.label)}</b><span style="display:inline-flex;align-items:center;gap:4px">${polSign(f)}<small>${sub(f)}</small>${f._provisional ? `<span class="sm muted provvisorio" title="${I18N.t("load.ph.index")}"> ⏳</span>` : ""}${hasPr ? `<span class="pr-tog${prOpen ? " on" : ""}" role="button" tabindex="0" aria-expanded="${prOpen}" aria-label="${I18N.t("files.showHidePrec")}" data-prk="${f.k}" title="${I18N.t("files.showHidePrec")}"></span>` : ""}</span></div>
      <button type="button" class="fl-x" data-rm="${f.k}" aria-label="${EH(I18N.t("files.row.remove", { name: f.label }))}" title="${EH(I18N.t("files.row.remove", { name: f.label }))}">×</button>
      ${hasPr ? `<div class="pr-container" style="padding-left:22px;width:100%;box-sizing:border-box;margin-top:2px;${prOpen ? "" : "display:none;"}">` +
        prList.map(pr => `<div class="fl pr" style="padding-left:0;min-height:0;margin-bottom:2px;border-bottom:none"><input type="checkbox" data-pr="${pr}" data-pk="${f.k}" id="pr-${f.k}-${EH(pr)}" ${ms2RowOn(pr, f.k) ? "checked" : ""} title="${I18N.t("files.addPair.title")}" aria-label="${EH(f.label)} · ${EH(I18N.t("files.prec.row", { mz: String(pr) }))}"><div class="fi"><b class="pn" data-pg="${pr}" data-pk="${f.k}" title="${I18N.t("files.precGo.title")}">${pr}</b></div></div>`).join("") +
        '</div>' : ""}
    </div>`;
  };
  const pre = window.DDA && DDA.listMode() ? DDA.listBlock() : "";
  E._grpOpen = E._grpOpen || {};
  const isExpOpen = (/** @type {string} */ t) => E._grpOpen[t] !== undefined ? E._grpOpen[t] : (ESP.voce(t) || {}).chiuso ? false : (t === E.tab || lrTog() || t.startsWith("u:"));
  // the open/closed state of a group: one per experiment type in the low-resolution list (its files are of one type), one for the whole list in the bench
  const gkOf = (/** @type {{ g: string, fs: FileAperto[] }} */ G) => G.g.startsWith("u:") ? G.g : lrTog() ? G.fs[0].kind : E.tab;
  // compact list (iPad, narrow window), high resolution: lower rows and groups that fold, remembered in qqq.flgrp
  const compact = !!(window.BANCO && BANCO.on() && flCompact());
  const grpClosed = (/** @type {string} */ g) => compact && flGrpGet()[g] === 1;
  const gh = (/** @type {{ g: string, fs: FileAperto[] }} */ G, /** @type {string} */ label) => {
    if (G.g.startsWith("u:")) return ugHead(G, isExpOpen(G.g));
    const gk = gkOf(G), open = isExpOpen(gk), tip = lrTog() && window.modeSchema ? ` data-tiph="${EH(modeSchema(gk))}"` : "";
    if (compact && G.g.startsWith("hr.")) return `<div class="fgh" data-gg="${EH(G.g)}" style="cursor:pointer"><input type="checkbox" class="gall" data-g="${EH(G.g)}" ${G.fs.every((/** @type {FileAperto} */ f) => f.vis) ? "checked" : ""} title="${I18N.t("files.showHideGroup")}"><span class="chv${grpClosed(G.g) ? "" : " on"}" role="button" tabindex="0" aria-expanded="${!grpClosed(G.g)}" aria-label="${I18N.t("files.grp.toggle")}"></span><span>${EH(label)}</span><em>${G.fs.length}</em></div>`;
    return `<div class="fgh" data-gt="${gk}" style="cursor:pointer"${tip}><input type="checkbox" class="gall" data-g="${EH(G.g)}" ${G.fs.every((/** @type {FileAperto} */ f) => f.vis) ? "checked" : ""} title="${I18N.t("files.showHideGroup")}"><span class="chv${open ? " on" : ""}" role="button" tabindex="0" aria-expanded="${open}" aria-label="${I18N.t("files.grp.toggle")}"></span><span>${EH(label)}</span><em>${G.fs.length}</em></div>`;
  };
  // files grouped strictly by experiment type; user groups that are all single files are listed flat (ugFlat, lista-file.js)
  const flatUg = ugFlat(groups), body = groups.map(G => { const flat = flatUg && G.g.startsWith("u:"); return (flat ? "" : gh(G, grpLabel(G.g))) + `<div class="grp-files" style="${flat || (isExpOpen(gkOf(G)) && !grpClosed(G.g)) ? "" : "display:none"}">${G.fs.map(row).join("")}</div>`; }).join("");
  // the files still being read or converted: one progress row each, under the open ones (upload-rows.js)
  Q("#flst").innerHTML = pre + body + UPROWS.html();
  UPROWS.wire(Q("#fl-up")); UPROWS.paint();
  if (window.BARRA) BARRA.fitNames();
  Q("#flst").querySelectorAll("input[data-pr]").forEach((/** @type {CampoDati} */ x) => x.onchange = () => ms2Toggle(x.dataset.pr, x.checked, +(/** @type {string} */ (x.dataset.pk))));
  { const dl = Q("#ddalist"); if (dl) dl.onclick = () => DDA.openList(); }
  Q("#flst").querySelectorAll("[data-pg]").forEach((/** @type {ElementoDati} */ x) => { x.onclick = () => ms2Goto(x.dataset.pg); x.ondblclick = () => ms2Spectrum(x.dataset.pg); });
  Q("#flst").querySelectorAll(".fgh[data-gt]").forEach((/** @type {ElementoDati} */ el) => {
    const flip = () => { const t = el.dataset.gt; E._grpOpen[t] = !isExpOpen(t); renderFileList(); };
    el.onclick = (/** @type {PointerEvent} */ e) => { if (!(/** @type {HTMLElement} */ (e.target)).closest("input")) flip(); };
    const cv = el.querySelector(".chv"); if (cv) cv.onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); flip(); } };
  });
  Q("#flst").querySelectorAll(".pr-tog").forEach((/** @type {ElementoDati} */ tog) => {
    const flip = () => { const f = E.files[+tog.dataset.prk]; if (f) { f._prOpen = f._prOpen === false; renderFileList(); } };
    tog.onclick = (/** @type {MouseEvent} */ e) => { e.stopPropagation(); flip(); };
    tog.onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); flip(); } };
  });
  Q("#flst").querySelectorAll(".fgh[data-gg]").forEach((/** @type {ElementoDati} */ el) => {
    const flip = () => { const m = flGrpGet(); m[el.dataset.gg] = m[el.dataset.gg] === 1 ? 0 : 1; flGrpSet(m); renderFileList(); };
    el.onclick = (/** @type {PointerEvent} */ e) => { if (!(/** @type {HTMLElement} */ (e.target)).closest("input")) flip(); };
    const cv = el.querySelector(".chv"); if (cv) cv.onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); flip(); } };
  });
  Q("#flst").querySelectorAll(".gall").forEach((/** @type {CampoDati} */ x) => x.onchange = () => {
    fileViewDo(() => side.filter((/** @type {FileAperto} */ f) => x.dataset.g === "__all" || grpOf(f) === x.dataset.g).forEach((/** @type {FileAperto} */ f) => f.vis = x.checked));
    if (!x.checked) cleanupOrphanPanels(); else if (lrTog()) ensureLayout();
    renderFileList(); renderNav();
    redrawAll();
    uiSave();
  });
  Q("#flst").querySelectorAll("input[data-k]").forEach((/** @type {CampoDati} */ x) => x.onchange = () => {
    fileViewDo(() => sibsOf(E.files[+x.dataset.k]).forEach((/** @type {FileAperto} */ f) => { f.vis = x.checked; }));
    if (!x.checked) cleanupOrphanPanels(); else if (lrTog()) ensureLayout();
    renderNav(); redrawAll();
    uiSave();
  });
  Q("#flst").querySelectorAll(".fl").forEach((/** @type {ElementoDati} */ el) => { const nm = el.querySelector(".nm"); if (nm) el.oncontextmenu = (/** @type {MouseEvent} */ e) => fileCtx(e, E.files[+nm.dataset.k]); });
  bindFileRows();
  bindFileReorder(); syncGroupBy();
  renderNav();
  syncWorkEmpty();
}
const vis = () => tabFiles().filter((/** @type {FileAperto} */ f) => f.vis);
// "Solo il selezionato": every graph follows the file chosen in the toolbar, so the controls that choose a file inside a graph are switched off
const follow = (/** @type {string} */ tab) => !!(E.browse && E.files[E.cur] && E.files[E.cur].kind === tab);
const shown = () => E.browse && E.files[E.cur] && E.files[E.cur].kind === E.tab ? [E.files[E.cur]] : ESP.soloUno(vis());
// the files a cell draws: those of ITS tab (the same as shown() unless, in the bench, the cell is not of the tab of the active cell)
const shownFor = (/** @type {Bag} */ p) => {
  const l = p && p.tab && p.tab !== E.tab ? ESP.soloUno(tabFiles(p.tab).filter((/** @type {FileAperto} */ f) => f.vis)) : shown();
  // an MS2 precursor chromatogram ticked file by file (the sidebar rows) draws only the files ticked
  return p && p.type === "chrom" && p.ks ? l.filter((/** @type {FileAperto} */ f) => !(f.precursors || []).length || p.ks.includes(f.k)) : l;
};
/** @template {{ kind?: unknown }} T @param {T[]} l */ const scanFiles = l => l.filter(f => f.kind !== "mrm");

// ------------------------------------------------------------------ barra: frecce tra i file, metodo, esporta
function renderNav() {
  const s = Q("#fsel"); if (!s) return;
  // a very long name is cut in the middle, so the final time stays visible
  const shortName = (/** @type {string} */ n) => n.length > 34 ? n.slice(0, 20) + "…" + n.slice(-13) : n;
  s.innerHTML = tabFiles().map((/** @type {FileAperto} */ f) => `<option value="${f.k}" ${f.k === E.cur ? "selected" : ""} title="${EH(f.label)}">${EH(shortName(/** @type {string} */ (f.label)))}</option>`).join("");
  const cur = tabFiles().find((/** @type {FileAperto} */ f) => f.k === E.cur);
  const g = /** @type {CanvasRenderingContext2D} */ (document.createElement("canvas").getContext("2d")), cs = getComputedStyle(s); g.font = `${cs.fontSize} ${cs.fontFamily}`;
  // as wide as the longest name (CSS sets the minimum and the maximum)
  // (the files still being read count too: the bar does not grow as they arrive)
  s.style.width = Math.ceil(maxOf([...[...s.options].map(o => o.textContent), ...(E.pending || []).filter((/** @type {Pendiente} */ x) => x.state !== "err").map((/** @type {Pendiente} */ x) => shortName(x.label === x.file ? x.file.replace(/\.[^.]+$/, "") : x.label))].map(n => g.measureText(n).width), 0) + 36) + "px";
  // with a single file nothing here has a meaning
  const tf = tabFiles(), one = tf.length <= 1, why = I18N.t("nav.needMore");
  // one file shown (ticked) = the graphs follow it; several = overlaid, the active one drives the spectrum
  syncBrowse();
  [Q("#fprev"), Q("#fnext"), Q("#fsel")].forEach(b => { if (b.dataset.t0 == null) b.dataset.t0 = b.title; b.disabled = one; b.title = one ? why : b.dataset.t0; });
  if (cur && !s.disabled) s.title = cur.label;                       // the whole name, also when it had to be cut
  // the graphs' own file choosers depend on the mode
  const sig = [E.browse, E.cur, tabFiles().length, E.tab].join("|"); if (/** @type {any} */ (renderNav)._sig !== sig) { /** @type {any} */ (renderNav)._sig = sig; E.panels.forEach((/** @type {Pannello} */ q) => q.el && ctl(q)); }
}
Q("#fprev").onclick = () => goFile(-1); Q("#fnext").onclick = () => goFile(1);
Q("#fsel").onchange = (/** @type {Event & { target: HTMLSelectElement }} */ e) => activeFile(+e.target.value);
// ← → (scan by scan or file by file), Maiusc + ← →, Spazio and Esc: see scroll.js (scrollKeys)
document.addEventListener("mousedown", e => { const t = /** @type {Element} */ (e.target); if (E.active && t.closest("#dpanels") && !t.closest(".pnl")) setActive(null); });
// @ts-ignore toolbar() is also window.toolbar (a BarProp) for lib.dom: the function declaration of a classic script wins at run time
function toolbar() {
  if (window.BANCO) BANCO.bar();       // high-resolution bench: its toolbar (banco.js)
  // the buttons follow the experiment types the files have
  const has = (/** @type {string} */ t) => tabFiles(t).length > 0, scan = has("full") || has("ms2") || has("infusion"), mrm = has("mrm"), empty = !scan && !mrm;
  Q("#np-chrom").hidden = !scan; Q("#np-spec").hidden = !scan; Q("#np-map").hidden = !has("full"); Q("#np-xic").hidden = !scan;
  Q("#np-mrm").hidden = !mrm;
  Q("#g-add").hidden = empty; Q("#g-tools").hidden = empty; Q("#g-file").hidden = empty; Q("#dpanels").hidden = empty; Q("#tools .sp").hidden = empty; Q("#np-method").hidden = empty;
}
// ------------------------------------------------------------------ piccole finestre e menu
function ask(/** @type {string} */ title, def = "") {
  return new Promise(res => {
    const d = Q("#askdlg"), i = Q("#askin");
    i.hidden = false; Q("#askno").hidden = false;
    Q("#asktxt").innerHTML = title; i.value = def;
    const done = (/** @type {string | null} */ v) => { d.close(); Q("#askok").onclick = Q("#askno").onclick = i.onkeydown = null; res(v); };
    Q("#askok").onclick = () => done(i.value.trim());
    Q("#askno").onclick = () => done(null);
    i.onkeydown = (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter") { e.preventDefault(); done(i.value.trim()); } };
    d.oncancel = () => done(null);
    d.showModal(); i.focus(); i.select();
  });
}
function yesno(/** @type {string} */ html) {
  return new Promise(res => {
    const d = Q("#askdlg"); Q("#asktxt").innerHTML = html; Q("#askin").hidden = true; Q("#askno").hidden = false;
    const done = (/** @type {boolean} */ v) => { d.close(); Q("#askin").hidden = false; Q("#askok").onclick = Q("#askno").onclick = null; res(v); };
    Q("#askok").onclick = () => done(true); Q("#askno").onclick = () => done(false); d.oncancel = () => done(false); d.showModal();
  });
}
function info(/** @type {string} */ html) {
  const d = Q("#askdlg");
  Q("#asktxt").innerHTML = html; Q("#askin").hidden = true; Q("#askno").hidden = true;
  Q("#askok").onclick = () => { d.close(); Q("#askin").hidden = false; Q("#askno").hidden = false; };
  d.showModal();
}
/** @param {string} title @param {string} html @param {() => void} [onopen] */ function big(title, html, onopen) {
  Q("#bigtt").textContent = title; Q("#bigbody").innerHTML = html;
  const d = Q("#bigdlg"); Q("#bigx").onclick = () => d.close(); d.showModal(); if (onopen) onopen();
}
function menu(/** @type {CtxEv} */ ev, /** @type {(Bag | string)[]} */ items) {
  ev.preventDefault(); items = AZIONI.pulisci(items);
  const m = Q("#ctx");
  m.innerHTML = "";
  // one entry; `fila` groups two or more entries in a single row (icons only, split in equal columns)
  const mk = (/** @type {Bag} */ it) => {
    const d = document.createElement("div");
    if (it.icon) {
      // decorative icon on the left (currentColor, so it follows the theme and the grey of a disabled entry); a checkable entry keeps its mark on the right
      const ic = document.createElement("span");
      ic.className = "ic";
      ic.setAttribute("aria-hidden", "true");
      ic.innerHTML = it.icon;
      d.appendChild(ic);
      if (!it.soloIcona) d.appendChild(document.createTextNode(it.label));
      if (it.check) { const ck = document.createElement("span"); ck.className = "ck"; ck.setAttribute("aria-hidden", "true"); ck.textContent = "\u2713"; d.appendChild(ck); }
    } else {
      d.textContent = (it.check ? "\u2713 " : "") + it.label;
    }
    if (it.tip || it.soloIcona) d.title = it.tip || it.label;
    if (it.aria) d.setAttribute("aria-label", it.aria);
    d.setAttribute("role", it.check != null ? "menuitemcheckbox" : "menuitem");
    if (it.check != null) d.setAttribute("aria-checked", String(!!it.check));
    // an action of the registry azioni/1: the same id in every menu that offers it
    if (it.id) d.dataset.azione = it.id;
    // an action not available now: grey, still reachable from the keyboard
    if (it.dim) { d.className = "dim"; if (it.id) { d.tabIndex = -1; d.setAttribute("aria-disabled", "true"); } }
    else { d.tabIndex = -1; d.onclick = () => { m.hidden = true; it.fn(); }; }
    return d;
  };
  items.forEach((/** @type {Bag | string} */ it) => {
    if (it === "-") { m.appendChild(document.createElement("hr")); return; }
    if (/** @type {Bag} */ (it).fila) {
      const r = document.createElement("div");
      r.className = "mrow"; r.setAttribute("role", "group");
      /** @type {Bag} */ (it).fila.forEach((/** @type {Bag} */ x) => r.appendChild(mk(x)));
      m.appendChild(r);
      return;
    }
    m.appendChild(mk(/** @type {Bag} */ (it)));
  });
  m.hidden = false; m.setAttribute("role", "menu");
  // keyboard: arrows move, Enter/Space choose, Esc closes
  if (!m._kb) { m._kb = true; m.addEventListener("keydown", (/** @type {KeyboardEvent} */ e) => {
    const its = [...m.querySelectorAll('div[role^=menuitem]:not(.dim), div[aria-disabled=true]')], i = its.indexOf(document.activeElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); if (its.length) its[(i + (e.key === "ArrowDown" ? 1 : -1) + its.length) % its.length].focus(); }
    else if (e.key === "Enter" || e.key === " ") { if (i >= 0) { e.preventDefault(); if (!its[i].hasAttribute("aria-disabled")) its[i].click(); } }
    else if (e.key === "Escape") { e.preventDefault(); m.hidden = true; if (m._ret && m._ret.focus) m._ret.focus(); }
  }); }
  m._ret = ev.kbd ? document.activeElement : null; m._lp = !!ev.lp; m._t = Date.now();
  // with a finger the menu opens beside it: lifting the finger must not click an item
  const off = document.documentElement.classList.contains("touch") ? 16 : 0;
  m.style.left = Math.min(ev.clientX + off, innerWidth - 270) + "px";
  m.style.top = Math.max(4, Math.min(ev.clientY + off, innerHeight - m.offsetHeight - 8)) + "px";
  if (ev.kbd) { const f = m.querySelector('div[role^=menuitem]:not(.dim)'); if (f) f.focus(); }
}
document.addEventListener("pointerup", () => { const m = Q("#ctx"); if (m._lp && !m.hidden) m._up = Date.now(); }, true);
// a long press opens the menu: the click that may follow the lifted finger must not close it
document.addEventListener("click", () => { const m = Q("#ctx"); if (m._lp && m._up && Date.now() - m._up < 400) { m._lp = false; m._up = 0; return; } m.hidden = true; });
// the Correzione menu closes when the click is elsewhere
document.addEventListener("click", e => { document.querySelectorAll(".corrm:not([hidden])").forEach(m => { if (!m.parentNode.contains(e.target)) m.hidden = true; }); });
document.addEventListener("keydown", e => { if (e.key === "Escape") document.querySelectorAll(".corrm:not([hidden])").forEach(m => { m.hidden = true; }); });
// _ctxWas: this Esc only closed the menu (the full-screen handler below leaves the panel alone)
document.addEventListener("keydown", e => { if (e.key === "Escape") { /** @type {any} */ (e)._ctxWas = !Q("#ctx").hidden; if (/** @type {any} */ (e)._ctxWas) e.preventDefault(); Q("#ctx").hidden = true; } });
// CSV/TSV text carries its provenance line (PROV, schema provenienza/1) by default; prov = the fields the export knows ({table, p, file}), or false for a file made for another program (mzFinder).
// `text` is a string, a Blob/Uint8Array or an (async) iterable of those: an iterable, or a string/Blob over 8 MB, goes piece by piece through OPFS (export-pezzi.js) with the same bytes.
async function dl(/** @type {string} */ name, /** @type {string | Blob | Uint8Array | Iterable<string | Uint8Array> | AsyncIterable<string | Uint8Array>} */ text, type = "application/octet-stream", prov = undefined) {
  const iter = typeof text !== "string" && !(text instanceof Blob) && !(text instanceof Uint8Array) && !!text && (Symbol.asyncIterator in text || Symbol.iterator in text);
  const size = iter ? 0 : typeof text === "string" ? text.length : /** @type {{ size?: number, byteLength?: number }} */ (text).size || /** @type {{ byteLength?: number }} */ (text).byteLength || 0;
  // the piece-by-piece writer is loaded on the first big export
  if ((iter || size >= 8 << 20) && window.carica) await window.carica("m:export-pezzi.js").catch(console.error);
  const EP = /** @type {any} */ (window).EXPORT_PEZZI, tab = /csv|tab-separated/.test(type);
  if (prov !== false && tab && typeof text === "string") text = await PROV.tsv(text, prov || {});
  else if (prov !== false && tab && EP && EP.isIter(text)) { const src = /** @type {AsyncIterable<string | Uint8Array>} */ (text); text = (async function* () { let first = true; for await (const p of src) { yield first && typeof p === "string" ? await PROV.tsv(p, prov || {}) : p; first = false; } })(); }
  const big = EP && (EP.isIter(text) || (typeof text === "string" ? text.length : /** @type {{ size?: number }} */ (text).size || /** @type {{ byteLength?: number }} */ (text).byteLength || 0) >= EP.MIN_STREAM);
  if (big) { try { await EP.save(name, text, type); } catch (e) { console.error(e); if (window.toast) toast(I18N.t("export.stream.fail")); } return; }
  saveBlob(new Blob([/** @type {BlobPart} */ (text)], { type }), name);
}
const LOCALE = I18N.lang === "it" ? "it-IT" : "en-GB";           // dates in the exported files
// data of a plot as columns side by side (each trace has its own x), one sheet with the units in the headers: for graphs and tables made in Excel
function plotSheets(/** @type {Pannello} */ p) {
  const a = p._a; if (!a) return null;
  const cols = [], yt = p.kind === "pda" ? I18N.t("export.col.pdaSignal") : p.kind === "press" ? I18N.t("axis.pressure") + (a.sr && a.sr[0] && a.sr[0].unit ? ` (${a.sr[0].unit})` : "") : I18N.t("export.col.intensity");
  if (p.type === "spec" && a.uvd) cols.push(["nm", a.uvd.nm], [I18N.t("tdati.col.mau"), a.uvd.y]);
  else if (p.type === "spec") a.data.forEach((/** @type {Bag} */ x) => { cols.push([`m/z - ${x.f.label}`, x.d.mz], [`${I18N.t("export.col.intensity")} - ${x.f.label}`, x.d.y0 || x.d.y]); });
  else if (a.sr) {
    // total chromatograms of one run type share the time axis: one RT column
    const same = p.type === "chrom" && a.sr.length > 1 && a.sr.every((/** @type {Bag} */ q) => q.x.length === a.sr[0].x.length && q.x.every((/** @type {number} */ v, /** @type {number} */ i) => v === a.sr[0].x[i]));
    if (same) { cols.push(["RT (min)", a.sr[0].x]); a.sr.forEach((/** @type {Bag} */ sr) => cols.push([`${yt} - ${sr.name}`, sr.y])); }
    else a.sr.forEach((/** @type {Bag} */ sr) => { cols.push([`RT (min) - ${sr.name}`, sr.x], [`${yt} - ${sr.name}`, sr.y]); });
  }
  if (!cols.length) return null;
  const n = maxOf(cols.map(c => c[1].length)), rows = [];
  for (let i = 0; i < n; i++) rows.push(cols.map(c => (i < c[1].length ? c[1][i] : null)));
  /** @type {any[]} */
  const out = [{ name: ptitle(p.title) || I18N.t("export.sheet.data"), head: cols.map(c => c[0]), rows, widths: cols.map(c => Math.min(44, Math.max(14, c[0].length + 2))) }];
  const metaRows = [
    [{ v: I18N.t("export.meta.property"), b: true }, { v: I18N.t("export.meta.value"), b: true }],
    [{ v: I18N.t("export.meta.title"), b: true }, ptitle(p.title) || I18N.t("export.meta.chart")],
    [{ v: I18N.t("export.meta.type"), b: true }, (/** @type {Record<string, string>} */ ({ chrom: I18N.t("export.type.chrom", { kind: (p.kind || "").toUpperCase() }), xic: I18N.t("export.type.xic"), mrm: I18N.t("panel.title.mrm"), spec: I18N.t("export.type.spec", { ms: p.level === 2 ? "MS²" : "MS¹" }), map: I18N.t("panel.title.map") }))[p.type] || p.type],
    [{ v: I18N.t("export.meta.samples"), b: true }, plotFiles(p).map((/** @type {FileAperto} */ f) => f.label).join(", ")],
    [{ v: I18N.t("export.meta.caption"), b: true }, plotCaption(p)],
    [{ v: I18N.t("export.meta.date"), b: true }, new Date().toLocaleString(LOCALE)],
    [{ v: "Software", b: true }, APP_NAME]
  ];
  if (p.r0 != null && p.r1 != null) metaRows.push([{ v: I18N.t("export.meta.rtRange"), b: true }, `${p.r0.toFixed(2)} - ${p.r1.toFixed(2)}`]);
  if (a.x0 != null && a.x1 != null) metaRows.push([{ v: I18N.t("export.meta.axis"), b: true }, `${a.x0.toFixed(2)} - ${a.x1.toFixed(2)}`]);
  if (ibOf(p)) metaRows.push([{ v: I18N.t("export.meta.ibk"), b: true }, ibkText(ibOf(p))]);
  else if (p.fondo && p.fondo.mode === "auto") metaRows.push([{ v: I18N.t("export.meta.fondoAuto"), b: true }, fondoText(p)]);
  const subT = window.SUBREF ? SUBREF.text(p) : "";
  if (subT) metaRows.push([{ v: I18N.t("export.meta.sub"), b: true }, subT]);
  out.push({ name: I18N.t("export.sheet.meta"), rows: metaRows, widths: [24, 60] });
  if (subT) out.push({ name: I18N.t("export.sheet.notes"), rows: [[{ v: I18N.t("export.meta.sub"), b: true }, subT], [I18N.t("export.note.sub")]], widths: [28, 60] });
  if (ibOf(p)) out.push({ name: I18N.t("export.sheet.notes"), rows: [[{ v: I18N.t("export.meta.ibkApplied"), b: true }, ibkText(ibOf(p))], [I18N.t("export.note.ibk")]], widths: [28, 60] });
  else if (p.fondo && p.fondo.mode === "auto") out.push({ name: I18N.t("export.sheet.notes"), rows: [[{ v: I18N.t("export.meta.fondoAuto"), b: true }, fondoText(p)], [I18N.t("export.note.fondoAuto")]], widths: [28, 60] });
  return out;
}

