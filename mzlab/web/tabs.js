// @ts-strict
// @ts-check
// Experiment types of the Dati view: Full Scan, MS2 (Product Ion) and MRM never share a graph. Classic script, loaded after explore.js
// (uses E, ESP, tabFiles, tabPanels, addPanel, ... from there). Each type has its own block of panels (and starting layout) on the same page; E.tab is the type of the current file.

const firstTab = () => (ESP.ids.find(t => tabFiles(t).length) || ESP.ids[0]);
const pickTab = want => (want && tabFiles(want).length ? want : firstTab());
E.curBy = {};

// the tooltip of a group of the file list: the scheme Q1 -> q2 -> Q3 in big, with the words (texts from modi.js)
function modeSchema(t) {
  try {
    const m = QMODI.M[ESP.didattica(t)], st = ["Q1", "q2", "Q3"].map((n, i) => `<div class="qb"><b>${n}</b><span>${m.q[i].replace(/^Q[123] /, "")}</span></div>`);
    return `<div class="qs"><b>${m.name}</b><div class="qr">${st.join('<span class="qa">&rarr;</span>')}</div><div class="qw"><b>${m.what}.</b> ${m.use}</div></div>`;
  } catch (e) { return ""; }
}
// the page goes to the first panel of an experiment type
function scrollToBlock(t) {
  setTimeout(() => { const f = E.panels.filter(q => q.tab === t && q.el).sort((u, v) => u.y - v.y)[0]; if (f) f.el.scrollIntoView({ block: "start", behavior: "smooth" }); }, 500);
}

function setTab(t, quiet, motivo) {
  if (t === E.tab && !quiet) return;
  E.curBy[E.tab] = E.cur;
  E.activeBy = E.activeBy || {};      // coming back to a tab: same active graph as when the student left it (no automatic scroll)
  E.activeBy[E.tab] = E.active; STATO.scheda(t, motivo || "tab");
  const back = E.curBy[t]; STATO.file(E.files[back]?.kind === t ? back : (tabFiles()[0] || { k: E.cur }).k, "tab");
  setActive(E.activeBy[t] && E.panels.includes(E.activeBy[t]) ? E.activeBy[t] : null); playStop();
  E.panels.forEach(q => { if (q.type === "spec") q.lock = null; });      // changing tab: the axes of the spectra are free again
  const firstLayout = tabFiles().length && !tabPanels(t).length;
  if (firstLayout) loading(true);                       // same funny loading screen as everywhere else
  ensureLayout();
  if (firstLayout) Promise.all(tabPanels(t).map(p => p.ready).filter(Boolean)).catch(() => {}).then(() => loading(false));
  fitWidth(); relayout(); fitHost(); renderFileList(); renderNav();
  // @ts-expect-error toolbar() of explore.js is also window.toolbar (a BarProp) for lib.dom
  toolbar();
  FRAME.next(() => redrawAll());
  uiSave();
}

// first time a tab is shown (or after all its panels were closed and the page reloaded): its starting layout
function ensureLayout() {
  if (tabFiles().length && !E.panels.some(p => p.tab === E.tab)) defaultLayoutTab(E.tab);
  if (lrTog()) ensureBlocks();
}
// every experiment type that has files gets its block (its starting layout) under the ones already on the page
function ensureBlocks() {
  ESP.ids.forEach(t => {
    if (!tabFiles(t).length || E.panels.some(p => p.tab === t)) return;
    const before = new Set(E.panels);
    STATO.conScheda(t, () => defaultLayoutTab(t), "blocks");
    const shift = () => {
      const nw = E.panels.filter(q => !before.has(q) && q.tab === t && !q._sh); if (!nw.length) return false;
      const top = Math.min(...nw.map(q => q.y)), off = E.panels.filter(q => q.tab !== t).reduce((m, q) => Math.max(m, q.y + q.h + 24), 0) - top;
      nw.forEach(q => { q._sh = 1; q.y += off; apply(q); });
      relayout(); fitHost(); renderFileList(); return true;
    };
    if (!shift() && t === "mrm") { let i = 0; const iv = setInterval(() => { if (shift() || ++i > 80) clearInterval(iv); }, 150); }
  });
}
// The blocks of the types one under the other, in the order of the types. Panels of a session saved when only one type was shown at a time share the same space: if the blocks overlap they are moved down here.
function stackBlocks() {
  const rg = ESP.ids.map(t => { const ps = E.panels.filter(p => p.tab === t); return ps.length ? [Math.min(...ps.map(p => p.y)), Math.max(...ps.map(p => p.y + p.h))] : null; }).filter(Boolean);
  if (!rg.some((r, i) => rg.some((u, j) => j > i && r[0] < u[1] && u[0] < r[1]))) return;
  let y = null;
  ESP.ids.forEach(t => {
    const ps = E.panels.filter(p => p.tab === t); if (!ps.length) return;
    const top = Math.min(...ps.map(p => p.y)), off = y === null ? 0 : y - top;
    ps.forEach(p => { p._sh = 1; if (off) { p.y += off; apply(p); } });
    y = Math.max(...ps.map(p => p.y + p.h)) + 24;
  });
}
// transitions of the MRM files and which one is the quantifier / qualifier (by the name in the method: Quant, Qual; otherwise the first and the second).
// The calibration line itself is NOT made by the program: the students do it in Excel from the integration table.
const CAL = { quant: null, qual: null, trs: /** @type {any[]} */ ([]) };
function calPick() {
  const t = CAL.trs;
  if (!t.some(x => x.key === CAL.quant)) CAL.quant = (t.find(x => /quant/i.test(x.name)) || t[0] || {}).key || null;
  if (!t.some(x => x.key === CAL.qual) || CAL.qual === CAL.quant) CAL.qual = (t.find(x => /qual/i.test(x.name) && x.key !== CAL.quant) || t.find(x => x.key !== CAL.quant) || {}).key || null;
}
async function calLoad() {
  const mf = E.files.filter(f => f.kind === "mrm"), seen = new Map();
  (await Promise.all(mf.map(f => getMrm(f.k)))).forEach(tr => tr.forEach(t => { const key = `${t.q1}>${t.q3}`; if (!seen.has(key)) seen.set(key, { key, name: t.name || "" }); }));
  CAL.trs = [...seen.values()]; calPick();
}

