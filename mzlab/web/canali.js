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
  const on = p => p.type === "chrom" && Array.isArray(p.canali) && p.canali.length > 0 && !["pda", "press", "reg"].includes(p.kind) && !(typeof p.kind === "string" && p.kind.startsWith("an:"));

  /** the files whose channels the selector of a panel lists @param {any} p */
  const filesOf = p => { const t = p.tab || E.tab, s = shownFor(p); return (s.length ? s : tabFiles(t)).filter((/** @type {any} */ f) => !f.gone); };
  /** loads the channels of the files of the panel; the controls are drawn again once when they arrive @param {any} p */
  function ensure(p) {
    const fl = filesOf(p).filter((/** @type {any} */ f) => !cached(f.k));
    if (!fl.length || p._canLoading) return;
    p._canLoading = true;
    Promise.all(fl.map((/** @type {any} */ f) => canali(f.k).catch(() => null))).then(() => { p._canLoading = false; if (p.el && p.el.isConnected) { ctl(p); } });
  }

  /** the label of the row for screen readers @param {Nodo} n */
  const rowLabel = n => T("canali.row", { et: n.et, n: n.n, a: n.n ? +n.rt0.toFixed(2) : 0, b: n.n ? +n.rt1.toFixed(2) : 0 });
  /** the rows of the tree (a collapsed parent hides its descendants) @param {any} p @param {Nodo[]} tree @param {Set<string>} sel @param {boolean} onlySrm */
  function rows(p, tree, sel, onlySrm) {
    const col = /** @type {Set<string>} */ (p._canCol || (p._canCol = new Set()));
    const depth = (/** @type {Nodo} */ n) => { let d = 1, u = n.up; const byO = new Map(tree.map(x => [x.omo, x])); while (u) { d++; u = (byO.get(u) || { up: null }).up; } return d; };
    const hidden = new Set(), many = filesOf(p).length > 1;
    tree.forEach(n => { if (n.up && (col.has(n.up) || hidden.has(n.up))) hidden.add(n.omo); });
    const vis = tree.filter(n => !hidden.has(n.omo) && (!onlySrm || n.fam === "srm"));
    return vis.map((n, i) => {
      const lv = onlySrm ? 1 : depth(n), has = n.kids.length > 0 && !onlySrm, open = has && !col.has(n.omo);
      const title = n.ids.length ? ` title="${EH(n.ids.slice(0, 3).join("\n"))}"` : "";
      return `<div class="can-row" role="treeitem" aria-level="${lv}" aria-selected="${sel.has(n.omo)}"${has ? ` aria-expanded="${open}"` : ""} tabindex="${i === 0 ? 0 : -1}" data-omo="${EH(n.omo)}" aria-label="${EH(rowLabel(n))}"${title}>` +
        `<span class="can-in" style="padding-left:${(lv - 1) * 14}px"><span class="can-car" aria-hidden="true">${has ? (open ? "▾" : "▸") : ""}</span><span class="can-ck" aria-hidden="true"></span><span class="can-et">${EH(n.et)}</span></span>` +
        `<span class="can-sg" aria-hidden="true">${EH(n.sigla)}</span><span class="can-n" aria-hidden="true">${n.n}</span><span class="can-rt" aria-hidden="true">${n.n ? `${+n.rt0.toFixed(1)}–${+n.rt1.toFixed(1)}` : ""}</span>` +
        (many ? `<span class="can-fl" aria-hidden="true" title="${EH(T("canali.files.title"))}">${n.files}</span>` : "") + `</div>`;
    }).join("");
  }
  /** transition key of a merged srm row @param {Nodo} n */
  const nodeKey = n => srmKey(/** @type {any} */ ({ id: n.ids[0] }));
  /** the omologhi that the panel shows as ticked @param {any} p @param {Nodo[]} tree */
  function ticked(p, tree) {
    if (p.type !== "mrm") return new Set(chosen(p));
    const all = !p.tr || (Array.isArray(p.tr) && !p.tr.length), want = new Set(Array.isArray(p.tr) ? p.tr : [p.tr]);
    return new Set(tree.filter(n => n.fam === "srm" && (all || want.has(nodeKey(n)))).map(n => n.omo));
  }
  /** the control of a panel: the button and the popover with the tree ("" while nothing is known about the files) @param {any} p */
  function html(p) {
    ensure(p);
    const files = filesOf(p), tree = albero(files);
    if (!tree.length) return "";
    const only = p.type === "mrm", sel = ticked(p, tree), open = !!p._canOpen;
    const n = only ? [...sel].length : chosen(p).length;
    const head = `<div class="can-hd" aria-hidden="true"><span>${T("canali.col.canale")}</span><span>${T("canali.col.sigla")}</span><span>n</span><span>RT</span>${files.length > 1 ? `<span>${T("canali.col.file")}</span>` : ""}</div>`;
    return `<span class="can"><button class="bt canb${n > 1 ? " on" : ""}" type="button" data-o="canali" aria-haspopup="tree" aria-expanded="${open}" title="${EH(T("canali.btn.title"))}" aria-label="${EH(T("canali.btn.aria", { n }))}"><span>${T("canali.btn")}</span>${n > 1 ? ` <b>${n}</b>` : ""}</button>` +
      `<div class="sp-pop can-pop" ${open ? "" : "hidden"}>${head}<div role="tree" aria-label="${EH(T("canali.btn"))}" aria-multiselectable="true" class="can-tree">${rows(p, tree, sel, only)}</div></div></span>`;
  }

  /** a click on a row: the channel is ticked or not @param {any} p @param {string} omo @param {Nodo[]} tree */
  function toggle(p, omo, tree) {
    COMANDI.fai("panel.source", p, () => {
      if (p.type === "mrm") {
        const srm = tree.filter(n => n.fam === "srm"), cur = new Set(ticked(p, tree));
        if (cur.has(omo)) cur.delete(omo); else cur.add(omo);
        const pick = srm.filter(n => cur.has(n.omo)).map(nodeKey);
        p.tr = pick.length === srm.length || !pick.length ? "" : pick;
        return;
      }
      const cur = new Set(chosen(p)); if (cur.has(omo)) cur.delete(omo); else cur.add(omo);
      const order = tree.map(n => n.omo).filter(o => cur.has(o));
      p.canali = order.length === 1 && order[0] === (p.kind === "bpc" ? "bpc" : "tic") ? undefined : order.length ? order : undefined;
    });
    ctl(p); draw(p); uiSave();
  }
  /** the focus moves among the visible rows @param {HTMLElement} row @param {number} d */
  function moveFocus(row, d) {
    const all = [...(row.closest("[role=tree]") || document).querySelectorAll(".can-row")], i = all.indexOf(row), t = /** @type {HTMLElement | undefined} */ (all[Math.max(0, Math.min(all.length - 1, i + d))]);
    if (t) { all.forEach(r => r.setAttribute("tabindex", "-1")); t.setAttribute("tabindex", "0"); t.focus(); }
  }
  /** the events of the control of a panel (after ctl() wrote it) @param {any} p @param {HTMLElement} c */
  function wire(p, c) {
    const b = /** @type {HTMLElement | null} */ (c.querySelector('[data-o="canali"]')), pop = /** @type {HTMLElement | null} */ (c.querySelector(".can-pop"));
    if (!b || !pop) return;
    const tree = albero(filesOf(p));
    const place = () => { if (pop.hidden) return; const op = /** @type {HTMLElement} */ (pop.offsetParent || p.el), pr = op.getBoundingClientRect(), br = b.getBoundingClientRect(); pop.style.left = Math.max(4, Math.min(br.left - pr.left, op.clientWidth - pop.offsetWidth - 6)) + "px"; pop.style.top = br.bottom - pr.top + 4 + "px"; };
    const set = (/** @type {boolean} */ v) => { p._canOpen = v; pop.hidden = !v; b.setAttribute("aria-expanded", String(v)); if (v) { place(); const r = /** @type {HTMLElement | null} */ (pop.querySelector(".can-row[aria-selected=true]") || pop.querySelector(".can-row")); if (r) { pop.querySelectorAll(".can-row").forEach(x => x.setAttribute("tabindex", "-1")); r.setAttribute("tabindex", "0"); r.focus({ preventScroll: true }); } } };
    b.onclick = () => set(!p._canOpen);
    if (p._canOpen) place();
    pop.querySelectorAll(".can-row").forEach(el => {
      const row = /** @type {HTMLElement} */ (el), omo = row.dataset.omo || "";
      row.onclick = e => {
        if ((/** @type {HTMLElement} */ (e.target)).closest(".can-car") && row.hasAttribute("aria-expanded")) { const col = /** @type {Set<string>} */ (p._canCol ||= new Set()); if (col.has(omo)) col.delete(omo); else col.add(omo); ctl(p); return; }
        toggle(p, omo, tree);
      };
      row.onkeydown = e => {
        const col = /** @type {Set<string>} */ (p._canCol ||= new Set());
        if (e.key === "ArrowDown") { e.preventDefault(); moveFocus(row, 1); }
        else if (e.key === "ArrowUp") { e.preventDefault(); moveFocus(row, -1); }
        else if (e.key === "Home") { e.preventDefault(); moveFocus(row, -1e6); }
        else if (e.key === "End") { e.preventDefault(); moveFocus(row, 1e6); }
        else if (e.key === "ArrowRight") { e.preventDefault(); if (row.getAttribute("aria-expanded") === "false") { col.delete(omo); p._canFocus = omo; ctl(p); } else moveFocus(row, 1); }
        else if (e.key === "ArrowLeft") {
          e.preventDefault();
          if (row.getAttribute("aria-expanded") === "true") { col.add(omo); p._canFocus = omo; ctl(p); }
          else { const lv = +(row.getAttribute("aria-level") || 1), all = [...pop.querySelectorAll(".can-row")], i = all.indexOf(row); for (let j = i - 1; j >= 0; j--) if (+(all[j].getAttribute("aria-level") || 1) < lv) { moveFocus(row, j - i); break; } }
        }
        else if (e.key === " " || e.key === "Enter") { e.preventDefault(); p._canFocus = omo; toggle(p, omo, tree); }
      };
    });
    if (p._canFocus) { const r = /** @type {HTMLElement | null} */ ([...pop.querySelectorAll(".can-row")].find(x => /** @type {HTMLElement} */ (x).dataset.omo === p._canFocus) || null); p._canFocus = null; if (r && p._canOpen) { pop.querySelectorAll(".can-row").forEach(x => x.setAttribute("tabindex", "-1")); r.setAttribute("tabindex", "0"); r.focus({ preventScroll: true }); } }
    if (!p._canAway) { p._canAway = (/** @type {Event} */ e) => { if (p._canOpen && p.el.isConnected && !(/** @type {HTMLElement} */ (e.target)).closest(".can")) { p._canOpen = false; const q = p.el.querySelector(".can-pop"); if (q) { /** @type {HTMLElement} */ (q).hidden = true; } const bb = p.el.querySelector('[data-o="canali"]'); if (bb) bb.setAttribute("aria-expanded", "false"); } }; document.addEventListener("mousedown", p._canAway, { capture: true, signal: p._ac && p._ac.signal }); }
  }
  ESC.registra({ id: "canali", prio: 27, aperto: () => E.panels.some(/** @param {any} p */ p => p._canOpen && p.el.isConnected), chiudi: () => {
    const p = E.panels.find(/** @param {any} q */ q => q._canOpen && q.el.isConnected); if (!p) return;
    p._canOpen = false; const q = /** @type {HTMLElement | null} */ (p.el.querySelector(".can-pop")); if (q) q.hidden = true;
    const bb = /** @type {HTMLElement | null} */ (p.el.querySelector('[data-o="canali"]')); if (bb) { bb.setAttribute("aria-expanded", "false"); bb.focus(); }
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
    const d = ESP.predefinito(c.tipo.proposto), v = c.canali;
    const pick = d.canale === "ms1" ? v.find(x => x.fam === "ms" && x.liv === 1) : d.canale === "ms2" ? v.find(x => x.fam === "ms" && x.liv === 2)
      : d.canale === "profondo" ? v.filter(x => x.fam === "ms").sort((a, b) => b.liv - a.liv)[0] : d.canale === "srm" ? v.find(x => x.fam === "srm" && srmKey(x) === (CAL.quant || "")) || v.find(x => x.fam === "srm")
      : d.canale === "uv" ? v.find(x => x.fam === "uv") : null;
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
