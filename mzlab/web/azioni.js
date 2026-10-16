// @ts-strict
"use strict";
// Registry of the actions of the panels (format azioni/1): every entry of the right-click menu, of the bar of a panel and of the command palette is one action.
// A panel composes its menu with AZIONI.per(c): the actions of its view, in the order of the groups, the ones not available now grey (never hidden).
// c = the context of the request: { vista: "map" | "map3d" | ..., p: the panel, ... } plus what the module that registered the action needs (the map passes its adapter).
// Classic script, loaded after esperimenti.js; the modules register their actions in their own files (mappa.js for the RT x m/z map).
/**
 * @typedef {{ vista: string, [k: string]: any }} Contesto
 * @typedef {{ id: string, gruppo: string, viste: string[], icona?: string, menu?: boolean, barra?: boolean,
 *   visibile?: (c: Contesto) => boolean, disponibile?: (c: Contesto) => boolean, attiva?: (c: Contesto) => boolean,
 *   nome?: (c: Contesto) => string, aria?: (c: Contesto) => string, voce?: (c: Contesto) => string, tip?: (c: Contesto) => string, svg?: (c: Contesto) => string,
 *   scorciatoia?: Scorciatoia, esegui?: (c: Contesto) => any, ordine?: number, doppio?: (c: Contesto) => boolean, doppioOrd?: number, voci?: (c: Contesto) => Riga[], [k: string]: any }} Azione
 * @typedef {{ tasto?: string, code?: string, mod?: boolean, alt?: boolean, shift?: boolean, dove: "chart" | "view", et?: string }} Scorciatoia
 *   tasto = `e.key` of the letter / Enter / ? (compared in lower case); code = `e.code` for the digits of Alt+1-3; mod = Ctrl on Windows and Linux, ⌘ on the Mac; dove = "chart": only with the focus on a chart, "view": anywhere in the view; et = i18n key of the short label
 * @typedef {{ nome: string, aria?: string, tip?: string, check?: boolean | null, dim?: boolean, icon?: string, esegui: () => any }} Riga
 * @typedef {{ id: string, label: string, aria: string, tip?: string, icon?: string, check: boolean | null, dim: boolean, fn: () => any }} VoceAzione
 */