// MS2 experiments = one per precursor, over all the MS2 files: [{prec, ce: [..], n scans, files: n}]
function ms2Exps() {
  const m = new Map();
  tabFiles("ms2").forEach(f => (f.ms2_exps || []).forEach(x => {
    const e = m.get(x.prec) || { prec: x.prec, ce: new Set(), n: 0, files: new Set(), k: f.k };
    if (x.ce != null) e.ce.add(x.ce); e.n += x.n; e.files.add(f.k); m.set(x.prec, e);
  }));
  const l = [...m.values()];
  return l.length > 8 ? l.sort((a, b) => b.n - a.n) : l.sort((a, b) => (a.prec ?? 0) - (b.prec ?? 0));      // a data-dependent file has many precursors with few scans: the most scanned first
}
// a triangle on the survey chromatogram: open that product-ion scan in the MS2 tab
async function goMs2(t) {
  setTab("ms2", true);
  await new Promise(r => setTimeout(r, 700));
  await ms2Switch(t.prec);
  const c = tabPanels("ms2").filter(q => q.type === "chrom").sort((a, b) => a.y - b.y)[0]; if (!c) return;
  STATO.file(t.k2, "transition"); c.cur = t.rt; c.sel = null; const dt = 0.01;
  pushLinked(c, t.rt - dt / 2, t.rt + dt / 2, t.k2); draw(c); uiSave();
}

