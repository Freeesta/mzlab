// @ts-strict
"use strict";
// Registry of the actions of the panels (format actions/1): every entry of the right-click menu, of the bar of a panel and of the command palette is one action.
// A panel composes its menu with AZIONI.forContext(c): the actions of its view, in the order of the groups, the ones not available now grey (never hidden).
// c = the context of the request: { view: "map" | "map3d" | ..., p: the panel, ... } plus what the module that registered the action needs (the map passes its adapter).
// Classic script, loaded after esperimenti.js; the modules register their actions in their own files (mappa.js for the RT x m/z map).
/**
 * @typedef {{ view: string, [k: string]: any }} Context
 * @typedef {{ id: string, group: string, views: string[], iconKey?: string, menu?: boolean, barra?: boolean, scope?: "point" | "panel",
 *   visible?: (c: Context) => boolean, available?: (c: Context) => boolean, checked?: (c: Context) => boolean,
 *   name?: (c: Context) => string, aria?: (c: Context) => string, menuLabel?: (c: Context) => string, tip?: (c: Context) => string, svg?: (c: Context) => string,
 *   shortcut?: Shortcut, run?: (c: Context) => any, order?: number, doubleClick?: (c: Context) => boolean, doubleClickOrder?: number, rows?: (c: Context) => Row[], [k: string]: any }} Action
 * @typedef {{ key?: string, code?: string, mod?: boolean, alt?: boolean, shift?: boolean, where: "chart" | "view", label?: string }} Shortcut
 *   key = `e.key` of the letter / Enter / ? (compared in lower case); code = `e.code` for the digits of Alt+1-3; mod = Ctrl on Windows and Linux, ⌘ on the Mac; where = "chart": only with the focus on a chart, "view": anywhere in the view; label = i18n key of the short label
 * @typedef {{ name: string, aria?: string, tip?: string, check?: boolean | null, dim?: boolean, info?: boolean, icon?: string, run: () => any }} Row
 * @typedef {{ id: string, scope?: string, label: string, aria: string, tip?: string, icon?: string, key?: string, check: boolean | null, dim: boolean, off: boolean, fn: () => any, fila?: string, soloIcona?: boolean, sub?: () => any[] }} MenuEntry
 */
