// @ts-check
"use strict";
// SIM files in the interface (engine side: canali-dati.md). A SIM file opens as an MRM-kind file whose transitions have a Q1 and no Q3; this module tells the two apart
// (f.sim = number of ions, 0 for MRM), keeps one colour per monitored m/z and builds the entries of the ion menu of the chromatogram. Only glue: every number comes from the engine.
// Classic helpers used by name: E, J, I18N, PAL, getMrm, shownFor, ctl, draw, uiSave, COMANDI, addPanel, reveal, apriAnnullabile, renderFiles, renderFileList, toolbar.
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
    await Promise.all(files.filter(f => isTr(f) || isNone(f)).map(async f => {
      let ions = /** @type {Ione[]} */ ([]);
      try {
        const tr = isTr(f) ? await getMrm(f.k) : [];
        ions = tr.map((/** @type {any} */ t) => ({ key: trKey(t), name: t.name || "", sim: t.q3 == null }));
        if (!ions.length) {
          const c = await CANALI.canali(f.k);
          ions = c.canali.filter((/** @type {any} */ x) => x.fam === "sim").map((/** @type {any} */ x) => ({ key: chKey(x.id), name: "", sim: true }));
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
    // transitions of one precursor keep the file order but the quantifier comes first (its name, from the method, starts with «Quant»)
    const rank = (/** @type {Ione} */ x) => /^quant/i.test(x.name) ? 0 : 1, q1 = (/** @type {Ione} */ x) => x.key.split(">")[0], l = [...m.values()], first = new Map(/** @type {[string, number][]} */ (l.map((x, i) => [q1(x), i])).reverse());
    return l.map((x, i) => ({ x, i })).sort((a, b) => (first.get(q1(a.x)) ?? 0) - (first.get(q1(b.x)) ?? 0) || rank(a.x) - rank(b.x) || a.i - b.i).map(o => o.x);
  }
  /** colour index of an ion: its place among the ions of every file of the session, so the same m/z has the same colour in every sample and panel @param {string} key */
  function colorIdx(key) {
    /** @type {string[]} */ const seen = [];
    E.files.filter((/** @type {any} */ f) => isTr(f) && !f.gone).forEach((/** @type {any} */ f) => (LIST.get(keyOf(f.k)) || []).forEach(x => { if (!seen.includes(x.key)) seen.push(x.key); }));
    const i = seen.indexOf(key); return i < 0 ? 0 : i;
  }
  const color = (/** @type {string} */ key) => PAL[colorIdx(key) % PAL.length];

  /** the keys the panel plots now (all when it names none) @param {any} p @param {Ione[]} all */
  const picked = (p, all) => { if (!panelTr(p)) return []; const tr = mrmSel(p.tr, all.map(x => x.key)); return !tr || (Array.isArray(tr) && !tr.length) ? all.map(x => x.key) : (Array.isArray(tr) ? tr : [tr]); };

  /** the entries of the «Extract ion ›» submenu: «All», then one per ion with its colour; click = only this one, Ctrl/⌘+click adds or removes it, Shift+click takes the range
   * from the last one clicked @param {any} c the context of the menu @returns {any[]} */
  function rows(c) {
    const p = c.p, all = ions(p), cur = new Set(picked(p, all)), T = (/** @type {string} */ k) => I18N.t(k);
    const set = (/** @type {string[]} */ keys) => {
      if (panelTr(p)) { COMANDI.fai("panel.source", p, () => { p.tr = keys.length === all.length || !keys.length ? "" : keys; }); ctl(p); draw(p); uiSave(); }
      else { reveal(apriAnnullabile(addPanel("mrm", { imode: "man", intf: "all", intfFixed: true, tr: keys.length === all.length ? "" : keys, title: all.every(x => x.sim) ? "@sim" : undefined }))); }
    };
    const out = [{ nome: T("canali.sim.all"), check: cur.size === all.length && all.length > 0, esegui: () => set(all.map(x => x.key)), aria: T("canali.sim.all") }];
    all.forEach((x, i) => out.push({
      nome: x.name ? `${label(x.key)} ${x.name}` : label(x.key), color: color(x.key), check: cur.has(x.key), aria: label(x.key) + (x.name ? " " + x.name : ""),
      // @ts-ignore the event comes from the menu
      esegui: (/** @type {MouseEvent | undefined} */ ev) => {
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

  /** the help line of the empty Data view of a SIM session (no graph yet): what to do next and a button to the SIM table */
  function help() {
    const el = document.getElementById("simhelp"); if (!el) return;
    const on = allSim() && !E.panels.some((/** @type {any} */ p) => p.el);
    if (!on) { el.hidden = true; return; }
    if (el.dataset.on !== I18N.lang) {
      el.innerHTML = h`${I18N.t("canali.sim.help")} <button type="button" class="sec sm">${I18N.t("canali.sim.helpbtn")}</button>`;
      /** @type {HTMLElement} */ (el.querySelector("button")).onclick = () => {
        S.table = true; syncWork(true);                                // the work area shows the sample table and the SIM table (area-lavoro.js)
        const c = document.getElementById("simtab"); if (!c) return;
        const d = c.querySelector("details"); if (d) d.open = true;
        c.hidden = false; c.scrollIntoView({ block: "start" }); const r = /** @type {HTMLElement | null} */ (c.querySelector("[role=region]")); if (r) r.focus();
      };
      el.dataset.on = I18N.lang;
    }
    el.hidden = false;
  }

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

  // a file of transitions or SIM ions has no XIC: the same place of the menu holds the ions to extract (a submenu, colour swatch and check mark each)
  AZIONI.registra({ id: "mrm.estrai", gruppo: "xic", viste: ["chrom", "mrm"], visibile: (/** @type {any} */ c) => c.p.kind !== "pda" && ions(c.p).length > 0,
    nome: (/** @type {any} */ c) => I18N.t(ions(c.p).every(x => x.sim) ? "canali.sim.extract" : "canali.tr.extract"), sotto: (/** @type {any} */ c) => rows(c) });

  window.SIMUI = { chKey, label, methodTable, cell, button, fill, areaOf, areaBtn, syncArea, wireArea, help, n, mark, filesOf, panelSim, allSim, ions, colorIdx, color, rows, area, trKey };
})();
