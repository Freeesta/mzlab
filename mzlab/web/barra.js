// @ts-strict
"use strict";
// Toolbar of the header (#stbar) and drawer ("cassetto") of the Dati view, LR and HR.
// One list of commands (CMD): the toolbar, its menus, "Altro" and the command palette are all built from it, so a new
// button is added in ONE place (id, i18n keys cmd.<id>.lab / .name / .tip, icon, run, visible).
// Side panel (#dfiles): hidden, or docked at full height to the left or right edge; the workspace (#v-workspace) always shrinks to the
// space left, never sits under it. Always-visible header button #sb-toggle (and Ctrl/Cmd+B) shows/hides it; a tool button shows the panel
// on that tool and a second click hides it. Below SHEET px it is a sheet that replaces the workspace. Width is draggable (splitter
// #sidebar-resize-handle, arrows, double click = default); side is left or right (drag the header to the other half of the window, or Alt+Shift+Left/Right).
// State: localStorage qqq.barra.lr / qqq.barra.hr ({tab, open, w, last}), side shared in qqq.barra.lato.

const BARRA = (() => {
  const K_LR = "qqq.barra.lr";
  const K_HR = "qqq.barra.hr";
  const K_SIDE = "qqq.barra.lato";
  const K_HINT = "qqq.barra.hint";   // first-visit hint under the toolbar: shown once per browser
  const HR_PANELS = ["hdr", "lst", "fil", "cmp", "iso", "idn", "disp"];
  const MIN_W = 280, MAX_W = 560;
  const SHEET = 900;                // below this width the panel is a sheet that replaces the workspace
  const AUTO_W = 1100;              // below this width the panel hides by itself (the first thing to go, before the charts); a tool button brings it back
  const PHONE_W = 600;
  const K_LAB = "qqq.barra.etichette";   // words next to the icons: auto (as many as fit) | always | hidden
  const MAIN = new Set(["file", "calc", "losses", "adducts", "lists", "scan", "form", "idn"]);      // the tools that keep their words longest
  const LEVELS = ["all", "main", "none"];
  const HYST = 8;                   // px of slack asked before a richer level comes back (no flicker at the edge)
  const DROP_ORDER = ["ptable", "qaqc", "lists", "adducts", "losses", "idn", "form", "scan"];       // first to go into "Altro" (File and Calculator never do)

  let _tab = "file";
  let _open = false;
  let _away = /** @type {{ open: boolean, top: number } | null} */ (null);       // state of the side panel when the student left Dati (Disegno, Teoria, loading)
  let _restore = false;             // hidden by the narrow window, not by the user: it comes back when the window widens
  let _forced = false;              // the user opened it in a narrow window: it stays until the window widens
  let _side = "left";
  let _w = 260;
  let _last = { scan: "hdr", form: "cmp" };
  let _mode = null;
  let _mounted = false;
  let _opener = null;
  let _before = null;
  let _dropped = new Set();
  let _labMode = "auto", _lvl = "none";
  let _fitRaf = 0;
  const _fitStats = { n: 0, last: 0, max: 0 };

  const Q1 = s => document.querySelector(s);
  const t = (k, p) => (typeof I18N !== "undefined" ? I18N.t(k, p) : k);
  const isHrMode = () => !!(window.HR && typeof BANCO !== "undefined" && BANCO.on());
  const storageKey = () => (isHrMode() ? K_HR : K_LR);
  const defaultW = () => (isHrMode() ? 340 : 300);
  const inData = () => typeof S !== "undefined" && S.view === "data" && !!S.sess && !S.adding;
  // context «theory»: the bar shows the commands of the Teoria (window.TCMD of the same-origin iframe), not the ones of the data
  let _ctx = "data";
  const inTheory = () => typeof S !== "undefined" && S.view === "theory";
  const tcmd = () => { try { const f = Q1("#tframe"); return (f && f.contentWindow && f.contentWindow.TCMD) || null; } catch (_) { return null; } };
  const sheet = () => window.innerWidth < SHEET;
  const maxW = () => Math.max(MIN_W, Math.min(MAX_W, Math.floor(window.innerWidth / 2)));
  const effW = () => Math.max(MIN_W, Math.min(maxW(), _w));
  const validTab = id => ["file", "calc", "losses", "adducts", "lists", "stat"].includes(id) || (id === "qaqc" && FEATURES.qaqc) || (HR_PANELS.includes(id) && isHrMode());

  // ---------------------------------------------------------------- icons
  const IC = d => `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${d}</svg>`;
  const QI = k => (typeof QICON !== "undefined" ? QICON.get(k, 16).replace(/ role="img" aria-label="[^"]*"/, ' aria-hidden="true" focusable="false"') : "");
  const ICONS = {
    file: IC('<path d="M3 2.5h6.5l3.5 3.5v7.5H3z"/><path d="M9.5 2.5V6H13"/>'),
    hdr: IC('<rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/><path d="M5 6h6M5 8.5h6M5 11h3"/>'),
    lst: IC('<path d="M3 4h10M3 8h10M3 12h10"/><circle cx="1.6" cy="4" r=".6"/><circle cx="1.6" cy="8" r=".6"/><circle cx="1.6" cy="12" r=".6"/>'),
    fil: IC('<path d="M3.5 2.5h6l3 3v8h-9z"/><path d="M9.5 2.5v3h3"/>'),
    cmp: IC('<path d="M2 12l3-8 3 8M3.2 9h3.6"/><path d="M10 5h4M10 8h4M10 11h4"/>'),
    iso: IC('<path d="M3 13V8M6 13V4M9 13V7M12 13V10"/>'),
    idn: IC('<path d="M3 2.5h8a2 2 0 0 1 2 2v9H5a2 2 0 0 1-2-2z"/><path d="M3 11.5a2 2 0 0 1 2-2h8"/>'),
    stat: IC('<path d="M2.5 13.5h11"/><path d="M4 11V8M7 11V4M10 11V6.5M13 11V9" stroke-linecap="round"/>'),
    qaqc: IC('<path d="M2.5 13.5h11"/><path d="M3.5 10l2.5-3 2.5 2 3-5"/><circle cx="11.5" cy="4" r="1" fill="currentColor"/>'),
    more: IC('<circle cx="3.5" cy="8" r="1" fill="currentColor"/><circle cx="8" cy="8" r="1" fill="currentColor"/><circle cx="12.5" cy="8" r="1" fill="currentColor"/>'),
    search: IC('<circle cx="7" cy="7" r="4.2"/><path d="M10.2 10.2L14 14"/>'),
    ext: IC('<path d="M9 2.5h4.5V7M13.5 2.5L7.5 8.5M11.5 9.5v3a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1h3"/>'),
    caret: IC('<path d="M4 6l4 4 4-4"/>'),
    panel: IC('<rect x="2" y="2.5" width="12" height="11" rx="1.5"/><path d="M9.5 2.5v11"/><path d="M11 5.5h1.5M11 8h1.5" stroke-width="1.2"/>'),
    left: IC('<rect x="2" y="2.5" width="12" height="11" rx="1.5"/><path d="M6.5 2.5v11"/>'),
    right: IC('<rect x="2" y="2.5" width="12" height="11" rx="1.5"/><path d="M9.5 2.5v11"/>')
  };

  // ---------------------------------------------------------------- the list of commands
  // id: key of cmd.<id>.* in lang/; dom: id given to the toolbar button (kept for tests and scripts); ic: icon; panel: drawer panel the
  // button opens (its state is aria-pressed); items: ids of the commands in its menu; run: what it does; visible: when it exists.
  /** @type {any[]} */
  const CMD = [
    { id: "file", ic: () => ICONS.file, panel: "file", visible: () => inData(), run: () => toggle("file") },
    { id: "calc", dom: "np-calc2", ic: () => QI("calc"), panel: "calc", run: () => (inData() ? toggle("calc") : window.calcToggleFloat && window.calcToggleFloat()) },
    { id: "losses", dom: "np-nl", ic: () => QI("nl"), panel: "losses", run: () => (inData() ? toggle("losses") : window.QQQRef && QQQRef.open("ls")) },
    { id: "adducts", dom: "np-ad", ic: () => QI("adduct"), panel: "adducts", run: () => (inData() ? toggle("adducts") : window.QQQRef && QQQRef.open("ad")) },
    { id: "lists", dom: "np-liste", ic: () => QI("liste"), panel: "lists", run: () => (inData() ? toggle("lists") : window.LISTE && LISTE.open()) },
    { id: "qaqc", feature: "qaqc", dom: "np-qaqc", ic: () => ICONS.qaqc, panel: "qaqc", visible: () => inData(), run: () => toggle("qaqc") },
    
    { id: "ptable", dom: "np-pt", ic: () => QI("pt"), run: () => window.QQQRef && QQQRef.open("pt") },
    // HR only (after a separator): two menus and one direct button
    { id: "scan", hr: true, items: ["hdr", "lst", "fil"], ic: () => ICONS[_last.scan] },
    { id: "form", hr: true, items: ["cmp", "iso"], ic: () => ICONS[_last.form] },
    { id: "idn", hr: true, ic: () => ICONS.idn, panel: "idn", run: () => toggle("idn") },
    // items of the HR menus (panels of the drawer, bodies in banco.js)
    { id: "hdr", item: "scan", ic: () => ICONS.hdr, panel: "hdr", run: () => pick("scan", "hdr") },
    { id: "lst", item: "scan", ic: () => ICONS.lst, panel: "lst", run: () => pick("scan", "lst") },
    { id: "fil", item: "scan", ic: () => ICONS.fil, panel: "fil", run: () => pick("scan", "fil") },
    { id: "cmp", item: "form", ic: () => ICONS.cmp, panel: "cmp", run: () => pick("form", "cmp") },
    { id: "iso", item: "form", ic: () => ICONS.iso, panel: "iso", run: () => pick("form", "iso") },
    // display options of a chart (right click on a chromatogram or a spectrum): only in the palette, the panel is in the drawer
    { id: "disp", bar: false, group: "graphs", panel: "disp", visible: () => inData() && isHrMode(), ic: () => IC('<path d="M2.5 4h6M11.5 4h2M2.5 8h2M7.5 8h6M2.5 12h8M13 12h.5"/><circle cx="10" cy="4" r="1.5"/><circle cx="6" cy="8" r="1.5"/><circle cx="12" cy="12" r="1.5"/>'), run: () => window.DISP && DISP.open() },
    // smoothing of the active chromatogram: also a button in its header, here for when the header has put it in the «⋯» menu
    { id: "smooth", bar: false, group: "graphs", visible: () => inData(), ic: () => IC('<path d="M2 11c2.5 0 3-6 6-6s3.5 6 6 6"/>'),
      run: () => { const q = E.active; if (q && E.panels.includes(q) && ["chrom", "xic", "mrm"].includes(q.type)) { q.smooth = !q.smooth; ctl(q); draw(q); uiSave(); } else nearMsg(I18N.t("cmd.smooth.none")); } },
    // command palette (also reachable from the 🔍 button and Ctrl/⌘ K)
    { id: "search", bar: "end", dom: "tb-search", group: "app", ic: () => ICONS.search, run: () => window.PALETTE && PALETTE.open() },
    // commands that live elsewhere on the page: found by the palette, never in the bar (group: where they are)
    { id: "method", bar: false, group: "graphs", ic: () => IC('<rect x="3" y="2.5" width="10" height="11.5" rx="1.6"/><path d="M6 6h4M6 9h4M6 12h2"/>'), visible: () => inData(), run: () => clickEl("#np-method") },
    { id: "subref", bar: false, group: "graphs", ic: () => IC('<path d="M2 11c2 0 2-6 4-6s2 6 4 6 2-3 4-3"/><path d="M2 13.5h12" stroke-dasharray="2 2"/>'), visible: () => inData() && window.SUBREF && tabFiles().length > 1, run: () => window.SUBREF && SUBREF.toggle() },
    { id: "gradient", bar: false, group: "graphs", ic: () => ICONS.qaqc, visible: () => inData() && !!lineHere(p => window.GRADIENTE && GRADIENTE.have(p)), run: () => { const p = lineHere(q => window.GRADIENTE && GRADIENTE.have(q)); if (p) { p.overlay = p.overlay === "grad" ? null : "grad"; draw(p); uiSave(); } } },
    { id: "pressure", bar: false, group: "graphs", ic: () => ICONS.qaqc, visible: () => inData() && !!lineHere(p => window.GRADIENTE && GRADIENTE.pressHave(p) && !isAnalog(p.kind)), run: () => { const p = lineHere(q => window.GRADIENTE && GRADIENTE.pressHave(q) && !isAnalog(q.kind)); if (p) { p.overlay = p.overlay === "press" ? null : "press"; draw(p); uiSave(); } } },
    { id: "marker", bar: false, group: "graphs", ic: () => ICONS.qaqc, visible: () => inData() && isHrMode() && !!specHere(a => a.markN != null), run: () => { const p = specHere(a => a.markN != null); if (p) { p.mk = !markOn(p); draw(p); uiSave(); } } },
    { id: "nocont", bar: false, group: "graphs", ic: () => ICONS.qaqc, visible: () => inData() && isHrMode() && !!specHere(a => a.hrp), run: () => { const p = specHere(a => a.hrp); if (p) { p.noCont = !p.noCont; draw(p); uiSave(); } } },
    { id: "openms2", bar: false, group: "graphs", ic: () => ICONS.qaqc, visible: () => inData() && isHrMode() && !!lastMs2(), run: () => { const q = lastMs2(); if (q) DDA.openMs2(q.p, q.j); } },
    { id: "labels", bar: false, group: "app", ic: () => ICONS.search, run: () => { setLabels(_labMode === "auto" ? "always" : _labMode === "always" ? "hidden" : "auto"); announce(t("cmd.labels.now." + _labMode)); } },
    { id: "report", bar: false, group: "app", ic: () => ICONS.qaqc, visible: () => inData(), run: () => window.RAPPORTO && RAPPORTO.export("html") },
    { id: "settings", bar: false, group: "app", ic: () => IC('<circle cx="8" cy="8" r="2.2"/><path d="M8 1.8v1.7M8 12.5v1.7M1.8 8h1.7M12.5 8h1.7M3.6 3.6l1.2 1.2M11.2 11.2l1.2 1.2M12.4 3.6l-1.2 1.2M4.8 11.2l-1.2 1.2"/>'), run: () => clickEl("#np-set") },
    { id: "chrom", bar: false, group: "graphs", ic: () => IC('<path d="M2 12l3-6 2.5 3 2-5 2.5 8"/>'), visible: () => inData() && shown("#np-chrom"), run: () => clickEl("#np-chrom") },
    { id: "spec", bar: false, group: "graphs", ic: () => IC('<path d="M3 13V8M6 13V4M9 13V9M12 13V6"/>'), visible: () => inData() && shown("#np-spec"), run: () => clickEl("#np-spec") },
    { id: "map", bar: false, group: "graphs", ic: () => IC('<rect x="2.5" y="2.5" width="11" height="11" rx="1.4"/><path d="M2.5 8h11M8 2.5v11"/>'), visible: () => inData() && shown("#np-map"), run: () => clickEl("#np-map") },
    { id: "xic", bar: false, group: "graphs", ic: () => IC('<path d="M2 13h12"/><path d="M3 12c2-7 3-7 5 0s3 3 5-3"/>'), visible: () => inData() && shown("#np-xic"), run: () => clickEl("#np-xic") },
    { id: "mrm", bar: false, group: "graphs", ic: () => IC('<path d="M2 13h12M4 13V9M7 13V4M10 13V7"/>'), visible: () => inData() && shown("#np-mrm"), run: () => clickEl("#np-mrm") },
    { id: "closeall", bar: false, group: "graphs", ic: () => IC('<path d="M4 4l8 8M12 4l-8 8"/>'), visible: () => inData(), run: () => clickEl("#np-closeall") },
    { id: "sideL", bar: false, group: "panel", ic: () => ICONS.left, visible: () => inData(), run: () => setSide("left") },
    { id: "sideR", bar: false, group: "panel", ic: () => ICONS.right, visible: () => inData(), run: () => setSide("right") },
    { id: "panel", bar: false, group: "panel", ic: () => ICONS.panel, visible: () => inData(), run: () => toggleDrawer() },
    // search: true = found only by the search (palette, Ctrl/⌘ K): never a button of the bar, never in "Altro"; external links, rare: in "Altro"
    { id: "stat", search: true, group: "tools", ic: () => ICONS.stat, panel: "stat", visible: () => inData(), run: () => toggle("stat") },
    { id: "pubchem", dom: "np-pc", more: true, ic: () => ICONS.ext, run: () => window.open("https://pubchem.ncbi.nlm.nih.gov/", "_blank", "noopener") },
    { id: "bio", dom: "np-bt", more: true, ic: () => ICONS.ext, run: () => window.openBT && window.openBT() }
  ];
  // the commands of the 2D map (the list lives in mappa.js, one for the menu, the bar on the map and here): only in the palette
  if (window.MAPPA) MAPPA.CMDS.filter(c => c.kinds.includes("2d")).forEach(c => CMD.push({ id: c.id, bar: false, group: "graphs", ic: () => MAPPA.SV(MAPPA.ICO[c.ic]), visible: () => inData() && MAPPA.paletteOk(c), run: () => MAPPA.paletteRun(c.id) }));
  // the two clean slates (reset.js): a short question says what is lost; also in the gear
  const IC_RESET = () => IC('<path d="M3 8a5 5 0 109-2.8"/><path d="M12.5 2v3.5H9"/>');
  CMD.push({ id: "resetall", bar: false, group: "app", ic: IC_RESET, visible: () => !!window.QQRESET, run: () => QQRESET.ask("all") }, { id: "resetfiles", bar: false, group: "app", ic: IC_RESET, visible: () => !!window.QQRESET, run: () => QQRESET.ask("files") });
  CMD.forEach(c => { if (c.search) { c.bar = false; delete c.more; } });      // a command found only by the search lives in no bar and no menu
  /** @type {(id: any) => any} */
  const byId = id => CMD.find(c => c.id === id);
  const clickEl = sel => { const e = Q1(sel); if (e) e.click(); };
  const shown = sel => { const e = Q1(sel); return !!e && !e.hidden && e.offsetParent !== null; };
  const visible = c => (!c.feature || !!FEATURES[c.feature]) && (c.visible ? !!c.visible() : true) && (!c.hr || (isHrMode() && inData())) && (!c.item || visible(byId(c.item)));
  const lab = c => (c.teo ? c.etichetta : t(`cmd.${c.id}.lab`));
  const name = c => (c.teo ? c.etichetta : t(`cmd.${c.id}.name`));
  const tip = c => t(`cmd.${c.id}.tip`);

  // ---------------------------------------------------------------- state
  function loadState() {
    let s = {};
    s = lsObj(storageKey());
    _w = typeof s.w === "number" && isFinite(s.w) && s.w >= MIN_W && s.w <= MAX_W ? Math.round(s.w) : defaultW();
    _tab = validTab(s.tab) ? s.tab : "file";
    _open = s.open === undefined ? innerWidth >= AUTO_W : s.open === true;      // first visit: the panel (File list) is open, except on a narrow screen where it would replace the workspace
    if (s.last && typeof s.last === "object") {
      if (["hdr", "lst", "fil"].includes(s.last.scan)) _last.scan = s.last.scan;
      if (["cmp", "iso"].includes(s.last.form)) _last.form = s.last.form;
    }
    let side = null;
    try { side = localStorage.getItem(K_SIDE); } catch (_) { /* none */ }
    _side = side === "right" || side === "left" ? side : s.side === "right" ? "right" : "left";
    _restore = _forced = false; autoNarrow();
  }

  // the narrow window hides the panel before anything else; widening gives it back (the saved state never changes)
  function autoNarrow() {
    if (window.innerWidth < AUTO_W) { if (_open && !_forced) { _open = false; _restore = true; } }
    else { _forced = false; if (_restore) { _open = true; _restore = false; } }
  }

  function saveState() {
    try {
      localStorage.setItem(storageKey(), JSON.stringify({ tab: _tab, open: _open || _restore, w: _w, last: _last }));
      localStorage.setItem(K_SIDE, _side);
    } catch (_) { /* no storage */ }
  }

  // ---------------------------------------------------------------- layout of the drawer
  function applyWidth() {
    document.documentElement.style.setProperty("--sb-w", effW() + "px");
  }

  // one redraw per frame, whatever fires it: the workspace changes width when the panel shows, hides or is resized
  let _refitRaf = 0, _lastW = -1;
  function paintFit() {
    const dp = Q1("#dpanels");
    if (dp) _lastW = dp.clientWidth;
    if (dp && _lastW < 100) return;                // the workspace is hidden (sheet) or not shown yet: nothing to draw
    if (typeof fitWidth === "function" && typeof redrawAll === "function") { fitWidth(); redrawAll(); }
  }
  function refit() {
    if (_refitRaf) return;
    _refitRaf = FRAME.next(() => { _refitRaf = 0; paintFit(); });
  }
  // a layout change that is animated (transizioni.js) is refitted at once, in the same frame: one redraw, then the panels slide
  function refitNow() {
    if (_refitRaf) { FRAME.cancel(_refitRaf); _refitRaf = 0; }
    paintFit();
  }
  // The panels' own observers read a width change that is not theirs (the page narrowed under them) as «resized by hand» and
  // unlock them from the page width: the full-width panels are noted before the layout moves and given back once it has settled.
  let _fullSnap = null, _fullTimer = 0;
  function snapFull() {
    if (typeof E === "undefined" || !E.panels) return;
    if (!_fullSnap) _fullSnap = E.panels.filter(p => p.full);
    clearTimeout(_fullTimer);
    _fullTimer = setTimeout(() => {
      const snap = _fullSnap || [];
      _fullSnap = null;
      let back = false;
      snap.forEach(p => { if (!p.full && E.panels.includes(p)) { p.full = true; back = true; } });
      const dp = Q1("#dpanels");
      if (back || snap.length && dp && dp.clientWidth !== _lastW) refit();
    }, 250);
  }
  function watchWorkspace() {
    const dp = Q1("#dpanels");
    if (!dp || typeof ResizeObserver === "undefined") return;
    FRAME.ro(dp, () => { if (dp.clientWidth !== _lastW) { snapFull(); refit(); } });
  }

  function applyLayout() {
    const v = Q1("#v-data"), aside = Q1("#dfiles");
    if (!v || !aside) return;
    const shown = _open && inData();
    const was = v.classList.contains("sb-open");
    const moved = shown && was && (_side === "right") !== v.classList.contains("sb-right");
    const change = shown !== was || shown && sheet() !== v.classList.contains("sb-sheet") || moved;
    if (change) snapFull();
    const apply = () => {
      v.classList.toggle("sb-right", _side === "right");
      v.classList.toggle("sb-open", shown);
      v.classList.toggle("sb-sheet", shown && sheet());
      aside.toggleAttribute("inert", !shown);
      aside.setAttribute("aria-hidden", shown ? "false" : "true");
      if (change) { applyWidth(); refitNow(); }
    };
    if (change && inData() && window.TRANS) TRANS.layout({ aside, was, shown, moved }, apply); else apply();
    const tg = Q1("#sb-toggle");
    if (tg) {
      tg.hidden = !inData();
      tg.setAttribute("aria-pressed", shown ? "true" : "false");
      tg.classList.toggle("on", shown);
      const tl = tg.querySelector(".tl"); if (tl) { tl.dataset.i18n = shown ? "cmd.panel.hide" : "cmd.panel.show"; tl.textContent = I18N.t(tl.dataset.i18n); }
    }
    const h = Q1("#sidebar-resize-handle");
    if (h) { h.setAttribute("aria-valuemin", MIN_W); h.setAttribute("aria-valuemax", maxW()); h.setAttribute("aria-valuenow", effW()); }
    applyWidth();
  }

  function setPanelVisible(id) {
    const map = { file: "#sb-panel-file", calc: "#sb-panel-calc", losses: "#sb-panel-losses", adducts: "#sb-panel-adducts", lists: "#sb-panel-lists", qaqc: "#sb-panel-qaqc", stat: "#sb-panel-stat" };
    Object.values(map).forEach(sel => { const p = Q1(sel); if (p) p.hidden = true; });
    const hr = Q1("#hrinfo");
    if (hr) hr.hidden = true;
    if (map[id]) {
      const p = Q1(map[id]);
      if (p) p.hidden = false;
    } else if (HR_PANELS.includes(id) && hr) {
      hr.hidden = false;
    }
  }

  let _painted = null;                 // the tool whose body was last built (a state restored as open paints it when the data arrive)
  function renderPane(id, opt = {}) {
    _painted = id;
    const c = byId(id);
    const ttl = Q1("#sb-title");
    if (ttl && c) ttl.textContent = name(c);
    const sbHelp = Q1("#sb-help");
    if (sbHelp) {
      const helpKeys = { losses: "losses", adducts: "adducts", lists: "liste" };
      sbHelp.style.display = helpKeys[id] ? "" : "none";
      sbHelp.dataset.help = helpKeys[id] || "";
    }
    const addf = Q1("#addf");
    if (addf) addf.style.display = id === "file" ? "" : "none";
    setPanelVisible(id);
    if (HR_PANELS.includes(id)) {
      if (typeof BANCO !== "undefined") {
        if (typeof BANCO.infoBar === "function") BANCO.infoBar();
        if (typeof BANCO.setInfoTab === "function") BANCO.setInfoTab(id);
        setPanelVisible(id);
        if (typeof BANCO.renderInfo === "function") BANCO.renderInfo();
      }
      return;
    }
    if (id === "calc") {
      dockCalc(true);
      setTimeout(() => {
        const inp = Q1("#calcin");
        if (inp && _open && _tab === "calc") inp.focus();
        if (typeof calcRun === "function") calcRun();
      }, 30);
    } else if (id === "losses") ensureLossesMounted(opt);
    else if (id === "adducts") ensureAdductsMounted();
    else if (id === "lists") ensureListsMounted(opt);
    else if (id === "stat") { const pnl = Q1("#sb-panel-stat"); if (pnl && window.STAT) STAT.mount(pnl); }
    else if (id === "qaqc") { const pnl = Q1("#sb-panel-qaqc"); if (pnl && window.QAQC) QAQC.mount(pnl); }      // painted again at every opening: the files may have changed
  }

  function ensureLossesMounted(opt = {}) {
    const pnl = Q1("#sb-panel-losses");
    if (!pnl) return;
    if (!pnl.dataset.mounted) {
      if (window.QQQRef && typeof QQQRef.mountLosses === "function") {
        QQQRef.mountLosses(pnl, opt);
        pnl.dataset.mounted = "1";
      }
    } else if (opt.q != null && window.QQQRef && typeof QQQRef.setLossQuery === "function") {
      QQQRef.setLossQuery(opt.q);
    }
  }

  function ensureAdductsMounted() {
    const pnl = Q1("#sb-panel-adducts");
    if (!pnl || pnl.dataset.mounted) return;
    if (window.QQQRef && typeof QQQRef.mountAdducts === "function") {
      QQQRef.mountAdducts(pnl);
      pnl.dataset.mounted = "1";
    }
  }

  function ensureListsMounted(opt = {}) {
    const pnl = Q1("#sb-panel-lists");
    if (!pnl) return;
    // mounted again at every opening (as the old window was): a list added meanwhile shows up
    if (window.LISTE && typeof LISTE.mount === "function") {
      LISTE.mount(pnl, opt);
      pnl.dataset.mounted = "1";
    }
  }

  // Dock or undock #calcdlg
  function dockCalc(inSidebar) {
    const dlg = Q1("#calcdlg");
    const pnl = Q1("#sb-panel-calc");
    if (!dlg || !pnl) return;
    if (inSidebar) {
      if (dlg.parentElement !== pnl) pnl.appendChild(dlg);
      dlg.classList.add("sb-docked");
      dlg.hidden = false;
    } else {
      if (dlg.parentElement !== document.body) document.body.appendChild(dlg);
      dlg.classList.remove("sb-docked");
      dlg.hidden = true;                      // outside the Dati view it is a window the student opens with its button
    }
  }

  // ---------------------------------------------------------------- open / close
  function openerFor(id) {
    const c = byId(id);
    if (!c) return null;
    const root = Q1("#stbar");
    if (!root) return null;
    const own = c.item ? byId(c.item) : c;
    const el = root.querySelector(`[data-cmd="${own.id}"]`);
    if (el && el.offsetParent !== null) return el.classList.contains("tb-split") ? el.querySelector(".tb-main") : el;
    return root.querySelector('[data-cmd="more"]');
  }

  function show(id, opt = {}) {
    hintClose(true);
    if (id === "ptable") { if (window.QQQRef) QQQRef.open("pt"); return false; }
    if (!inData() || !validTab(id)) return false;
    const wasOpen = _open;
    _tab = id;
    _open = true; _restore = false; _forced = innerWidth < AUTO_W;
    if (!_opener || !wasOpen) _opener = opt.opener || openerFor(id);
    renderPane(id, opt);
    applyLayout();
    updateButtons();
    saveState();
    if (!opt.noFocus) {
      setTimeout(() => {
        const aside = Q1("#dfiles");
        if (_open && aside && !aside.contains(document.activeElement)) {
          const pane = Q1("#sb-pane");
          if (pane) pane.focus({ preventScroll: true });
        }
      }, 90);
    }
    return true;
  }

  function toggle(id) {
    if (_open && _tab === id) close(true);
    else show(id);
  }

  function close(restoreFocus) {
    if (!_open) return;
    _open = false; _restore = false;
    applyLayout();
    updateButtons();
    saveState();
    if (restoreFocus && !hintSeen()) hintShow();      // the first time the user hides the tools
    const o = _opener;
    _opener = null;
    if (restoreFocus && o && document.contains(o) && o.offsetParent !== null) o.focus({ preventScroll: true });
    else if (restoreFocus) {
      const rov = Q1("#stbar .tb[tabindex='0']");
      if (rov) rov.focus({ preventScroll: true });
    }
  }

  function pick(group, id) {
    _last[group] = id;
    const ok = show(id);
    if (ok) buildBar();
    return ok;
  }

  // the header button: show the last tool used (first start: File) or hide the panel
  function toggleDrawer() {
    if (_open && inData()) close(true);
    else show(validTab(_tab) ? _tab : "file", { opener: Q1("#sb-toggle") });
  }

  function setWidth(w) {
    _w = Math.max(MIN_W, Math.min(maxW(), Math.round(w)));
    applyLayout();
    saveState();
  }

  function setSide(side, opt = {}) {
    if (side !== "left" && side !== "right") return;
    const changed = _side !== side;
    _side = side;
    applyLayout();
    saveState();
    if (changed && opt.announce !== false) announce(t(side === "left" ? "side.movedLeft" : "side.movedRight"));
  }
  function toggleSide() { setSide(_side === "left" ? "right" : "left"); }

  function announce(msg) {
    let a = Q1("#sb-live");
    if (!a) {
      a = document.createElement("div");
      a.id = "sb-live";
      a.setAttribute("role", "status");
      a.setAttribute("aria-live", "polite");
      a.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap";
      document.body.appendChild(a);
    }
    a.textContent = "";
    setTimeout(() => { a.textContent = msg; }, 30);
  }

  // ---------------------------------------------------------------- toolbar
  function btnHtml(c, extra = "") {
    const dom = c.dom ? ` id="${c.dom}"` : "";
    return `<button type="button" class="tb ${extra}"${dom}${MAIN.has(c.id) ? " data-main" : ""} data-cmd="${c.id}" title="${EHs(tip(c))}" aria-label="${EHs(name(c))}"><span class="ti">${c.ic()}</span><span class="tl">${EHs(lab(c))}</span></button>`;
  }
  const EHs = s => String(s).replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");

  function splitHtml(c) {
    const last = byId(_last[c.id]);
    const full = `${name(c)}: ${name(last)}`;
    return `<span class="tb-split" data-cmd="${c.id}" role="presentation">` +
      `<button type="button" class="tb tb-main"${MAIN.has(c.id) ? " data-main" : ""} data-cmd-main="${c.id}" title="${EHs(full)}" aria-label="${EHs(full)}"><span class="ti">${last.ic()}</span><span class="tl">${EHs(lab(c))}</span></button>` +
      `<button type="button" class="tb tb-car" data-cmd-car="${c.id}" aria-haspopup="menu" aria-expanded="false" title="${EHs(name(c))}" aria-label="${EHs(name(c))}"><span class="ti">${ICONS.caret}</span></button></span>`;
  }

  // the commands of the Teoria as the bar and the palette use them: {id, teo, etichetta, ic, run, items}
  function theoryCmds() {
    const T = tcmd(); if (!T) return [];
    return T.lista.map(c => ({ id: c.id, teo: true, end: c.gruppo === "fine", group: "tools", etichetta: c.etichetta, titolo: c.titolo || c.etichetta, chiave: c.chiave, off: !!c.disabilitato,
      ic: () => T.icona(c.icona), run: () => c.run(), items: c.figli && c.figli.map(f => ({ id: f.id, teo: true, group: "tools", etichetta: f.etichetta, ic: () => T.icona(c.icona), run: () => f.run() })) }));
  }
  const theoryAll = () => theoryCmds().flatMap(c => (c.items ? c.items : [c]));       // the palette finds the tools one by one
  function buildTheoryBar(bar) {
    const sep = '<span class="tb-sep" role="separator"></span>';
    const cs = theoryCmds().filter(c => !c.end);
    const gear = theoryCmds().find(c => c.end);
    const one = c => `<button type="button" class="tb" data-tcmd="${c.id}"${c.off ? ' aria-disabled="true"' : ""} title="${EHs(c.titolo)}" aria-label="${EHs(c.etichetta + (c.chiave ? " (" + c.chiave + ")" : ""))}"${c.items ? ' aria-haspopup="menu" aria-expanded="false"' : ""}><span class="ti">${c.ic()}</span><span class="tl">${EHs(c.etichetta)}</span></button>`;
    bar.innerHTML = cs.filter(c => c.id === "prec" || c.id === "succ").map(one).join("") + sep + cs.filter(c => c.id !== "prec" && c.id !== "succ").map(one).join("") + (gear ? sep + one(gear) : "");
    bar.querySelectorAll("button[data-tcmd]").forEach(b => {
      const c = cs.concat(gear || []).find(x => x.id === b.dataset.tcmd);
      b.tabIndex = -1;
      b.onclick = () => {
        if (c.off) return;
        if (c.items) { if (Q1("#stmenu") && Q1("#stmenu")._anchor === b) return closeMenu(true); openMenu(b, c.items.map(f => ({ label: f.etichetta, ic: f.ic(), dom: null, fn: () => f.run() }))); b.setAttribute("aria-expanded", "true"); }
        else c.run();
      };
    });
    setRover(firstItem());
    fit();
  }
  // the bar follows the view: Teoria commands in the Teoria, the data ones elsewhere
  function syncContext() {
    const c = inTheory() ? "theory" : "data";
    document.documentElement.classList.toggle("in-theory", c === "theory");
    if (c === _ctx) return false;
    _ctx = c; closeMenu(false); buildBar();
    return true;
  }

  function buildBar() {
    const bar = Q1("#stbar");
    if (!bar) return;
    if (_ctx === "theory") return buildTheoryBar(bar);
    const hadFocus = bar.contains(document.activeElement) ? /** @type {Element} */ (document.activeElement).getAttribute("data-cmd") || /** @type {Element} */ (document.activeElement).getAttribute("data-cmd-main") || /** @type {Element} */ (document.activeElement).getAttribute("data-cmd-car") : null;
    const common = CMD.filter(c => !c.hr && !c.item && !c.more && c.bar !== false && c.bar !== "end" && (!c.feature || FEATURES[c.feature]));
    const hr = CMD.filter(c => c.hr);
    let h = common.map(c => btnHtml(c)).join("");
    h += `<span class="tb-sep" role="separator" data-hrgrp="1"></span>`;
    h += hr.map(c => (c.items ? splitHtml(c) : btnHtml(c, "hrb"))).join("");
    h += `<span class="tb-sep" role="separator" data-sep2="1"></span>`;
    h += `<button type="button" class="tb" id="tb-more" data-cmd="more" aria-haspopup="menu" aria-expanded="false" title="${EHs(t("cmd.more.name"))}" aria-label="${EHs(t("cmd.more.name"))}"><span class="ti">${ICONS.more}</span><span class="tl">${EHs(t("cmd.more.lab"))}</span></button>`;
    h += CMD.filter(c => c.bar === "end").map(c => btnHtml(c)).join("");
    bar.innerHTML = h;
    bar.querySelectorAll("button[data-cmd]").forEach(b => {
      const c = byId(b.dataset.cmd);
      b.onclick = () => { if (b.dataset.cmd === "more") openMoreMenu(b); else if (c && c.run) c.run(null, b); };
    });
    bar.querySelectorAll("[data-cmd-main]").forEach(b => {
      b.onclick = () => { _opener = null; const c = byId(_last[b.dataset.cmdMain]); toggleItem(c); };
    });
    bar.querySelectorAll("[data-cmd-car]").forEach(b => { b.onclick = () => openGroupMenu(b); });
    bar.querySelectorAll(".tb").forEach(b => { b.tabIndex = -1; });
    syncVisibility();
    updateButtons();
    fit();
    const target = hadFocus && (bar.querySelector(`[data-cmd="${hadFocus}"]`) || bar.querySelector(`[data-cmd-main="${hadFocus}"]`) || bar.querySelector(`[data-cmd-car="${hadFocus}"]`));
    if (target) { setRover(target); target.focus({ preventScroll: true }); }
    else setRover(firstItem());
  }

  function toggleItem(c) {
    if (!c) return;
    if (_open && _tab === c.id) close(true);
    else c.run();
  }

  function syncVisibility() {
    const bar = Q1("#stbar");
    if (!bar || _ctx === "theory") return;
    CMD.forEach(c => {
      if (c.item || c.more || c.bar === false) return;
      const el = bar.querySelector(`[data-cmd="${c.id}"]`);
      if (el) el.hidden = !visible(c) || _dropped.has(c.id);
    });
    const hrOn = isHrMode() && inData();
    bar.querySelector('[data-hrgrp]').hidden = !hrOn;
    const more = bar.querySelector("#tb-more");
    if (more) more.hidden = false;
    const s2 = bar.querySelector("[data-sep2]");
    if (s2) s2.hidden = false;
  }

  function updateButtons() {
    const bar = Q1("#stbar");
    if (!bar || _ctx === "theory") return;
    const open = _open && inData();
    bar.querySelectorAll("button[data-cmd]").forEach(b => {
      const c = byId(b.dataset.cmd);
      if (!c || !c.panel) return;
      const on = open && _tab === c.panel;
      b.classList.toggle("on", on);
      b.setAttribute("aria-pressed", on ? "true" : "false");
    });
    bar.querySelectorAll("[data-cmd-main]").forEach(b => {
      const c = byId(b.dataset.cmdMain);
      const on = open && c.items.includes(_tab);
      b.classList.toggle("on", on);
      b.setAttribute("aria-pressed", on ? "true" : "false");
      const last = byId(_last[c.id]);
      const full = `${name(c)}: ${name(last)}`;
      b.title = full; b.setAttribute("aria-label", full);
      const ti = b.querySelector(".ti");
      if (ti) ti.innerHTML = last.ic();
    });
  }

  // Measured layout: the richest label level that fits (words on all, on the main tools, none); then the least used groups move into
  // "Altro". Never a second row, nothing cut. One measure per frame (schedFit), whatever fires it.
  function setLevel(lv) {
    const bar = Q1("#stbar");
    bar.classList.toggle("lab-all", lv === "all");
    bar.classList.toggle("lab-main", lv === "main");
    document.documentElement.dataset.barLab = lv;
  }
  function schedFit() {
    if (_fitRaf) return;
    _fitRaf = FRAME.next(() => { _fitRaf = 0; fit(); });
  }
  function fit() {
    const bar = Q1("#stbar");
    if (!bar || !bar.offsetParent) return;
    if (_ctx === "theory") {          // words next to the icons while they fit, else the icons alone
      setLevel("all");
      if (bar.scrollWidth > bar.clientWidth) setLevel("none");
      return;
    }
    const t0 = performance.now();
    _dropped = new Set();
    syncVisibility();
    const avail = () => bar.clientWidth;
    const need = () => {
      const kids = [...bar.children].filter(e => !e.hidden);
      const gap = parseFloat(getComputedStyle(bar).columnGap) || 0;
      return kids.reduce((s, e) => { const cs = getComputedStyle(e); return s + e.getBoundingClientRect().width + (parseFloat(cs.marginLeft) || 0) + (parseFloat(cs.marginRight) || 0); }, 0) + Math.max(0, kids.length - 1) * gap;
    };
    const phone = window.innerWidth < PHONE_W;        // very narrow: everything but File goes into "Altro"
    const tryLv = _labMode === "hidden" || phone ? ["none"] : _labMode === "always" ? ["all"] : LEVELS;
    let chosen = tryLv[tryLv.length - 1];
    for (const lv of tryLv) {
      setLevel(lv);
      const slack = LEVELS.indexOf(lv) < LEVELS.indexOf(_lvl) ? HYST : 0;
      if (need() + slack <= avail()) { chosen = lv; break; }
    }
    setLevel(chosen);
    _lvl = chosen;
    let i = 0;
    while ((phone || need() > avail()) && i < DROP_ORDER.length) {
      const id = DROP_ORDER[i++];
      const c = byId(id);
      if (!c || !visible(c)) continue;
      _dropped.add(id);
      syncVisibility();
    }
    if (phone) { _dropped.add("calc"); syncVisibility(); }
    const more = bar.querySelector("#tb-more");
    if (more) more.dataset.dropped = [..._dropped].join(",");
    const rov = bar.querySelector(".tb[tabindex='0']");
    if (!rov || rov.hidden || rov.offsetParent === null) setRover(firstItem());
    const dt = performance.now() - t0;
    _fitStats.n++; _fitStats.last = dt; _fitStats.max = Math.max(_fitStats.max, dt);
  }

  function loadLabMode() {
    let v = null;
    try { v = localStorage.getItem(K_LAB); } catch (_) { /* none */ }
    _labMode = v === "always" || v === "hidden" ? v : "auto";
  }
  function setLabels(mode) {
    _labMode = mode === "always" || mode === "hidden" ? mode : "auto";
    try { localStorage.setItem(K_LAB, _labMode); } catch (_) { /* no storage */ }
    fit();
  }

  // roving tabindex
  function items() { return [...document.querySelectorAll("#stbar .tb")].filter(b => !b.hidden && b.offsetParent !== null); }
  function firstItem() { return items()[0] || null; }
  function setRover(el) {
    document.querySelectorAll("#stbar .tb").forEach(b => { b.tabIndex = b === el ? 0 : -1; });
  }
  function focusToolbar() {
    const bar = Q1("#stbar");
    if (!bar || bar.offsetParent === null) return false;
    _before = document.activeElement && !bar.contains(document.activeElement) ? document.activeElement : _before;
    const el = bar.querySelector(".tb[tabindex='0']:not([hidden])") || firstItem();
    if (!el) return false;
    setRover(el);
    el.focus();
    return true;
  }
  // back to where the focus was before the bar (the body cannot take focus: then the bar just lets go of it)
  function leaveToolbar() {
    if (_before && _before !== document.body && document.contains(_before)) _before.focus();
    else if (document.activeElement && /** @type {HTMLElement} */ (document.activeElement).blur) /** @type {HTMLElement} */ (document.activeElement).blur();
  }
  function onBarKey(e) {
    const bar = Q1("#stbar");
    if (!bar || !bar.contains(e.target)) return;
    const its = items();
    const i = its.indexOf(e.target.closest(".tb"));
    if (i < 0) return;
    let j = null;
    if (e.key === "ArrowRight") j = (i + 1) % its.length;
    else if (e.key === "ArrowLeft") j = (i - 1 + its.length) % its.length;
    else if (e.key === "Home") j = 0;
    else if (e.key === "End") j = its.length - 1;
    else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const b = its[i];
      if (b.hasAttribute("aria-haspopup")) { e.preventDefault(); b.click(); }
      else if (b.dataset.cmdMain) { e.preventDefault(); const car = bar.querySelector(`[data-cmd-car="${b.dataset.cmdMain}"]`); if (car) car.click(); }
      return;
    } else if (e.key === "Escape") {
      e.preventDefault(); leaveToolbar();
      return;
    }
    if (j == null) return;
    e.preventDefault();
    setRover(its[j]);
    its[j].focus();
  }

  // ---------------------------------------------------------------- menus (groups, "Altro")
  function closeMenu(restore) {
    const m = Q1("#stmenu");
    if (!m) return;
    const a = /** @type {any} */ (m)._anchor;
    m.remove();
    if (a) a.setAttribute("aria-expanded", "false");
    if (restore && a && document.contains(a)) a.focus({ preventScroll: true });
  }

  function openMenu(anchor, rows) {
    closeMenu(false);
    const m = document.createElement("div");
    m.id = "stmenu";
    m.setAttribute("role", "menu");
    m.setAttribute("aria-label", anchor.getAttribute("aria-label") || "");
    /** @type {any} */ (m)._anchor = anchor;
    rows.forEach(r => {
      if (r.sep) { const hr = document.createElement("hr"); hr.setAttribute("role", "separator"); m.appendChild(hr); return; }
      if (r.head) { const h = document.createElement("div"); h.className = "mh"; h.setAttribute("role", "presentation"); h.textContent = r.head; m.appendChild(h); return; }
      const b = document.createElement("button");
      b.type = "button";
      b.setAttribute("role", r.checked != null ? "menuitemradio" : "menuitem");
      if (r.checked != null) b.setAttribute("aria-checked", r.checked ? "true" : "false");
      if (r.dom) b.id = r.dom;
      b.tabIndex = -1;
      b.innerHTML = `<span class="mi">${r.ic || ""}</span><span>${EHs(r.label)}</span>`;
      if (r.title) b.title = r.title;
      b.onclick = () => { closeMenu(false); r.fn(); };
      m.appendChild(b);
    });
    document.body.appendChild(m);
    anchor.setAttribute("aria-expanded", "true");
    const ar = anchor.getBoundingClientRect(), mw = m.offsetWidth, mh = m.offsetHeight;
    m.style.top = Math.min(Math.round(ar.bottom + 4), Math.max(8, innerHeight - mh - 8)) + "px";
    m.style.left = Math.max(8, Math.min(innerWidth - mw - 8, ar.right - mw > 8 && ar.left + mw > innerWidth - 8 ? ar.right - mw : ar.left)) + "px";
    const btns = [...m.querySelectorAll("button")];
    (m.querySelector("[aria-checked=true]") || btns[0]).focus();
    m.addEventListener("keydown", e => {
      const k = btns.indexOf(document.activeElement);
      if (e.key === "ArrowDown") { e.preventDefault(); btns[(k + 1) % btns.length].focus(); }
      else if (e.key === "ArrowUp") { e.preventDefault(); btns[(k - 1 + btns.length) % btns.length].focus(); }
      else if (e.key === "Home") { e.preventDefault(); btns[0].focus(); }
      else if (e.key === "End") { e.preventDefault(); btns[btns.length - 1].focus(); }
      else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); closeMenu(true); }
      else if (e.key === "Tab") { closeMenu(false); }
      else if (e.key === "ArrowLeft" || e.key === "ArrowRight") { e.preventDefault(); const a = /** @type {any} */ (m)._anchor; closeMenu(true); onBarKey({ target: a, key: e.key, preventDefault() {} }); }
    });
  }

  function groupRows(c) {
    return c.items.map(id => {
      const it = byId(id);
      return { label: name(it), ic: it.ic(), checked: _last[c.id] === id, dom: null, fn: () => it.run() };
    });
  }
  function openGroupMenu(anchor) {
    if (Q1("#stmenu") && Q1("#stmenu")._anchor === anchor) return closeMenu(true);
    const c = byId(anchor.dataset.cmdCar);
    openMenu(anchor, groupRows(c));
  }
  function openMoreMenu(anchor) {
    if (Q1("#stmenu") && Q1("#stmenu")._anchor === anchor) return closeMenu(true);
    const rows = [];
    const gone = [...CMD.filter(c => !c.item && !c.more && !c.hr), ...CMD.filter(c => c.hr)].filter(c => _dropped.has(c.id));
    gone.forEach(c => {
      if (c.items) {
        rows.push({ head: name(c) });
        c.items.forEach(id => { const it = byId(id); rows.push({ label: name(it), ic: it.ic(), fn: () => { _opener = anchor; it.run(); } }); });
      } else {
        rows.push({ label: name(c), ic: c.ic(), fn: () => { _opener = anchor; if (c.run) c.run(); } });
      }
    });
    if (gone.length) rows.push({ sep: true });
    CMD.filter(c => c.more).forEach(c => rows.push({ label: name(c), ic: c.ic(), dom: c.dom, title: tip(c), fn: () => c.run() }));
    openMenu(anchor, rows);
  }

  // ---------------------------------------------------------------- resizer (width of the drawer)
  function bindResizer() {
    const handle = Q1("#sidebar-resize-handle");
    if (!handle) return;
    let dragging = false;
    const onMove = e => {
      if (!dragging) return;
      const pane = Q1("#sb-pane");
      if (!pane) return;
      const rect = pane.getBoundingClientRect();
      _w = Math.max(MIN_W, Math.min(maxW(), Math.round(_side === "right" ? rect.right - e.clientX : e.clientX - rect.left)));
      applyLayout();
    };
    const onUp = e => {
      if (!dragging) return;
      dragging = false;
      document.body.classList.remove("sb-resizing");
      try { handle.releasePointerCapture(e.pointerId); } catch (_) { /* gone */ }
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      saveState();
    };
    handle.onpointerdown = e => {
      if (e.button !== 0) return;
      dragging = true;
      document.body.classList.add("sb-resizing");
      try { handle.setPointerCapture(e.pointerId); } catch (_) { /* none */ }
      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
      e.preventDefault();
    };
    handle.ondblclick = () => {
      _w = defaultW();
      applyLayout();
      saveState();
    };
    handle.onkeydown = e => {
      const step = e.shiftKey ? 48 : 16;
      let w = null;
      if (e.key === "ArrowLeft") w = effW() + (_side === "right" ? step : -step);
      else if (e.key === "ArrowRight") w = effW() + (_side === "right" ? -step : step);
      else if (e.key === "Home") w = MIN_W;
      else if (e.key === "End") w = maxW();
      else if (e.key === "Enter") w = defaultW();
      if (w == null) return;
      e.preventDefault();
      setWidth(w);
    };
  }

  // ---------------------------------------------------------------- side: drag the header to the other half of the window
  function bindSideDrag() {
    const head = Q1("#sb-header");
    if (!head) return;
    /** @type {any} */
    let st = null;
    const zones = () => {
      let l = Q1(".sb-dz.l"), r = Q1(".sb-dz.r"), pv = Q1(".sb-dz-prev");
      if (!l) {
        l = document.createElement("div"); l.className = "sb-dz l"; l.textContent = t("side.dropLeft");
        r = document.createElement("div"); r.className = "sb-dz r"; r.textContent = t("side.dropRight");
        pv = document.createElement("div"); pv.className = "sb-dz-prev";
        [l, r, pv].forEach(z => { z.setAttribute("aria-hidden", "true"); document.body.appendChild(z); });
      }
      return { l, r, pv };
    };
    const hot = x => {
      const z = zones(), side = x < innerWidth / 2 ? "left" : "right";
      z.l.classList.toggle("hot", side === "left");
      z.r.classList.toggle("hot", side === "right");
      const w = sheet() ? innerWidth : effW();
      z.pv.style.width = w + "px";
      z.pv.style.left = side === "left" ? "0" : "auto";
      z.pv.style.right = side === "right" ? "0" : "auto";
      return side;
    };
    const end = () => {
      document.body.classList.remove("sb-dragging");
      document.querySelectorAll(".sb-dz,.sb-dz-prev").forEach(z => z.remove());
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
      if (st && st.timer) clearTimeout(st.timer);
      st = null;
    };
    const begin = () => {
      st.active = true;
      document.body.classList.add("sb-dragging");
      try { head.setPointerCapture(st.id); } catch (_) { /* gone */ }
      hot(st.x);
    };
    const move = e => {
      if (!st || e.pointerId !== st.id) return;
      st.x = e.clientX;
      if (!st.active) {
        const d = Math.hypot(e.clientX - st.x0, e.clientY - st.y0);
        if (st.touch) { if (d > 10) end(); return; }       // a finger that moves before the long press is a scroll
        if (d < 6) return;
        begin();
      }
      e.preventDefault();
      hot(e.clientX);
    };
    const up = e => {
      if (!st || e.pointerId !== st.id) return;
      const was = st.active, side = was ? (e.clientX < innerWidth / 2 ? "left" : "right") : null;
      end();
      if (was && side) setSide(side);
    };
    const cancel = () => end();
    head.addEventListener("pointerdown", e => {
      if (e.button !== 0 || e.target.closest("button,input,a,select")) return;
      st = { id: e.pointerId, x0: e.clientX, y0: e.clientY, x: e.clientX, active: false, touch: e.pointerType === "touch" || e.pointerType === "pen" };
      window.addEventListener("pointermove", move, { passive: false });
      window.addEventListener("pointerup", up);
      window.addEventListener("pointercancel", cancel);
      if (st.touch) st.timer = setTimeout(() => { if (st) { begin(); if (navigator.vibrate) { try { navigator.vibrate(15); } catch (_) { /* none */ } } } }, 380);
    });
    head.addEventListener("contextmenu", e => { if (st && st.touch) e.preventDefault(); });
  }

  // ---------------------------------------------------------------- global events
  function bindGlobalEvents() {
    document.addEventListener("keydown", e => {
      // toolbar from the keyboard: F6 or Alt+0 (Esc goes back where the focus was)
      if (e.key === "F6" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        const bar = Q1("#stbar");
        if (bar && bar.contains(document.activeElement)) {
          const pane = Q1("#sb-pane");
          if (_open && pane && !e.shiftKey) { e.preventDefault(); pane.focus(); return; }
          e.preventDefault(); leaveToolbar(); return;
        }
        if (focusToolbar()) e.preventDefault();
        return;
      }
      if ((e.code === "Digit0" || e.key === "0") && e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
        if (focusToolbar()) e.preventDefault();
        return;
      }
      // side of the drawer from the keyboard
      if (e.altKey && e.shiftKey && !e.ctrlKey && !e.metaKey && (e.key === "ArrowLeft" || e.key === "ArrowRight") && inData()) {
        e.preventDefault(); e.stopPropagation();
        setSide(e.key === "ArrowLeft" ? "left" : "right");
        return;
      }
      if (e.key !== "Escape" || e.defaultPrevented || /** @type {any} */ (e)._ctxWas) return;
      if (Q1("#stmenu") || (window.PALETTE && PALETTE.isOpen)) return;
      const calcin = Q1("#calcin");
      if (calcin && document.activeElement === calcin && calcin.value.trim().length > 0) return;      // Esc first clears the field
      if (document.querySelector("dialog[open]") || Q1("#helppop:not([hidden])") || Q1("#uipset") || (Q1("#ctx") && !Q1("#ctx").hidden)) return;
      if (_open && sheet() && inData()) { e.preventDefault(); close(true); }       // the sheet replaces the workspace: Esc gives it back
    }, true);

    // show/hide the side panel from the keyboard
    document.addEventListener("keydown", e => {
      if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && (e.code === "KeyB") && inData()) {
        e.preventDefault(); e.stopPropagation();
        toggleDrawer();
      }
    }, true);

    document.addEventListener("keydown", onBarKey);

    document.addEventListener("pointerdown", e => {
      const m = Q1("#stmenu");
      if (m && !m.contains(e.target) && !(m._anchor && m._anchor.contains(e.target))) closeMenu(false);
    }, true);

    window.addEventListener("resize", () => {
      const o = _open; autoNarrow();
      if (_open !== o) { if (_open && validTab(_tab) && _painted !== _tab) renderPane(_tab); updateButtons(); }
      applyLayout();
      schedFit();
    });
    let wasData = false;
    const tf = Q1("#tframe");
    if (tf) tf.addEventListener("load", () => { if (_ctx === "theory") { buildBar(); if (!tcmd()) setTimeout(() => _ctx === "theory" && buildBar(), 400); } });
    document.addEventListener("tpview", () => {
      syncContext();
      const sc = Q1("#sb-panels");
      if (wasData && !inData()) { _away = { open: _open, top: sc ? sc.scrollTop : 0 }; _open = false; }        // leaving Dati hides the panel, remembering how it was (open or shut, scroll)
      else if (!wasData && inData() && _away) {                                                          // back in Dati: the panel returns as it was left (width and tool are kept in _w / _tab)
        _open = _away.open; _forced = false; _restore = false; autoNarrow();
        const top = _away.top; _away = null;
        FRAME.next(() => { const e = Q1("#sb-panels"); if (e) e.scrollTop = top; });
      }
      wasData = inData();
      syncMode();
      if (_open && inData() && validTab(_tab) && _painted !== _tab) renderPane(_tab);
      applyLayout();
      syncVisibility();
      updateButtons();
      fit();
    });
  }

  // LR / HR: the tools of the HR world appear when a high-resolution file is open
  function syncMode() {
    const m = isHrMode() ? "hr" : "lr";
    if (m === _mode) { syncVisibility(); return; }
    const prevOpen = _open;
    _mode = m;
    loadState();
    applyWidth();
    syncVisibility();
    if (_open && validTab(_tab)) renderPane(_tab);
    else if (prevOpen && !_open) { /* the other world remembers its own state */ }
    applyLayout();
    refit();
    updateButtons();
    fit();
  }

  // Hint anchored under the toolbar: shown only the first time the user hides the tools (marked seen at once, so never again), closes on Esc, ×, the first tool opened or after 10 s.
  let _hintMem = false, _hintTimer = 0, _hintRo = null;
  function hintSeen() { try { return !!localStorage.getItem(K_HINT); } catch (_) { return _hintMem; } }
  function hintMark() { _hintMem = true; try { localStorage.setItem(K_HINT, "1"); } catch (_) { /* none */ } }
  function hintPlace() {
    const el = document.getElementById("bar-hint"), bar = Q1("#stbar"), tg = Q1("#sb-toggle");
    if (!el || !bar) return;
    const r = bar.getBoundingClientRect(), tr = tg && tg.offsetParent ? tg.getBoundingClientRect() : null;      // the arrow points at the Panel button (else at the toolbar)
    const aim = tr ? tr.left + tr.width / 2 : r.left + 24, w = el.offsetWidth, vw = document.documentElement.clientWidth;
    const left = Math.max(8, Math.min(Math.round(tr ? Math.min(r.left, aim - 28) : r.left), vw - w - 8));
    el.style.top = Math.round(Math.max(r.bottom, tr ? tr.bottom : 0) + 8) + "px";
    el.style.left = left + "px";
    el.style.setProperty("--ha", Math.round(Math.max(10, Math.min(aim - left, w - 20)) - 5) + "px");      // centre of the arrow (10 px wide) on the target
  }
  function hintOnKey(e) { if (e.key === "Escape") hintClose(true); }
  function hintClose(mark) {
    const el = document.getElementById("bar-hint");
    if (!el) return;             // not on screen
    clearTimeout(_hintTimer);
    el.remove();
    document.removeEventListener("keydown", hintOnKey);
    window.removeEventListener("resize", hintPlace);
    document.removeEventListener("fullscreenchange", hintPlace);
    if (_hintRo) { _hintRo(); _hintRo = null; }
    if (mark) hintMark();
  }
  function hintShow() {
    if (document.getElementById("bar-hint") || !inData() || window.QQQ_PHONE) return false;
    const bar = Q1("#stbar");
    if (!bar || bar.offsetParent === null) return false;
    const el = document.createElement("div");
    el.id = "bar-hint";
    el.setAttribute("role", "status");
    el.setAttribute("aria-live", "polite");
    const txt = document.createElement("span");
    txt.textContent = t("hint.testo");
    const x = document.createElement("button");
    x.type = "button"; x.className = "x";      // the shared close X (index.html)
    x.setAttribute("aria-label", t("hint.chiudi"));
    x.onclick = () => hintClose(true);
    el.append(txt, x);
    document.body.appendChild(el);
    hintPlace();
    hintMark();
    _hintTimer = setTimeout(() => hintClose(false), 10000);
    document.addEventListener("keydown", hintOnKey);
    window.addEventListener("resize", hintPlace);
    document.addEventListener("fullscreenchange", hintPlace);
    { const offs = [bar, document.querySelector("header"), el].map(x => x && FRAME.ro(x, hintPlace)); _hintRo = () => offs.forEach(f => f && f()); }      // the header or toolbar change size: the arrow follows
    return true;
  }
  function init() {
    if (_mounted) return;
    _mounted = true;
    _mode = isHrMode() ? "hr" : "lr";
    loadState();
    loadLabMode();
    window.CMD = CMD;
    if (!syncContext()) buildBar();
    const x = Q1("#sb-x");
    if (x) x.onclick = () => close(true);
    const tg = Q1("#sb-toggle");
    if (tg) tg.onclick = () => toggleDrawer();
    const hd = Q1("#sidebar-resize-handle");
    if (hd) { hd.setAttribute("role", "separator"); hd.setAttribute("aria-orientation", "vertical"); hd.tabIndex = 0; hd.setAttribute("aria-label", t("side.resize.aria")); }
    watchWorkspace();
    saveState();                 // writes back only the valid fields: an old state (pinned, odd side or width) is cleaned
    const add = Q1("#addf");
    if (add) add.addEventListener("click", () => { _open = false; });
    bindResizer();
    bindSideDrag();
    bindGlobalEvents();
    dockCalc(true);
    if (typeof ResizeObserver !== "undefined") {
      const bar = Q1("#stbar");
      if (bar) FRAME.ro(bar, () => schedFit());
    }
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(fit);
    if (_open && inData() && validTab(_tab)) renderPane(_tab);
    applyLayout();
    updateButtons();
  }

  return {
    init,
    hintShow,
    hintClose,
    commands: CMD,
    setTab: show,
    show,
    toggle,
    close,
    toggleDrawer,
    setSide,
    toggleSide,
    setWidth,
    dockCalc,
    syncMode,
    syncHR: syncMode,
    fit,
    setLabels,
    get labels() { return _labMode; },
    get labelLevel() { return _lvl; },
    get fitStats() { return _fitStats; },
    focusToolbar,
    isDataView: inData,
    isOpen: id => _open && inData() && (id == null || _tab === id),
    get curTab() { return _tab; },
    get side() { return _side; },
    get width() { return effW(); },
    get open() { return _open && inData(); },
    get sheet() { return _open && inData() && sheet(); },
    get dockedWidth() { return _open && inData() && !sheet() ? effW() : 0; },
    byId,
    name,
    lab,
    visible,
    kw: c => (c.teo ? "" : t(`cmd.${c.id}.kw`)),
    get inTheory() { return _ctx === "theory"; },
    theoryCommands: theoryAll
  };
})();

if (typeof window !== "undefined") {
  window.BARRA = BARRA;
}

if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => BARRA.init());
  } else {
    BARRA.init();
  }
}