function defaultLayoutTab(t) {
  const w = hostWidth();
  if (t === "full") {
    const f0 = tabFiles("full")[0];
    // chromatogram + linked spectrum must both fit in the window (a laptop at 1280x720 would push the spectrum below the fold and the arrows need both in view):
    // the usual 330 + 310 px when there is room, otherwise the height left in the window shared 50/50 (never below 190 px each)
    const dp = Q("#dpanels"), top = dp.offsetParent ? dp.getBoundingClientRect().top + scrollY : Q("header").getBoundingClientRect().height + 84, avail = innerHeight - top - 42;   // (not laid out yet while the loading screen is up: header + tabs + toolbar)
    const g = pnlGap(); let hc = 330, hs = 310; if (avail < hc + hs + g) { hc = Math.max(190, Math.round((avail - g) * 0.5)); hs = Math.max(190, Math.round(avail - g - hc)); }
    const c = addPanel("chrom", { tab: "full", x: 0, y: 0, w, h: hc, full: true });
    const sp = addPanel("spec", { tab: "full", link: c.id, k: f0.k, level: 1, x: 0, y: hc + g, w, h: hs, full: true });
    if (window.DDA && f0.dda) DDA.make(c, sp);                 // a data-dependent file: the MS2 scans sit to the right of the Full Scan
    apexSpectrum(c, sp);
    let tries = 0, h0 = [c.h, sp.h];       // once the graphs are on screen the real top of the page is known (a wrapped toolbar moves it): correct the heights then
    const fix = () => {
      if (!E.panels.includes(c) || !E.panels.includes(sp) || c.h !== h0[0] || sp.h !== h0[1]) return;      // the student already resized them
      if (!dp.offsetParent) { if (++tries < 60) FRAME.next(fix); return; }
      const av = innerHeight - (dp.getBoundingClientRect().top + scrollY) - 14;
      const g = pnlGap(); if (c.h + sp.h + g <= av) return;
      c.h = Math.max(190, Math.round((av - g) * 0.5)); sp.h = Math.max(190, Math.round(av - g - c.h)); sp.y = c.y + c.h + gapAfter(c, sp); apply(c); apply(sp); relayout(); fitHost(); draw(c); draw(sp);
    };
    FRAME.next(fix);
    if (window.BANCO) BANCO.afterLayout();       // high-resolution bench: product ions that are not a DDA (infusion MSn, PRM) get their own cells under the survey
  } else if (t === "ms2") {
    const exps = ms2Exps(), f0 = tabFiles("ms2")[0]; let y = 0;
    const list = exps.length ? exps.slice(0, 1) : [{ prec: null, k: f0.k }];       // ONE pair (chromatogram + spectrum) for the first precursor; the list on the left switches it
    list.forEach(e => { addMs2Pair(e, y); y += 550; });
  } else if (t === "mrm") {
    if (E._mrmLoading) return; E._mrmLoading = true;
    calLoad().catch(() => {}).then(() => {
      E._mrmLoading = false; if (tabPanels("mrm").length) return;
      const mfs = tabFiles("mrm");
      mfs.forEach((f, i) => { f.vis = (i === 0); });
      if (typeof renderFileList === "function") renderFileList();
      const w2 = hostWidth(), keys = [CAL.quant, CAL.qual].filter(Boolean);
      addPanel("mrm", { tab: "mrm", tr: keys.length ? keys : "", title: PT.mrm, x: 0, y: 0, w: w2, h: 420, full: true, imode: "man", intf: "all" });
      relayout(); fitHost(); uiSave();
    });
  }
}

// one MS2 experiment = precursor chromatogram (above) + product-ion spectrum (below, linked)
function addMs2Pair(e, y) {
  const w = hostWidth(), f0 = tabFiles("ms2")[0], k = e.k ?? f0.k, label = ptMs2(e.prec);
  const c = addPanel("chrom", { tab: "ms2", prec: e.prec, title: label, x: 0, y, w, h: 250, full: true });
  const sp = addPanel("spec", { tab: "ms2", link: c.id, k, level: 2, prec: e.prec, title: ptProduct(e.prec), x: 0, y: y + 260, w, h: 280, full: true });
  apexSpectrum(c, sp); return c;
}
const ms2PairOf = prec => E.panels.find(p => p.tab === "ms2" && p.type === "chrom" && String(p.prec) === String(prec));
// sidebar of the MS2 tab: tick a precursor to show its two graphs (they are created if missing), untick to close them, click the name to go to them
function ms2Toggle(prec, on) {
  const c = ms2PairOf(prec);
  if (on && !c) { const e = ms2Exps().find(x => String(x.prec) === String(prec)); if (e) { addMs2Pair(e, tabPanels().reduce((m, q) => Math.max(m, q.y + q.h + pnlGap()), 0)); relayout(); fitHost(); } }
  if (!on && c) { [...E.panels.filter(q => q.link === c.id || q.src === c.id), c].forEach(q => q.el.querySelector(".x").click()); }
  renderFileList(); uiSave();
}
// click on a precursor in the list: if it already has its graphs, go there; otherwise the main pair (the top one) switches to it: no new panels
function ms2Goto(prec) {
  const c = ms2PairOf(prec); if (!c) return ms2Switch(prec);
  setActive(c); front(c.el); window.scrollTo({ top: Q("#dpanels").getBoundingClientRect().top + scrollY + c.y - hdrH() - 12, behavior: "smooth" });
}
function ms2Switch(prec) {
  const c = tabPanels("ms2").filter(q => q.type === "chrom").sort((a, b) => a.y - b.y)[0];
  if (!c) return ms2Toggle(prec, true);
  c.prec = prec; c.title = ptMs2(prec); c.sel = null;
  const live = E.panels.filter(q => q.type === "spec" && q.link === c.id);
  live.forEach(sp => { sp.prec = prec; sp.zoom = null; sp.zoomY = null; sp.lock = null; sp.title = ptProduct(prec); });
  ctl(c); live.forEach(ctl);
  renderFileList(); uiSave();
  return Promise.resolve(draw(c)).then(() => live[0] ? apexSpectrum(c, live[0]) : null);       // the spectrum of the highest scan of the new precursor
}
