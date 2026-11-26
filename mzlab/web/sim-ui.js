// @ts-check
"use strict";
// SIM files in the interface (engine side: data-channels.md). A SIM file opens as an MRM-kind file whose transitions have a Q1 and no Q3; this module tells the two apart
// (f.sim = number of ions, 0 for MRM), colours overlaid traces by sample and builds the entries of the ion menu of the chromatogram. Only glue: every number comes from the engine.
// Classic helpers used by name: E, J, I18N, PAL, DASHES, fdash, getMrm, shownFor, ctl, draw, uiSave, COMMANDS, addPanel, reveal, openWithUndo, renderFiles, renderFileList, toolbar.
(() => {
  /** @typedef {{ key: string, name: string, sim: boolean }} Ione */
  /** transitions of a file, read once: k|file -> list @type {Map<string, Ione[]>} */
  const LIST = new Map();
  /** a file of an experiment made of transitions (no spectra), the engine's «empty» kind (no experiment) and a panel of transitions: read from ESP, never by name */
  const isTr = (/** @type {any} */ f) => ESP.voce(f.kind)?.spettri === false, isNone = (/** @type {any} */ f) => !ESP.voce(f.kind), panelTr = (/** @type {any} */ p) => ESP.mrm(p), tabTr = (/** @type {any} */ p) => ESP.voce(p.tab)?.spettri === false;
  const keyOf = (/** @type {number} */ k) => `${k}|${E.files[k] ? E.files[k].file : ""}`;
  const trKey = (/** @type {{q1?: any, q3?: any, idx?: number}} */ t) => t.q3 == null ? (t.q1 == null ? `i${t.idx}` : String(t.q1)) : `${t.q1}>${t.q3}`;
  /** the key of a SIM channel from its id: the Q1 m/z, or «i<N>» (its place in the experiment) when the reader gave no Q1 @param {string} id */
  const chKey = id => { const q = /Q1=([\d.]+)/.exec(id), t = /transition=(\d+)/.exec(id); return q ? String(+q[1]) : t ? `i${t[1]}` : ""; };
  /** what a key is called to the user: the m/z, or «ione N» for a channel without Q1 @param {string} key */
  const label = key => /^i\d+$/.test(key) ? I18N.t("canali.sim.ion", { n: +key.slice(1) + 1 }) : key;

  /** the number of SIM ions of a file of the session or of a row of the file table (0: not a SIM file) @param {any} f */
  const n = f => {
    if (!f) return 0;
    if (f.sim != null && f.k != null) return f.sim;
    const e = typeof E !== "undefined" && E.files.find((/** @type {any} */ x) => x.file === (f.file || f.name));
    return e && e.sim || 0;
  };

  /** reads the ions of the MRM-kind and empty files and sets f.sim; the file table and the lists are painted again when something changed @param {any[]} files */
  async function mark(files) {
    let changed = false;
    if (files.some(f => f.pda && isTr(f))) import("./dad-mrm.js");      // the DAD on the MRM panel
    await Promise.all(files.filter(f => isTr(f) || isNone(f)).map(async f => {
      let ions = /** @type {Ione[]} */ ([]);
      try {
        const tr = isTr(f) ? await getMrm(f.k) : [];
        ions = tr.map((/** @type {any} */ t) => ({ key: trKey(t), name: t.name || "", sim: t.q3 == null }));
        if (!ions.length) {
          const c = await CANALI.canali(f.k);
          ions = c.channels.filter((/** @type {any} */ x) => x.family === "sim").map((/** @type {any} */ x) => ({ key: chKey(x.id), name: "", sim: true }));
        }
      } catch (e) { return; }
      LIST.set(keyOf(f.k), ions);
      const s = ions.length && ions.every(x => x.sim) ? ions.length : 0;
      if (f.sim !== s) { f.sim = s; changed = true; }
    }));
    // @ts-ignore toolbar() is also window.toolbar (a BarProp) for lib.dom
    if (changed) { try { renderFiles(); renderFileList(); toolbar(); } catch (e) { /* a screen that is not up yet */ } }
  }

  /** the SIM files a panel draws (the MRM-kind ones among the files it shows) @param {any} p */
  const filesOf = p => shownFor(p).filter((/** @type {any} */ f) => isTr(f));
  /** true when every file the panel draws is a SIM file @param {any} p */
  const panelSim = p => { const l = filesOf(p); return l.length > 0 && l.every((/** @type {any} */ f) => f.sim > 0); };
  /** true when the files of the session that are of the MRM kind are all SIM (the toolbar button says «ions» then) */
  const allSim = () => { const l = E.files.filter((/** @type {any} */ f) => isTr(f) && !f.gone); return l.length > 0 && l.every((/** @type {any} */ f) => f.sim > 0); };

  /** the transitions (or ions) of the MRM-kind files in view, merged by key, in the order of the files: [{ key, name, sim }] @param {any} p */
  function ions(p) {
    /** @type {Map<string, Ione>} */ const m = new Map();
    (panelTr(p) ? filesOf(p) : !tabTr(p) ? [] : E.files.filter((/** @type {any} */ f) => isTr(f) && f.vis && !f.gone)).forEach((/** @type {any} */ f) => (LIST.get(keyOf(f.k)) || []).forEach(x => { if (!m.has(x.key)) m.set(x.key, x); }));
    return sortIons([...m.values()]);
  }
  /** transitions of one precursor keep the order they come in but the quantifier comes first (its name, from the method, starts with «Quant»); the order of the list AND the place of an ion in the palette @param {Ione[]} l */
  function sortIons(l) {
    const rank = (/** @type {Ione} */ x) => /^quant/i.test(x.name) ? 0 : 1, q1 = (/** @type {Ione} */ x) => x.key.split(">")[0], first = new Map(/** @type {[string, number][]} */ (l.map((x, i) => [q1(x), i])).reverse());
    return l.map((x, i) => ({ x, i })).sort((a, b) => (first.get(q1(a.x)) ?? 0) - (first.get(q1(b.x)) ?? 0) || rank(a.x) - rank(b.x) || a.i - b.i).map(o => o.x);
  }
  /** the place of a file among the files of transitions of the session, by its key (never by the order of the file list, so moving a file does not change its dash) @param {any} f */
  const rankOf = f => Math.max(0, E.files.filter((/** @type {any} */ x) => isTr(x) && !x.gone).map((/** @type {any} */ x) => x.k).sort((a, b) => a - b).indexOf(f.k));
  /** colour and line style of a trace of an MRM / SIM overlay. Rule: every ion has ITS OWN colour, PAL[place % PAL.length], where the place is that of the ion among ALL ions of the panel
   * (sortIons order), never among the ticked ones, so ticking, unticking, stacking or moving a file never recolours a trace. Samples are told apart by dash (the sample's rank picks the pattern, the first is solid) and
   * each round of the palette (ion 12, 13... take the colours again) shifts the pattern by one, so (colour, dash) stays distinct for 48 ions. A lone ion keeps the colour of its sample.
   * @param {any} f the file @param {number} ci place of the ion among all of the panel @param {number} nCh ions of the panel @param {number} [rank] sample rank (0 = the swatch of the list: the solid line) */
  function style(f, ci, nCh, rank = rankOf(f)) {
    if (nCh <= 1) return { color: f.color, dash: fdash(f) };
    return { color: PAL[ci % PAL.length], dash: DASHES[(rank + Math.floor(ci / PAL.length)) % DASHES.length] };
  }

  /** CSS background of a trace sample: the colour, or the dash pattern of the trace in that colour (as the legend draws it) @param {string} c @param {number[]} d */
  const paint = (c, d) => d && d.length ? `repeating-linear-gradient(90deg,${c} 0 4px,transparent 4px 7px)` : c;
  /** the line of every channel of a panel as the drawing used it, key -> { c: colour or null (a lone ion of several samples), d: dash }. The drawing writes it from the
   * same style() call as the traces and the lists read it, so a row and its trace cannot differ. True when it changed (the lists must be written again) @param {any} p @param {Record<string, any>} look */
  const keep = (p, look) => { const a = JSON.stringify(look); if (a === p._lookS) return false; p._look = look; p._lookS = a; return true; };
  /** the line of an ion, in the form keep() stores it: its colour (null for a lone ion plotted for several samples: the colour is then the sample's) and the dash of the first sample @param {{ color: string, dash: number[] }} st @param {number} nCh @param {number} nFiles */
  const lookOf = (st, nCh, nFiles) => ({ c: nCh > 1 || nFiles === 1 ? st.color : null, d: st.dash });
  /** the swatch of a row of a list: the line of its trace, dimmed while the row is unticked; the text stays beside it (colour is never the only mark). Nothing known (a channel never drawn): an empty box of the same width
   * @param {any} p @param {string} key @param {boolean} on */
  function swatch(p, key, on) {
    const l = p._look && p._look[key];
    if (!l) return `<span class="trsw none" aria-hidden="true"></span>`;
    const c = l.c || "currentColor";
    return `<span class="trsw${on ? "" : " off"}" aria-hidden="true" data-tc="${EH(l.c || "")}" data-td="${l.d.join(",")}" style="color:${EH(c)};background:${EH(paint(c, l.d))}"></span>`;
  }
  /** a row of a list or a trace under the mouse or the focus: its trace is drawn on, the others fade (cleared with null); one redraw per frame @param {any} p @param {string | null} key */
  function lit(p, key) {
    if ((p._hl ?? null) === key) return;
    p._hl = key; if (p._hlQ) return; p._hlQ = 1;
    FRAME.next(() => { p._hlQ = 0; if (p.el && p.el.isConnected) draw(p); });
  }

  /** the keys the panel plots now (all when it names none) @param {any} p @param {Ione[]} all */
  const picked = (p, all) => { if (!panelTr(p)) return []; const tr = mrmSel(p.tr, all.map(x => x.key)); return !tr ? all.map(x => x.key) : (Array.isArray(tr) ? tr : [tr]); };

  /** the swatch of a row of the «Extract ion ›» submenu, as a CSS background: the line the panel drew for that ion (p._look); a panel that does not draw it yet (a chromatogram that would open an MRM panel) gets the line the panel will draw, from the same style() call with the place of the ion in this list
   * @param {any} p @param {string} key @param {number} i place of the ion @param {number} n ions of the list */
  function lookFor(p, key, i, n) {
    let l = panelTr(p) && p._look && p._look[key];
    if (!l) { const fs = E.files.filter((/** @type {any} */ f) => isTr(f) && f.vis && !f.gone); if (!fs.length) return undefined; l = lookOf(style(fs[0], i, n, 0), n, fs.length); }
    return l;
  }
  const swatchCss = (/** @type {any} */ p, /** @type {string} */ key, /** @type {number} */ i, /** @type {number} */ n) => { const l = lookFor(p, key, i, n); return l ? paint(l.c || "currentColor", l.d) : undefined; };

  /** the entries of the «Extract ion ›» submenu: «All», then one per ion; click = only this one, Ctrl/⌘+click adds or removes it, Shift+click takes the range
   * from the last one clicked @param {any} c the context of the menu @returns {any[]} */
  function rows(c) {
    const p = c.p, all = ions(p), cur = new Set(picked(p, all)), T = (/** @type {string} */ k) => I18N.t(k);
    const set = (/** @type {string[]} */ keys) => {
      if (panelTr(p)) { COMMANDS.run("panel.source", p, () => { p.tr = keys.length === all.length || !keys.length ? "" : keys; }); ctl(p); draw(p); uiSave(); }
      else { reveal(openWithUndo(addPanel("mrm", { imode: "man", intf: "all", intfFixed: true, tr: keys.length === all.length ? "" : keys, title: all.every(x => x.sim) ? "@sim" : undefined }))); }
    };
    const out = [{ name: T("canali.sim.all"), check: cur.size === all.length && all.length > 0, run: () => set(all.map(x => x.key)), aria: T("canali.sim.all") }];
    all.forEach((x, i) => out.push({
      color: swatchCss(p, x.key, i, all.length), tone: (lookFor(p, x.key, i, all.length) || {}).c || "", hover: (/** @type {boolean} */ on) => lit(p, on ? x.key : null),
      name: x.name ? `${label(x.key)} ${x.name}` : label(x.key), check: cur.has(x.key), aria: label(x.key) + (x.name ? " " + x.name : ""),
      // @ts-ignore the event comes from the menu
      run: (/** @type {MouseEvent | undefined} */ ev) => {
        const multi = panelTr(p) && ev && (ev.ctrlKey || ev.metaKey), range = panelTr(p) && ev && ev.shiftKey;
        if (range && p._ionAnc != null) {
          const a = Math.min(p._ionAnc, i), b = Math.max(p._ionAnc, i); set(all.slice(a, b + 1).map(y => y.key)); return;
        }
        p._ionAnc = i;
        if (multi) { const s = new Set(cur); if (s.has(x.key)) s.delete(x.key); else s.add(x.key); set(all.filter(y => s.has(y.key)).map(y => y.key)); return; }
        set([x.key]);
      },
      keep: (/** @type {MouseEvent | undefined} */ ev) => !!(ev && panelTr(p) && (ev.ctrlKey || ev.metaKey || ev.shiftKey)),
    }));
    return out;
  }

  /** the transition table of the «Method» panel of a SIM sample: name, Q1 and dwell only (a SIM ion has no Q3 or collision energy), «—» for a value the file does not give; null for an MRM table @param {any[]} tr */
  function methodTable(tr) {
    if (!tr.length || !tr.every(t => t.q3 == null)) return null;
    const v = (/** @type {any} */ x) => x == null || x === "" ? "—" : String(x);
    return h`<table><tr><th>${I18N.t("method.tr.name")}</th><th class="num">Q1 (m/z)</th><th class="num">Dwell (ms)</th></tr>${raw(tr.map(t => h`<tr><td>${t.name || "—"}</td><td class="num">${v(t.q1)}</td><td class="num">${t.dwell != null ? Math.round(t.dwell * 1000) : "—"}</td></tr>`).join(""))}</table>`;
  }

  /** the panel draws the area under the traces (a tint of the line colour): MRM and SIM overlays, on by default @param {any} p */
  const area = p => panelTr(p) && p.area !== false;

  /** the text of the «Experiment» cell for a SIM file, «SIM · N ions» (also for a file the engine called empty); null for any other file @param {any} f @param {string} kind */
  const cell = (f, kind) => { const c = f ? n(f) : 0; return c > 0 && ESP.voce(kind)?.spettri !== true && kind !== "infusion" ? I18N.t(c === 1 ? "canali.sim.cell1" : "canali.sim.cell", { n: c }) : null; };

  /** the toolbar button of the MRM panel says «+ SIM ions» when the files are SIM */
  function button() {
    const b = document.getElementById("np-mrm"); if (!b) return;
    const k = allSim() ? "canali.sim.add" : "bar.addMrm";
    if (b.dataset.i18n !== k) { b.dataset.i18n = k; b.textContent = I18N.t(k); }
  }

  /** the tint under each trace of the MRM / SIM panels: the line colour at low opacity (a light tint of it in the exported figure) @param {CanvasRenderingContext2D} g @param {any} p @param {any} s @param {number[]} v @param {(y: number) => number} V @param {boolean} exporting @param {(c: string) => string} tint */
  function fill(g, p, s, v, V, exporting, tint) {
    if (!area(p) || v.length < 4) return;
    const b = V(0); g.save(); g.beginPath(); g.moveTo(v[0], b);
    for (let k = 0; k < v.length; k += 2) g.lineTo(v[k], v[k + 1]);
    g.lineTo(v[v.length - 2], b); g.closePath();
    if (exporting) g.fillStyle = tint(s.color); else { g.fillStyle = s.color; g.globalAlpha = 0.22; }
    g.fill(); g.restore();
  }
  /** the saved value of the «Area» toggle of a panel (undefined for the other types) @param {any} p */
  const areaOf = p => panelTr(p) ? p.area !== false : undefined;
  /** the «Area» button of the toolbar of an MRM / SIM panel @param {string} type */
  const areaBtn = type => !ESP.mrm({ type }) ? "" : `<button type="button" class="bt tgl" data-a="area" aria-pressed="true" aria-label="${EH(I18N.t("ctl.area.title"))}" title="${EH(I18N.t("ctl.area.title"))}">${EH(I18N.t("ctl.area"))}</button>`;
  /** @param {any} p */
  function syncArea(p) { const b = p.el && p.el.querySelector('[data-a="area"]'); if (b) { b.classList.toggle("on", p.area !== false); b.setAttribute("aria-pressed", String(p.area !== false)); } }
  /** @param {any} p */
  function wireArea(p) { const b = p.el.querySelector('[data-a="area"]'); if (b) b.onclick = () => { p.area = p.area === false; syncArea(p); draw(p); uiSave(); }; }

  // a file of transitions or SIM ions has no XIC: the same place of the menu holds the ions to extract (a submenu, a check mark each)
  AZIONI.register({ id: "mrm.estrai", group: "xic", views: ["chrom", "mrm"], visible: (/** @type {any} */ c) => c.p.kind !== "pda" && ions(c.p).length > 0,
    name: (/** @type {any} */ c) => I18N.t(ions(c.p).every(x => x.sim) ? "canali.sim.extract" : "canali.tr.extract"), submenu: (/** @type {any} */ c) => rows(c) });

  window.SIMUI = { chKey, label, methodTable, cell, button, fill, areaOf, areaBtn, syncArea, wireArea, n, mark, filesOf, panelSim, allSim, ions, sortIons, rankOf, style, rows, area, trKey, paint, keep, lookOf, swatch, lit };
})();