// an entry not available now (`off`, set by AZIONI.forContext) is not shown at all; a separator left first, last or doubled by that goes too.
// A menu never has more than MAX_ROWS rows at the top level (study §3.7): the tail (the panel block comes last, so it spills first) goes into ONE submenu «More panel actions».
const MAX_ROWS = 12;
/** @param {any[]} out the items, separators ("-") included: no separator first, last or doubled */
const trimSeps = out => {
  const r = /** @type {any[]} */ ([]);
  for (const it of out) if (!(it === "-" && (!r.length || r[r.length - 1] === "-"))) r.push(it);
  if (r[r.length - 1] === "-") r.pop();
  return r;
};
/** @param {any} it one entry (or an icon row of several) as the rows of the submenu @returns {any[]} */
const subRows = it => (it.fila || [it]).map((/** @type {any} */ x) => ({ name: x.label, aria: x.aria, tip: x.tip, check: x.check, key: x.key, run: () => x.fn() }));
/** @param {any[]} items */
const cleanMenu = items => {
  let out = /** @type {any[]} */ ([]);
  for (let it of items) {
    if (it !== "-" && it.fila) { const f = it.fila.filter((x) => !x.off); if (!f.length) continue; it = { ...it, fila: f }; }
    else if (it !== "-" && it.off) continue;
    out.push(it);
  }
  out = trimSeps(out);
  const rows = out.filter(it => it !== "-").length;
  // only a menu built from the registry (its entries carry a `scope`) is capped: the menu of the drawing has its own rows
  if (rows > MAX_ROWS && out.some(it => it !== "-" && (it.scope || it.fila?.[0].scope))) {
    // from the end: only a plain entry can move (an entry that already has a submenu, a grey row or the readout row stays)
    const move = /** @type {Set<any>} */ (new Set()), need = rows - (MAX_ROWS - 1);
    for (let i = out.length - 1; i >= 0 && move.size < need; i--) { const it = out[i]; if (it !== "-" && !it.sub && !it.dim && (it.fn || it.fila)) move.add(it); }
    if (move.size > 1) {
      const sub = /** @type {any[]} */ ([]), rest = /** @type {any[]} */ ([]);
      for (const it of out) {
        if (!move.has(it)) { rest.push(it); continue; }
        if (!sub.length) { const name = typeof I18N !== "undefined" ? I18N.t("ctx.more") : "More"; rest.push({ id: "menu.more", label: name, aria: name, icon: typeof AZIONI_ICONE !== "undefined" ? AZIONI_ICONE.svg("list") : "", check: null, dim: false, off: false, fn: () => {}, sub: () => sub.flatMap(subRows) }); }
        sub.push(it);
      }
      out = trimSeps(rest);
    }
  }
  return out;
};
// `scope`: "point" (under the pointer, the default) or "panel" (the whole panel, repeated from the bar). Block of an action in the menu: 0 = point actions, 1 = panel commands of the menu, 2 = panel commands that come from the bar
/** @param {any} a */
const block = a => (a.scope === "panel" ? (a.barra ? 2 : 1) : 0);
// the key of an action as the user's system writes it (⌘O or Ctrl+O), for the menu (AZIONI.keyText is added below, with the keys)
/** @param {any} a */
const keyOf = a => (a.shortcut && /** @type {any} */ (AZIONI).keyText ? /** @type {any} */ (AZIONI).keyText(a.shortcut) : undefined);
const AZIONI = (() => {
  // the order of the groups in every menu: one table for all the panels (the spectrum: value .. link; the map: extract .. help; zoom reset first and the annotations last in every graph)
  const GROUPS = ["reset", "value", "ms2", "gen", "extract", "xic", "cont", "tools", "dda", "lib", "link", "lock", "view", "zoom", "disp", "learn", "copy", "help", "ann"];
  /** @type {Action[]} */
  const actions = [];
  /** @param {Action} a */
  function register(a) {
    if (actions.some(v => v.id === a.id)) throw new Error("AZIONI: id already registered: " + a.id);
    if (!GROUPS.includes(a.group)) throw new Error("AZIONI: unknown group " + a.group + " of " + a.id);
    if (!["point", "panel"].includes(a.scope ||= "point")) throw new Error("AZIONI: unknown scope " + a.scope + " of " + a.id);
    // the icon of the action: its own key, else the table of azioni-icone.js
    if (!a.iconKey && typeof AZIONI_ICONE !== "undefined") a.iconKey = AZIONI_ICONE.POR_ID[a.id];
    actions.push(a);
    return a;
  }
  // the decorative SVG of an action: its own `svg` (the map), else the icon `iconKey` of the table
  /** @param {Action} a @param {Context} c @returns {string} */
  const iconOf = (a, c) => (a.svg ? a.svg(c) : "") || (a.iconKey && typeof AZIONI_ICONE !== "undefined" ? AZIONI_ICONE.svg(a.iconKey) : "");
  /** @param {string} id */
  const get = id => actions.find(v => v.id === id);
  /** @param {Action} a @param {Context} c */
  const shown = (a, c) => (a.views.includes(c.view) || a.views.includes("*")) && (!a.visible || a.visible(c));
  /** @param {Action} a @param {Context} c */
  const ready = (a, c) => !a.available || a.available(c);
  // the actions of the view in the order of the groups (inside a group: `order`, then the order of registration); the menu lists the point actions, then the panel ones
  /** @param {Context} c @param {"menu" | "barra" | "all"} [where] */
  function list(c, where = "menu") {
    const l = actions.filter(a => shown(a, c) && (where === "all" || (where === "menu" ? a.menu !== false : a.barra === true)));
    return l.sort((a, b) => (where === "menu" ? block(a) - block(b) : 0) || GROUPS.indexOf(a.group) - GROUPS.indexOf(b.group) || (a.order || 0) - (b.order || 0) || actions.indexOf(a) - actions.indexOf(b));
  }
  // the entries of the right-click menu (menu() of explore.js), a separator between two groups; an action with `rows` gives several rows (one per open XIC…), all with its id
  /** @param {Context} c @returns {(MenuEntry | "-")[]} */
  function forContext(c) {
    /** @type {(MenuEntry | "-")[]} */
    const items = [];
    let last = "";
    for (const a of list(c)) {
      const sec = block(a) + a.group;                  // a separator between two groups and between the point block and the panel block
      if (a.rows) {
        const ok = ready(a, c), rs = a.rows(c);
        if (rs.length && last && sec !== last) items.push("-");
        if (rs.length) last = sec;
        rs.forEach(r => items.push({ id: a.id, scope: a.scope, label: r.name, aria: r.aria || r.name, tip: r.tip, icon: r.icon || iconOf(a, c), key: keyOf(a), check: r.check ?? null, dim: !ok || !!r.dim, off: !ok || (!!r.dim && !r.info), fn: r.run }));
        continue;
      }
      if (last && sec !== last) items.push("-");
      last = sec;
      const name = a.name ? a.name(c) : a.id;
      if (a.submenu) { items.push({ id: a.id, scope: a.scope, label: name, aria: name, icon: iconOf(a, c), check: null, dim: !ready(a, c), off: !ready(a, c), fn: () => {}, sub: () => a.submenu(c) }); continue; }
      items.push({ id: a.id, scope: a.scope, label: a.menuLabel ? a.menuLabel(c) : name, aria: a.aria ? a.aria(c) : name, tip: a.tip && a.tip(c), icon: iconOf(a, c), key: keyOf(a),
        check: a.checked ? a.checked(c) : null, dim: !ready(a, c), off: !ready(a, c), fn: () => run(a.id, c), fila: a.fila, soloIcona: !!a.fila });
    }
    // consecutive entries of the same `fila` share one row of the menu
    return items.reduce((/** @type {any[]} */ out, it) => {
      const prev = out[out.length - 1];
      if (it !== "-" && it.fila) {
        if (prev && prev.fila && prev.fila[0].fila === it.fila) prev.fila.push(it); else out.push({ fila: [it] });
      } else out.push(it);
      return out;
    }, []);
  }
  // the downloads of a view (`download: true`, never in the right-click menu): rows of the download button of the panel, only the available ones
  /** @param {Context} c */
  const downloads = c => list(c, "all").filter(a => a.download && ready(a, c)).map(a => ({ label: a.name ? a.name(c) : a.id, icon: iconOf(a, c), fn: () => run(a.id, c) }));
  /** @param {string} id @param {Context} c */
  function run(id, c) {
    const a = get(id);
    if (a && a.run && shown(a, c) && ready(a, c)) return a.run(c);
  }
  // the double click on a graph: the first available action that claims it (`doubleClick(c)` true), by `doubleClickOrder` (default 5); false when none does
  /** @param {Context} c */
  function doubleClick(c) {
    const l = list(c, "all").filter(a => a.doubleClick && ready(a, c) && a.doubleClick(c)).sort((a, b) => (a.doubleClickOrder ?? 5) - (b.doubleClickOrder ?? 5));
    if (!l.length) return false;
    run(l[0].id, c);
    return true;
  }
  return { format: "actions/1", GROUPS, actions, register, get, list, forContext, downloads, clean: cleanMenu, run, shown, ready, doubleClick, iconOf };
})();
// ---- shortcuts: the field `shortcut` of an action, ONE keydown for all of them (the functions are also added to AZIONI)
const KEYS = (() => {
  const { actions, shown, ready, run } = AZIONI;
  const mac = () => typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent || "");
  /** The text of a shortcut as the user's system writes it: ⌘O / ⌥1 on the Mac, Ctrl+O / Alt+1 elsewhere. @param {Shortcut} s */
  function keyText(s) {
    const n = String(s.key || "").replace("Arrow", "").replace("Left", "←").replace("Right", "→"), t = s.code ? s.code.replace(/^(Digit|Key)/, "") : n.length > 1 ? n : n.toUpperCase();
    if (mac()) return (s.mod ? "⌘" : "") + (s.alt ? "⌥" : "") + (s.shift ? "⇧" : "") + t;
    return [s.mod ? "Ctrl" : "", s.alt ? "Alt" : "", s.shift ? "Shift" : "", t].filter(Boolean).join("+");
  }
  /** Does this key event press that shortcut? Letters ignore case (Shift only if the shortcut asks for it); Alt+digit uses `code`. @param {Shortcut} s @param {KeyboardEvent} e */
  function pressed(s, e) {
    if (!!s.mod !== (e.ctrlKey || e.metaKey) || !!s.alt !== e.altKey) return false;
    if (s.code) return e.code === s.code && !!s.shift === e.shiftKey;
    const k = String(s.key || "");
    if (k === "?") return e.key === "?";
    return !e.shiftKey === !s.shift && (e.key || "").toLowerCase() === k.toLowerCase();
  }
  /** The actions with a shortcut for this view, in the order of registration. @param {string} view */
  const withKey = view => actions.filter(a => a.shortcut && (a.views.includes(view) || a.views.includes("*")));
  // the text field or dialog that keeps its own keys
  /** @param {EventTarget | null} t */
  const inTextField = t => !!document.querySelector("dialog[open]") || !!(t instanceof Element && t.closest("input,textarea,select,[contenteditable='true'],[contenteditable='']"));
  /** The panel whose chart has the focus (canvas, or the panel itself in full screen). @param {EventTarget | null} t */
  const panelOf = t => (typeof E === "undefined" || !(t instanceof Element)) ? null : E.panels.find((/** @type {any} */ q) => q.el && (q.cv === t || q.el === t)) || null;
  /** The context of a key on a chart: the point of the cursor (RT of the traces) or the highest visible peak (spectrum), as Shift+F10 does. @param {any} p @param {KeyboardEvent} e */
  function chartContext(p, e) {
    const a = p._a, c = /** @type {any} */ ({ view: p.type, p, e, near: null, a, has: false, m: null, lab: "…" });
    if (!a || !a.X) return c;
    let x = p.cur != null && p.cur >= a.x0 && p.cur <= a.x1 ? p.cur : (a.x0 + a.x1) / 2;
    if (p.type === "spec" && a.data && a.data[0]) {
      const d = a.data[0].d; let b = -1;
      for (let i = 0; i < d.mz.length; i++) if (d.mz[i] >= a.x0 && d.mz[i] <= a.x1 && (b < 0 || d.y[i] > d.y[b])) b = i;
      if (b >= 0) { x = d.mz[b]; c.has = true; c.m = x; c.lab = x.toFixed(typeof mzd === "function" ? mzd(p, p.level) : 2); }
      const ch = window.XICBTN ? XICBTN.chosen(p) : null;                       // the peak chosen for the XIC button wins over the highest one
      if (ch != null) { x = ch; c.has = true; c.m = x; c.lab = x.toFixed(typeof mzd === "function" ? mzd(p, p.level) : 2); }
    }
    c.x = x; c.px = a.X(x); c.py = p.cv.getBoundingClientRect().height / 2; c.k = p.type === "spec" ? 0 : typeof nearestFile === "function" ? nearestFile(p, x) : 0;
    return c;
  }
  /** @param {KeyboardEvent} e */
  function onKey(e) {
    if (e.defaultPrevented || e.isComposing || e.repeat || typeof E === "undefined") return;
    if (e.key === "Shift" || e.key === "Control" || e.key === "Alt" || e.key === "Meta") return;
    const combo = e.ctrlKey || e.metaKey || e.altKey;
    if (document.querySelector("dialog[open]")) return;
    if (!combo && inTextField(e.target)) return;
    // the letters of a chart: only with the focus on it
    const p = combo ? null : panelOf(e.target);
    const view = p ? p.type : (typeof S !== "undefined" && S.view) || "data";
    const c = p ? chartContext(p, e) : { view, p: E.active, e };
    for (const a of withKey(view)) {
      const s = /** @type {Shortcut} */ (a.shortcut);
      if ((s.where === "chart" && !p) || !pressed(s, e) || !shown(a, c) || !ready(a, c)) continue;
      e.preventDefault(); e.stopPropagation(); run(a.id, c); return;
    }
  }
  if (typeof document !== "undefined") document.addEventListener("keydown", onKey);
  return { keyText, pressed, withKey, inTextField, panelOf, chartContext };
})();
Object.assign(AZIONI, KEYS);