const AZIONI = (() => {
  // the order of the groups in every menu: one table for all the panels (the spectrum: value .. link; the map: extract .. help; zoom reset first and the annotations last in every graph)
  const GRUPPI = ["reset", "value", "ms2", "gen", "extract", "xic", "cont", "tools", "dda", "lib", "link", "lock", "view", "zoom", "disp", "learn", "copy", "help", "ann"];
  /** @type {Azione[]} */
  const voci = [];
  /** @param {Azione} a */
  function registra(a) {
    if (voci.some(v => v.id === a.id)) throw new Error("AZIONI: id already registered: " + a.id);
    if (!GRUPPI.includes(a.gruppo)) throw new Error("AZIONI: unknown group " + a.gruppo + " of " + a.id);
    if (!a.icona && typeof AZIONI_ICONE !== "undefined") a.icona = AZIONI_ICONE.POR_ID[a.id];       // the icon of the action: its own key, else the table of azioni-icone.js
    voci.push(a);
    return a;
  }
  // the decorative SVG of an action: its own `svg` (the map), else the icon `icona` of the table
  /** @param {Azione} a @param {Contesto} c @returns {string} */
  const icona = (a, c) => (a.svg ? a.svg(c) : "") || (a.icona && typeof AZIONI_ICONE !== "undefined" ? AZIONI_ICONE.svg(a.icona) : "");
  /** @param {string} id */
  const voce = id => voci.find(v => v.id === id);
  /** @param {Azione} a @param {Contesto} c */
  const mostra = (a, c) => (a.viste.includes(c.vista) || a.viste.includes("*")) && (!a.visibile || a.visibile(c));
  /** @param {Azione} a @param {Contesto} c */
  const pronta = (a, c) => !a.disponibile || a.disponibile(c);
  // the actions of the view in the order of the groups (inside a group: `ordine`, then the order of registration)
  /** @param {Contesto} c @param {"menu" | "barra" | "tutte"} [dove] */
  function elenco(c, dove = "menu") {
    const l = voci.filter(a => mostra(a, c) && (dove === "tutte" || (dove === "menu" ? a.menu !== false : a.barra === true)));
    return l.sort((a, b) => GRUPPI.indexOf(a.gruppo) - GRUPPI.indexOf(b.gruppo) || (a.ordine || 0) - (b.ordine || 0) || voci.indexOf(a) - voci.indexOf(b));
  }
  // the entries of the right-click menu (shared component: menu() of explore.js), a separator between two groups;
  // an action with `voci` gives several rows (one per open XIC, per measure…), all with its id
  /** @param {Contesto} c @returns {(VoceAzione | "-")[]} */
  function per(c) {
    /** @type {(VoceAzione | "-")[]} */
    const items = [];
    let last = null;
    for (const a of elenco(c)) {
      if (a.voci) {
        const ok = pronta(a, c), rs = a.voci(c);
        if (rs.length && last && a.gruppo !== last) items.push("-");
        if (rs.length) last = a.gruppo;
        rs.forEach(r => items.push({ id: a.id, label: r.nome, aria: r.aria || r.nome, tip: r.tip, icon: r.icon || icona(a, c), check: r.check ?? null, dim: !ok || !!r.dim, fn: r.esegui }));
        continue;
      }
      if (last && a.gruppo !== last) items.push("-");
      last = a.gruppo;
      const nome = a.nome ? a.nome(c) : a.id;
      items.push({ id: a.id, label: a.voce ? a.voce(c) : nome, aria: a.aria ? a.aria(c) : nome, tip: a.tip && a.tip(c), icon: icona(a, c),
        check: a.attiva ? a.attiva(c) : null, dim: !pronta(a, c), fn: () => esegui(a.id, c) });
    }
    return items;
  }
  /** @param {string} id @param {Contesto} c */
  function esegui(id, c) {
    const a = voce(id);
    if (a && a.esegui && mostra(a, c) && pronta(a, c)) return a.esegui(c);
  }
  // the double click on a graph: the first available action that claims it (`doppio(c)` true), by `doppioOrd` (default 5); false when none does
  /** @param {Contesto} c */
  function doppio(c) {
    const l = elenco(c, "tutte").filter(a => a.doppio && pronta(a, c) && a.doppio(c)).sort((a, b) => (a.doppioOrd ?? 5) - (b.doppioOrd ?? 5));
    if (!l.length) return false;
    esegui(l[0].id, c);
    return true;
  }
  return { formato: "azioni/1", GRUPPI, voci, registra, voce, elenco, per, esegui, mostra, pronta, doppio, icona };
})();
// ---- shortcuts: the field `scorciatoia` of an action, ONE keydown for all of them (the functions are also added to AZIONI)
const TASTI = (() => {
  const { voci, mostra, pronta, esegui } = AZIONI;
  const mac = () => /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent || "");
  /** The text of a shortcut as the user's system writes it: ⌘O / ⌥1 on the Mac, Ctrl+O / Alt+1 elsewhere. @param {Scorciatoia} s */
  function testo(s) {
    const t = s.code ? s.code.replace(/^(Digit|Key)/, "") : s.tasto === "Enter" ? "Enter" : String(s.tasto || "").toUpperCase();
    if (mac()) return (s.mod ? "⌘" : "") + (s.alt ? "⌥" : "") + (s.shift ? "⇧" : "") + t;
    return [s.mod ? "Ctrl" : "", s.alt ? "Alt" : "", s.shift ? "Shift" : "", t].filter(Boolean).join("+");
  }
  /** Does this key event press that shortcut? Letters ignore case (Shift only if the shortcut asks for it); Alt+digit uses `code`. @param {Scorciatoia} s @param {KeyboardEvent} e */
  function premuto(s, e) {
    if (!!s.mod !== (e.ctrlKey || e.metaKey) || !!s.alt !== e.altKey) return false;
    if (s.code) return e.code === s.code && !!s.shift === e.shiftKey;
    const k = String(s.tasto || "");
    if (k === "?") return e.key === "?";
    return !e.shiftKey === !s.shift && e.key.toLowerCase() === k.toLowerCase();
  }
  /** The actions with a shortcut for this view, in the order of registration. @param {string} vista */
  const conTasto = vista => voci.filter(a => a.scorciatoia && (a.viste.includes(vista) || a.viste.includes("*")));
  // the text field or dialog that keeps its own keys
  /** @param {EventTarget | null} t */
  const inCampo = t => !!document.querySelector("dialog[open]") || !!(t instanceof Element && t.closest("input,textarea,select,[contenteditable='true'],[contenteditable='']"));
  /** The panel whose chart has the focus (canvas, or the panel itself in full screen). @param {EventTarget | null} t */
  const pannelloDi = t => (typeof E === "undefined" || !(t instanceof Element)) ? null : E.panels.find((/** @type {any} */ q) => q.el && (q.cv === t || q.el === t)) || null;
  /** The context of a key on a chart: the point of the cursor (RT of the traces) or the highest visible peak (spectrum), as Shift+F10 does. @param {any} p @param {KeyboardEvent} e */
  function contestoGrafico(p, e) {
    const a = p._a, c = /** @type {any} */ ({ vista: p.type, p, e, near: null, a, has: false, m: null, lab: "…" });
    if (!a || !a.X) return c;
    let x = p.cur != null && p.cur >= a.x0 && p.cur <= a.x1 ? p.cur : (a.x0 + a.x1) / 2;
    if (p.type === "spec" && a.data && a.data[0]) {
      const d = a.data[0].d; let b = -1;
      for (let i = 0; i < d.mz.length; i++) if (d.mz[i] >= a.x0 && d.mz[i] <= a.x1 && (b < 0 || d.y[i] > d.y[b])) b = i;
      if (b >= 0) { x = d.mz[b]; c.has = true; c.m = x; c.lab = x.toFixed(typeof mzd === "function" ? mzd(p, p.level) : 2); }
    }
    c.x = x; c.px = a.X(x); c.py = p.cv.getBoundingClientRect().height / 2; c.k = p.type === "spec" ? 0 : typeof nearestFile === "function" ? nearestFile(p, x) : 0;
    return c;
  }
  /** @param {KeyboardEvent} e */
  function tasto(e) {
    if (e.defaultPrevented || e.isComposing || e.repeat || typeof E === "undefined") return;
    if (e.key === "Shift" || e.key === "Control" || e.key === "Alt" || e.key === "Meta") return;
    const combo = e.ctrlKey || e.metaKey || e.altKey;
    if (document.querySelector("dialog[open]")) return;
    if (!combo && inCampo(e.target)) return;
    const p = combo ? null : pannelloDi(e.target);                                    // the letters of a chart: only with the focus on it
    const vista = p ? p.type : (typeof S !== "undefined" && S.view) || "data";
    const c = p ? contestoGrafico(p, e) : { vista, p: E.active, e };
    for (const a of conTasto(vista)) {
      const s = /** @type {Scorciatoia} */ (a.scorciatoia);
      if ((s.dove === "chart" && !p) || !premuto(s, e) || !mostra(a, c) || !pronta(a, c)) continue;
      e.preventDefault(); e.stopPropagation(); esegui(a.id, c); return;
    }
  }
  if (typeof document !== "undefined") document.addEventListener("keydown", tasto);
  return { testo, premuto, conTasto, inCampo, pannelloDi, contestoGrafico };
})();
Object.assign(AZIONI, TASTI);
