// @ts-check
"use strict";
// Channels of the files (api/canali, canali-dati.md): the cache of the channels of a file, the trace of a channel, the same channel across files (omologo),
// the tree selector of the chromatogram and MRM panels and the default channel of an experiment type. Only glue: every number comes from the engine.
// Classic helpers used by name: E, I18N, EH, J, memo, getMrm, PAL, DASHES, fdash, shownFor, tabFiles, ESC, COMANDI, ctl, draw, uiSave, CAL, ESP.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  /** @typedef {{ id: string, fam: string, gen: number | null, liv: number, pol: number, sigla: string | null, et: string, omo: string, n: number, rt0: number, rt1: number, spettri: boolean }} Canale */
  /** @typedef {{ tipo: { proposto: string, motivo: string }, canali: Canale[] }} Canali */
  /** @type {Map<string, Canali>} */ const DONE = new Map();
  /** @type {Map<string, Promise<Canali>>} */ const WAIT = new Map();
  const keyOf = (/** @type {number} */ k) => `${k}|${E.files[k] ? E.files[k].file : ""}`;

  /** the channels of file k, asked once @param {number} k @returns {Promise<Canali>} */
  function canali(k) {
    const key = keyOf(k);
    let w = WAIT.get(key);
    if (!w) {
      w = J(`api/canali?k=${k}`).then((/** @type {Canali} */ j) => { DONE.set(key, j); return j; });
      w.catch(() => WAIT.delete(key));
      WAIT.set(key, w);
    }
    return w;
  }
  /** the channels already read (null: not yet) @param {number} k @returns {Canali | null} */
  const cached = k => DONE.get(keyOf(k)) || null;
  /** @param {number} k @param {string} omo @returns {Canale | null} */
  const find = (k, omo) => { const c = cached(k); return c && c.canali.find(x => x.omo === omo) || null; };

  /** transition key «q1>q3» of an srm channel (the key of the MRM panel) @param {Canale} c */
  const srmKey = c => c.id.slice(4).replace(/@.*$/, "");
  /** the MRM transition of the file that an srm channel names @param {Canale} c @param {any} t */
  const srmIs = (c, t) => `${t.q1}>${t.q3}` === srmKey(c) && (!/@/.test(c.id) || Math.abs(+c.id.split("@")[1] - (t.ce ?? 0)) < 0.5);

  /** the trace of channel c of file k: { rt, y }, or null for a channel the engine has no trace for @param {number} k @param {Canale} c @param {{ kind?: string, mz0?: number | null, mz1?: number | null }} [o] */
  async function traccia(k, c, o = {}) {
    if (c.fam === "srm") {
      const t = (await getMrm(k)).find((/** @type {any} */ x) => srmIs(c, x));
      return t ? { rt: t.rt, y: t.y } : null;
    }
    if (!["tic", "bpc", "ms", "dd"].includes(c.fam)) return null;
    const kind = c.fam === "bpc" || o.kind === "bpc" ? "bpc" : "tic";
    const url = `api/chrom?k=${k}&canale=${encodeURIComponent(c.id)}&kind=${kind}` + (o.mz0 != null ? `&mz0=${o.mz0}` : "") + (o.mz1 != null ? `&mz1=${o.mz1}` : "");
    const d = await memo(`can${k}|${c.id}|${kind}|${o.mz0 ?? ""}|${o.mz1 ?? ""}`, () => J(url));
    return { rt: d.rt, y: d.y };
  }

  /** for each file its channel with this omologo (null: the file has none) @param {any[]} files @param {string} omo */
  async function omologhi(files, omo) {
    return Promise.all(files.map(async f => { const c = await canali(f.k).catch(() => null); return { f, c: c && c.canali.find(x => x.omo === omo) || null }; }));
  }

  // ---------------------------------------------------------------- the selector
  /** @typedef {{ omo: string, et: string, sigla: string, fam: string, liv: number, up: string | null, n: number, rt0: number, rt1: number, ids: string[], files: number, kids: string[] }} Nodo */
  /** The channels of the files merged by omologo, as a tree (parents before children) @param {any[]} files @returns {Nodo[]} */
  function albero(files) {
    /** @type {Map<string, Nodo>} */ const m = new Map();
    files.forEach(f => {
      const c = cached(f.k); if (!c) return;
      c.canali.forEach(x => {
        const up = x.gen != null && c.canali[x.gen] ? c.canali[x.gen].omo : null;
        let n = m.get(x.omo);
        if (!n) { n = { omo: x.omo, et: x.et, sigla: x.sigla || "", fam: x.fam, liv: x.liv, up, n: 0, rt0: Infinity, rt1: -Infinity, ids: [], files: 0, kids: [] }; m.set(x.omo, n); }
        n.n += x.n; n.files++; if (x.n) { n.rt0 = Math.min(n.rt0, x.rt0); n.rt1 = Math.max(n.rt1, x.rt1); }
        if (!n.ids.includes(x.id)) n.ids.push(x.id);
      });
    });
    m.forEach(n => { if (n.up && m.has(n.up)) /** @type {Nodo} */ (m.get(n.up)).kids.push(n.omo); else n.up = null; });
    /** @type {Nodo[]} */ const out = [];
    const visit = (/** @type {Nodo} */ n) => { out.push(n); n.kids.forEach(o => visit(/** @type {Nodo} */ (m.get(o)))); };
    [...m.values()].filter(n => !n.up).forEach(visit);
    return out;
  }
  /** the omologhi a panel plots: its own list, or the channel its kind stands for @param {any} p */
  const chosen = p => p.canali && p.canali.length ? p.canali : [p.kind === "bpc" ? "bpc" : "tic"];
  /** a chromatogram panel that plots channels (and not one of its kinds: PDA, pressure, log of the instrument) @param {any} p */
  const on = p => p.type === "chrom" && Array.isArray(p.canali) && p.canali.length > 0 && !isOther(p);

  /** the files whose channels the selector of a panel lists @param {any} p */
  const filesOf = p => { const t = p.tab || E.tab, s = shownFor(p); return (s.length ? s : tabFiles(t)).filter((/** @type {any} */ f) => !f.gone); };
  /** loads the channels of the files of the panel; the controls are drawn again once when they arrive @param {any} p */
  function ensure(p) {
    const fl = filesOf(p).filter((/** @type {any} */ f) => !cached(f.k));
    if (!fl.length || p._canLoading) return;
    p._canLoading = true;
    Promise.all(fl.map((/** @type {any} */ f) => canali(f.k).catch(() => null))).then(() => { p._canLoading = false; if (p.el && p.el.isConnected) { ctl(p); } });
  }

  /** @typedef {{ hasScan: boolean, hasPda: boolean, an: [string, any][], reg: boolean }} Ctx what the files of the tab have besides mass spectra */
  /** the kinds that are not channels of scans: [value, label, title] by group @param {Ctx} cx @returns {[string, string, string][][]} */
  const others = cx => {
    /** @type {[string, string, string][]} */ const det = [];
    /** @type {[string, string, string][]} */ const log = [];
    if (cx.hasPda) det.push(["pda", T("ctl.kind.pda"), T("ctl.kind.pda.title")]);
    cx.an.forEach(([v, a]) => det.push([v, v === "press" ? T("ctl.kind.press") : a.id, ""]));
    if (cx.reg) log.push(["reg", T("ctl.kind.reg"), T("ctl.kind.reg.title")]);
    return [det, log];
  };
  /** a panel that plots a kind that is not a channel of scans (PDA, pressure, log of the instrument) @param {any} p */
  const isOther = p => ["pda", "press", "reg"].includes(p.kind) || (typeof p.kind === "string" && p.kind.startsWith("an:"));
  /** the rows of the tree the picker lists: the channels of the files, or TIC and BPC while they are not known @param {Nodo[]} tree @param {Ctx | null} cx @returns {Nodo[]} */
  function msTree(tree, cx) {
    const fall = (/** @type {string} */ omo, /** @type {string} */ et) => ({ omo, et, sigla: "", fam: omo, liv: 1, up: null, n: 0, rt0: Infinity, rt1: -Infinity, ids: [], files: 0, kids: [] });
    // only the families traccia() can draw: a channel with no trace in the list is ticked and nothing appears
    const t = tree.length ? tree.filter(n => ["tic", "bpc", "ms", "dd", "srm"].includes(n.fam)) : [fall("tic", T("ctl.kind.tic")), fall("bpc", T("ctl.kind.bpc"))];
    return cx && !cx.hasScan ? t.filter(n => n.fam !== "bpc") : t;
  }
  /** the label of a row: the engine's name, with TIC and BPC in the words of the page @param {Nodo} n */
  const label = n => n.omo === "tic" ? T("ctl.kind.tic") : n.omo === "bpc" ? T("ctl.kind.bpc") : n.et;
  /** the text of the button: what the panel plots, «TIC + 1» for several channels @param {any} p @param {Nodo[]} tree @param {Ctx} cx */
  function summary(p, tree, cx) {
    if (isOther(p)) { const r = others(cx).flat().find(x => x[0] === p.kind); return r ? r[1] : String(p.kind); }
    const ids = chosen(p), one = (/** @type {string} */ o) => { const n = tree.find(x => x.omo === o); return n ? label(n) : o === "bpc" ? T("ctl.kind.bpc") : T("ctl.kind.tic"); };
    return ids.length > 1 ? `${one(ids[0])} + ${ids.length - 1}` : one(ids[0]);
  }
  /** the hover text of a row: what the channel is, the time range, the exact ids @param {Nodo} n */
  function tip(n) {
    const what = n.fam === "dd" ? T("canali.tip.dd") : n.fam === "ms" && n.liv === 1 ? T("canali.tip.ms1") : n.omo === "tic" ? T("canali.tip.tic") : n.omo === "bpc" ? T("ctl.kind.bpc.title") : "";
    return [what, n.n ? T("canali.tip.time", { a: +n.rt0.toFixed(2), b: +n.rt1.toFixed(2) }) : "", ...n.ids.slice(0, 3)].filter(Boolean).join("\n");
  }
  /** the label of the row for screen readers @param {Nodo} n */
  const rowLabel = n => n.n ? T("canali.row", { et: label(n), n: n.n, a: +n.rt0.toFixed(2), b: +n.rt1.toFixed(2) }) : label(n);
  /** the rows of the tree (a collapsed parent hides its descendants) @param {any} p @param {Nodo[]} tree @param {Set<string>} sel @param {boolean} onlySrm @param {number} nfiles */
  function rows(p, tree, sel, onlySrm, nfiles) {
    const col = /** @type {Set<string>} */ (p._canCol || (p._canCol = new Set()));
    const byO = new Map(tree.map(x => [x.omo, x]));
    const depth = (/** @type {Nodo} */ n) => { let d = 1, u = n.up; while (u) { d++; u = (byO.get(u) || { up: null }).up; } return d; };
    const hidden = new Set(), many = nfiles > 1;
    tree.forEach(n => { if (n.up && (col.has(n.up) || hidden.has(n.up))) hidden.add(n.omo); });
    const vis = tree.filter(n => !hidden.has(n.omo) && (!onlySrm || n.fam === "srm"));
    return vis.map(n => {
      const lv = onlySrm ? 1 : depth(n), has = n.kids.length > 0 && !onlySrm, open = has && !col.has(n.omo), t = tip(n);
      return `<div class="can-row" role="treeitem" aria-level="${lv}" aria-selected="${sel.has(n.omo)}"${has ? ` aria-expanded="${open}"` : ""} tabindex="-1" data-omo="${EH(n.omo)}" aria-label="${EH(rowLabel(n))}"${t ? ` title="${EH(t)}"` : ""}>` +
        `<span class="can-in" style="padding-left:${(lv - 1) * 14}px"><span class="can-car" aria-hidden="true">${has ? (open ? "▾" : "▸") : ""}</span><span class="can-ck" aria-hidden="true"></span><span class="can-et">${EH(label(n))}</span></span>` +
        `<span class="can-sg" aria-hidden="true">${EH(n.sigla)}</span><span class="can-n" aria-hidden="true">${n.n || ""}</span><span class="can-rt" aria-hidden="true">${n.n ? `${+n.rt0.toFixed(1)}–${+n.rt1.toFixed(1)}` : ""}</span>` +
        (many ? `<span class="can-fl" aria-hidden="true" title="${EH(T("canali.files.title"))}">${n.files}</span>` : "") + `</div>`;
    }).join("");
  }
  /** the rows of the other kinds (one at a time, a round mark) @param {any} p @param {[string, string, string][]} list */
  const kindRows = (p, list) => list.map(([v, et, t]) =>
    `<div class="can-row" role="treeitem" aria-level="1" aria-selected="${p.kind === v}" tabindex="-1" data-kind="${EH(v)}" aria-label="${EH(et)}"${t ? ` title="${EH(t)}"` : ""}><span class="can-in"><span class="can-car" aria-hidden="true"></span><span class="can-ck can-rd" aria-hidden="true"></span><span class="can-et">${EH(et)}</span></span></div>`).join("");
  /** transition key of a merged srm row @param {Nodo} n */
  const nodeKey = n => srmKey(/** @type {any} */ ({ id: n.ids[0] }));
  /** the omologhi that the panel shows as ticked @param {any} p @param {Nodo[]} tree */
  function ticked(p, tree) {
    if (!ESP.mrm(p)) return isOther(p) ? new Set() : new Set(chosen(p));
    const all = !p.tr || (Array.isArray(p.tr) && !p.tr.length), want = new Set(Array.isArray(p.tr) ? p.tr : [p.tr]);
    return new Set(tree.filter(n => n.fam === "srm" && (all || want.has(nodeKey(n)))).map(n => n.omo));
  }
  /** the control of a panel: a button with what is plotted and the popover with the channels in groups. The chromatogram passes what its files have (cx); the MRM panel
   * lists only its transitions ("" while nothing is known about the files) @param {any} p @param {Ctx} [cx] */
  function html(p, cx) {
    ensure(p);
    const files = filesOf(p), only = ESP.mrm(p), raw = albero(files);
    if (only && !raw.length) return "";
    const tree = only ? raw : msTree(raw, cx || null), sel = ticked(p, tree), open = !!p._canOpen;
    const head = `<div class="can-hd" aria-hidden="true"><span>${T("canali.col.canale")}</span><span>${T("canali.col.sigla")}</span><span title="${EH(T("canali.col.n.title"))}">${T("canali.col.n")}</span><span title="${EH(T("canali.col.rt.title"))}">${T("canali.col.rt")}</span>${files.length > 1 ? `<span>${T("canali.col.file")}</span>` : ""}</div>`;
    const grp = (/** @type {string} */ name, /** @type {string} */ body) => body ? `<div role="group" aria-label="${EH(name)}"><div class="can-gh" aria-hidden="true">${EH(name)}</div>${body}</div>` : "";
    const [det, log] = cx ? others(cx) : [[], []];
    const body = only ? rows(p, tree, sel, true, files.length) : grp(T("canali.grp.ms"), rows(p, tree, sel, false, files.length)) + grp(T("canali.grp.det"), kindRows(p, det)) + grp(T("canali.grp.log"), kindRows(p, log));
    const txt = only ? T("canali.btn.mrm") : summary(p, tree, /** @type {Ctx} */ (cx)), aria = only ? T("canali.btn.aria", { n: sel.size }) : T("canali.kind.aria", { v: txt });
    return `<span class="can"><button class="bt canb" type="button" data-o="${only ? "canali" : "kind"}" aria-haspopup="tree" aria-expanded="${open}" title="${EH(T(only ? "canali.btn.title" : "ctl.kind.title"))}" aria-label="${EH(aria)}"><span class="can-bt">${EH(txt)}</span><span class="can-dn" aria-hidden="true">▾</span></button>` +
      `<div class="sp-pop can-pop" ${open ? "" : "hidden"}>${head}<div role="tree" aria-label="${EH(T("canali.btn"))}" aria-multiselectable="true" class="can-tree">${body}</div></div></span>`;
  }

  /** the channels ticked in the picker: one channel of scans that is TIC or BPC is the kind, anything else a list @param {any} p @param {string[]} order */
  function setMs(p, order) {
    if (order.length === 1 && (order[0] === "tic" || order[0] === "bpc")) { p.kind = order[0]; p.canali = undefined; } else { p.kind = "tic"; p.canali = order; }
  }
  /** a click on a row of scans: the channel is ticked or not (the first click after another kind starts again from that channel) @param {any} p @param {string} omo @param {Nodo[]} tree */
  function toggle(p, omo, tree) {
    COMANDI.fai("panel.source", p, () => {
      if (ESP.mrm(p)) {
        const srm = tree.filter(n => n.fam === "srm"), cur = new Set(ticked(p, tree));
        if (cur.has(omo)) cur.delete(omo); else cur.add(omo);
        const pick = srm.filter(n => cur.has(n.omo)).map(nodeKey);
        p.tr = pick.length === srm.length || !pick.length ? "" : pick;
        return;
      }
      const cur = new Set(isOther(p) ? [] : chosen(p)); if (cur.has(omo)) cur.delete(omo); else cur.add(omo);
      const order = tree.map(n => n.omo).filter(o => cur.has(o));
      if (order.length) setMs(p, order);
    });
    ctl(p); draw(p); uiSave();
  }
  /** a click on a row of another kind (PDA, a detector, the log): the panel plots that and nothing else @param {any} p @param {string} v */
  function pickKind(p, v) {
    COMANDI.fai("panel.source", p, () => { p.kind = v; p.canali = undefined; });
    p._canOpen = false; ctl(p); draw(p); uiSave();
    const b = /** @type {HTMLElement | null} */ (p.el.querySelector('[data-o="kind"]')); if (b) b.focus();
  }
  /** the focus moves among the visible rows @param {HTMLElement} row @param {number} d */
  function moveFocus(row, d) {
    const all = [...(row.closest("[role=tree]") || document).querySelectorAll(".can-row")], i = all.indexOf(row), t = /** @type {HTMLElement | undefined} */ (all[Math.max(0, Math.min(all.length - 1, i + d))]);
    if (t) { all.forEach(r => r.setAttribute("tabindex", "-1")); t.setAttribute("tabindex", "0"); t.focus(); }
  }
  /** the events of the control of a panel (after ctl() wrote it) @param {any} p @param {HTMLElement} c */
  function wire(p, c) {
    // the older selectors (precursor, transition) and the channels say the same thing: the last choice wins
    const cc = /** @type {any} */ (c);
    if (!cc._canR) { cc._canR = true; c.addEventListener("change", e => { if (/^(prec|tr)$/.test((/** @type {HTMLElement} */ (e.target)).dataset?.o || "")) p.canali = undefined; }, true); }
    const b = /** @type {HTMLElement | null} */ (c.querySelector('[data-o="canali"],[data-o="kind"]')), pop = /** @type {HTMLElement | null} */ (c.querySelector(".can-pop"));
    if (!b || !pop) return;
    const tree = msTree(albero(filesOf(p)), null);
    const place = () => { if (pop.hidden) return; const op = /** @type {HTMLElement} */ (pop.offsetParent || p.el), pr = op.getBoundingClientRect(), br = b.getBoundingClientRect(); pop.style.left = Math.max(4, Math.min(br.left - pr.left, op.clientWidth - pop.offsetWidth - 6)) + "px"; pop.style.top = br.bottom - pr.top + 4 + "px"; };
    const rowsOf = () => [...pop.querySelectorAll(".can-row")].filter(x => /** @type {HTMLElement} */ (x).offsetParent);
    const focusRow = (/** @type {Element | null | undefined} */ r) => { if (!r) return; pop.querySelectorAll(".can-row").forEach(x => x.setAttribute("tabindex", "-1")); r.setAttribute("tabindex", "0"); /** @type {HTMLElement} */ (r).focus({ preventScroll: true }); };
    const set = (/** @type {boolean} */ v) => { p._canOpen = v; pop.hidden = !v; b.setAttribute("aria-expanded", String(v)); if (v) { place(); focusRow(pop.querySelector(".can-row[aria-selected=true]") || pop.querySelector(".can-row")); } };
    b.onclick = () => set(!p._canOpen);
    b.onkeydown = e => { if (e.key === "ArrowDown" && !p._canOpen) { e.preventDefault(); set(true); } };
    if (p._canOpen) place();
    pop.querySelectorAll(".can-row").forEach(el => {
      const row = /** @type {HTMLElement} */ (el), omo = row.dataset.omo || "", kind = row.dataset.kind || "";
      const act = () => { if (kind) pickKind(p, kind); else toggle(p, omo, tree); };
      row.onclick = e => {
        if ((/** @type {HTMLElement} */ (e.target)).closest(".can-car") && row.hasAttribute("aria-expanded")) { const col = /** @type {Set<string>} */ (p._canCol ||= new Set()); if (col.has(omo)) col.delete(omo); else col.add(omo); p._canFocus = omo; ctl(p); return; }
        act();
      };
      row.onkeydown = e => {
        // the tree owns its keys: the page-wide handlers (Space = play the scans, arrows = step them) must not see them
        if (/^(Arrow(Up|Down|Left|Right)|Home|End| |Enter)$/.test(e.key)) e.stopPropagation();
        const col = /** @type {Set<string>} */ (p._canCol ||= new Set());
        if (e.key === "ArrowDown") { e.preventDefault(); moveFocus(row, 1); }
        else if (e.key === "ArrowUp") { e.preventDefault(); moveFocus(row, -1); }
        else if (e.key === "Home") { e.preventDefault(); moveFocus(row, -1e6); }
        else if (e.key === "End") { e.preventDefault(); moveFocus(row, 1e6); }
        else if (e.key === "ArrowRight") { e.preventDefault(); if (row.getAttribute("aria-expanded") === "false") { col.delete(omo); p._canFocus = omo; ctl(p); } else moveFocus(row, 1); }
        else if (e.key === "ArrowLeft") {
          e.preventDefault();
          if (row.getAttribute("aria-expanded") === "true") { col.add(omo); p._canFocus = omo; ctl(p); }
          else { const lv = +(row.getAttribute("aria-level") || 1), all = rowsOf(), i = all.indexOf(row); for (let j = i - 1; j >= 0; j--) if (+(all[j].getAttribute("aria-level") || 1) < lv) { moveFocus(row, j - i); break; } }
        }
        else if (e.key === " " || e.key === "Enter") { e.preventDefault(); p._canFocus = omo || kind; act(); }
      };
    });
    if (p._canFocus) { const key = p._canFocus, r = [...pop.querySelectorAll(".can-row")].find(x => /** @type {HTMLElement} */ (x).dataset.omo === key || /** @type {HTMLElement} */ (x).dataset.kind === key); p._canFocus = null; if (r && p._canOpen) focusRow(r); }
    if (!p._canAway) { p._canAway = (/** @type {Event} */ e) => { if (p._canOpen && p.el.isConnected && !(/** @type {HTMLElement} */ (e.target)).closest(".can")) { p._canOpen = false; const q = p.el.querySelector(".can-pop"); if (q) { /** @type {HTMLElement} */ (q).hidden = true; } const bb = p.el.querySelector('[data-o="canali"],[data-o="kind"]'); if (bb) bb.setAttribute("aria-expanded", "false"); } }; document.addEventListener("mousedown", p._canAway, { capture: true, signal: p._ac && p._ac.signal }); }
  }
  ESC.registra({ id: "canali", prio: 27, aperto: () => E.panels.some(/** @param {any} p */ p => p._canOpen && p.el.isConnected), chiudi: () => {
    const p = E.panels.find(/** @param {any} q */ q => q._canOpen && q.el.isConnected); if (!p) return;
    p._canOpen = false; const q = /** @type {HTMLElement | null} */ (p.el.querySelector(".can-pop")); if (q) q.hidden = true;
    const bb = /** @type {HTMLElement | null} */ (p.el.querySelector('[data-o="canali"],[data-o="kind"]')); if (bb) { bb.setAttribute("aria-expanded", "false"); bb.focus(); }
  } });

  // ---------------------------------------------------------------- the traces of a panel
  /** the traces of the channels a panel plots, one per file and channel; a file without the channel is kept in p._canMiss for the legend @param {any} p @param {any[]} files */
  async function series(p, files) {
    const list = chosen(p).filter((/** @type {string} */ o) => !(p.kind === "bpc" && o === "tic")), miss = [];
    await Promise.all(files.map((/** @type {any} */ f) => canali(f.k).catch(() => null)));
    const one = list.length === 1, out = [];
    for (let ci = 0; ci < list.length; ci++) {
      const omo = list[ci];
      const got = await Promise.all(files.map(async (/** @type {any} */ f, /** @type {number} */ fi) => {
        const c = find(f.k, omo);
        if (!c) { miss.push({ f, omo }); return null; }
        const t = await traccia(f.k, c, { kind: p.kind, mz0: p.mz0, mz1: p.mz1 }); if (!t || !t.rt.length) return null;
        const et = c.et;
        return { x: t.rt, y: t.y, k: f.k, key: `c|${f.k}|can|${omo}`, ion: et, time: f.time, color: one ? f.color : PAL[ci % PAL.length], dash: one ? fdash(f) : fi > 0 ? DASHES[fi % 4] : [], name: one ? f.label : files.length > 1 ? `${et} · ${f.label}` : et, tid_label: et };
      }));
      got.forEach(s => { if (s) out.push(s); });
    }
    p._canMiss = miss.map(m => ({ f: m.f, et: (albero(files).find(n => n.omo === m.omo) || { et: m.omo }).et, one }));
    return out;
  }
  /** the entries «—» of the legend: the files that have not a channel the panel plots @param {any} p */
  const legend = p => (p._canMiss || []).map((/** @type {any} */ m) => `<span class="tg off" title="${EH(T("canali.miss.title"))}"><i style="background:${m.f.color}"></i>${EH(m.one ? m.f.label : `${m.et} · ${m.f.label}`)} —</span>`).join("");

  // ---------------------------------------------------------------- default channel of an experiment type
  /** the channel an experiment type starts on in the file (esperimenti.js says which kind of channel) @param {any} f @returns {Promise<Canale | null>} */
  async function iniziale(f) {
    const c = await canali(f.k).catch(() => null); if (!c) return null;
    const v = c.canali, pick = ESP.scegli(c.tipo.proposto, v, (/** @type {any} */ x) => srmKey(x) === (CAL.quant || ""));
    return pick || v.find(x => x.fam === "tic") || null;
  }
  /** the type of experiment the engine proposes for a file (null until its channels are known) @param {any} f */
  const tipo = f => {
    if (!f) return null;
    const c = cached(f.k);
    if (!c) canali(f.k).catch(() => null);        // read now: the next drawing knows it
    return c ? c.tipo.proposto : null;
  };
  /** the omologo of the channel the linked chromatogram of a spectrum plots in file f, if it is a channel of scans of the level of the file (null: the whole level) @param {any} p @param {any} f */
  function xicOmo(p, f) {
    const q = p && p.link ? E.panels.find((/** @type {any} */ x) => x.id === p.link) : null;
    if (!q || !f || !on(q)) return null;
    for (const o of q.canali) { const c = find(f.k, o); if (c && (c.fam === "ms" || c.fam === "dd") && c.liv === (f.lv || 1)) return o; }
    return null;
  }
  /** the id of the channel with this omologo in file f (null: none, or no omologo) @param {any} f @param {string | null | undefined} omo */
  const idOf = (f, omo) => { const c = omo && f ? find(f.k, omo) : null; return c ? c.id : null; };
  /** @type {any} */ (window).CANALI = { canali, cached, find, traccia, omologhi, albero, on, html, wire, series, legend, iniziale, tipo, xicOmo, idOf, ensure, chosen };
})();
