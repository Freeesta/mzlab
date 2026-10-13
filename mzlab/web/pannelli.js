// @ts-strict
// @ts-check
// Floating panels of the Dati view: placement, drag, resize, tabs, saved images / Excel names. Split from explore.js: classic script, loaded right after it (shared globals).
// ------------------------------------------------------------------ pannelli mobili
function place(w, h) {
  // a new panel goes under the existing ones, as wide as the page
  const W = w || hostWidth(), H = h || 300;
  const tp = tabPanels(), y = tp.length ? Math.max(...tp.map(p => p.y + p.h)) + pnlGap() : 0;
  return { x: 0, y, w: W, h: H, full: !w };
}
// fixed slots: full-width panels sit one under the other; dragging one up or down makes the others change place
// (final = false: the dragged panel follows the mouse; final = true: it drops into its slot)
// spectra made from a chromatogram (double click, or linked) travel with it when it is dragged
const dockedP = q => !!(window.DDA && DDA.docked(q));                       // MS2 spectrum of a DDA trio sitting to the right of its Full Scan (dda.js)
const hasDocked = p => !!(window.DDA && E.panels.some(q => q.dock === p.id && DDA.docked(q)));
const followers = p => {
  const f = tabPanels().filter(q => q !== p && q.full && q.el && q.type === "spec" && (q.link === p.id || q.src === p.id));
  return [...f, ...tabPanels().filter(q => q.full && q.el && q.type === "spec" && q.dock != null && f.some(x => x.id === q.dock))];       // + the MS2 panel docked to one of them
};
const pnlGap = () => /** @type {any} */ (pnlGap).v ??= (parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--pnl-gap")) || 6);      // space between stacked panels: the CSS token
const gapAfter = (a, b) => (window.DDA && b && DDA.joined(a, b)) ? 0 : pnlGap();       // the chromatogram and its Full Scan of a DDA trio touch each other
function restack(drag, final, grp = []) {
  grp = grp.filter(q => !dockedP(q));
  const st = tabPanels().filter(q => q.full && q.el && !dockedP(q));
  const others = st.filter(q => q !== drag && !grp.includes(q)).sort((a, b) => a.y - b.y), c = drag.y + drag.h / 2;
  let idx = others.filter(q => q.y + q.h / 2 < c).length;
  if (others.length && drag.y < others[0].y + others[0].h * 0.5) idx = 0;
  const order = others.slice(); order.splice(idx, 0, drag, ...grp);      // the group stays together, in its own order
  placeRows(order, q => final || (q !== drag && !grp.includes(q)));           // the others close the gap; the dragged ones are placed at the drop
  E.panels.sort(LAYOUT.ordine);
  if (window.DDA) DDA.sync();
}
function relayout() {      // after a panel changes height: keep the order, close or open the gap
  placeRows(stackOrder());
  if (window.DDA) DDA.sync();
}
const minW = q => q.type === "map" ? 480 : 440;       // narrowest a panel is drawn side by side (a row that does not fit is shown stacked)
function placeRows(order, only) {      // rows of the stack (layout.js): y for all, x/w/h for the panels that share a row
  const g = LAYOUT.colloca(order, hostWidth(), pnlGap(), minW, gapAfter);
  order.forEach(q => { const r = g.pos.get(q); if (!r || only && !only(q)) return; q.x = r.x; q.y = r.y; q.w = r.w; q.h = r.h; apply(q); });
}
// small grey number in the title of every panel of the tab, in order from the top (follows moves); keys 1-9 activate the panel
function numberPanels() {
  const st = tabPanels().filter(q => q.el).sort(LAYOUT.ordine);
  E.panels.forEach(q => { const n = q.el && q.el.querySelector(".pnum"); if (n) n.hidden = true; });
  st.forEach((q, i) => { const n = q.el.querySelector(".pnum"); q.num = i + 1; n.textContent = i + 1; n.hidden = false; n.title = I18N.t("panel.num.title", { n: i + 1 }); });
}
document.addEventListener("keydown", e => {
  if (S.view !== "data" || e.ctrlKey || e.metaKey || e.altKey || !/^[1-9]$/.test(e.key) || /INPUT|SELECT|TEXTAREA/.test(document.activeElement?.tagName || "") || Q("dialog[open]")) return;
  if (fsPanel()) return;
  const q = tabPanels().filter(x => x.el).sort(LAYOUT.ordine)[+e.key - 1]; if (!q) return;
  setActive(q); front(q.el); window.scrollTo({ top: Q("#dpanels").getBoundingClientRect().top + scrollY + q.y - hdrH() - 12, behavior: "smooth" });
});
function fitHost() { if (window.DDA) DDA.sync(); numberPanels(); if (window.AFFIANCA) AFFIANCA.sync(); Q("#dpanels").style.height = Math.max(520, ...tabPanels().map(p => p.y + p.h + 16)) + "px"; pairArrows(); }
// MS2 tab: a short arrow between the precursor chromatogram (above) and the product-ion spectrum it feeds (below)
function pairArrows() {
  const host = Q("#dpanels"); host.querySelectorAll(".parr").forEach(e => e.remove());
  tabPanels().filter(s => s.type === "spec" && s.tab === "ms2" && s.link != null && s.el && s.full).forEach(s => {
    const c = E.panels.find(q => q.id === s.link && q.type === "chrom"); if (!c || !c.el) return;
    const gap = s.y - (c.y + c.h); if (gap < 0 || gap > 24) return;
    const a = document.createElement("div"); a.className = "parr"; a.title = I18N.t("panel.pairArrow.title");
    a.style.top = (c.y + c.h - 3) + "px"; a.style.height = (gap + 6) + "px";
    a.innerHTML = `<svg width="14" height="${gap + 6}" viewBox="0 0 14 16" preserveAspectRatio="none" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M7 1v12M3 9.5l4 4 4-4"/></svg>`;
    host.appendChild(a);
  });
}
// a full-width panel (with its child spectra) swaps with its neighbour; the panels are moved by dragging their header (movePanel is the same move, for tests)
function stackOrder() { return tabPanels().filter(q => q.full && q.el && !dockedP(q)).sort(LAYOUT.ordine); }
// block that moves together: a chromatogram with its child spectra; the others, in order
function blockOf(p) { const grp = [p, ...(p.type === "chrom" || p.type === "xic" || p.type === "mrm" ? followers(p).sort((a, b) => a.y - b.y) : [])].filter(q => q === p || !dockedP(q)), st = stackOrder(), others = st.filter(q => !grp.includes(q)); return { grp, others, at: others.filter(q => q.y < p.y).length }; }
function movePanel(p, dir) {
  if (!p.full) return;
  const b = blockOf(p), j = b.at + dir; if (j < 0 || j > b.others.length) return;
  const order = b.others.slice(); order.splice(j, 0, ...b.grp);
  placeRows(order);
  if (window.DDA) DDA.sync();
  E.panels.sort(LAYOUT.ordine); front(p.el); uiSave();
}
const apply = p => { p.el.style.left = p.x + "px"; p.el.style.top = p.y + "px"; p.el.style.width = p.w + "px"; p.el.style.height = p.h + "px"; };

// a panel the student has just asked for may be below the fold: the page scrolls to it and its border flashes for a second (only a scroll if the system asks for less motion)
function reveal(p) {
  if (!p || !p.el) return p;
  const calm = matchMedia("(prefers-reduced-motion: reduce)").matches;
  FRAME.next(() => FRAME.next(() => {
    p.el.scrollIntoView({ block: "nearest", behavior: calm ? "auto" : "smooth" });
    if (!calm) { p.el.classList.remove("flash"); void p.el.offsetWidth; p.el.classList.add("flash"); setTimeout(() => p.el.classList.remove("flash"), 1100); }
  }));
  return p;
}
// Panel titles: the default ones are saved as CODES ("@chrom"), never as translated text, so a saved session opens in either language.
// A title the student typed is kept as written. Italian titles saved by earlier versions become codes when the panel is created.
const PT = { chrom: "@chrom", spec: "@spec", xic: "@xic", mrm: "@mrm", map: "@map", product: "@product", isosim: "@isosim", composite: "@composite" };
const PT_OLD = { "Cromatogramma": PT.chrom, "Spettro di massa": PT.spec, "Ione estratto (XIC)": PT.xic, "Transizioni MRM": PT.mrm, "Mappa RT-m/z": PT.map, "Spettro degli ioni prodotto": PT.product, "Simulazione isotopica": PT.isosim, "Spettro composito": PT.composite };
const ptProduct = prec => prec != null ? `@product:${prec}` : PT.product;
const ptMs2 = prec => prec != null ? `@ms2prec:${prec}` : "MS2";
const ptComposite = (t0, t1) => `@composite:${t0}:${t1}`;
function ptNorm(s) {
  if (!s || typeof s !== "string") return s;
  if (PT_OLD[s]) return PT_OLD[s];
  let m = /^Spettro degli ioni prodotto · (\S+)$/.exec(s); if (m) return "@product:" + m[1];
  m = /^MS2 · precursore (\S+)$/.exec(s); if (m) return "@ms2prec:" + m[1];
  m = /^Spettro composito · RT (\S+)–(\S+) min$/.exec(s); if (m) return ptComposite(m[1], m[2]);
  return s;
}
function ptitle(s) {                                    // the title to show
  if (!s || s[0] !== "@") return s || "";
  const [c, ...a] = s.slice(1).split(":");
  if (c === "product") return I18N.t("panel.title.product") + (a[0] != null ? " · " + a[0] : "");
  if (c === "ms2prec") return I18N.t("panel.title.ms2prec", { prec: a[0] });
  if (c === "composite") return a.length ? I18N.t("panel.title.composite", { t0: a[0], t1: a[1] }) : I18N.t("panel.title.compositeBare");
  return ["chrom", "spec", "xic", "mrm", "map", "isosim"].includes(c) ? I18N.t(`panel.title.${c}`) : s;
}
function closePanel(p) {
  if (!p) return;
  if (window.DDA) DDA.onClose(p);
  fsInert(null, p);
  if (p._ro) p._ro();
  if (p._ac) p._ac.abort();                              // every listener the panel put on document/window goes with it
  p._a = null;
  if (p.el) p.el.remove();
  if (E.active === p) setActive(null);
  STATO.rimuovi(p, "close");
}

function closeAllPanels() {
  const all = E.panels.slice();
  all.forEach(p => closePanel(p));
  relayout();
  fitHost();
  uiSave();
  if (tabFiles("ms2").length) renderFileList();
  redrawAll();
}

function cleanupOrphanPanels() {
  let changed = false;
  for (let iter = 0; iter < 5; iter++) {
    const toClose = [];
    for (const p of E.panels) {
      if (p.type === "spec") {
        const f = E.files[p.k];
        const linked = p.link ? E.panels.find(q => q.id === p.link) : null;
        if (p.link) {
          if (!linked || toClose.includes(linked)) {
            toClose.push(p);
          } else if (!f || !f.vis || f.gone) {
            const nf = vis()[0] || tabFiles(p.tab).find(x => x.vis && !x.gone);
            if (nf) { p.k = nf.k; if (p.el) draw(p); }
            else toClose.push(p);
          }
        } else {
          if (!f || !f.vis || f.gone) {
            toClose.push(p);
          }
        }
      } else if (p.type === "map") {
        const f = E.files[p.k];
        if (!f || !f.vis || f.gone) {
          toClose.push(p);
        }
      } else if (p.type === "chrom" || p.type === "mrm" || p.type === "xic") {
        const tab = p.tab || E.tab;
        const fs = (tab && tab !== E.tab ? tabFiles(tab) : shownFor(p)).filter(f => f.vis && !f.gone);
        if (fs.length === 0) {
          toClose.push(p);
        }
      }
    }
    if (!toClose.length) break;
    toClose.forEach(p => closePanel(p));
    changed = true;
  }
  if (changed) {
    relayout();
    fitHost();
    if (tabFiles("ms2").length) renderFileList();
  }
}

function addPanel(type, o, after) {
  const p = { id: E.seq++, type, anns: [], ints: [], ...o };
  if (!p.tab) { p.tab = type === "mrm" ? "mrm" : E.tab === "mrm" ? (tabFiles("full").length || !tabFiles("ms2").length ? "full" : "ms2") : E.tab; if (p.tab !== E.tab && window.setTab) window.setTab(p.tab, true, "new-panel"); }
  const lv0 = (E.files[o?.k ?? 0] || {}).lv || 1;
  if (type === "chrom") Object.assign(p, { prec: o.prec ?? null, kind: o.kind || "tic", sel: null, cur: null, smooth: o.smooth ?? true, zoom: o.zoom || null, title: ptNorm(o.title) || PT.chrom });
  if (type === "spec") Object.assign(p, { k: o.k ?? 0, all: !!o.all, r0: o.r0 ?? null, r1: o.r1 ?? null, level: o.level ?? lv0, prec: o.prec ?? null, zoom: o.zoom || null, title: ptNorm(o.title) || PT.spec });
  if (type === "xic") Object.assign(p, { traces: o.traces || [], tol: o.tol ?? TOL0, smooth: o.smooth ?? true, sel: null, cur: null, zoom: o.zoom || null, title: ptNorm(o.title) || PT.xic });
  if (type === "mrm") Object.assign(p, { tr: o.tr ?? "", smooth: false, sel: null, cur: null, zoom: o.zoom || null, title: ptNorm(o.title) || PT.mrm, _trs: [] });
  if (type === "map") Object.assign(p, { k: o.k ?? 0, scale: o.scale || "lin", gamma: o.gamma, ref: o.ref ?? "", norm: o.norm || "abs", corr: o.corr, bgr: o.bgr || null, zoom: o.zoom || null, zoomY: o.zoomY || null, sel: null, cur: null, title: ptNorm(o.title) || PT.map });
  if (type !== "map") p.zoomY = Array.isArray(o.zoomY) && o.zoomY.length === 2 ? o.zoomY : null;   // y zoom of the plot (an old notebook's "yz" is simply ignored)
  if (type !== "spec" && type !== "map") Object.assign(p, { mode: o.mode || "ovl", log: !!o.log, hid: o.hid || {} });
  if (type !== "spec" && type !== "map") Object.assign(p, { bk: o.bk ?? "", snip: !!o.snip, snipw: o.snipw ?? 1 });
  if (type !== "spec" && type !== "map") {
    p.pdaAlign = !!o.pdaAlign; p.pdaDelay = typeof o.pdaDelay === "number" ? o.pdaDelay : null; p.overlay = o.overlay === "grad" || o.overlay === "press" ? o.overlay : null; p.gdelay = +o.gdelay || 0;           // gradient over the traces (gradiente.js) and its delay at the column, in min
    p.tl = o.tl ?? true;                                                  // the chromatograms of a view are linked by default (one panel can be unlinked)
    const mate = p.tl && o.zoom === undefined ? E.panels.find(q => q.tl && q.tab === p.tab && q.type !== "spec" && q.type !== "map" && q.zoom) : null;
    if (mate) p.zoom = [...mate.zoom];
  }                 // linked time axis
  if (type === "chrom") p.ms2tri = !!o.ms2tri;                          // triangles of the MS2 scans on the survey chromatogram
  if (type === "xic") p.adduct = o.adduct || defAdduct();
  if (type === "spec") Object.assign(p, { bg: o.bg ?? "", bw0: o.bw0 ?? null, bw1: o.bw1 ?? null, lock: o.lock ? { pending: true } : null, si: null,
    rel: typeof o.rel === "boolean" ? o.rel : null, sticks: !!o.sticks, mk: typeof o.mk === "boolean" ? o.mk : null, noCont: !!o.noCont, thr: o.thr ?? 5, nlab: o.nlab ?? 10, dec: o.dec ?? null, meas: o.meas && Array.isArray(o.meas.list) ? o.meas : null, rul: false });   // lock: only open/closed is saved, the values are measured again
  if (type !== "xic") p.traces = p.traces || [];
  const g = (p.w && p.h) ? { x: p.x ?? 0, y: p.y ?? 0, w: p.w, h: p.h } : place(undefined, type === "map" ? 360 : undefined);
  Object.assign(p, g);
  const el = document.createElement("div");
  el.className = "card pnl " + type;
  el.innerHTML = `<div class="hd"><span class="pnum" hidden></span><b class="ttl" title="${EH(shortHelp("pnl-" + type))} ${I18N.t("panel.rename.hint")}"></b>${type === "spec" ? '<span class="rtl"></span>' : ""}<span class="ctl"></span><span class="rd"></span>${(() => {
    // button groups, separated by a vertical bar: [zoom + reset] | [integration] | [XIC] | [PNG / Excel] □ ×
    const lens = type === "chrom" || type === "xic" || type === "mrm";
    const fit = `<button class="bt" data-a="fit" disabled title="${I18N.t("panel.fit.title")}">${IC_FIT}</button>`;
    const gv = `<span class="tbg">${lens ? `<button class="bt" data-a="izoom" title="${I18N.t("panel.izoom.title")}">${IC_ZOOM}</button>` : ""}${fit}</span>`;
    const gs = type === "spec" ? `<span class="tbg"><button class="bt" data-a="rul" title="${I18N.t("panel.rul.title")}">${IC_RULER}</button><button class="bt" data-a="par" title="${I18N.t("panel.par.title")}" aria-label="${I18N.t("panel.par.aria")}">&#9662;</button><button class="bt${p.rel ? " on" : ""}" data-a="rel" title="${I18N.t("panel.rel.title")}">%</button></span>` : "";
    const gl = lens ? `<span class="tbg"><button class="bt${p.tl ? " on" : ""}" data-a="tlink" aria-pressed="${!!p.tl}" aria-label="${I18N.t("panel.tlink.aria")}" title="${I18N.t("panel.tlink.title")}">${IC_SYNC}</button></span>` : "";
    const gi = lens ? `<span class="tbg"><button class="bt" data-a="iauto" title="${I18N.t("panel.iauto.title")}">${IC_AUTO}</button><button class="bt" data-a="iman" title="${I18N.t("panel.iman.title")}">${IC_MAN}</button><button class="bt" data-a="idel" title="${I18N.t("panel.idel.title")}">${IC_DEL_INT}</button><select class="bt" data-a="intf" hidden title="${I18N.t("panel.intf.title")}"></select><button class="bt" data-a="iclr" hidden title="${I18N.t("panel.iclr.title")}">${I18N.t("panel.iclr")}</button><button class="bt" data-a="itab" hidden title="${I18N.t("panel.itab.title")}">${IC_TAB}</button></span>` : "";
    const gt = lens ? `<span class="tbg"><button type="button" class="bt tgs" data-a="smooth" aria-pressed="false" aria-label="${I18N.t("ctl.smooth")}" title="${I18N.t("ctl.smooth.title")}">${IC_SMOOTH}</button><button type="button" class="bt tgl" data-a="log" aria-pressed="false" aria-label="${I18N.t("ctl.log.title")}" title="${I18N.t("ctl.log.title")}">${I18N.t("ctl.log")}</button></span>` : "";
    const gx = type === "chrom" ? `<span class="tbg"><button class="bt" data-a="xic" ${p.tab === "ms2" ? "disabled " : ""}title="${p.tab === "ms2" ? I18N.t("panel.xic.off") : I18N.t("panel.xic.title")}">XIC</button></span>` : "";
    const go = `<span class="tbg"><button class="bt" data-a="tab" title="${I18N.t("tdati.btn.title")}" aria-label="${I18N.t("tdati.btn.title")}">${IC_DATA}</button><button class="bt" data-a="dl" title="${I18N.t("panel.dl.title")}" aria-label="${I18N.t("panel.dl.title")}">${IC_DL}</button></span>`;
    const more = type === "map" ? "" : `<button class="bt hmb" type="button" data-a="more" hidden aria-haspopup="true" aria-expanded="false" title="${I18N.t("panel.more.title")}" aria-label="${I18N.t("panel.more.title")}">\u22EF</button><span class="hmore" hidden role="group" aria-label="${I18N.t("panel.more.title")}"></span>`;
    return `<span class="tbs">${gv + gl + gs + gi + gt + gx + go}${more}<button class="bt fsb" data-a="max" title="${I18N.t("panel.max.title")}">${IC_FS}</button><button class="x" title="${I18N.t("panel.close")}">&times;</button></span>`;
  })()}</div><canvas></canvas>${type === "spec" ? `<button class="bt lkb" data-a="lock">${IC_UNLOCK}</button>` : ""}<div class="vl" hidden></div><div class="cl" hidden></div>${type === "spec" || type === "map" ? "" : '<button type="button" class="clh" role="slider" hidden></button>'}<div class="tip" hidden></div><div class="leg"></div>${type === "chrom" || type === "xic" || type === "mrm" ? '<div class="corrbar"></div>' : ""}${type === "spec" ? '<div class="leg2"></div>' : ""}`;
  p._ac = new AbortController();
  p.el = el; p.vl = el.querySelector(".vl"); p.cl = el.querySelector(".cl"); p.ch = el.querySelector(".clh"); p.tip = el.querySelector(".tip"); p.cv = el.querySelector("canvas"); p.rd = el.querySelector(".rd"); p.leg = el.querySelector(".leg"); p.leg2 = el.querySelector(".leg2"); p.corrbar = el.querySelector(".corrbar");
  Q("#dpanels").appendChild(el);
  STATO.aggiungi(p, "open"); apply(p); fitHost();
  // sposta (trascina l'intestazione), porta davanti, ridimensiona (maniglia in basso a destra), ingrandisci
  el.addEventListener("mousedown", () => { front(el); setActive(p); });
  el.querySelector(".hd").addEventListener("mousedown", e => {
    if (e.target.closest("input,select,button,label,option") || el.classList.contains("max")) return;
    const sx = e.clientX - p.x, sy = e.clientY - p.y;
    const grp = p.full ? followers(p).sort((a, b) => a.y - b.y) : [], off = new Map(grp.map(q => [q, q.y - p.y]));
    if (p.full) { el.classList.add("drag"); grp.forEach(q => q.el.classList.add("drag")); }
    let left = false;
    const mv = ev => {
      if (!left && p.full) { left = true; if (LAYOUT.inRiga(p, stackOrder())) LAYOUT.esci(p, E.panels, hostWidth()); }      // a panel dragged by its header leaves its row
      p.y = Math.max(0, ev.clientY - sy);
      if (p.full) { p.x = 0; grp.forEach(q => { q.y = Math.max(0, p.y + off.get(q)); q.x = 0; apply(q); }); restack(p, false, grp); } else p.x = Math.max(0, ev.clientX - sx);
      apply(p); fitHost();
    };
    const up = () => { removeEventListener("mousemove", mv); removeEventListener("mouseup", up); if (p.full) { el.classList.remove("drag"); grp.forEach(q => q.el.classList.remove("drag")); restack(p, true, grp); fitHost(); } uiSave(); };
    addEventListener("mousemove", mv); addEventListener("mouseup", up); e.preventDefault();
  });
  let skip = 2;                                                    // the first notification of each of the two watched elements is the initial size
  const onSize = () => {
    if (skip > 0) { skip--; return; }
    if (!el.classList.contains("max") && el.offsetWidth > 60) { p.w = el.offsetWidth; p.h = el.offsetHeight; const row = p.full && LAYOUT.inRiga(p, stackOrder()); if (row) LAYOUT.altezza(p, E.panels); if (p.full && !row && !dockedP(p) && !hasDocked(p) && Math.abs(p.w - Q("#dpanels").clientWidth) > 8) p.full = false; if (p.full) relayout(); fitHost(); uiSave(); }
    FRAME.cancel(p._raf); p._raf = FRAME.next(() => draw(p));
    fitHdSoon(p);
  };
  const offA = FRAME.ro(el, onSize), offB = FRAME.ro(p.cv, onSize); p._ro = () => { offA(); offB(); };   // the canvas too: it shrinks when the legend or the controls wrap to more lines
  // full screen of THIS panel (any panel, also the ones added later): the button, Esc, and the graph is redrawn at the new size on the way in and out
  const fsB = el.querySelector('[data-a="max"]');
  p._setMax = on => {
    el.classList.toggle("max", on); fsB.innerHTML = on ? IC_FSX : IC_FS;
    fsInert(on ? p : null);
    if (on) setActive(p);
    fsB.title = I18N.t(on ? "panel.max.exit" : "panel.max.title");
    if (on) front(el);
    FRAME.cancel(p._raf); p._raf = FRAME.next(() => { draw(p); setTimeout(() => draw(p), 120); });
  };
  fsB.onclick = () => p._setMax(!el.classList.contains("max"));
  hdMoreInit(p);
  AFFIANCA.grip(p);
  el.querySelector('[data-a="fit"]').onclick = () => { pushZh(p); p.zoom = null; p.zoomY = null; if (p.type === "spec") p.lock = null; draw(p); uiSave(); };
  const lkB = el.querySelector('[data-a="lock"]'); if (lkB) lkB.onclick = () => toggleLock(p);
  const rulB = el.querySelector('[data-a="rul"]'); if (rulB) rulB.onclick = () => { p.rul = !p.rul; rulB.classList.toggle("on", p.rul); if (!p.rul && p.meas && p.meas.ref != null && !p.meas.list.length) { p.meas = null; draw(p); } };
  const relB = el.querySelector('[data-a="rel"]'); if (relB) relB.onclick = () => { p.rel = !p.rel; relB.classList.toggle("on", p.rel); draw(p); uiSave(); };
  const parB = el.querySelector('[data-a="par"]'); if (parB) parB.onclick = () => specParams(p, parB);
  const smB = el.querySelector('[data-a="smooth"]'); if (smB) smB.onclick = () => { p.smooth = !p.smooth; syncTb(p); draw(p); uiSave(); };
  const lgB = el.querySelector('[data-a="log"]'); if (lgB) lgB.onclick = () => { if (lgB.disabled) return; p.log = !p.log; syncTb(p); draw(p); uiSave(); };
  const tlB = el.querySelector('[data-a="tlink"]'); if (tlB) tlB.onclick = () => toggleTl(p);
  const tabB = el.querySelector('[data-a="tab"]'); if (tabB) tabB.onclick = e => { e.stopPropagation(); if (window.TDATI) TDATI.open(p.cv); };
  const dlB = el.querySelector('[data-a="dl"]');
  if (dlB) dlB.onclick = e => {
    e.stopPropagation();                            // else the click reaches the document and closes the menu at once
    menu(e, [
      { label: I18N.t("panel.dl.png"), icon: IC_DL, fn: async () => {
        let cvs; p._exp = true; EXPORTING = true;
        try { await draw(p); cvs = whiteCanvas(p.cv, plotCaption(p)); } finally { p._exp = false; EXPORTING = false; draw(p); }
        cvs.toBlob(async b => {
          let out = b; try { out = await pngWithMeta(b, plotMeta(p)); } catch (e) { }
          saveBlob(out, plotName(p) + ".png");
        });
      }},
      ...(p.type === "map" ? [] : [{ label: I18N.t("panel.dl.excel"), icon: IC_DL, fn: () => {
        const t = plotSheets(p); if (t) dlx(plotName(p) + ".xlsx", t); else info(I18N.t("panel.dl.nodata"));
      }}])
    ]);
  };
  const bZ = el.querySelector('[data-a="izoom"]'), bA = el.querySelector('[data-a="iauto"]'), bM = el.querySelector('[data-a="iman"]'), bD = el.querySelector('[data-a="idel"]'), bT = el.querySelector('[data-a="itab"]'), bX = el.querySelector('[data-a="xic"]');
  if (bA) {                                             // tools (one at a time): zoom (drag an interval), automatic integration (click a peak), manual integration (drag an interval), delete integration (click a peak)
    const btns = { zoom: bZ, auto: bA, man: bM, idel: bD };
    const setMode = m => { p.imode = p.imode === m ? null : m; Object.entries(btns).forEach(([k, b]) => { if (b) b.classList.toggle("on", p.imode === k); }); p.cv.style.cursor = p.imode === "zoom" ? "zoom-in" : p.imode === "idel" ? "pointer" : p.imode ? "cell" : "crosshair"; refreshIntf(p); };
    if (p.imode && btns[p.imode]) { btns[p.imode].classList.add("on"); p.cv.style.cursor = p.imode === "idel" ? "pointer" : "cell"; }      // MRM panels open with the manual integration tool ready
    bZ.onclick = () => setMode("zoom"); bA.onclick = () => setMode("auto"); bM.onclick = () => setMode("man"); if (bD) bD.onclick = () => setMode("idel"); bT.onclick = showInts;
    el.querySelector('[data-a="iclr"]').onclick = () => { if (p.ints.length) delInts(p, [...p.ints]); };
    el.querySelector('[data-a="intf"]').onchange = e => { p.intf = e.target.value; };
  }
  if (bX) bX.onclick = () => openXic(null, { after: p });      // the new XIC sits right under this chromatogram
  el.querySelector(".x").onclick = () => { closePanel(p); relayout(); fitHost(); uiSave(); if (tabFiles("ms2").length) renderFileList(); };
  el.querySelector(".ttl").ondblclick = async () => { const v = await ask(I18N.t("panel.rename.prompt"), ptitle(p.title)); if (v) { p.title = v === ptitle(p.title) ? p.title : v; ctl(p); uiSave(); } };
  attach(p); ctl(p); p.ready = draw(p);
  return p;
}
const redrawAll = () => tabPanels().forEach(draw);
// many requests in the same frame (window resize, a burst of answers) become one redraw
let redrawRaf = 0;
const redrawSoon = () => { if (!redrawRaf) redrawRaf = FRAME.next(() => { redrawRaf = 0; redrawAll(); }); };
// Esc leaves the full screen of a panel (not when a dialog, a menu or a help box is open: they use Esc themselves)
document.addEventListener("keydown", e => {
  if (e.key !== "Escape" || /** @type {any} */ (e)._ctxWas || Q("dialog[open]") || Q("#helppop:not([hidden])") || (Q("#ctx") && !Q("#ctx").hidden) || Q("#uipset") || E.panels.some(q => pendKey(q))) return;     // a pending two-step action takes this Esc first (eventi.js)
  const m = E.panels.find(q => q.el && q.el.classList.contains("max")); if (m && m._setMax) m._setMax(false);
});
// full screen: ONLY that panel exists. Everything else (header, file list, tabs, toolbar, the other panels) is inert: no click, wheel or key reaches it
const fsPanel = () => E.panels.find(q => q.el && q.el.classList.contains("max"));
function fsInert(on, closing) {
  document.querySelectorAll("header,#dfiles,#tools,#dpanels>.pnl").forEach(e => {
    if (on && e !== on.el) e.setAttribute("inert", ""); else if (!on) e.removeAttribute("inert");
  });
  if (!on && !closing) document.documentElement.classList.remove("fs-on"); else if (on) document.documentElement.classList.add("fs-on");
}
// copy of a plot on a white background (for PNG files and the report: a transparent PNG looks black in some viewers)
function whiteCanvas(cv, caption = "") {
  const sc = cv.width / Math.max(cv.clientWidth, 1), top = caption ? Math.round(30 * sc) : 0;
  const c = document.createElement("canvas"); c.width = cv.width; c.height = cv.height + top;
  const g = /** @type {CanvasRenderingContext2D} */ (c.getContext("2d")); g.fillStyle = "#ffffff"; g.fillRect(0, 0, c.width, c.height); g.drawImage(cv, 0, top);
  if (caption) { g.fillStyle = "#222"; g.font = `${Math.round(12 * sc)}px system-ui`; g.textBaseline = "middle"; g.fillText(caption, Math.round(14 * sc), top / 2); }
  return c;
}
// line written on the saved image of a mass spectrum: file, MS level, retention time and number of scans
function specCaption(p) {
  const parts = [...p.leg.children, ...(p.leg2 ? p.leg2.children : [])].map(x => x.textContent.trim()).filter(Boolean);
  const fn = p._a && p._a.data ? [...new Set(p._a.data.map(x => x.f.label))].join(", ") : (E.files[p.k]?.label || "");
  return (p.all || !fn ? "" : fn + " · ") + parts.slice(0, 1).join("") + (parts.length > 1 ? " · " + parts.slice(1).join(", ") : "");
}
function plotCaption(p) {
  if (p.type === "spec") return specCaption(p);
  const a = p._a || {};
  const fs = plotFiles(p);
  const fn = fs.map(f => f.label).join(", ");
  const pol = [...new Set(fs.map(f => polLabel(f)).filter(x => x && x !== "ESI?"))].join(" ");
  const parts = [];
  if (fn) parts.push(fn + (pol ? ` (${pol})` : ""));
  if (p.type === "chrom") {
    parts.push((p.kind || "TIC").toUpperCase());
  } else if (p.type === "xic") {
    const trLab = (p.traces || []).map(t => t.label || ("m/z " + t.mz.toFixed(mzd(p)))).join(", ");
    parts.push("XIC" + (trLab ? ` · ${trLab}` : ""));
  } else if (p.type === "mrm") {
    const trs = (p.traces && p.traces.length) ? p.traces.map(t => t.label || t.tr).join(", ") : (Array.isArray(p.tr) ? p.tr.join(", ") : (p.tr || ""));
    parts.push("MRM" + (trs ? ` · ${trs}` : ""));
  } else if (p.type === "map") {
    parts.push("Mappa RT-m/z");
    if (a.y0 != null && a.y1 != null) parts.push(`m/z ${a.y0.toFixed(0)}–${a.y1.toFixed(0)}`);
  }
  if (a.x0 != null && a.x1 != null) {
    parts.push(`RT ${a.x0.toFixed(2)}–${a.x1.toFixed(2)} min`);
  }
  return parts.filter(Boolean).join(" · ");
}

// bring a panel to the front (z-index is renumbered before it can reach the menus, which sit above 10000)
function front(el) {
  el.style.zIndex = ++E.z;
  if (E.z > 5000) { E.panels.slice().sort((a, b) => (+a.el.style.zIndex || 0) - (+b.el.style.zIndex || 0)).forEach((q, i) => { q.el.style.zIndex = 10 + i; }); E.z = 10 + E.panels.length; el.style.zIndex = ++E.z; }
}
// active panel (outlined): its cursor can be moved scan by scan with the arrow keys
function setActive(p) { STATO.attiva(p, "activate"); E.panels.forEach(q => q.el.classList.toggle("act", q === p)); if (window.BANCO) BANCO.sync2(); }
// MS2 precursor chromatograms: scans are sparse (one every few cycles) and many are empty. Nearest scan WITH data, either around x (d = 0)
// or strictly beyond x in the direction d; g = half the smaller gap to its neighbours, so the spectrum window holds that scan and no other.
function ms2Near(p, x, d) {
  let best = null;
  for (const s of p._a.sr) {
    if (E.files[s.k]?.kind === "mrm") continue;
    const xs = s.x, yy = s.y, e = 1e-6;
    for (let i = 0; i < xs.length; i++) {
      if (!(yy[i] > 0) || (d > 0 && xs[i] <= x + e) || (d < 0 && xs[i] >= x - e)) continue;
      const dist = Math.abs(xs[i] - x); if (best && dist >= best.dist) { if (d > 0) break; continue; }
      best = { dist, rt: xs[i], k: s.k, i, s };
      if (d > 0) break;                                   // first scan beyond x going right
    }
  }
  if (!best) return null;
  const xs = best.s.x, i = best.i, gaps = [xs[i] - xs[i - 1], xs[i + 1] - xs[i]].filter(v => v > 0);
  return { ...best, g: (gaps.length ? Math.min(...gaps) : scanStep()) * 0.9 };
}
// One step along the scans of the active chromatogram (d = +-1, or +-5 with Maiusc). The cursor line moves by itself (no redraw of the whole chromatogram
// unless the view or the selection changes) and the linked spectra are asked to redraw once per frame. Returns false at the first/last scan.
function stepScan(p, d, o = {}) {
  const sg = Math.sign(d), n = Math.abs(d);
  let rt, i, k, lo, hi, tot;
  if (p.tab === "ms2") {                                  // jump straight to the nearest scan with data in the direction pressed
    let b = null, x = p.cur;
    for (let j = 0; j < n; j++) { const q = ms2Near(p, x, sg); if (!q) break; b = q; x = q.rt; }
    if (!b) { stepMsg(p, I18N.t(sg > 0 ? "ctl.lastScan" : "ctl.firstScan")); return false; }
    rt = b.rt; i = b.i; k = b.k; lo = rt - b.g / 2; hi = rt + b.g / 2; tot = b.s.x.length;
  } else {
    const a = p._a, ok = s => E.files[s.k]?.kind !== "mrm";
    const k0 = p._sk != null && a.sr.some(x => x.k === p._sk && ok(x)) ? p._sk : nearestFile(p, p.cur);      // the same file all along, even if another one becomes the highest
    const s = a.sr.find(x => x.k === k0 && ok(x)) || a.sr.find(ok) || a.sr[0]; if (!s || !s.x.length) return false;
    const yy = s.y || s.ys, i0 = nearIdx(s.x, p.cur); i = i0;
    for (let j = 0; j < n; j++) {
      let t = i + sg;
      while (t >= 0 && t < s.x.length && !(yy[t] > 0)) t += sg;             // empty scans (MS2 files have many) are skipped: the cursor jumps to the nearest scan with data
      if (t < 0 || t >= s.x.length) break;
      i = t;
    }
    if (i === i0) { stepMsg(p, I18N.t(sg > 0 ? "ctl.lastScan" : "ctl.firstScan")); return false; }
    rt = s.x[i]; k = s.k; tot = s.x.length;
    const dt = Math.abs(s.x[Math.min(i + 1, s.x.length - 1)] - s.x[Math.max(i - 1, 0)]) / 2 || scanStep();
    lo = rt - dt / 2; hi = rt + dt / 2;
  }
  const a = p._a; let full = false;
  p.cur = rt; if (p.sel && !o.keepSel) { p.sel = null; full = true; }
  if (p.zoom && (rt < p.zoom[0] || rt > p.zoom[1])) { const w = p.zoom[1] - p.zoom[0]; p.zoom = clampView(rt - w / 2, rt + w / 2, a.full[0], a.full[1]); full = true; }
  p._sk = k;
  if (!o.noLock) lockStart(p, k);
  if (full) draw(p); else { cursorLine(p); uiSave(); }
  pushLinked(p, lo, hi, k, true, { step: true, i, dir: sg });
  stepMsg(p, I18N.t("ms2.scanStep", { i: i + 1, tot, rt: rt.toFixed(2) }), true);
  return true;
}
function stepMsg(p, t, keep, notice) { p.rd.textContent = t; p.rd.classList.toggle("show", !!notice); if (keep) p._msg = t; }   // the line under the graph is gone: only the "nearest scan with data" notice of MS2 still shows
// the legend lives INSIDE the graph (top right) for chromatograms, XIC and MRM; for the spectrum the scan line sits on the row of the x axis title and the file legend inside the graph
function placeLeg(p) {
  if (!p.cv || p.type === "map") return;
  if (p.type === "spec") {
    const right = p.el.clientWidth - p.cv.offsetLeft - p.cv.offsetWidth + M.r + 6;
    p.leg.style.right = right + "px"; p.leg.style.top = p.cv.offsetTop + p.cv.offsetHeight - 17 + "px";
  }
  const el = p.type === "spec" ? p.leg2 : p.leg;
  if (el && LEGENDE.place(p, el)) drawSoon(p);
}
// "zoom bar" in the top margin: where the visible window sits in the whole axis, plus the visible range in words; "Vista intera" button
function afterDraw(p) {
  const rl = p.el.querySelector(".rtl");           // retention time of the spectrum, next to the title
  if (rl) { if (p.r0 == null) rl.textContent = ""; else { const m = (p.r0 + p.r1) / 2, wide = p.r1 - p.r0 > 1.6 * scanStep(); const rt = wide ? `RT ${p.r0.toFixed(2)}-${p.r1.toFixed(2)} min` : `RT ${m.toFixed(2)} min`; rl.textContent = p.src != null && !p.link && p.type === "spec" ? "fermo a " + rt : rt; } }
  if (window.BANCO) BANCO.header(p);
  const btn = p.el.querySelector('[data-a="fit"]'), z = !!(p._a && (p.zoom || p.zoomY));
  if (p.type !== "spec" && p.type !== "map" && p._rngOn != null && p._rngOn !== rngIsOn(p)) ctl(p);      // the «Range» button follows zooming too
  if (btn) btn.disabled = !z;                      // always there (the layout does not jump), active only when zoomed
  lockButton(p); cursorLine(p);
  placeLeg(p);
  { const hd = p.el.querySelector(".hd"); if (hd && hd.scrollWidth > hd.clientWidth + 1) fitHdSoon(p); }       // the title or the reading changed the width of the header: fit it again
  if (window.TDATI_P) TDATI_P.update(p);                      // table of the data and description for screen readers (tdati.js)
  const tb = p.el.querySelector('[data-a="itab"]'); if (tb) tb.hidden = !p.ints.length;
  const cb = p.el.querySelector('[data-a="iclr"]'); if (cb) cb.hidden = !p.ints.length;
  refreshIntf(p);
  if (!(p._a && (p.zoom || p.zoomY)) || p._exp) return;
  const a = p._a, g = p.cv.getContext("2d"), bw = 90, x = a.W - M.r - bw, y = 3, f0 = a.full[0], f1 = a.full[1], u = p.type === "spec" ? "m/z" : "RT";
  const txt = `${u} ${a.x0.toFixed(p.type === "spec" ? 1 : 2)}-${a.x1.toFixed(p.type === "spec" ? 1 : 2)}` + (p.type === "spec" ? "" : " min") + (a.map && p.zoomY ? ` · m/z ${a.y0.toFixed(0)}-${a.y1.toFixed(0)}` : "");
  g.save(); g.font = fpx(12); g.textAlign = "right"; g.fillStyle = css("--muted"); g.fillText(I18N.t("ctl.zoomed", { range: txt }), x - 6, y + 7);
  g.fillStyle = "rgba(120,120,120,.25)"; g.fillRect(x, y, bw, 6);
  g.fillStyle = css("--accent"); const l = x + (a.x0 - f0) / (f1 - f0) * bw, r = x + (a.x1 - f0) / (f1 - f0) * bw; g.fillRect(l, y, Math.max(2, r - l), 6);
  g.restore();
}
// ---- names and metadata of the saved images / Excel files
const fname = t => String(t).replace(/[^\w.+\-]+/g, "_").replace(/^_+|_+$/g, "");
function plotFiles(p) {
  const a = p._a, seen = new Map();
  (a && a.sr ? a.sr.map(s => E.files[s.k]) : a && a.data ? a.data.map(x => x.f) : a && a.f ? [a.f] : []).forEach(f => { if (f) seen.set(f.k, f); });
  return [...seen.values()];
}
function plotName(p) {
  const fs = plotFiles(p), a = p._a;
  const ft = fs.length === 1 ? (fs[0].type === "sample" && fs[0].time != null ? "t" + fs[0].time : fname(fs[0].label).slice(0, 18)) : fs.length ? I18N.t("export.file.files", { n: fs.length }) : "";
  const mzs = a && a.sr ? [...new Set(a.sr.map(s => s.mz).filter(v => v != null))] : [];
  const trs = a && a.sr && p.type === "mrm" ? [...new Set(a.sr.map(s => s.ion))] : [];
  const lst = (arr, f) => arr.length <= 3 ? arr.map(f).join("+") : I18N.t("export.file.ions", { n: arr.length });
  const rtm = p.r0 != null ? "RT" + ((p.r0 + p.r1) / 2).toFixed(1) : "";
  let parts;
  if (p.type === "chrom") parts = [p.kind.toUpperCase(), ft];
  else if (p.type === "xic") parts = ["XIC", mzs.length ? "m-z" + lst(mzs, v => +v.toFixed(mzd(p))) : "", ft];
  else if (p.type === "mrm") parts = ["MRM", trs.length ? lst(trs, v => String(v).replace(">", "-")) : "", ft];
  else if (p.type === "spec") parts = [I18N.t(p.level === 2 ? "export.file.spectrumMs2" : "export.file.spectrum"), fs.length > 1 ? I18N.t("export.file.overlaid") + "_" + ft : ft, rtm];
  else parts = [I18N.t("export.file.map"), ft];
  if (p.title && p.title[0] !== "@") parts = [p.title, ft];            // a title typed by the student names the file; the default titles are codes
  return fname(parts.filter(Boolean).join("_")).slice(0, 60) || I18N.t("export.file.chart");
}
function plotMeta(p) {
  const a = p._a || {}, fs = plotFiles(p), d = [];
  d.push({ chrom: I18N.t("export.desc.chrom", { kind: (p.kind || "").toUpperCase() }), xic: "XIC (" + p.traces.map(t => t.w == null ? t.mz + " +-" + p.tol : (t.mz - t.w).toFixed(mzd(p)) + "-" + (t.mz + t.w).toFixed(mzd(p))).join("; ") + ")", mrm: I18N.t("export.desc.mrm"), spec: I18N.t("export.desc.spec", { ms: p.level === 2 ? "MS2" : "MS1" }), map: I18N.t("export.desc.map") }[p.type] || p.type);
  const mzs = a.sr ? [...new Set(a.sr.map(s => s.mz).filter(v => v != null))] : [];
  if (mzs.length) d.push("m/z " + mzs.map(v => v.toFixed(mzd(p))).join(", "));
  if (p.type === "spec" && p.r0 != null) d.push(`RT ${p.r0.toFixed(2)}-${p.r1.toFixed(2)} min`);
  if (p.ibk) d.push(I18N.t("export.desc.ibk", { text: ibkText(p.ibk) }));
  if (window.SUBREF && SUBREF.text(p)) d.push(SUBREF.text(p));
  if (a.x0 != null) d.push(I18N.t("export.desc.axis", { a: a.x0.toFixed(p.type === "spec" ? 1 : 2), b: a.x1.toFixed(p.type === "spec" ? 1 : 2) }));
  return [["Title", ptitle(p.title) || I18N.t("export.meta.chart")], ["Description", d.join("; ")], ["Source", fs.map(f => f.label).join(", ")], ["Software", APP_NAME], ["Creation Time", new Date().toISOString()]];
}
// writes tEXt chunks (Latin-1) right after the IHDR of a PNG
const CRCT = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
const crc32 = u => { let c = 0xffffffff; for (const b of u) c = CRCT[(c ^ b) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
async function pngWithMeta(blob, fields) {
  const src = new Uint8Array(await blob.arrayBuffer()), l1 = t => Uint8Array.from(String(t).replace(/[\u2013\u2014]/g, "-").replace(/[^\x20-\x7e\xa0-\xff]/g, "?"), c => c.charCodeAt(0));
  const parts = [src.slice(0, 33)];
  fields.forEach(([k, v]) => {
    const kb = l1(k), vb = l1(v), body = new Uint8Array(4 + kb.length + 1 + vb.length);
    body.set([116, 69, 88, 116], 0); body.set(kb, 4); body.set(vb, 5 + kb.length);       // "tEXt" + keyword + 0 + text
    const ch = new Uint8Array(12 + body.length - 4), dv = new DataView(ch.buffer);
    dv.setUint32(0, body.length - 4); ch.set(body, 4); dv.setUint32(ch.length - 4, crc32(body)); parts.push(ch);
  });
  parts.push(src.slice(33)); return new Blob(parts, { type: "image/png" });
}

// ---- «Correzione»: ONE menu for chromatogram and spectrum (None · Subtract file with its chooser · Background of a stretch · Automatic baseline). Icon + one-line hints;
// o = { files: files that can be subtracted, cur: "" | "f<k>" | "tract" | "snip", snip: offer the automatic baseline, dis: reason why it is off ("" = on) }
const ICN = d => `<svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${d}</svg>`;
const CIC = { none: ICN('<circle cx="9" cy="9" r="6"/><path d="M4.8 13.2L13.2 4.8"/>'), blank: ICN('<path d="M5 2.5h5.5L13.5 5.5v10h-8.5z"/><path d="M10.5 2.5v3h3"/><path d="M7 11.5h4"/>'),
  tract: ICN('<path d="M2 13c3 0 4-8 7-8s4 8 7 8"/><path d="M2 15.5h14" stroke-width="2.2"/><path d="M5 14v3M13 14v3"/>'), snip: ICN('<path d="M2 14c3 0 4-9 7-9s4 9 7 9"/><path d="M2 13c3-2 4-.5 7-.5s4-1.5 7 .5" stroke-dasharray="2 2"/>') };
  // one «Subtract file» entry with a file chooser (default: the session reference, else the t0); tract and SNIP keep their one-line hint
function corrWidget(p, o) {
  const T0 = p.tab || E.tab, fileOpts = o.files.length > (o.min ?? 1) ? o.files : [], cfile = o.cur[0] === "f";
  const refK = S.ref.k !== "auto" && fileOpts.some(x => x.k === S.ref.k) ? S.ref.k : (SUBREF.autoFile(T0) || {}).k;      // default file: the reference of the session
  const subK = cfile ? +o.cur.slice(1) : fileOpts.some(x => x.k === refK) ? refK : fileOpts.length ? fileOpts[0].k : null, subF = subK != null ? E.files[subK] : null;
  const items = [["", "none", I18N.t("corr.none"), I18N.t("corr.none.desc")],
    [subF ? "f" + subF.k : "", "blank", subF ? I18N.t("corr.file", { label: subF.label }) : I18N.t("corr.fileNone"), subF ? o.fileDesc || I18N.t("corr.file.desc") : I18N.t("corr.fileNone.desc"), subF ? I18N.t("corr.file.short", { label: subF.label }) : ""],
    ["tract", "tract", I18N.t("corr.tract"), o.tractDesc || I18N.t("corr.tract.desc")],
    ...(o.snip ? [["snip", "snip", I18N.t("corr.snip"), o.snipDesc || I18N.t("corr.snip.desc")]] : [])];
  const now = items.find(i => i[1] === "blank" ? cfile : i[0] === o.cur && i[1] !== "none") || items[0];
  const tip = I18N.t("corr.button.title") + (now === items[0] ? "" : ": " + (now[4] ?? now[2]));
  const sel = fileOpts.length > 1 ? `<select class="corrsel" data-o="corrf" aria-label="${EH(I18N.t("corr.file.pick"))}" title="${EH(I18N.t("corr.file.pick"))}">${fileOpts.map(x => `<option value="${x.k}"${x.k === subK ? " selected" : ""}>${EH(x.label)}</option>`).join("")}</select>` : "";
  return `<span class="corrw"><button type="button" data-o="corr" class="corrb${now === items[0] ? "" : " on"}" aria-haspopup="true" aria-expanded="false"${o.dis ? ` disabled title="${o.dis}"` : ` title="${EH(tip)}"`} aria-label="${EH(tip)}">${CIC[now[1]]}<span aria-hidden="true">&#9662;</span></button>
    <span class="corrm" role="group" aria-label="${EH(I18N.t("corr.button.title"))}" hidden>${items.map(i => `<button type="button" class="corri${i === now ? " on" : ""}" aria-pressed="${i === now}" data-c="${EH(i[0])}"${i[0] === "" && i[1] === "blank" ? ` disabled title="${EH(I18N.t("corr.fileNone.desc"))}"` : ""}>${CIC[i[1]]}<span><b>${EH(i[2])}</b><span class="sm muted">${EH(i[3])}</span></span></button>${i[1] === "blank" ? sel : ""}`).join("")}</span></span>`;
}
function corrWire(x, pick) {          // pick("" | "f<k>" | "tract" | "snip")
  const m = x.parentNode.querySelector(".corrm"), items = () => [...m.querySelectorAll("button:not(:disabled),select")];
  const shut = back => { m.hidden = true; x.setAttribute("aria-expanded", "false"); if (back) x.focus(); };
  x.onclick = () => { m.hidden = !m.hidden; x.setAttribute("aria-expanded", String(!m.hidden)); if (!m.hidden) (m.querySelector(".corri.on:not(:disabled)") || items()[0] || m).focus(); };
  x.onkeydown = e => { if ((e.key === "ArrowDown" || e.key === "ArrowUp") && m.hidden && !x.disabled) { e.preventDefault(); x.click(); } };
  m.querySelectorAll("[data-c]").forEach(b => b.onclick = () => { shut(false); pick(b.dataset.c); });
  const cf = m.querySelector('[data-o="corrf"]'); if (cf) cf.onchange = () => { shut(false); pick("f" + cf.value); };
  m.onkeydown = e => {                           // arrows move between the entries, Enter activates (native), Esc closes and gives the focus back
    if (e.key === "Escape") { e.stopPropagation(); shut(true); return; }
    if ((e.key !== "ArrowDown" && e.key !== "ArrowUp") || e.target.tagName === "SELECT") return;
    e.preventDefault(); const l = items(), i = l.indexOf(document.activeElement); if (l.length) l[(i + (e.key === "ArrowDown" ? 1 : l.length - 1)) % l.length].focus();
  };
}
function ctl(p) {
  hdRestore(p);
  const c = p.el.querySelector(".ctl"), t = p.el.querySelector(".ttl");
  t.textContent = p.type === "xic" && p.traces.length === 1 && p.title === PT.xic ? I18N.t("panel.title.xicOne", { label: p.traces[0].label }) : ptitle(p.title);      // a single trace is named in the title
  t.style.display = p.type === "chrom" && p.title === PT.chrom ? "none" : "";          // the menu of the kind (TIC / BPC / PDA) is the title
  if (p.type === "spec" && p.link && (!E.panels.some(q => q.id === p.link) || E.panels.some(q => q !== p && q.type === "spec" && q.link === p.link && q.id > p.id))) { p.src = p.src || p.link; p.link = null; }   // only the newest spectrum of a chromatogram is live; a link to a closed chromatogram is dropped
  const T0 = p.tab || E.tab, ms2c = p.type === "chrom" && T0 === "ms2", mrmP = p.type === "mrm";
  if (ms2c) { p.kind = "tic"; p.mz0 = p.mz1 = null; p.bk = ""; p.snip = false; p.log = false; }      // precursor chromatogram: only the MS2 scans of the precursor
  if (mrmP) { p.bk = ""; p.snip = false; }
  if (p.type === "spec" && T0 === "ms2" && !p.filt) p.level = 2;
  const NOMS2 = I18N.t("ctl.noMs2"), NOMRM = I18N.t("ctl.noMrm");
  const off = (on, why) => on ? ` disabled title="${why}"` : "";
  const chk = (k, lab, dis, why, tip) => `<label class="muted${dis ? " dis" : ""}"${dis ? ` title="${why}"` : tip ? ` title="${tip}"` : ""}><input type="checkbox" data-o="${k}" ${p[k] ? "checked" : ""}${dis ? " disabled" : ""}> ${lab}</label>`;
  const ONEF = tabFiles(p.tab || E.tab).length <= 1, ONEWHY = I18N.t("ctl.oneFile");
  const view = `<select data-o="mode" aria-label="${I18N.t("ctl.mode.aria")}"${p.type === "chrom" || (p.type === "xic" && p.traces.length <= 1) ? off(ONEF, ONEWHY) : ""} title="${I18N.t("ctl.mode.title")}"><option value="ovl" ${p.mode !== "stk" && p.mode !== "cas" ? "selected" : ""}>${I18N.t("ctl.mode.ovl")}</option><option value="stk" ${p.mode === "stk" ? "selected" : ""}>${I18N.t("ctl.mode.stk")}</option><option value="cas" ${p.mode === "cas" ? "selected" : ""}>${I18N.t("ctl.mode.cas")}</option></select>`;
  // low resolution: the gradient delay and the PDA alignment are in this popover (in high resolution they are in «Opzioni di visualizzazione»)
  const num = (k, v, lo, hi, st) => `<input data-o="${k}" type="number" class="mzf" min="${lo}" max="${hi}" step="${st}" value="${v}" style="width:5em">`;
  const lrExtra = window.BANCO && BANCO.on() ? "" :
    (p.overlay === "grad" ? `<label class="muted" title="${I18N.t("disp.gdelay.title")}">${I18N.t("disp.gdelay", { input: num("gdelay", p.gdelay || 0, 0, 10, 0.1) })}</label>` : "") +
    (p.kind === "pda" && window.DISP && DISP.pdaHere(p) ? `<label class="muted" title="${I18N.t("disp.pdaDelay.title")}">${I18N.t("disp.pdaDelay", { input: num("pdadelay", DISP.pdaDelayValue(p), -2, 2, 0.01) })}</label>${chk("pdaAlign", I18N.t("disp.pdaAlign"), false, "", I18N.t("disp.pdaAlign.title"))}` : "");
  const T = p.tab || E.tab, TF = tabFiles(T), sfl = scanFiles(TF), mlo = Math.min(...sfl.map(x => x.mz_min ?? Infinity)), mhi = Math.max(...sfl.map(x => x.mz_max ?? -Infinity));
  const mzbar = `<label class="muted" title="${I18N.t("ctl.mz.title")}"><i>m/z</i> ${I18N.t("ctl.range.from")} <input data-o="mz0" class="mzf" inputmode="decimal" autocomplete="off"${ms2c ? " disabled" : ""} value="${p.mz0 != null ? fmz(p.mz0) : ""}" placeholder="${isFinite(mlo) ? mlo.toFixed(mzd(p)) : ""}"> ${I18N.t("ctl.range.to")} <input data-o="mz1" class="mzf" inputmode="decimal" autocomplete="off"${ms2c ? " disabled" : ""} value="${p.mz1 != null ? fmz(p.mz1) : ""}" placeholder="${isFinite(mhi) ? mhi.toFixed(mzd(p)) : ""}"></label>`;
  const rtbar = p.type === "map" ? "" : `<label class="muted" title="${I18N.t("ctl.rt.title")}"><i>RT</i> ${I18N.t("ctl.range.from")} <input data-o="rt0" class="mzf" inputmode="decimal" autocomplete="off" value="${p.zoom ? fmt2(p.zoom[0]) : ""}" placeholder="${I18N.t("ctl.rt.min")}"> ${I18N.t("ctl.range.to")} <input data-o="rt1" class="mzf" inputmode="decimal" autocomplete="off" value="${p.zoom ? fmt2(p.zoom[1]) : ""}" placeholder="${I18N.t("ctl.rt.max")}"> min</label>`;
  const hasScan = sfl.length > 0, ms2s = TF.filter(x => x.kind === "ms2"), hasPda = TF.some(x => x.pda);
  const anKinds = [...new Map(TF.flatMap(x => x.analog || []).map(a => [a.kind === "pressure" ? "press" : "an:" + a.id, a])).entries()];       // [value, trace] of the analog traces of the files of the tab
  if (p.type === "chrom" && ((!hasScan && p.kind === "bpc") || (!hasPda && p.kind === "pda") || (isAnalog(p.kind) && !anKinds.some(([v]) => v === p.kind)))) p.kind = "tic";      // options that make no sense for these files are switched off
  const precs = [...new Set(ms2s.flatMap(x => x.precursors))].sort((a, b) => a - b);
  const precSel = precs.length && p.kind !== "pda" ? `<select data-o="prec" style="flex:none;min-width:11em" title="${I18N.t("ctl.prec.title")}"><option value="">${I18N.t("ctl.prec.all")}</option>${precs.map(v => `<option value="${v}" ${String(p.prec) === String(v) ? "selected" : ""}>${I18N.t("ctl.prec.one", { v })}</option>`).join("")}</select>` : "";
    const blanks = TF.filter(x => x.kind !== "mrm" || p.type === "mrm"), bkOn = p.bk !== "" && p.bk != null && E.files[+p.bk];
  const bkList = blanks.concat(bkOn && !blanks.includes(E.files[+p.bk]) ? [E.files[+p.bk]] : []);          // an old notebook may point to a file that is not marked as blank: keep it
  const cval = bkOn ? "f" + p.bk : p.ibk || p._pickIbk ? "tract" : p.snip ? "snip" : "";
  const corr = corrWidget(p, { files: bkList, cur: cval, snip: true, dis: ms2c ? NOMS2 : mrmP ? NOMRM : "" })
    + `${p.snip && !bkOn ? `<label class="muted" title="${I18N.t("corr.snipw.title")}">&le;<input data-o="snipw" type="number" step="0.5" min="0.2" value="${p.snipw}" style="width:50px"> min</label>` : ""}`;
  const fsel = `<button data-o="fpop" class="fcount"${off(ONEF, I18N.t("ctl.files.one"))} title="${I18N.t("ctl.files.title")}" aria-label="${I18N.t("ctl.files.aria", { n: TF.filter(f => f.vis).length, tot: TF.length })}">${I18N.t("ctl.files", { n: TF.filter(f => f.vis).length, tot: TF.length })}</button>`;
  // «Range»: m/z, RT (and the precursor of an MS2 file) in a small popover; the button is accented when one of them is set, and a «×» clears them
  const rngOn = p._rngOn = rngIsOn(p);
  const parBox = inner => `<span class="cpar"><button class="bt rngb${rngOn ? " on" : ""}" type="button" data-a="cpar" aria-expanded="${!!p._parOpen}" title="${I18N.t("ctl.range.title")}" aria-label="${I18N.t("ctl.range")}${rngOn ? " (" + I18N.t("ctl.range.set") + ")" : ""}">${IC_RANGE}<span>${I18N.t("ctl.range")}</span></button>${rngOn ? `<button class="bt rngx" type="button" data-a="rngx" title="${I18N.t("ctl.range.clear")}" aria-label="${I18N.t("ctl.range.clear")}">&times;</button>` : ""}<span class="sp-pop cpop" ${p._parOpen ? "" : "hidden"}><span class="muted sm">${I18N.t("ctl.range.note")}</span>${inner}</span></span>`;
  const kindSel = ms2c ? `<select disabled title="${NOMS2}"><option>${I18N.t("ctl.ms2scans")}</option></select>` : null;
  const trSel = ms2c && (p._trs || []).length ? `<select data-o="tr" title="${I18N.t("ctl.ms2tr.title")}"><option value="" ${!p.tr ? "selected" : ""}>${I18N.t("ctl.ms2tr.none")}</option><option value="__all" ${p.tr === "__all" ? "selected" : ""}>${I18N.t("ctl.ms2tr.all")}</option>${p._trs.map(t => `<option value="${EH(t.key)}" ${p.tr === t.key ? "selected" : ""}>${EH(I18N.t("ctl.ms2tr.opt", { q1: t.q1, q3: t.q3 }))}</option>`).join("")}</select>` : "";
  if (p.type === "chrom") c.innerHTML = `${kindSel || `<select data-o="kind" class="ttlsel" title="${I18N.t("ctl.kind.title")}"><option value="tic" ${p.kind === "tic" ? "selected" : ""}>${I18N.t("ctl.kind.tic")}</option><option value="bpc" ${p.kind === "bpc" ? "selected" : ""} ${hasScan ? "" : "disabled"} title="${I18N.t("ctl.kind.bpc.title")}">${I18N.t("ctl.kind.bpc")}</option><option value="pda" ${p.kind === "pda" ? "selected" : ""} ${hasPda ? "" : "disabled"} title="${I18N.t("ctl.kind.pda.title")}">${I18N.t("ctl.kind.pda")}</option>${anKinds.map(([v, a]) => `<option value="${EH(v)}" ${p.kind === v ? "selected" : ""}>${EH(v === "press" ? I18N.t("ctl.kind.press") : a.id)}</option>`).join("")}</select>`}${trSel}${view}${corr}${parBox(`${p.kind === "pda" || isAnalog(p.kind) ? "" : precSel + (hasScan ? mzbar : "")}${rtbar}${p.type === "chrom" && p.tab === "full" && E.files.some(x => x.ms2_events) ? chk("ms2tri", I18N.t("ctl.ms2tri"), false, "", I18N.t("ctl.ms2tri.title")) : ""}`)}`;
  if (p.type === "xic") {
    p._xt = Math.min(p._xt || 0, Math.max(0, p.traces.length - 1));
    const xt = p.traces[p._xt], xw1 = xt ? (xt.w ?? p.tol) : 0, xed = xt && window.HR ? HR.xicEdges(xt, p) : null;      // xed: lower / upper m/z of an ion read in high resolution (ppm window)
    const xsel = p.traces.length > 1 ? `<select data-o="xt" title="${I18N.t("ctl.xt.title")}">${p.traces.map((t, i) => `<option value="${i}" ${i === p._xt ? "selected" : ""}>XIC ${i + 1}</option>`).join("")}</select>` : "";
    const hrX = !!(window.HR && HR.xicFiles(p.tab).some(x => HR.isHr(x, x.lv)));      // high resolution: only the label, m/z and interval are in «Display options» of this chromatogram
    const xr = xt && hrX ? `${xsel}<b class="xlab" title="${EH(I18N.t("ctl.xlab.title"))}">XIC \u2022 m/z ${(+xt.mz).toFixed(HR.dec(HR.xicFiles(p.tab), 1))}</b>` : xt ? `${xsel}<label class="muted" title="${I18N.t("ctl.xr.title")}"><i>m/z</i> ${I18N.t("ctl.range.from")} <input data-o="xlo" class="mzf" inputmode="decimal" autocomplete="off" value="${xed ? xed[0] : fmz(xt.mz - xw1)}"> ${I18N.t("ctl.range.to")} <input data-o="xhi" class="mzf" inputmode="decimal" autocomplete="off" value="${xed ? xed[1] : fmz(xt.mz + xw1)}"></label>` : "";
    c.innerHTML = `${xr}${fsel}${view}${corr}${parBox(rtbar + lrExtra)}${p.traces.length > 1 ? '<button data-o="split" title="${I18N.t("ctl.split.title")}">${I18N.t("ctl.split")}</button>' : ""}`;
  }
  if (p.type === "mrm") {
    const isMulti = Array.isArray(p.tr) && p.tr.length > 1;
    const qqOpt = isMulti ? `<option value="__quant_qual" selected>${I18N.t("ctl.mrm.quantQual")}</option>` : "";
    c.innerHTML = `<select data-o="tr" title="${I18N.t("ctl.mrm.tr.title")}"><option value="" ${(!p.tr && !isMulti) ? "selected" : ""}>${I18N.t("ctl.mrm.all")}</option>${qqOpt}${p._trs.map(t => `<option value="${EH(t.key)}" ${(!isMulti && p.tr === t.key) ? "selected" : ""}>${EH(t.key)} ${EH(t.name)}</option>`).join("")}</select>${fsel}${view}${corr}${parBox(rtbar + lrExtra)}`;
  }
  if (p.type === "map") {
    const sf = TF, opt = (v, cur, lab) => `<option value="${v}" ${String(cur) === String(v) ? "selected" : ""}>${EH(lab)}</option>`;
    const G = (lab, inner) => `<span class="mzg" role="group" aria-label="${lab}">${inner}</span>`, fol = follow(p.tab || E.tab), lvf = (E.files[fol ? E.cur : p.k] || {}).lv || 1;
    const cur = E.files[fol ? E.cur : p.k] || {}, mc = MAPPA.corrState(p), others = sf.filter(x => x.k !== cur.k), isF = mc.now[0] === "f";      // a file minus itself is not offered
    const mCorr = corrWidget(p, { files: others, min: 0, cur: mc.now === "auto" ? "snip" : mc.now, snip: true, dis: "", tractDesc: I18N.t("corr.tract.mapdesc"), snipDesc: I18N.t("corr.auto.mapdesc"), fileDesc: I18N.t("corr.file.mapdesc") });
    const mChips = [];                                                   // next to «Options»: the correction and what is not at its default value
    if (isF) mChips.push(I18N.t("corr.file.short", { label: (E.files[+mc.now.slice(1)] || {}).label || "" })); else if (mc.now === "tract") mChips.push(I18N.t("corr.tract")); else if (mc.now === "auto") mChips.push(I18N.t("corr.snip"));
    mChips.push(...MAPPA.optChips(p));
    const mPar = inner => `<span class="cpar"><button class="bt cparl" type="button" data-a="cpar" aria-expanded="${!!p._parOpen}" title="${I18N.t("ctl.cpar.titleMap")}" aria-label="${I18N.t("ctl.cpar.map")}">${I18N.t("ctl.cpar.map.lab")} &#9662;</button>${mChips.map(t => `<button class="chip" type="button" data-a="cparchip" title="${I18N.t("ctl.cpar.titleMap")}">${EH(t)}</button>`).join("")}<span class="sp-pop cpop" ${p._parOpen ? "" : "hidden"}>${inner}</span></span>`;
    c.innerHTML = G(I18N.t("ctl.map.grp.file"), `<select data-o="k" aria-label="${I18N.t("ctl.map.file")}" title="${fol ? I18N.t("ctl.map.follow") : I18N.t("ctl.map.file")}">${sf.map(x => opt(x.k, fol ? E.cur : p.k, x.label)).join("")}</select><span class="muted mzlv" title="${I18N.t("ctl.map.level.title")}">MS<sup>${lvf}</sup></span>`) +
      mCorr +
      mPar(`<label class="muted" title="${isF ? "" : I18N.t("ctl.map.norm.need") + " "}${I18N.t("ctl.map.norm.title")}">${I18N.t("ctl.map.norm.lab")} <select data-o="norm"${isF ? "" : " disabled"}>${opt("abs", p.norm, I18N.t("ctl.map.norm.abs"))}${opt("max", p.norm, I18N.t("ctl.map.norm.max"))}${opt("tic", p.norm, I18N.t("ctl.map.norm.tic"))}</select></label><span data-mzslot="diff"></span>` +
        `<details class="mzcol"${p._colOpen ? " open" : ""}><summary>${I18N.t("ctl.map.colors")}</summary><label class="muted" title="${I18N.t("ctl.map.scale.title")}">${I18N.t("ctl.map.scale.lab")} <select data-o="scale">${opt("lin", p.scale, I18N.t("ctl.map.scale.lin"))}${opt("sqrt", p.scale, I18N.t("ctl.map.scale.sqrt"))}${opt("log", p.scale, I18N.t("ctl.map.scale.log"))}${opt("gamma", p.scale, I18N.t("ctl.map.scale.gamma"))}</select></label>` +
        `${p.scale === "gamma" ? `<label class="muted">${I18N.t("ctl.map.gamma")} <input data-o="gamma" type="number" class="mzf" min="0.1" max="3" step="0.05" value="${p.gamma || 0.6}" style="width:5em"></label>` : ""}<span data-mzslot="col"></span></details>`);
  }
  if (p.type === "spec") {
    const f = TF, hasMs2 = f.some(x => x.ms2), sref0 = window.SUBREF ? SUBREF.eff(p) : null;      // the «fondo» menu starts from the reference of the session
    const fol = follow(T0), FW = I18N.t("ctl.spec.follow");
    c.innerHTML = `<select data-o="k" aria-label="${I18N.t("ctl.map.file")}"${off(fol, FW)}>${f.map(x => `<option value="${x.k}" ${x.k === (fol ? E.cur : p.k) ? "selected" : ""}>${EH(x.label)}</option>`).join("")}</select>${chk("all", I18N.t("ctl.spec.all"), fol || ONEF, fol ? FW : I18N.t("ctl.files.one"))}` +
            (hasMs2 ? `<select data-o="level"${off(T0 === "ms2", I18N.t("ctl.spec.levelMs2"))}><option value="1" ${p.level === 1 ? "selected" : ""}>MS1</option><option value="2" ${p.level === 2 ? "selected" : ""}>MS2</option></select>` : "") +
      corrWidget(p, { files: f.filter(x => x.kind !== "mrm"), cur: p.bg === "w" ? "tract" : p.bg === "x" ? "" : p.bg !== "" && p.bg != null ? "f" + p.bg : sref0 ? "f" + sref0.k : "", snip: false, dis: "", tractDesc: I18N.t("corr.tract.spec") }) +
      (p.bg === "w" ? `<label class="muted">${I18N.t("ctl.range.from")} <input data-o="bw0" type="number" step="0.1" value="${p.bw0 ?? ""}" style="width:56px"> ${I18N.t("ctl.range.to")} <input data-o="bw1" type="number" step="0.1" value="${p.bw1 ?? ""}" style="width:56px"> min</label>` : "") +
      (p.level === 2 ? `<select data-o="prec" style="min-width:11em"><option value="">${I18N.t("ctl.prec.all")}</option>${[...new Set(f.flatMap(x => x.precursors))].sort((a, b) => a - b).map(v => `<option value="${v}" ${String(p.prec) === String(v) ? "selected" : ""}>${I18N.t("ctl.prec.one", { v })}</option>`).join("")}</select>` : "");
  }
  if (p.type === "spec" && c.firstElementChild) {          // «Follows: panel N» (live) or «Fixed»: the menu chooses which chromatogram drives the spectrum
    const srcP = p.link ? E.panels.find(q => q.id === p.link) : null, n = srcP && srcP.num ? srcP.num : "?";
    c.firstElementChild.insertAdjacentHTML("afterend", `<button type="button" class="lk fol${p.link ? " on" : ""}" aria-haspopup="menu" aria-label="${EH(p.link ? I18N.t("spec.follow.aria", { n }) : I18N.t("spec.follow.none.aria"))}" title="${EH(I18N.t(p.link ? "spec.follow.title" : "spec.follow.none.title"))}">${p.link ? I18N.t("spec.follow", { n }) : I18N.t("spec.follow.none")} &#9662;</button>`);
    const fb = c.querySelector(".fol");
    if (!p.link) fb.classList.remove("lk");
    fb.onclick = e => {
      e.stopPropagation();
      const cands = E.panels.filter(q => ["chrom", "xic", "mrm"].includes(q.type) && (q.tab || E.tab) === (p.tab || E.tab) && q.el);
      menu(e, [...cands.map(q => ({ label: (p.link === q.id ? "\u2713 " : "") + I18N.t("spec.follow.item", { n: q.num || "?", title: ptitle(q.title) }), fn: () => liveSpec(p, q) })),
        ...(cands.length ? ["-"] : []), { label: (p.link ? "" : "\u2713 ") + I18N.t("spec.follow.menu.none"), fn: () => { freezeSpec(p); uiSave(); } }]);
    };
  }
  if (window.BANCO) BANCO.decorate(p, c);
  if (window.MAPPA && p.type === "map") MAPPA.decorate(p, c);
  const cpB = c.querySelector('[data-a="cpar"]'), cpP = c.querySelector(".cpop");
  const parPlace = () => { if (!cpB || !cpP || cpP.hidden) return; const op = cpP.offsetParent || p.el, pr = op.getBoundingClientRect(), br = cpB.getBoundingClientRect(); cpP.style.left = Math.max(4, Math.min(br.left - pr.left, op.clientWidth - cpP.offsetWidth - 6)) + "px"; cpP.style.top = br.bottom - pr.top + 4 + "px"; };
  const parToggle = on => { p._parOpen = on; if (cpB) cpB.setAttribute("aria-expanded", String(!!on)); if (cpP) { cpP.hidden = !on; if (on) { const q = cpP.querySelectorAll('[data-o="rt0"],[data-o="rt1"]'); if (q.length === 2) { q[0].value = p.zoom ? fmt2(p.zoom[0]) : ""; q[1].value = p.zoom ? fmt2(p.zoom[1]) : ""; } } parPlace(); } };
  if (cpB) { cpB.onclick = () => parToggle(!p._parOpen); parPlace();
    const rx = c.querySelector('[data-a="rngx"]'); if (rx) rx.onclick = () => { p.mz0 = p.mz1 = null; if (p.type === "chrom" && !ms2c) p.prec = null; pushZh(p); p.zoom = null; p.zoomY = null; p._parOpen = false; ctl(p); draw(p); uiSave(); };
    if (!p._parAway) { p._parAway = e => { if (p._parOpen && p.el.isConnected && !e.target.closest(".cpop") && !e.target.closest('[data-a="cpar"]')) { p._parOpen = false; const q = p.el.querySelector(".cpop"); if (q) q.hidden = true; } }; document.addEventListener("mousedown", p._parAway, { capture: true, signal: p._ac.signal }); } }
  c.querySelectorAll("[data-o]").forEach(x => {
    const k = x.dataset.o;
    if (k === "fpop") x.onclick = e => { e.stopPropagation(); fileMenu(x); };
    else if (k === "xt") x.onchange = () => { p._xt = +x.value; ctl(p); };
    else if (k === "gdelay") x.onchange = () => { p.gdelay = Math.max(0, Math.min(10, Math.round((numIt(x.value) || 0) * 10) / 10)); draw(p); uiSave(); };
    else if (k === "pdadelay") x.onchange = () => { const n = numIt(x.value); p.pdaDelay = isFinite(n) ? Math.max(-2, Math.min(2, n)) : null; draw(p); uiSave(); };
    else if (k === "pdaAlign") x.onchange = () => { p.pdaAlign = x.checked; draw(p); uiSave(); };
    else if (k === "xlo" || k === "xhi") x.onchange = () => {
      const t = p.traces[p._xt], hrE = t && t.ion && window.HR && HR.xicEdges(t, p), nm = v => hrE ? (isFinite(parseFloat(v)) && parseFloat(v) > 0 ? rh(numIt(v), 5) : null) : numMz1(v);
      const lo = nm(c.querySelector('[data-o="xlo"]').value), hi = nm(c.querySelector('[data-o="xhi"]').value);
      if (t && lo != null && hi != null && hi > lo) {                       // the student writes the edges: from now on it is an explicit window in Da, the same for every file
        const d = hrE ? HR.dec(HR.xicFiles(p.tab), 1) : 1;
        t.mz = rh((lo + hi) / 2, hrE ? 5 : 2); t.w = rh((hi - lo) / 2, hrE ? 5 : 2); t.label = `m/z ${hrE ? t.mz.toFixed(4) : t.mz.toFixed(mzd(p))}`; delete t.ion; delete t.obs;
      }
      ctl(p); draw(p);
    };
    else if (k === "view") x.onclick = () => MAPPA.open3d(p, x);       // mappa.js: the 3D viewer (mappa3d.js), a modal over the page
    else if (k === "split") x.onclick = () => splitPanel(p);
    else if (k === "rt0" || k === "rt1") x.onchange = () => {            // visible time interval of the panel
      const q = c.querySelectorAll('[data-o="rt0"],[data-o="rt1"]'), a = p._a || {}, u = numMz(q[0].value), v = numMz(q[1].value);
      if (u == null && v == null) { pushZh(p); p.zoom = null; p.zoomY = null; draw(p); uiSave(); return; }
      const lo = u ?? a.xmin ?? a.x0, hi = v ?? a.xmax ?? a.x1;
      if (lo != null && hi != null && hi > lo) { pushZh(p); zoomTo(p, lo, hi); uiSave(); }
    };
    else if (k === "corrf") { /* wired by the Correzione button just below */ }
    else if (k === "corr") corrWire(x, v => setCorr(p, v));
        else x.onchange = () => {
      if (p.type === "map" && k === "k" && follow(p.tab || E.tab)) {      // the map follows the file chosen in the bar: the menu chooses that file, like the bar does
        p.k = +x.value; STATO.file(+x.value, "file"); renderFileList(); renderNav(); redrawAll(); uiSave(); return;
      }
      if (k === "tr" && x.value === "__quant_qual") {
        p.tr = [CAL.quant, CAL.qual].filter(Boolean);                // CAL is a const of tabs.js: a global lexical binding, not a property of window
      } else {
        p[k] = (k === "mz0" || k === "mz1") ? numMz(x.value) : x.type === "checkbox" ? x.checked : x.type === "number" ? +x.value : (k === "k" || k === "level") ? +x.value : x.value === "" ? (k === "fk" || k === "tr" ? "" : null) : x.value;
      }
      if (k === "bk" || k === "bg") p[k] = x.value === "" ? "" : x.value === "w" || x.value === "x" ? x.value : +x.value;
      if (p.type === "spec" && ["k", "level", "prec", "all"].includes(k)) p.meas = null;      // the measures belong to the spectrum they were made on
      if (["level", "all", "snip", "bg", "kind", "mz0", "mz1", "scale"].includes(k)) ctl(p);
      draw(p);
    };
  });
  syncTb(p);
  fitHdSoon(p);
}
function rngIsOn(p) {
  const ms2c = p.type === "chrom" && (p.tab || E.tab) === "ms2";
  return (p.type !== "mrm" && !(p.type === "chrom" && p.kind === "pda") && (p.mz0 != null || p.mz1 != null)) || !!p.zoom || (p.type === "chrom" && p.prec != null && p.prec !== "" && !ms2c);
}
// state of the smoothing and log buttons of the right-hand group (they live in the static header, not in ctl)
function syncTb(p) {
  const el = p.el; if (!el) return;
  const T0 = p.tab || E.tab, ms2c = p.type === "chrom" && T0 === "ms2";
  const sm = el.querySelector('[data-a="smooth"]'), lg = el.querySelector('[data-a="log"]');
  if (sm) { const tip = I18N.t(p.type === "mrm" ? "ctl.mrm.smooth.title" : "ctl.smooth.title"); sm.classList.toggle("on", !!p.smooth); sm.setAttribute("aria-pressed", String(!!p.smooth)); sm.title = tip; }
  if (lg) {
    const off = ms2c || (p.type === "chrom" && p.kind === "pda"); if (ms2c) p.log = false;
    lg.disabled = off; lg.classList.toggle("on", !!p.log && !off); lg.setAttribute("aria-pressed", String(!!p.log && !off)); lg.title = off && ms2c ? I18N.t("ctl.noMs2") : I18N.t("ctl.log.title");
  }
}
// ---- one low header row: when the groups do not fit in the width of the panel the least needed ones go into the «⋯» menu (the nodes are moved, so they keep their
// handlers, and put back in place when there is room again); nothing wraps and nothing is cut. Order: the NL note, then the icon groups from the right, then the controls from the right.
function hdRestore(p) {
  const l = p._hm || []; p._hm = [];
  l.slice().reverse().forEach(n => { if (n._ph) { n._ph.replaceWith(n); n._ph = null; } n.classList.remove("hd-ov"); });
  const m = p.el && p.el.querySelector(".hmore"), b = p.el && p.el.querySelector('[data-a="more"]');
  if (m) m.hidden = true; if (b) { b.hidden = true; b.setAttribute("aria-expanded", "false"); }
}
function fitHd(p) {
  const el = p.el; if (!el || !el.isConnected || p._fitBusy) return;
  const hd = el.querySelector(".hd"), more = el.querySelector(".hmore"), mb = el.querySelector('[data-a="more"]'); if (!hd || !more || !mb) return;
  p._fitBusy = true;
  try {
    const open = !more.hidden;
    hdRestore(p);
    if (!hd.offsetParent && getComputedStyle(el).position !== "fixed") return;           // tab not shown
    const c = el.querySelector(".ctl");
    const over = () => hd.scrollWidth > hd.clientWidth + 1 || (c && c.scrollWidth > c.clientWidth + 1);
    if (!over()) return;
    mb.hidden = false;
    const cands = [...el.querySelectorAll(".bnl")], groups = [...el.querySelectorAll(".tbs > .tbg")].reverse(), kids = c ? [...c.children].slice(1).reverse() : [];
    for (const n of [...cands, ...groups, ...kids]) {
      if (!over()) break;
      if (n.hidden || !n.offsetWidth) continue;
      if (n.classList.contains("bnl")) { n.classList.add("hd-ov"); p._hm.push(n); continue; }
      const ph = document.createComment(""); n._ph = ph; n.replaceWith(ph); more.appendChild(n); p._hm.push(n);
    }
    if (!p._hm.some(n => n._ph)) mb.hidden = true;
    else if (open) { more.hidden = false; mb.setAttribute("aria-expanded", "true"); hdMorePlace(p); }
  } finally { p._fitBusy = false; }
}
function fitHdSoon(p) { FRAME.cancel(p._fitRaf); p._fitRaf = FRAME.next(() => fitHd(p)); clearTimeout(p._fitT); p._fitT = setTimeout(() => fitHd(p), 300); }       // a second pass: widgets that settle after the first layout (fonts, async parts of the header) must not leave it too wide
function hdMorePlace(p) { const hd = p.el.querySelector(".hd"), m = p.el.querySelector(".hmore"); if (hd && m) { m.style.top = hd.offsetTop + hd.offsetHeight + 2 + "px"; m.style.maxHeight = Math.max(80, p.el.clientHeight - hd.offsetHeight - 16) + "px"; } }
function hdMoreInit(p) {
  const mb = p.el.querySelector('[data-a="more"]'), m = p.el.querySelector(".hmore"); if (!mb || !m) return;
  const close = back => { m.hidden = true; mb.setAttribute("aria-expanded", "false"); if (back) mb.focus(); };
  mb.onclick = e => { e.stopPropagation(); if (m.hidden) { m.hidden = false; mb.setAttribute("aria-expanded", "true"); hdMorePlace(p); } else close(false); };
  m.addEventListener("keydown", e => { if (e.key === "Escape") { e.stopPropagation(); close(true); } });
  p._moreAway = e => { if (!m.hidden && p.el.isConnected && !e.target.closest(".hmore") && !e.target.closest('[data-a="more"]') && !e.target.closest(".corrm,.cpop,#fpop,#ctx")) close(false); };
  document.addEventListener("mousedown", p._moreAway, { capture: true, signal: p._ac.signal });
  document.addEventListener("keydown", e => { if (e.key === "Escape" && !m.hidden && p.el.isConnected) { close(true); } }, { signal: p._ac.signal });
}
function addTrace(p, mz, label, w) {
  mz = rh(mz, w != null ? 2 : 1);       // (a window centre is half of two 0.1 values: two decimals, the edges stay at one) unit-resolution instrument: one decimal is all that means anything
  p.traces.push({ id: E.seq++, mz, w: w ?? null, label: label || "m/z " + mz.toFixed(mzd(p)) });
  ctl(p); draw(p);
}
const numMz = t => { const v = numIt(t); return isFinite(v) && v > 0 ? Math.round(v * 100) / 100 : null; };
// ONE rounding rule everywhere (half up on the decimal text, like mzlab.chem round_half_up): 364.15 gives 364.2 although 364.15*10 is 3641.4999... in binary
const rh = (v, d = 1) => { const n = Number(v); if (!isFinite(n)) return n; const sg = n < 0 ? -1 : 1; return sg * Number(Math.round(Number(Number(Math.abs(n).toFixed(9)) + "e" + d)) + "e-" + d); };   // the 9-digit prefilter is the same as round_half_up in elements.py
window.rh = rh;
const numMz1 = t => { const v = numIt(t); return isFinite(v) && v > 0 ? rh(v, 1) : null; };   // XIC fields: one decimal only
const mzd = (p, lv = 1) => window.HR ? HR.dec(scanFiles(tabFiles(p ? p.tab || E.tab : E.tab)), p && p.level ? p.level : lv) : 1;
const fmt2 = v => String(+(+v).toFixed(2));
const fmz = (v, p, lv = 1) => String(rh(v, mzd(p, lv)));                   // 363.8, 364 (no trailing zeros)
// THE tool that extracts an ion (XIC). The student gives ONE m/z value, or the NEUTRAL formula plus the adduct; the window is always a unit window (1 Da, unit resolution)
// anchored to the nominal mass n of the calculated value: [n - XIC_BELOW, n + 1 - XIC_BELOW]. Every way to create an XIC (XIC button, warning when integrating from a TIC,
// right click on a chromatogram, on a spectrum, on the RT-m/z map, Calcolatrice m/z, Disegno) opens this window, possibly pre-filled:
//   pre = { mz, obs }   one m/z value (obs = it is an OBSERVED centroid, so the calibration drift is taken off before the nominal mass is found)   pre = { formula, adduct }
const TIP_DELAY = 1250;   // ms before the label of a button appears (one value for the whole program; 1500 before 7 Oct 2026)
const XIC_BELOW = 0.2;   // the unit window starts 0.2 Da under the nominal mass n and ends 0.8 Da over it: [n-0.2, n+0.8]. The centroids of the lab files sit +0.14..+0.37 Da over the calculated m/z (AGENTS.md, 6 Oct 2026), so a window [n-0.5, n+0.5] would lose every ion whose mass defect plus drift passes +0.5 (e.g. calc n+0.2 observed up to n+0.57), while [n-0.2, n+0.8] keeps them with 0.13+ Da of margin and still leaves out the neighbours (n-1 observed <= n-0.3, n+1 observed >= n+1.04)
const XIC_DRIFT = 0.25;  // mean offset (Da) of the observed centroids over the calculated m/z, used only to go from a CLICKED centroid back to the calculated nominal mass (364.37 -> 364.12 -> n = 364, not 365)
const xicNominal = (v, obs) => Math.round(obs ? v - XIC_DRIFT : v);
const xicWin = (v, obs) => { const n = xicNominal(v, obs); return [rh(n - XIC_BELOW, 1), rh(n + 1 - XIC_BELOW, 1)]; };
let XIC_LAST = null;       // the last ion the student asked for ({mz} or {formula, adduct}): the window opens with it, still editable
// Right click on a peak of the spectrum: the XIC appears at once, with the default unit window and no dialog; the window is changed afterwards from the header of the XIC panel.
function xicDirect(mz, after, panel) {
  const [a, b] = xicWin(mz, true), c = rh((a + b) / 2, 2), w = rh((b - a) / 2, 2), lab = `m/z ${c.toFixed(mzd(panel))}`;
  XIC_LAST = { mz };
  const ht = window.HR && HR.ionTrace(mz, { obs: true, tab: panel && panel.tab });          // a high-resolution file among the files: the ion with its ppm window
  if (panel && E.panels.includes(panel)) { if (ht) { panel.traces.push(ht); ctl(panel); draw(panel); } else addTrace(panel, c, lab, w); reveal(panel); return panel; }
  const np = addPanel("xic", { traces: [ht || { id: E.seq++, mz: c, w, label: lab }] });
  if (after && E.panels.includes(after)) {
    np.el.classList.add("drag"); np.y = after.y + after.h; np.el.style.top = np.y + "px"; void np.el.offsetHeight; np.el.classList.remove("drag");
    stackAfter(np, after); relayout(); fitHost(); 
  } else reveal(np);
  return np;
}
const xicName = q => { const t = q.traces[0]; return t && t.ion ? t.label + (q.traces.length > 1 ? " +" + (q.traces.length - 1) : "") : t ? "m/z " + t.mz.toFixed(mzd(q)) + (q.traces.length > 1 ? " +" + (q.traces.length - 1) : "") : I18N.t("xic.empty"); };
// Estrai ioni (XIC): a list of rows (2 at the start, up to 10), each one an m/z or a neutral formula (+ adduct); all the ions go into the SAME panel, one trace per ion.
const XIC_MAXROWS = 10, XIC_ADDUCTS = ["[M+H]+", "[M+NH4]+", "[M+Na]+", "[M+K]+", "[M-H]-", "[M+Cl]-", "[M+HCOO]-"];
function openXic(panel, pre = {}) {
  if (!pre.formula && pre.mz == null && XIC_LAST) pre = { ...XIC_LAST, ...pre };
  const d = Q("#xicdlg"), host = Q("#xic-rows"), err = Q("#xic-err"), fsel = Q("#xic-file"), addB = Q("#xic-add");
  const dAd = pre.adduct || defAdduct(panel && E.panels.includes(panel) ? scanFiles(shown()) : undefined);
  const rows = [];
  const isNum = t => /^[\d.,\s]+$/.test(t);
  const renumber = () => {
    rows.forEach((r, i) => { r.lab.textContent = i === 0 ? I18N.t("xic.ion1") : I18N.t("xic.ionN", { n: i + 1 }); r.x.hidden = rows.length < 2; r.inp.placeholder = i === 0 ? I18N.t("xic.placeholder") : I18N.t("xic.optional"); r.inp.id = i === 0 ? "xic-mz" : ""; r.ad.id = i === 0 ? "xic-ad" : ""; r.sum.id = i === 0 ? "xic-sum" : ""; });
    addB.disabled = rows.length >= XIC_MAXROWS;
  };
  const xtab = () => panel && E.panels.includes(panel) ? panel.tab || E.tab : E.tab;
  const hrFile = () => window.HR ? HR.xicFiles(xtab()).find(f => HR.isHr(f, f.lv)) : null;       // high resolution: the ion is a number with the decimals of the profile, ppm window
  const lowFiles = () => window.HR ? HR.xicFiles(xtab()).some(f => !HR.isHr(f, f.lv)) : true;
  const sumText = r => {                                                    // one line under the row: the window(s) that will be used
    const hf = hrFile(), unit = r.win ? `<i>m/z</i> ${fmz(r.win[0])} - ${fmz(r.win[1])}` : "";
    return hf && r.ex != null ? HR.ionText(r.ex, hf).replace("m/z", "<i>m/z</i>") + (lowFiles() ? ` <span class="muted">${I18N.t("xic.lowRes", { unit })}</span>` : "") : unit;
  };
  const evalRow = r => {
    const t = r.inp.value.trim(), ed = ++r.edits; err.textContent = ""; r.ex = null;
    r.ad.hidden = !t || isNum(t);
    if (!t) { r.win = null; r.label = ""; r.sum.textContent = ""; r.pending = Promise.resolve(); return; }
    if (isNum(t)) {
      const v = numMz1(t), x = numIt(t);
      if (hrFile() && isFinite(x) && x > 0) r.ex = rh(x, 5);
      r.win = v != null ? xicWin(v, r.obs) : null; r.label = ""; r.sum.innerHTML = r.win ? sumText(r) : ""; r.pending = Promise.resolve(); return;
    }
    r.win = null; r.sum.textContent = "";
    r.pending = (async () => {
      try {
        const f = await getFormula(t, r.ad.value);
        if (ed !== r.edits) return;
        r.label = `${f.formula} ${f.adduct}`; r.win = xicWin(f.mz, false); if (hrFile()) r.ex = f.mz;
        r.sum.innerHTML = (t.replace(/\s+/g, "") !== f.formula ? I18N.t("xic.interpreted", { formula: fmtFormula(f.formula) }) + " " : "") + sumText(r);
      } catch (e) { if (ed === r.edits) r.sum.innerHTML = `<span class="fail">${I18N.t("xic.badFormula")}</span>`; }
    })();
  };
  const addRow = (init = {}) => {
    const el = document.createElement("div"); el.className = "xrw";
    el.innerHTML = `<label></label><div class="xin"><input class="mzf" autocomplete="off" inputmode="decimal" style="width:auto"><select hidden title="${I18N.t("xic.adduct.title")}">${XIC_ADDUCTS.map(a => `<option ${a === (init.adduct || dAd) ? "selected" : ""}>${a}</option>`).join("")}</select><button class="xr-x" type="button" title="${I18N.t("xic.remove.title")}">&times;</button></div><div class="xr-sum"></div>`;
    const r = { el, lab: el.querySelector("label"), inp: el.querySelector("input"), ad: el.querySelector("select"), x: el.querySelector(".xr-x"), sum: el.querySelector(".xr-sum"), win: null, label: "", obs: !!init.obs, edits: 0, pending: Promise.resolve(), tq: 0 };
    r.inp.value = init.text || "";
    r.inp.oninput = () => { r.obs = false; clearTimeout(r.tq); const t = r.inp.value.trim(); if (!t || isNum(t)) evalRow(r); else { r.ad.hidden = false; r.tq = setTimeout(() => evalRow(r), 500); } };
    r.ad.onchange = () => evalRow(r);
    r.inp.onkeydown = e => {                                                // Enter extracts at once (empty field: only the message); Shift+Enter moves to the next field
      if (e.key !== "Enter") return; e.preventDefault();
      if (e.shiftKey) { const i = rows.indexOf(r); if (i < rows.length - 1) rows[i + 1].inp.focus(); return; }
      if (!r.inp.value.trim()) { err.textContent = I18N.t("xic.err.empty"); return; }
      Q("#xic-go").click();
    };
    r.x.onclick = () => { if (rows.length < 2) return; rows.splice(rows.indexOf(r), 1); el.remove(); renumber(); };
    rows.push(r); host.appendChild(el); renumber(); evalRow(r);
    return r;
  };
  host.innerHTML = ""; err.textContent = "";
  const first = pre.formula ? { text: pre.formula, adduct: pre.adduct } : pre.mz != null ? { text: fmz(pre.mz), obs: !!pre.obs } : {};
  addRow(first); addRow();
  addB.onclick = () => { if (rows.length < XIC_MAXROWS) addRow().inp.focus(); };
  const ff = scanFiles(tabFiles(panel && E.panels.includes(panel) ? panel.tab || E.tab : E.tab));
  fsel.innerHTML = `<option value="">${I18N.t("xic.shownFiles")}</option>` + ff.map(f => `<option value="${f.k}">${EH(f.label)}</option>`).join("");
  Q("#xic-no").onclick = () => d.close();
  Q("#xic-go").onclick = async () => {
    await Promise.all(rows.map(r => r.pending));
    const used = rows.filter(r => r.inp.value.trim());
    if (!used.length) { err.textContent = I18N.t("xic.err.empty"); return; }
    const bad = used.find(r => !r.win); if (bad) { err.textContent = I18N.t("xic.err.row"); bad.inp.focus(); return; }
    const seen = new Set(), traces = [];
    used.forEach(r => {
      if (r.ex != null && hrFile()) {                                       // high resolution: the ion (exact m/z), each file reads it with its own tolerance
        const ht = HR.ionTrace(r.ex, { obs: r.obs, prefix: r.label, tab: xtab() }), key = "ion|" + ht.mz; if (seen.has(key)) return; seen.add(key); traces.push(ht); return;
      }
      const [a, b] = r.win, mz = rh((a + b) / 2, 2), w = rh((b - a) / 2, 2), key = mz + "|" + w; if (seen.has(key)) return; seen.add(key);
      traces.push({ id: E.seq++, mz, w, label: (r.label ? r.label + " · " : "") + `m/z ${mz.toFixed(mzd(panel))}` });
    });
    d.close();
    const r0 = used[0]; XIC_LAST = isNum(r0.inp.value.trim()) ? { mz: r0.ex ?? numMz1(r0.inp.value) ?? (r0.win[0] + r0.win[1]) / 2 } : { formula: r0.inp.value.trim(), adduct: r0.ad.value };
    let np;
    if (panel && E.panels.includes(panel)) { traces.forEach(t => panel.traces.push(t)); np = panel; ctl(np); }
    else {
      np = addPanel("xic", { traces });
      if (pre.after && E.panels.includes(pre.after)) { 
        np.el.classList.add("drag"); np.y = pre.after.y + pre.after.h; np.el.style.top = np.y + "px"; void np.el.offsetHeight; np.el.classList.remove("drag");
        stackAfter(np, pre.after); relayout(); fitHost(); 
      }
    }
    // the file: the one chosen in the window; with more than 3 ions and several files the panel starts on the selected file only (otherwise it is a tangle of lines)
    const keep = fsel.value !== "" ? E.files[+fsel.value] : traces.length > 3 && ff.length > 1 && ff.includes(E.files[E.cur]) ? E.files[E.cur] : null;
    if (keep) ff.forEach(f => { if (f !== keep) traces.forEach(t => { np.hid["x|" + f.k + "|" + t.mz] = true; }); });
    if (np === panel) draw(np); else if (!(pre.after && E.panels.includes(pre.after))) reveal(np);
  };
  d.showModal();
  rows[0].inp.focus();
}
function splitPanel(p) {
  const rest = p.traces.slice(1); p.traces = p.traces.slice(0, 1); ctl(p); draw(p);
  let prev = p;                                                 // the new panels go right under this one, in the order of the traces
  rest.forEach(t => { const q = addPanel("xic", { traces: [t], tol: p.tol, smooth: p.smooth, mode: p.mode, log: p.log }); stackAfter(q, prev); prev = q; });
  relayout(); fitHost(); reveal(prev);
}

