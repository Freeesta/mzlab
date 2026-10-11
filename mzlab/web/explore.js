
window.toast = function(msg) {
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

"use strict";
// Esplorazione dei dati: si parte dal cromatogramma totale, lo studente decide cosa estrarre.
// Script classico (usa gli helper di index.html: S, setup, nice, fmt, css, esc, smooth, M, setView, applyView).
const E = { files: [], panels: [], seq: 1, key: "", cur: 0, browse: false, z: 10, tab: "full" };
const NB = { ui: null, session: null };   // taccuino: stato dell'interfaccia (+ disegno, vedi draw.js)
// user preferences (settings gear): text size, theme, chart colours, high resolution (hr: automatic | off, tolerance in ppm, decimals; see hr.js). Filled from localStorage by uipLoad() (settings.js).
const UIP = { theme: "auto", pal: "time", merge: true, hrPpm: 5, hrDec: 4, tog: false };      // merge = join the centroids of the same nominal mass (centroid files only)
// areas: at least 3 decimals in the label (2.243e+6), whole number with thousands separated by a thin space in the tooltip
const fmtA = v => !Number.isFinite(v) || v === 0 ? "0" : Math.abs(v) >= 1e4 || Math.abs(v) < 0.01 ? v.toExponential(3) : (+v.toPrecision(4)).toString();
const fmtFull = v => Number.isFinite(v) ? Math.round(v).toString().replace(/\B(?=(\d{3})+(?!\d))/g, "\u2009") : "";
// ---- chart colours (settings: "Colori dei grafici"). PAL = categorical colours of the current palette (filled in place by setPal, so every user of PAL follows it);
// files of the Full Scan tab that have a time get a SEQUENTIAL scale ordered by time (seqColor), see paintFiles.
function isDark() {
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
const PALS = {
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
    hue: u => isDark() ? `hsl(${Math.round(265 - 250 * u)} 85% 65%)` : `hsl(${Math.round(265 - 250 * u)} 80% 40%)`,
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
};
const PAL = [];
function setPal(mode) {
  UIP.pal = PALS[mode] ? mode : "time";
  const P = PALS[UIP.pal], dark = isDark();
  PAL.length = 0; PAL.push(...(dark && P.catDark ? P.catDark : P.cat));
  document.documentElement.dataset.pal = UIP.pal;
}
setPal(UIP.pal);
function seqColor(u) {                                       // u in 0..1 -> colour of the current palette's sequential scale
  const P = PALS[UIP.pal], dark = isDark();
  if (P.hue) return P.hue(u);
  const seq = dark && P.seqDark ? P.seqDark : P.seq;
  const a = dark && P.aDark != null ? P.aDark : P.a;
  const b = dark && P.bDark != null ? P.bDark : P.b;
  const x = (a + (b - a) * u) * (seq.length - 1), i = Math.min(Math.floor(x), seq.length - 2), f = x - i;
  const c = h => [1, 3, 5].map(k => parseInt(h.slice(k, k + 2), 16)), A = c(seq[i]), B = c(seq[i + 1]);
  return "#" + A.map((v, k) => Math.round(v + (B[k] - v) * f).toString(16).padStart(2, "0")).join("");
}
const DASHES = [[], [6, 3], [2, 3], [10, 3, 2, 3]];
const LWX = () => PALS[UIP.pal].lw || 0;                      // extra line width (px) of the high contrast palette
// line style of a file: blank dashed, Full Scan standard dash-dot (both neutral grey); with the colour-blind / high-contrast palettes the files of the
// time scale are also told apart by line style (second code besides the colour)
function fdash(f) {
  if (f.type === "blank") return [4, 3];
  if (f.kind === "full" && f.type === "standard") return [10, 4];
  return PALS[UIP.pal].dash && f.rank != null ? DASHES[f.rank % 4] : [];
}
const Q = s => document.querySelector(s);
const EH = t => String(t).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const fmtFormula = f => !f ? "" : EH(f).replace(/([A-Z][a-z]?|\))(\d+)/g, "$1<sub>$2</sub>");
const fmtAdduct = a => !a ? "" : fmtFormula(a).replace(/([+-]+)$/, "<sup>$1</sup>");
window.fmtFormula = fmtFormula; window.fmtAdduct = fmtAdduct;
const KIND = { full: "Full Scan", ms2: "MS2 (Product Ion)", mrm: "MRM" };
const kindOf = f => f.kind === "empty" ? I18N.t("kind.empty") : KIND[f.kind] || f.kind;
// the three kinds of experiment never share a graph: Dati has one sub-tab for each (Full Scan, MS2, MRM); panels and files belong to one
const TABS = [["full", "Full Scan"], ["ms2", "MS2 (Product Ion)"], ["mrm", "MRM"]];
const tabFiles = (t = E.tab) => E.files.filter(f => f.kind === t && !f.gone);   // f.gone = removed from the session (the file on disk is untouched)
// the panels of a tab; in the high-resolution bench (banco.js) the cells of every experiment are on the page together, so "the panels of the tab" is all of them
// «Mostra tutti gli esperimenti insieme» (gear, off by default; low-resolution world): one block per experiment type on the same page. A panel still holds ONE experiment type.
const lrTog = () => !!UIP.tog && !(window.BANCO && BANCO.on());
const allTabs = () => !!(window.BANCO && BANCO.on()) || !!UIP.tog;
const tabPanels = t => (t === undefined && allTabs() ? E.panels : E.panels.filter(p => p.tab === (t ?? E.tab)));
// group of a file in the list: in LR strictly the experiment name (Full Scan, MS2, MRM); in HR bench sub-groups
const grpOf = f => (window.BANCO && BANCO.on()) ? (f.kind === "mrm" ? (["standard", "sample", "blank"].includes(f.type) ? f.type : "sample") : kindOf(f)) : kindOf(f);
const grpLabel = g => ["standard", "sample", "blank"].includes(g) ? I18N.t(`files.group.${g}`) : g;
const TOL0 = 1.0;                                   // strumento datato: finestra XIC di +-1 Da

// ------------------------------------------------------------------ schermata di caricamento
// small inline icons (no external files): download arrow and "fit to window" corners
const IC_DL = '<svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-1px;margin-right:4px"><path d="M6 1v7M3 5.5 6 8.5 9 5.5M1.5 11h9"/></svg>';
const IC_AUTO = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"><path d="M1 14h14"/><path d="M3 14C5 14 6 4.5 8 4.5S11 14 13 14z" fill="currentColor" fill-opacity=".3"/><path d="M13 1.2l.7 1.6 1.6.7-1.6.7-.7 1.6-.7-1.6-1.6-.7 1.6-.7z" fill="currentColor" stroke="none"/></svg>';   // peak with its area + sparkle = automatic
const IC_MAN = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"><path d="M1 14h14"/><path d="M4.5 14C6 14 6.5 5 8 5s2 9 3.5 9" /><path d="M4 2.5v11.5M12 2.5v11.5" stroke-dasharray="2 1.6"/></svg>';   // peak between two edge bars = manual
const IC_DEL_INT = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M1 14h14"/><path d="M3 14c1.5 0 2.5-9 5-9s3.5 9 5 9"/><path d="M10.5 2.5l4 4M14.5 2.5l-4 4"/></svg>';   // peak with an X = delete integration
const IC_D2 = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M1.5 13.5h13"/><path d="M2.5 13.5c2 0 3-9 5.5-9s3.5 9 5.5 9"/><path d="M7 2.5l1.2 2.2M9.2 2.5l1.2 2.2"/></svg>';   // inverted second derivative
const IC_ZOOM = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"><circle cx="6.5" cy="6.5" r="4.6"/><path d="M10 10l4.6 4.6M4.4 6.5h4.2M6.5 4.4v4.2"/></svg>';   // magnifier = zoom tool
const IC_TAB = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3"><rect x="1.5" y="2" width="11" height="10" rx="1.2"/><path d="M1.5 5.5h11M1.5 8.7h11M6 5.5V12"/></svg>';
const IC_FS = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><path d="M8.5 1.5h4v4M12.5 1.5 8 6M5.5 12.5h-4v-4M1.5 12.5 6 8"/></svg>';   // full screen: arrows pointing out
const IC_FSX = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><path d="M12 5.5H8.5V2M8.5 5.5 13 1M2 8.5h3.5V12M5.5 8.5 1 13"/></svg>';  // leave full screen: arrows pointing in
const IC_FIT = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><path d="M1 5V1h4M9 1h4v4M13 9v4H9M5 13H1V9"/></svg>';
const IC_LOCK = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><rect x="2.5" y="6.2" width="9" height="6.3" rx="1.2"/><path d="M4.5 6.2V4.3a2.5 2.5 0 0 1 5 0v1.9"/></svg>';
const IC_UNLOCK = '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px"><rect x="2.5" y="6.2" width="9" height="6.3" rx="1.2"/><path d="M4.5 6.2V4.3a2.5 2.5 0 0 1 4.8-1"/></svg>';
// PHRASES and ldNext() are in index.html, right after #loading: the first phrase shows before the ~600 kB of scripts arrive
let ldSlow = 0;
function loading(on, msg) {
  const L = Q("#loading");
  if (on) {
    ldSince = Date.now(); ldMsg = msg ? { text: msg, pct: null } : null; ldRefresh();
    clearTimeout(ldSlow); ldSlow = setTimeout(() => { if (!window.qqFailed && !Q("#loading").hidden && !ldMsg && !window.qqPhaseCur) { ldMsg = { text: I18N.t("load.slow"), pct: null }; ldRefresh(); } }, 20000);
    ldIon(true);                                                     // engine start and data loading: the ions (one analyser per load)
    if (!ldTimer) { ldNext(); ldTimer = setInterval(ldNext, 5000); } L.hidden = false;   // a phrase already turning (first start) is kept
  } else {
    if (window.qqFailed) return;                                   // a failed engine start keeps its message (and the «start from scratch» button) on screen
    const wait = Math.max(0, 900 - (Date.now() - ldSince));
    clearTimeout(ldSlow); setTimeout(() => {
      L.classList.add("fade-out"); ldMsg = null;
      setTimeout(() => { L.hidden = true; L.classList.remove("fade-out"); ldIon(false); clearInterval(ldTimer); clearInterval(ldDotTimer); ldTimer = null; }, 400);
    }, wait);
  }
}

// progress bar for any API call (the work runs in the worker) that lasts more than ~1 s; the loading screen already covers file opening
(() => {
  const bar = () => document.getElementById("busybar"); let pending = 0, timer = 0; const orig = window.fetch.bind(window);
  const update = () => { const b = bar(); if (b) b.hidden = !(pending > 0 && !timer && Q("#loading").hidden); };
  window.fetch = (input, init) => {
    const url = typeof input === "string" ? input : (input && input.url) || "";
    if (!/^(\.\/)?api\//.test(url)) return orig(input, init);
    if (!pending++) timer = setTimeout(() => { timer = 0; update(); }, 1000);
    const done = () => { if (!--pending) { clearTimeout(timer); timer = 0; } update(); };
    const p = orig(input, init); p.then(done, done); return p;
  };
})();

// ------------------------------------------------------------------ taccuino (salvato accanto ai dati)
let nbTimer = null, nbLast = "";
function nbSave(now = false) {
  clearTimeout(nbTimer);
  const go = () => {
    const body = JSON.stringify(NB); if (body === nbLast) return;      // nothing changed: no request (fewer aborted saves on reload)
    nbLast = body; fetch("api/notebook", { method: "POST", body, keepalive: true }).catch(() => { nbLast = ""; });
  };
  if (now) go(); else nbTimer = setTimeout(go, 500);
}
let nbEverLoaded = false;
async function nbLoad() {
  try { Object.assign(NB, await (await fetch("api/notebook")).json()); } catch (_) { /* prima volta */ }
  nbEverLoaded = true;
  document.dispatchEvent(new Event("nbloaded"));
}
// the Disegno tab can be opened before any data file is loaded: the notebook (with the drawing) must be read first, otherwise the first save
// would overwrite the saved drawing with an empty one
window.nbEnsure = async () => { if (!nbEverLoaded) await nbLoad(); };
let uiTimer = null;
function uiSave(now = false) {
  clearTimeout(uiTimer);
  const run = () => {
    if (!E.files.length) return;
    NB.session = E.files.filter(f => !f.gone).map(f => ({ file: f.file, label: f.label, time: f.time, type: f.type, conc: f.conc ?? null, cunit: f.cunit ?? null }));
    NB.ui = {
      contEx: window.LISTE ? LISTE.exclExport() : undefined,
      tab: E.tab, cur: E.cur, browse: E._forced ? E._prev : E.browse, files: E.files.map(f => ({ file: f.file, label: f.label, vis: f.vis, color: f.colorSet ? f.color : undefined, gone: f.gone || undefined })),
      panels: E.panels.map(p => ({
        type: p.type, tab: p.tab, title: p.title, x: p.x, y: p.y, w: p.w, h: p.h, full: !!p.full, kind: p.kind, smooth: p.smooth, tol: p.tol,
        traces: (p.traces || []).map(t => ({ mz: t.mz, w: t.w, label: t.label, ion: t.ion || undefined, obs: t.obs || undefined })),
        k: E.files[p.k]?.file ?? null, r0: p.r0, r1: p.r1, level: p.level, prec: p.prec, filt: p.filt || undefined, flv: p.flv ?? undefined, pin: p.pin || undefined, ranges: p.ranges || undefined, norm100: p.norm100 || undefined, all: p.all, zoom: p.zoom, anns: p.anns, ints: p.ints, tr: p.tr,
        link: p.link ? E.panels.findIndex(q => q.id === p.link) : -1, src: p.src ? E.panels.findIndex(q => q.id === p.src) : -1,
        dda: p.dda != null ? E.panels.findIndex(q => q.id === p.dda) : undefined, dock: p.dock != null ? E.panels.findIndex(q => q.id === p.dock) : undefined, duo: p.duo != null ? E.panels.findIndex(q => q.id === p.duo) : undefined, extra: p.extra || undefined, sid: p.sid ?? undefined, ion: p.ion ?? undefined, avg: p.avg || undefined, imode: p.imode || null, intf: p.intf || "",
        lock: p.type === "spec" ? !!p.lock : undefined, rel: p.type === "spec" && typeof p.rel === "boolean" ? p.rel : undefined, sticks: p.type === "spec" && p.sticks ? true : undefined, thr: p.type === "spec" ? p.thr : undefined, nlab: p.type === "spec" ? p.nlab : undefined, dec: p.type === "spec" ? p.dec : undefined, meas: p.type === "spec" && p.meas ? p.meas : undefined, tl: p.tl || undefined, ms2tri: p.ms2tri || undefined, iso: p.iso || null, ibk: p.ibk || null, mz0: p.mz0 ?? null, mz1: p.mz1 ?? null, mode: p.mode, log: p.log, hid: p.hid, bk: p.bk === "" || p.bk == null ? "" : E.files[+p.bk]?.file ?? "", snip: p.snip, snipw: p.snipw, adduct: p.adduct, bg: p.bg === "" || p.bg == null ? "" : p.bg === "w" ? "w" : E.files[+p.bg]?.file ?? "", bw0: p.bw0, bw1: p.bw1, scale: p.scale, norm: p.norm, zoomY: p.zoomY, ref: p.ref === "" || p.ref == null ? "" : E.files[+p.ref]?.file ?? ""
      }))
    };
    nbSave(now);
  };
  if (now) run(); else uiTimer = setTimeout(run, 800);
}
// alla chiusura della pagina (o quando la scheda va in secondo piano) si salva subito: l'ultima modifica non va persa
addEventListener("pagehide", () => uiSave(true));
document.addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") uiSave(true); });

let EXPORTING = false;
const fz = () => 1;        // (the text size of the graphs follows the browser zoom: Ctrl/Cmd + and −)
const fpx = (n, st = "") => `${st}${+(n * fz()).toFixed(1)}px system-ui`;
// ------------------------------------------------------------------ dati (con cache)
const CACHE = new Map();
const memo = (key, fn) => { if (!CACHE.has(key)) CACHE.set(key, fn().catch(e => { CACHE.delete(key); throw e; })); return CACHE.get(key); };
const J = u => fetch(u).then(async r => { const j = await r.json(); if (j.error) throw I18N.err(j); return j; });
// precursor + scan filter (high-resolution bench: the type of scan chosen in a cell) travel together in the "precursor" slot of the caches and the requests
const prk = (pr, ft) => ft ? `${pr ?? ""}~${ft}` : pr;
const prSplit = pr => { const t = String(pr ?? ""), i = t.indexOf("~"); return i < 0 ? [pr ?? "", ""] : [t.slice(0, i), t.slice(i + 1)]; };
const FT = ft => ft ? `&filt=${encodeURIComponent(ft)}` : "";
const getChrom = (k, kind, lv, mz0, mz1, prec, filt) => memo(`c${k}|${kind}|${lv}|${mz0 ?? ""}|${mz1 ?? ""}|${prec ?? ""}|${filt ?? ""}`, () => J(`api/chrom?k=${k}&kind=${kind}&level=${lv}` + (mz0 != null ? `&mz0=${mz0}` : "") + (mz1 != null ? `&mz1=${mz1}` : "") + (prec ? `&prec=${prec}` : "") + FT(filt)));
const getXic = (k, mz, tol, lv) => memo(`x${k}|${mz}|${tol}|${lv}`, () => J(`api/xic?k=${k}&mz=${mz}&tol=${tol}&level=${lv}`).then(j => j.traces[0]));
const MERGE = () => (UIP.merge ? "&merge=1" : "") + (window.HR ? HR.q() : "");     // + "&hr=0" when high resolution is switched off (hr.js)
const getSpec = (k, a, b, lv, pr, bg) => memo(`s${k}|${a}|${b}|${lv}|${pr}|${bg ? bg.join(",") : ""}|${UIP.merge ? 1 : 0}`, () => J(`api/spectrum?k=${k}&rt0=${a}&rt1=${b}&level=${lv}&precursor=${prSplit(pr)[0]}${FT(prSplit(pr)[1])}` + (bg ? `&bgk=${bg[0]}&bgrt0=${bg[1]}&bgrt1=${bg[2]}` : "") + MERGE()));
const getFormula = (f, ad) => J(`api/formula?f=${encodeURIComponent(f)}&adduct=${encodeURIComponent(ad || "")}`);
// default adduct from the polarity of the visible files
// polarity of the files (read from the mzML): a small sign next to the name, the default adduct of a graph follows ITS files
const POLS = { positive: ["+", "pol.positive"], negative: ["−", "pol.negative"], mixed: ["±", "pol.mixed"], unknown: ["?", "pol.unknown"] };
// the polarity as a small readable badge "ESI+" / "ESI−" (never a bare sign); in lists it appears ONLY when the files of the session do not all have the same polarity
const polKnown = list => new Set((list || []).map(f => f && f.polarity).filter(x => x && x !== "unknown"));
const polAll = () => polKnown(E.files.filter(f => !f.gone));
const polLabel = f => ({ positive: "ESI+", negative: "ESI\u2212", mixed: "ESI\u00b1" }[f && f.polarity] || "ESI?");
const polSign = (f, list) => { const q = POLS[f && f.polarity] || POLS.unknown; if ((list ? polKnown(list) : polAll()).size < 2) return ""; return `<span class="pol" title="${I18N.t(q[1])}">${polLabel(f)}</span>`; };
// the polarity written once in the heading of a group when all its files share it: " · ESI+"
const polHead = list => { const s = polKnown(list); return s.size === 1 && list.every(f => f.polarity && f.polarity !== "unknown") ? " \u00b7 " + polLabel(list[0]) : ""; };
const polMixed = () => new Set(shown().map(f => f.polarity).filter(x => x && x !== "unknown")).size > 1;
const legPol = f => polMixed() && f ? " " + polSign(f, shown()) : "";                      // the legend shows the sign only when positive and negative files are drawn together
const defAdduct = fs => { const l = fs || (shown().length ? shown() : E.files.filter(f => f.vis)); return l.some(f => f.polarity === "negative") && !l.some(f => f.polarity === "positive") ? "[M-H]-" : "[M+H]+"; };
const getMrm = k => memo(`m${k}`, () => J(`api/mrm?k=${k}`).then(j => j.files[0].transitions));

// a method (.dam) of another experiment type than the open files (MRM against Full Scan...): a visible notice above the panels, not a toast
async function methodWarn() {
  const el = Q("#methwarn"); if (!el) return;
  try {
    const j = await J("api/method_warnings");
    if (!j.warnings.length) { el.hidden = true; el.innerHTML = ""; return; }
    el.innerHTML = I18N.t("methwarn.title") + " " + j.warnings.map(w => I18N.t("methwarn.item", { method: EH(w.method), expects: EH(w.expects), files: EH(w.files.length === 2 ? I18N.t("methwarn.and", { a: w.files[0], b: w.files[1] }) : w.files.join(", ")) })).join(" ") + ` <button type="button" id="methwarn-x" title="${I18N.t("methwarn.hide")}">&times;</button>`;
    el.hidden = false; Q("#methwarn-x").onclick = () => { el.hidden = true; };
  } catch (_) { /* no session or no method: nothing to say */ }
}
async function bootSession() { await bootSession0(); methodWarn(); window.TOUR && TOUR.offri(); }
async function bootSession0() {
  const j = await J("api/session");
  S.sess = j.session;
  if (!j.session) { E.files = []; E.key = ""; return; }
  const key = JSON.stringify(j.session.map(s => s.file));
  if (key === E.key) return;
  const oldNames = E.files.map(f => f.file), oldVis = Object.fromEntries(E.files.map(f => [f.file, f]));
  E.key = key;
  CACHE.clear(); if (typeof SC !== "undefined") { SC.m.clear(); SC.n.clear(); }
  const isFirstLoad = !oldNames.length;
  const isHr = j.session.some(s => s.hr || (s.prof1 && s.prof1.hr) || (s.prof2 && s.prof2.hr) || s.dda);
  E.files = j.session.map((s, k) => {
    let vis = true;
    if (oldVis[s.file]) {
      vis = oldVis[s.file].vis;
    } else if (isFirstLoad && !isHr && j.session.length > 1) {
      const firstIdx = j.session.findIndex(x => x.kind === s.kind);
      vis = (k === firstIdx);
    }
    return { ...s, k, vis, label: oldVis[s.file] ? oldVis[s.file].label : s.label, lv: s.kind === "ms2" ? 2 : 1 };
  });
  paintFiles();
  if (window.HR) HR.notice(E.files);
  const idx = n => { const i = E.files.findIndex(f => f.file === n); return i < 0 ? 0 : i; };
  if (E.panels.length && oldNames.length) {                // caricamento successivo: i pannelli restano
    E.panels.forEach(p => { if (p.k != null) p.k = idx(oldNames[p.k]); if (p.ref !== "" && p.ref != null) p.ref = idx(oldNames[+p.ref]); });
    E.cur = Math.min(E.cur, E.files.length - 1);
    renderFileList(); toolbar(); E.panels.forEach(ctl); redrawAll();
    const added = E.files.filter(f => !oldNames.includes(f.file)); if (added.length && window.setTab && added[0].kind !== E.tab) setTab(added[0].kind); else if (added.length) { ensureLayout(); }
    return;
  }
  renderFileList();
  E.panels = []; Q("#dpanels").innerHTML = "";
  await nbLoad();
  if (!restoreUi()) E.tab = pickTab(E.tab);
  ensureLayout(); toolbar(); uiSave();
}
// the spectrum starts at the retention time with the highest intensity (of all the visible full-scan/MS2 files)
async function apexSpectrum(c, sp) {
  try {
    const all = (await rawSeries(c)).filter(s => E.files[s.k] && E.files[s.k].kind !== "mrm");
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
  return w > 100 ? w : Math.max(600, innerWidth - 28 - (window.BARRA ? BARRA.pinnedWidth : 0));
}
// full-width panels follow the page: when the window or the file list changes size
function fitWidth() {
  const w = Q("#dpanels").clientWidth; if (w < 100) return;
  E.panels.forEach(p => { if (p.full && !dockedP(p) && !hasDocked(p) && Math.abs(p.w - w) > 1) { p.x = 0; p.w = w; apply(p); } });
  if (window.DDA && E.panels.some(q => q.dock != null)) { relayout(); DDA.sync(); }       // Full Scan and MS2 of a DDA file side by side only when the page is wide enough
}
function restoreUi() {
  const u = NB.ui;
  if (u && u.contEx && window.LISTE) LISTE.exclImport(u.contEx);                 // the contaminant exclusions travel with the session
  if (!u || !u.panels || !u.panels.length) return false;
  const names = new Set(E.files.map(f => f.file));
  if (!u.files.every(f => names.has(f.file))) return false;
  u.files.forEach(sf => { const f = E.files.find(x => x.file === sf.file); f.label = sf.label || f.label; f.vis = sf.vis !== false; if (sf.color) { f.color = sf.color; f.colorSet = true; } if (sf.gone) { f.gone = true; f.vis = false; } });
  E.cur = Math.min(u.cur || 0, E.files.length - 1); E.browse = !!u.browse; E.tab = pickTab(u.tab);
  const kof = n => { const i = E.files.findIndex(f => f.file === n); return i < 0 ? 0 : i; };
  const made = u.panels.map(sp => addPanel(sp.type, {
    ...sp, tab: sp.tab || (sp.type === "mrm" ? "mrm" : E.files[kof(sp.k)]?.kind === "ms2" ? "ms2" : "full"), k: sp.k ? kof(sp.k) : undefined, ref: sp.ref ? kof(sp.ref) : "", bk: sp.bk ? kof(sp.bk) : "", bg: sp.bg === "w" ? "w" : sp.bg ? kof(sp.bg) : "", traces: (sp.traces || []).map(t => ({ id: E.seq++, ...t })), link: null, ints: sp.ints || [], anns: sp.anns || []
  }));
  u.panels.forEach((sp, i) => { if (sp.link >= 0 && made[sp.link]) made[i].link = made[sp.link].id; if (sp.src >= 0 && made[sp.src]) made[i].src = made[sp.src].id; });
  u.panels.forEach((sp, i) => { for (const k of ["dda", "dock", "duo"]) if (sp[k] != null) made[i][k] = sp[k] >= 0 && made[sp[k]] ? made[sp[k]].id : null; });       // DDA trio: Full Scan / MS2 side by side (dda.js)
  if (window.DDA) DDA.restored(made);
  made.forEach(q => { if (q.type === "spec" && q.link) ctl(q); });     // one live spectrum per chromatogram, even in an old notebook
  if (E.files[E.cur]?.kind !== E.tab) E.cur = (tabFiles()[0] || { k: 0 }).k;
  renderFileList(); return true;
}

// File colours. Full Scan files of type sample WITH a time: one sequential scale ordered by time, spread over the n files of the tab (rank, not the real
// time, so 7 files at 0-5-10-15-30-45-60 min are as far apart as 3 files; hiding a file does not change any colour, removing one does). Blank and
// Full Scan standards are outside the scale (neutral grey, dashed); Full Scan samples without a time use the categorical colours. MS2 and MRM files keep
// the categorical colours (MRM: transition, not time). A colour chosen by the student (colorSet) is never overwritten.
const STD_SEQ_LIGHT = ["#fdae6b", "#e6550d", "#a63603", "#5a1d02"];     // one orange hue, light (low concentration) to dark (high) on white
const STD_SEQ_DARK  = ["#ffedd5", "#fed7aa", "#fba94b", "#f97316", "#ff5722"]; // luminous peach to bright orange/coral on dark
function stdColor(u) {                                       // u in 0..1
  const seq = isDark() ? STD_SEQ_DARK : STD_SEQ_LIGHT;
  const P = PALS[UIP.pal], x = (P.hc ? 0.45 + 0.55 * u : u) * (seq.length - 1), i = Math.min(Math.floor(x), seq.length - 2), f = x - i;
  const c = h => [1, 3, 5].map(k => parseInt(h.slice(k, k + 2), 16)), A = c(seq[i]), B = c(seq[i + 1]);
  return "#" + A.map((v, k) => Math.round(v + (B[k] - v) * f).toString(16).padStart(2, "0")).join("");
}
function paintFiles() {
  setPal(UIP.pal);
  const P = PALS[UIP.pal], dark = isDark();
  const blankCol = dark && P.blankDark ? P.blankDark : P.blank;
  const stdCol = dark && P.stdDark ? P.stdDark : P.std;
  const byTime = kind => E.files.filter(f => !f.gone && f.kind === kind && f.type === "sample" && f.time != null).sort((a, b) => a.time - b.time || a.k - b.k);
  const scales = [byTime("full"), byTime("mrm")], inScale = new Set(scales.flat());
  E.files.forEach(f => { f.rank = null; });
  scales.forEach(scale => scale.forEach((f, i) => { f.rank = i; if (!f.colorSet) f.color = seqColor(scale.length > 1 ? i / (scale.length - 1) : 0.5); }));
  const cal = E.files.filter(f => !f.gone && f.kind === "mrm" && f.type === "standard" && f.conc != null).sort((a, b) => a.conc - b.conc || a.k - b.k), inCal = new Set(cal);
  cal.forEach((f, i) => { if (!f.colorSet) f.color = stdColor(cal.length > 1 ? i / (cal.length - 1) : 0.6); });
  let n = 0;
  E.files.forEach(f => {
    if (inScale.has(f) || inCal.has(f)) return;
    const own = f.colorSet;
    if (f.type === "blank") { if (!own) f.color = blankCol; }
    else if (f.type === "standard") { if (f.kind === "full") { if (!own) f.color = stdCol; } else if (!own) f.color = PAL[(n++ + 3) % PAL.length]; }
    else if (!own) f.color = PAL[n++ % PAL.length]; else n++;
  });
}
// back to the automatic colours of the current palette (the student's own colours are dropped)
function resetColors() { E.files.forEach(f => { f.colorSet = false; }); paintFiles(); renderFileList(); redrawAll(); uiSave(); }
const modeIcon = () => "";       // the little icons of the three experiments are gone from the tabs and the file list (7 Oct 2026, Federico): they stay only in Teoria, where they are big enough to read
// go to the tab of a file that is not usable in the current one, with a one-line message
function goToFileTab(f) {
  setTab(f.kind); E.cur = f.k; renderFileList(); renderNav(); redrawAll(); uiSave();
  const n = Q("#fnote"); if (n) { n.textContent = I18N.t("files.goTab", { label: f.label, kind: kindOf(f), tab: TABS.find(x => x[0] === f.kind)[1] }); clearTimeout(n._t); n._t = setTimeout(() => { n.textContent = ""; }, 7000); }
}
function reorderFiles(fromK, toK, isBefore) {
  if (fromK == null || toK == null || fromK === toK) return;
  const srcIdx = E.files.findIndex(f => f.k === fromK);
  const dstIdx = E.files.findIndex(f => f.k === toK);
  if (srcIdx < 0 || dstIdx < 0) return;
  const [src] = E.files.splice(srcIdx, 1);
  let insertAt = E.files.findIndex(f => f.k === toK);
  if (!isBefore) insertAt++;
  E.files.splice(insertAt, 0, src);

  const oldKToNewK = new Map();
  E.files.forEach((f, newIdx) => {
    oldKToNewK.set(f.k, newIdx);
    f.k = newIdx;
  });

  if (oldKToNewK.has(E.cur)) E.cur = oldKToNewK.get(E.cur);
  if (E.curBy) {
    for (const t in E.curBy) {
      if (oldKToNewK.has(E.curBy[t])) E.curBy[t] = oldKToNewK.get(E.curBy[t]);
    }
  }

  E.panels.forEach(p => {
    if (p.k != null && oldKToNewK.has(p.k)) p.k = oldKToNewK.get(p.k);
    if (p.ref !== "" && p.ref != null && oldKToNewK.has(+p.ref)) p.ref = oldKToNewK.get(+p.ref);
    if (p.bk !== "" && p.bk != null && oldKToNewK.has(+p.bk)) p.bk = oldKToNewK.get(+p.bk);
    if (p.bg !== "" && p.bg != null && p.bg !== "w" && oldKToNewK.has(+p.bg)) p.bg = oldKToNewK.get(+p.bg);
    if (Array.isArray(p.traces)) {
      p.traces.forEach(tr => {
        if (tr.k != null && oldKToNewK.has(tr.k)) tr.k = oldKToNewK.get(tr.k);
      });
    }
  });

  renderFileList();
  redrawAll();
  uiSave();
}

function renderFileList() {
  // files grouped by experiment type (Full scan, MS2, MRM...): the type is written once per group, not per file. ALL the files are listed:
  // those of the other tabs are greyed and a click takes you to their tab
  const groups = [];
  const side = window.BANCO && BANCO.on() ? E.files.filter(f => !f.gone && f.kind !== "mrm") : lrTog() ? E.files.filter(f => !f.gone) : tabFiles();       // the bench lists the files of every experiment, all of them live
  side.forEach(f => { const g = grpOf(f); let G = groups.find(x => x.g === g); if (!G) groups.push(G = { g, fs: [] }); G.fs.push(f); });
  if (window.BANCO && BANCO.on() && (E.tab === "mrm" || lrTog())) groups.sort((a, b) => ["standard", "sample", "blank"].indexOf(a.g) - ["standard", "sample", "blank"].indexOf(b.g));
  const sub = f => f.type === "sample" ? fmtTime(f.time) : (f.type === "blank" ? I18N.t("files.sub.blank") : I18N.t("files.sub.standard") + (f.conc != null && f.kind === "mrm" ? " " + f.conc + " " + (f.cunit || "") : ""));
  const row = f => {
    const hasPr = E.tab === "ms2" && f.precursors && f.precursors.length;
    const prOpen = f._prOpen !== false;
    const prList = hasPr ? [...new Set(f.precursors)].sort((a, b) => a - b) : [];
    return `<div class="fl ${f.k === E.cur ? "cur" : ""}" data-tip="${EH(f.label)}" data-k="${f.k}" draggable="true" style="flex-wrap:wrap">
      <input type="checkbox" data-k="${f.k}" ${f.vis ? "checked" : ""} title="${I18N.t("files.showHide")}">
      <i style="background:${f.color}"></i><div class="fi"><b class="nm" data-k="${f.k}">${EH(f.label)}</b><span style="display:inline-flex;align-items:center;gap:4px">${polSign(f)}<small>${sub(f)}</small>${hasPr ? `<span class="pr-tog" data-prk="${f.k}" title="${I18N.t("files.showHidePrec")}" style="cursor:pointer;padding:0 2px;user-select:none;font-size:10px;color:var(--muted)">${prOpen ? "▾" : "▸"}</span>` : ""}</span></div>
      ${hasPr ? `<div class="pr-container" style="padding-left:22px;width:100%;box-sizing:border-box;margin-top:2px;${prOpen ? "" : "display:none;"}">` +
        prList.map(pr => `<div class="fl pr" style="padding-left:0;min-height:0;margin-bottom:2px;border-bottom:none"><input type="checkbox" data-pr="${pr}" ${ms2PairOf(pr) ? "checked" : ""} title="${I18N.t("files.addPair.title")}"><div class="fi"><b class="pn" data-pg="${pr}" title="${I18N.t("files.precGo.title")}"><span style="font-style:italic">m/z</span> ${EH(pr)}</b></div></div>`).join("") +
        '</div>' : ""}
    </div>`;
  };
  const ghost = f => `<div class="fl ghost" data-go="${f.k}" data-tip="${EH(f.label)} (${EH(kindOf(f))})"><input type="checkbox" disabled><i style="background:${f.color}"></i><div class="fi"><b class="nm">${EH(f.label)}</b><small>${sub(f)}</small></div></div>`;
  const pre = window.DDA && DDA.listMode() ? DDA.listBlock() : "";
  E._grpOpen = E._grpOpen || {};
  const isExpOpen = t => E._grpOpen[t] !== undefined ? E._grpOpen[t] : (t === E.tab || lrTog());
  const others = (allTabs() ? [] : TABS).filter(([t]) => t !== E.tab && tabFiles(t).length).map(([t, n]) => {
    const open = isExpOpen(t);
    return `<div class="fgh sep" data-gt="${t}" style="cursor:pointer"><span class="chv" style="user-select:none;font-size:10px;margin-right:2px">${open ? "▾" : "▸"}</span>${modeIcon(t, 18)}<span>${EH(n)}</span><em>${tabFiles(t).length}</em></div><div class="grp-files" style="${open ? "" : "display:none"}">${tabFiles(t).map(ghost).join("")}</div>`;
  }).join("");
  const isMainOpen = isExpOpen(E.tab);
  const gh = (G, label, cls = "") => {
    const isSub = !!cls;
    const open = isSub ? true : isMainOpen;
    return `<div class="fgh ${cls}" ${isSub ? "" : `data-gt="${E.tab}" style="cursor:pointer"`}><input type="checkbox" class="gall" data-g="${EH(G.g)}" ${G.fs.every(f => f.vis) ? "checked" : ""} title="${I18N.t("files.showHideGroup")}">${isSub ? "" : `<span class="chv" style="user-select:none;font-size:10px;margin-right:2px">${open ? "▾" : "▸"}</span>`}${cls ? "" : modeIcon(E.tab, 18)}<span>${EH(label)}</span><em>${G.fs.length}</em></div>`;
  };
  // files grouped strictly by experiment type
  const body = groups.map(G => gh(G, grpLabel(G.g)) + `<div class="grp-files" style="${isMainOpen ? "" : "display:none"}">${G.fs.map(row).join("")}</div>`).join("");
  Q("#flst").innerHTML = pre + body + others;
  Q("#flst").querySelectorAll("input[data-pr]").forEach(x => x.onchange = () => ms2Toggle(x.dataset.pr, x.checked));
  { const dl = Q("#ddalist"); if (dl) dl.onclick = () => DDA.openList(); }
  Q("#flst").querySelectorAll("[data-pg]").forEach(x => x.onclick = () => ms2Goto(x.dataset.pg));
  Q("#flst").querySelectorAll("[data-go]").forEach(x => x.onclick = () => goToFileTab(E.files[+x.dataset.go]));
  Q("#flst").querySelectorAll(".fgh[data-gt]").forEach(el => el.onclick = e => {
    if (e.target.closest("input")) return;
    const t = el.dataset.gt;
    E._grpOpen[t] = !isExpOpen(t);
    renderFileList();
  });
  Q("#flst").querySelectorAll(".pr-tog").forEach(tog => tog.onclick = e => {
    e.stopPropagation();
    const f = E.files[+tog.dataset.prk];
    if (f) { f._prOpen = f._prOpen === false; renderFileList(); }
  });
  Q("#flst").querySelectorAll(".gall").forEach(x => x.onchange = () => {
    side.filter(f => x.dataset.g === "__all" || grpOf(f) === x.dataset.g).forEach(f => f.vis = x.checked);
    if (!x.checked) cleanupOrphanPanels();
    renderFileList();
    redrawAll();
    uiSave();
  });
  Q("#flst").querySelectorAll("input[data-k]").forEach(x => x.onchange = () => {
    E.files[+x.dataset.k].vis = x.checked;
    if (!x.checked) cleanupOrphanPanels();
    redrawAll();
    uiSave();
  });
  Q("#flst").querySelectorAll(".fl:not(.ghost)").forEach(el => { const nm = el.querySelector(".nm"); if (nm) el.oncontextmenu = e => fileCtx(e, E.files[+nm.dataset.k]); });
  // the WHOLE row selects the file (colour square, time, empty space); only the check box shows / hides. The list is not rebuilt on a click (only the classes change), so a double click on the name still reaches it.
  Q("#flst").querySelectorAll(".fl:not(.ghost):not(.pr)").forEach(row => {
    const nm = row.querySelector(".nm"); if (!nm) return;
    row.onclick = e => {
      if (e.target.closest("input") || e.target.closest(".pr-tog")) return;
      E.cur = +nm.dataset.k; Q("#flst").querySelectorAll(".fl").forEach(r => r.classList.toggle("cur", r === row)); renderNav(); if (E.browse) redrawAll();
    };
    nm.ondblclick = () => renameFile(E.files[+nm.dataset.k]);
  });

  // Reorder files via HTML5 drag & drop and touch long-press
  let touchTimer = null, touchDragging = null, touchStartX = 0, touchStartY = 0;
  Q("#flst").querySelectorAll(".fl[data-k]:not(.ghost):not(.pr)").forEach(el => {
    const k = +el.dataset.k;

    el.ondragstart = e => {
      e.dataTransfer.setData("text/plain", String(k));
      e.dataTransfer.effectAllowed = "move";
      el.classList.add("fl-dragging");
    };
    el.ondragover = e => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
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
    el.ondrop = e => {
      e.preventDefault();
      const fromK = +e.dataTransfer.getData("text/plain");
      const isBefore = el.classList.contains("drop-before");
      el.classList.remove("drop-before", "drop-after");
      if (!isNaN(fromK) && fromK !== k) {
        reorderFiles(fromK, k, isBefore);
      }
    };
    el.ondragend = () => {
      document.querySelectorAll("#flst .fl").forEach(f => f.classList.remove("fl-dragging", "drop-before", "drop-after"));
    };

    el.onpointerdown = e => {
      if (e.target.closest("input") || e.target.closest(".pr-tog") || e.button !== 0) return;
      touchStartX = e.clientX;
      touchStartY = e.clientY;
      touchTimer = setTimeout(() => {
        touchDragging = k;
        el.classList.add("fl-dragging");
        try { el.setPointerCapture(e.pointerId); } catch (_) {}
      }, 350);
    };

    el.onpointermove = e => {
      if (!touchDragging && touchTimer) {
        if (Math.hypot(e.clientX - touchStartX, e.clientY - touchStartY) > 8) {
          clearTimeout(touchTimer);
          touchTimer = null;
        }
      }
      if (touchDragging != null) {
        const target = document.elementFromPoint(e.clientX, e.clientY)?.closest(".fl[data-k]:not(.ghost):not(.pr)");
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

    const endTouch = e => {
      if (touchTimer) {
        clearTimeout(touchTimer);
        touchTimer = null;
      }
      if (touchDragging != null) {
        const fromK = touchDragging;
        touchDragging = null;
        try { el.releasePointerCapture(e.pointerId); } catch (_) {}
        const target = document.elementFromPoint(e.clientX, e.clientY)?.closest(".fl[data-k]:not(.ghost):not(.pr)");
        const isBefore = target?.classList.contains("drop-before");
        document.querySelectorAll("#flst .fl").forEach(f => f.classList.remove("fl-dragging", "drop-before", "drop-after"));
        if (target && +target.dataset.k !== fromK) {
          reorderFiles(fromK, +target.dataset.k, isBefore);
        }
      }
    };
    el.onpointerup = endTouch;
    el.onpointercancel = endTouch;
  });
  renderNav();
}
// right click on a file of the list: rename (only a label, the file on disk is not touched), colour, remove from the session
const FILE_COLORS = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#F0E442", "#000000", "#7a3b9b", "#8a8a8a"];   // Okabe-Ito + 2: distinguishable with colour-blindness
async function renameFile(f) { const v = await ask(I18N.t("files.rename.prompt"), f.label); if (v) { f.label = v; renderFileList(); renderNav(); redrawAll(); uiSave(); } }
function colorFile(f, ev) {
  Q("#colpop")?.remove();
  const d = document.createElement("div"); d.id = "colpop";
  d.innerHTML = `<div class="sm" style="margin-bottom:4px">${I18N.t("files.color.of", { label: EH(f.label) })}</div><div class="sw">${FILE_COLORS.map(c => `<button data-c="${c}" style="background:${c}" title="${c}"></button>`).join("")}</div><label class="sm">${I18N.t("files.color.custom")} <input type="color" value="${/^#[0-9a-f]{6}$/i.test(f.color) ? f.color : "#1f77b4"}"></label>`;
  const set = c => { f.color = c; f.colorSet = true; d.remove(); renderFileList(); redrawAll(); uiSave(); };
  d.querySelectorAll("[data-c]").forEach(b => b.onclick = () => set(b.dataset.c));
  d.querySelector("input").onchange = e => set(e.target.value);
  document.body.appendChild(d); d.style.left = Math.min(ev.clientX, innerWidth - 200) + "px"; d.style.top = Math.min(ev.clientY, innerHeight - 150) + "px";
  setTimeout(() => document.addEventListener("mousedown", function h(e) { if (!d.contains(e.target)) { d.remove(); document.removeEventListener("mousedown", h); } }), 0);
}
async function removeFile(f) {
  if (!await yesno(I18N.t("files.remove.confirm", { label: EH(f.label) }))) return;
  f.gone = true; f.vis = false; paintFiles();
  E.panels.filter(q => q.k === f.k && q.type !== "xic").forEach(q => { const o = tabFiles(q.tab).find(x => x.k !== f.k); if (o) q.k = o.k; });
  if (E.cur === f.k) E.cur = (tabFiles()[0] || { k: 0 }).k;
  renderTabs(); renderFileList(); renderNav(); toolbar(); E.panels.forEach(ctl); redrawAll(); uiSave();
}
function fileCtx(e, f) {
  if (!f) return; e.stopPropagation();
  menu(e, [{ label: I18N.t("files.ctx.rename"), fn: () => renameFile(f) }, { label: I18N.t("files.ctx.color"), fn: () => colorFile(f, e) }, { label: I18N.t("files.ctx.resetColors"), fn: resetColors }, "-", { label: I18N.t("files.ctx.remove"), fn: () => removeFile(f) }]);
}
// quick file chooser opened from a graph header: tick/untick files (same switch as the list on the left), click a name to show only that file
function fileMenu(btn) {
  Q("#fpop")?.remove();
  const d = document.createElement("div"); d.id = "fpop";
  const paint = () => {
    const gs = []; tabFiles().forEach(f => { const g = grpOf(f); let G = gs.find(x => x.g === g); if (!G) gs.push(G = { g, fs: [] }); G.fs.push(f); });
    d.innerHTML = `<div class="fpb"><button data-all="1">${I18N.t("files.pop.all")}</button><button data-all="0">${I18N.t("files.pop.none")}</button></div>` + gs.map(G => `<div class="fgh"><span>${EH(grpLabel(G.g))}</span></div>` + G.fs.map(f => `<label><input type="checkbox" data-k="${f.k}" ${f.vis ? "checked" : ""}><i style="background:${f.color}"></i><span class="fn" data-k="${f.k}" title="${I18N.t("files.pop.only.title")}">${EH(f.label)}</span></label>`).join("")).join("");
    const done = () => { renderFileList(); redrawAll(); uiSave(); };
    d.querySelectorAll("[data-all]").forEach(b => b.onclick = () => { tabFiles().forEach(f => f.vis = b.dataset.all === "1"); if (b.dataset.all === "0") cleanupOrphanPanels(); paint(); done(); });
    d.querySelectorAll("input").forEach(i => i.onchange = () => { E.files[+i.dataset.k].vis = i.checked; if (!i.checked) cleanupOrphanPanels(); done(); });
    d.querySelectorAll(".fn").forEach(n => n.onclick = e => { e.preventDefault(); tabFiles().forEach(f => f.vis = f.k === +n.dataset.k); cleanupOrphanPanels(); paint(); done(); });
  };
  paint(); document.body.appendChild(d);
  const r = btn.getBoundingClientRect(); d.style.left = Math.min(r.left, innerWidth - 240) + "px"; d.style.top = r.bottom + 4 + "px";
  setTimeout(() => document.addEventListener("mousedown", function h(e) { if (!d.contains(e.target)) { d.remove(); document.removeEventListener("mousedown", h); } }), 0);
}
const vis = () => tabFiles().filter(f => f.vis);
// "Solo il selezionato": every graph follows the file chosen in the toolbar, so the controls that choose a file inside a graph are switched off
const follow = tab => !!(E.browse && E.files[E.cur] && E.files[E.cur].kind === tab);
const shown = () => E.browse && E.files[E.cur] && E.files[E.cur].kind === E.tab ? [E.files[E.cur]] : vis();
// the files a cell draws: those of ITS tab (the same as shown() unless, in the bench, the cell is not of the tab of the active cell)
const shownFor = p => (p && p.tab && p.tab !== E.tab ? tabFiles(p.tab).filter(f => f.vis) : shown());
const scanFiles = l => l.filter(f => f.kind !== "mrm");

// ------------------------------------------------------------------ barra: frecce tra i file, metodo, esporta
function renderNav() {
  const s = Q("#fsel"); if (!s) return;
  const shortName = n => n.length > 34 ? n.slice(0, 20) + "…" + n.slice(-13) : n;          // a very long name is cut in the middle, so the final time stays visible
  s.innerHTML = tabFiles().map(f => `<option value="${f.k}" ${f.k === E.cur ? "selected" : ""} title="${EH(f.label)}">${EH(shortName(f.label))}</option>`).join("");
  const cur = tabFiles().find(f => f.k === E.cur);
  const g = document.createElement("canvas").getContext("2d"), cs = getComputedStyle(s); g.font = `${cs.fontSize} ${cs.fontFamily}`;
  s.style.width = Math.ceil(Math.max(0, ...[...s.options].map(o => g.measureText(o.textContent).width)) + 36) + "px";   // as wide as the longest name (CSS sets the minimum and the maximum)
  const tf = tabFiles(), one = tf.length <= 1, why = I18N.t("nav.needMore");   // with a single file nothing here has a meaning
  // a single file: "Solo il selezionato" is on and "Tutti sovrapposti" is off; with two or more files the choice the student had made comes back
  if (one && tf.length) { if (!E._forced) { E._forced = true; E._prev = !!E.browse; } E.browse = true; E.cur = tf[0].k; }
  else if (E._forced) { E._forced = false; E.browse = !!E._prev; }
  Q("#fmode").querySelectorAll("button").forEach(b => b.classList.toggle("on", (b.dataset.m === "sel") === !!E.browse));
  const nowhy = I18N.t("nav.onlySelected");
  [Q("#fprev"), Q("#fnext"), Q("#fsel"), ...Q("#fmode").querySelectorAll("button")].forEach(b => { if (b.dataset.t0 == null) b.dataset.t0 = b.title; const nav = !b.closest("#fmode"), all = b.dataset.m === "all"; b.disabled = nav ? one || !E.browse : one && all; b.title = one ? (all ? I18N.t("nav.oneFile") : nav ? why : b.dataset.t0) : nav && !E.browse ? nowhy : b.dataset.t0; });
  if (cur && !s.disabled) s.title = cur.label;                       // the whole name, also when it had to be cut
  const sig = [E.browse, E.cur, tabFiles().length, E.tab].join("|"); if (renderNav._sig !== sig) { renderNav._sig = sig; E.panels.forEach(q => q.el && ctl(q)); }      // the graphs' own file choosers depend on the mode
  Q("#fmode").classList.toggle("dis", one);
}
function goFile(d) {
  const tf = tabFiles(); if (tf.length < 2 || !E.browse) return;
  E.cur = tf[(Math.max(0, tf.findIndex(f => f.k === E.cur)) + d + tf.length) % tf.length].k; renderNav();
  renderFileList(); redrawAll(); uiSave();
}
Q("#fprev").onclick = () => goFile(-1); Q("#fnext").onclick = () => goFile(1);
Q("#fsel").onchange = e => { E.cur = +e.target.value; renderFileList(); renderNav(); if (E.browse) redrawAll(); uiSave(); };
Q("#fmode").querySelectorAll("button").forEach(b => b.onclick = () => { E.browse = b.dataset.m === "sel"; renderNav(); redrawAll(); uiSave(); });
// ← → (scan by scan or file by file), Maiusc + ← →, Spazio and Esc: see scroll.js (scrollKeys)
document.addEventListener("mousedown", e => { if (E.active && e.target.closest("#dpanels") && !e.target.closest(".pnl")) setActive(null); });
function toolbar() {
  if (window.renderTabs) renderTabs();
  const t = E.tab, empty = !tabFiles().length, scan = t !== "mrm", mrm = t === "mrm";
  Q("#np-chrom").hidden = !scan; Q("#np-spec").hidden = !scan; Q("#np-map").hidden = t !== "full"; Q("#np-xic").hidden = !scan;
  Q("#np-mrm").hidden = !mrm; 
  Q("#g-add").hidden = empty; Q("#g-tools").hidden = empty; Q("#g-file").hidden = empty; Q("#dpanels").hidden = empty; Q("#tools .sp").hidden = empty; Q("#np-method").hidden = empty;
  if (window.emptyTab) emptyTab();
}
// ------------------------------------------------------------------ piccole finestre e menu
function ask(title, def = "") {
  return new Promise(res => {
    const d = Q("#askdlg"), i = Q("#askin");
    i.hidden = false; Q("#askno").hidden = false;
    Q("#asktxt").innerHTML = title; i.value = def;
    const done = v => { d.close(); Q("#askok").onclick = Q("#askno").onclick = i.onkeydown = null; res(v); };
    Q("#askok").onclick = () => done(i.value.trim());
    Q("#askno").onclick = () => done(null);
    i.onkeydown = e => { if (e.key === "Enter") { e.preventDefault(); done(i.value.trim()); } };
    d.oncancel = () => done(null);
    d.showModal(); i.focus(); i.select();
  });
}
function yesno(html) {
  return new Promise(res => {
    const d = Q("#askdlg"); Q("#asktxt").innerHTML = html; Q("#askin").hidden = true; Q("#askno").hidden = false;
    const done = v => { d.close(); Q("#askin").hidden = false; Q("#askok").onclick = Q("#askno").onclick = null; res(v); };
    Q("#askok").onclick = () => done(true); Q("#askno").onclick = () => done(false); d.oncancel = () => done(false); d.showModal();
  });
}
function info(html) {
  const d = Q("#askdlg");
  Q("#asktxt").innerHTML = html; Q("#askin").hidden = true; Q("#askno").hidden = true;
  Q("#askok").onclick = () => { d.close(); Q("#askin").hidden = false; Q("#askno").hidden = false; };
  d.showModal();
}
function big(title, html, onopen) {
  Q("#bigtt").textContent = title; Q("#bigbody").innerHTML = html;
  const d = Q("#bigdlg"); Q("#bigx").onclick = () => d.close(); d.showModal(); if (onopen) onopen();
}
function menu(ev, items) {
  ev.preventDefault();
  const m = Q("#ctx");
  m.innerHTML = "";
  items.forEach(it => {
    if (it === "-") { m.appendChild(document.createElement("hr")); return; }
    const d = document.createElement("div");
    if (it.icon) {
      const ic = document.createElement("span");
      ic.innerHTML = it.icon;
      ic.style.display = "inline-flex";
      ic.style.alignItems = "center";
      ic.style.marginRight = "6px";
      ic.style.verticalAlign = "middle";
      d.appendChild(ic);
      d.appendChild(document.createTextNode(it.label));
    } else {
      d.textContent = it.label;
    }
    if (it.tip) d.title = it.tip;
    d.setAttribute("role", "menuitem");
    if (it.dim) d.className = "dim"; else { d.tabIndex = -1; d.onclick = () => { m.hidden = true; it.fn(); }; }
    m.appendChild(d);
  });
  m.hidden = false; m.setAttribute("role", "menu");
  if (!m._kb) { m._kb = true; m.addEventListener("keydown", e => {                 // keyboard: arrows move, Enter/Space choose, Esc closes
    const its = [...m.querySelectorAll('div[role=menuitem]:not(.dim)')], i = its.indexOf(document.activeElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); if (its.length) its[(i + (e.key === "ArrowDown" ? 1 : -1) + its.length) % its.length].focus(); }
    else if (e.key === "Enter" || e.key === " ") { if (i >= 0) { e.preventDefault(); its[i].click(); } }
    else if (e.key === "Escape") { e.preventDefault(); m.hidden = true; if (m._ret && m._ret.focus) m._ret.focus(); }
  }); }
  m._ret = ev.kbd ? document.activeElement : null;
  const off = document.documentElement.classList.contains("touch") ? 16 : 0;      // with a finger the menu opens beside it: lifting the finger must not click an item
  m.style.left = Math.min(ev.clientX + off, innerWidth - 270) + "px";
  m.style.top = Math.max(4, Math.min(ev.clientY + off, innerHeight - m.offsetHeight - 8)) + "px";
  if (ev.kbd) { const f = m.querySelector('div[role=menuitem]:not(.dim)'); if (f) f.focus(); }
}
document.addEventListener("click", () => { Q("#ctx").hidden = true; });
document.addEventListener("click", e => { document.querySelectorAll(".corrm:not([hidden])").forEach(m => { if (!m.parentNode.contains(e.target)) m.hidden = true; }); });      // the Correzione menu closes when the click is elsewhere
document.addEventListener("keydown", e => { if (e.key === "Escape") document.querySelectorAll(".corrm:not([hidden])").forEach(m => { m.hidden = true; }); });
document.addEventListener("keydown", e => { if (e.key === "Escape") { e._ctxWas = !Q("#ctx").hidden; Q("#ctx").hidden = true; } });      // _ctxWas: this Esc only closed the menu (the full-screen handler below leaves the panel alone)
function dl(name, text, type = "application/octet-stream") {
  const a = document.createElement("a"); a.href = URL.createObjectURL(new Blob([text], { type })); a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 4000);
}
const INT_COLS = ["panel", "ion", "file", "time", "conc", "cunit", "a", "b", "rt", "area", "height", "ibk"];
const INT_HEADS = ["panel", "trace", "sample", "time", "conc", "unit", "rt0", "rt1", "apex", "area", "height", "ibk"].map(k => I18N.t(`export.int.${k}`));
const LOCALE = I18N.lang === "it" ? "it-IT" : "en-GB";           // dates in the exported files
// data of a plot as columns side by side (each trace has its own x), one sheet with the units in the headers: for graphs and tables made in Excel
function plotSheets(p) {
  const a = p._a; if (!a) return null;
  const cols = [], yt = p.kind === "pda" ? I18N.t("export.col.pdaSignal") : I18N.t("export.col.intensity");
  if (p.type === "spec") a.data.forEach(x => { cols.push([`m/z - ${x.f.label}`, x.d.mz], [`${I18N.t("export.col.intensity")} - ${x.f.label}`, x.d.y0 || x.d.y]); });
  else if (a.sr) {
    const same = p.type === "chrom" && a.sr.length > 1 && a.sr.every(q => q.x.length === a.sr[0].x.length && q.x.every((v, i) => v === a.sr[0].x[i]));   // total chromatograms of one run type share the time axis: one RT column
    if (same) { cols.push(["RT (min)", a.sr[0].x]); a.sr.forEach(sr => cols.push([`${yt} - ${sr.name}`, sr.y])); }
    else a.sr.forEach(sr => { cols.push([`RT (min) - ${sr.name}`, sr.x], [`${yt} - ${sr.name}`, sr.y]); });
  }
  if (!cols.length) return null;
  const n = Math.max(...cols.map(c => c[1].length)), rows = [];
  for (let i = 0; i < n; i++) rows.push(cols.map(c => (i < c[1].length ? c[1][i] : null)));
  const out = [{ name: ptitle(p.title) || I18N.t("export.sheet.data"), head: cols.map(c => c[0]), rows, widths: cols.map(c => Math.min(44, Math.max(14, c[0].length + 2))) }];
  const metaRows = [
    [{ v: I18N.t("export.meta.property"), b: true }, { v: I18N.t("export.meta.value"), b: true }],
    [{ v: I18N.t("export.meta.title"), b: true }, ptitle(p.title) || I18N.t("export.meta.chart")],
    [{ v: I18N.t("export.meta.type"), b: true }, { chrom: I18N.t("export.type.chrom", { kind: (p.kind || "").toUpperCase() }), xic: I18N.t("export.type.xic"), mrm: I18N.t("panel.title.mrm"), spec: I18N.t("export.type.spec", { ms: p.level === 2 ? "MS²" : "MS¹" }), map: I18N.t("panel.title.map") }[p.type] || p.type],
    [{ v: I18N.t("export.meta.samples"), b: true }, plotFiles(p).map(f => f.label).join(", ")],
    [{ v: I18N.t("export.meta.caption"), b: true }, plotCaption(p)],
    [{ v: I18N.t("export.meta.date"), b: true }, new Date().toLocaleString(LOCALE)],
    [{ v: "Software", b: true }, APP_NAME]
  ];
  if (p.r0 != null && p.r1 != null) metaRows.push([{ v: I18N.t("export.meta.rtRange"), b: true }, `${p.r0.toFixed(2)} - ${p.r1.toFixed(2)}`]);
  if (a.x0 != null && a.x1 != null) metaRows.push([{ v: I18N.t("export.meta.axis"), b: true }, `${a.x0.toFixed(2)} - ${a.x1.toFixed(2)}`]);
  if (p.ibk) metaRows.push([{ v: I18N.t("export.meta.ibk"), b: true }, ibkText(p.ibk)]);
  out.push({ name: I18N.t("export.sheet.meta"), rows: metaRows, widths: [24, 60] });
  if (p.ibk) out.push({ name: I18N.t("export.sheet.notes"), rows: [[{ v: I18N.t("export.meta.ibkApplied"), b: true }, ibkText(p.ibk)], [I18N.t("export.note.ibk")]], widths: [28, 60] });
  return out;
}

