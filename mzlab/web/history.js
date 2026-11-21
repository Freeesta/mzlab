// @ts-strict
// @ts-check
// One undo / redo stack for the Data view: Ctrl/Cmd+Z undoes the last action of the visible tab, Shift+Ctrl/Cmd+Z or Ctrl+Y does it again.
// Classic script, loaded after picchi.js. An entry = { n, p, kind, label, before, after }: a small snapshot of the touched part only
// (zoom, the JSON of p.ints, the correction fields), never traces, spectra or p._a. No persistence: reload empties it.
// order of the undoable actions (also used by arrange.js)
let HSEQ = 0;

/** The stack itself (no DOM, tested in Node): at most maxN entries / maxB characters of snapshots, the oldest are dropped. */
class HistoryStack {
  constructor(maxN = 100, maxB = 2e6) { this.maxN = maxN; this.maxB = maxB; /** @type {any[]} */ this.v = []; /** @type {any[]} */ this.r = []; this.bytes = 0; }
  /** @param {any} e */
  weight(e) { return String(e.before || "").length + String(e.after || "").length; }
  /** @param {any} e */
  push(e) {
    e.n = ++HSEQ; this.v.push(e); this.r.length = 0; this.bytes += this.weight(e);
    while (this.v.length > 1 && (this.v.length > this.maxN || this.bytes > this.maxB)) this.bytes -= this.weight(/** @type {any} */ (this.v.shift()));
    return e;
  }
  /** @param {any[]} l @param {(e: any) => boolean} ok */
  static last(l, ok) { for (let i = l.length - 1; i >= 0; i--) if (ok(l[i])) return i; return -1; }
  /** Takes the last entry that passes ok, lets fn(e, state) restore `before` and return the state it replaced. @param {(e: any) => boolean} ok @param {(e: any, st: any) => any} fn */
  undo(ok, fn) {
    const i = HistoryStack.last(this.v, ok); if (i < 0) return null;
    const e = this.v.splice(i, 1)[0]; this.bytes -= this.weight(e); e.after = fn(e, e.before); this.r.push(e); return e;
  }
  /** @param {(e: any) => boolean} ok @param {(e: any, st: any) => any} fn */
  redo(ok, fn) {
    const i = HistoryStack.last(this.r, ok); if (i < 0) return null;
    const e = this.r.splice(i, 1)[0]; e.before = fn(e, e.after); e.n = ++HSEQ; this.v.push(e); this.bytes += this.weight(e); return e;
  }
  /** Drops an entry for good (a removal whose undo window has closed). @param {any} e */
  drop(e) { const i = this.v.indexOf(e); if (i >= 0) { this.v.splice(i, 1); this.bytes -= this.weight(e); } const j = this.r.indexOf(e); if (j >= 0) this.r.splice(j, 1); }
  clear() { this.v = []; this.r = []; this.bytes = 0; }
}

const HISTORY = typeof document === "undefined" ? null : (() => {
  /** @type {{ id: number, ps: Set<any> } | null} */ let batch = null;
  const stack = new HistoryStack(), KEYS = ["bk", "bg", "fondo", "smooth", "corr", "ref", "bgr"];
  /** @param {string} k @param {any} [v] */ const T = (k, v) => I18N.t(k, v);
  /** The live region of the bar (barra.js creates the same one). @param {string} m */
  const say = m => {
    let a = document.getElementById("sb-live");
    if (!a) { a = document.createElement("div"); a.id = "sb-live"; a.setAttribute("role", "status"); a.setAttribute("aria-live", "polite"); a.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap"; document.body.appendChild(a); }
    a.textContent = ""; setTimeout(() => { /** @type {HTMLElement} */ (a).textContent = m; }, 30);
  };
  const fullscreen = () => E.panels.find((/** @type {any} */ q) => q.el && q.el.classList.contains("max"));
  /** Visible now: the panel is open, in the visible tab, and in full screen only that panel. @param {any} e */
  // a removed file or closed panel belongs to a tab (remove-all to none), not to a live panel
  const visible = e => { if (e.kind === "removed" && e.p === undefined) return !fullscreen() && (e.tab === undefined || e.tab === E.tab);
    if (e.p && e.p.global) return !fullscreen();
    if (!E.panels.includes(e.p)) return false; const m = fullscreen(); return m ? e.p === m : (e.p.tab || E.tab) === E.tab; };
  /** @param {any} p @param {string} kind @param {any} [snap] */
  const capture = (p, kind, snap) => kind === "int" ? (snap ?? intSnap(p))
    : kind === "fields" ? JSON.stringify(Object.fromEntries(KEYS.filter(k => p[k] !== undefined).map(k => [k, p[k]])))
    : JSON.stringify({ zoom: p.zoom || null, zoomY: p.zoomY || null, meas: p.meas || null, rul: !!p.rul });
  /** Puts a snapshot back and returns the one it replaced. @param {any} e @param {string} st */
  const restore = (e, st) => {
    // before = "U" + data (its weight), after = "R"
    if (e.kind === "removed") { if (st.charAt(0) === "U") { e.undo(); return "R"; } e.redo(); return "U" + e.data; }
    const p = e.p, cur = e.kind === "cmd" ? "" : capture(p, e.kind), o = e.kind === "cmd" ? null : JSON.parse(st);
    if (e.kind === "int") { p.ints = Array.isArray(o) ? o : o.i; p.noise = Array.isArray(o) ? p.noise : o.n; p.isel = null; }
    else if (e.kind === "fields") { KEYS.forEach(k => { if (k in o) p[k] = o[k]; else delete p[k]; }); if (typeof ctl === "function") ctl(p); if (typeof syncTb === "function") syncTb(p); }
    else if (e.kind === "cmd") { const g = COMMANDS.handlerOf(e.cmd); const prev = g.read(p); COMMANDS.restore(e.cmd, p, st); if (!p.global) draw(p); uiSave(); return prev; }
    else { p.zoom = o.zoom; p.zoomY = o.zoomY; if (p.type === "spec") { p.lock = null; if ("meas" in o) { p.meas = o.meas; p.rul = o.rul; const b = p.el.querySelector('[data-a="rul"]'); if (b) b.classList.toggle("on", !!o.rul); } } }
    draw(p); uiSave(); return cur;
  };
  const label = (/** @type {any} */ e) => e.kind === "removed" ? e.what : e.kind === "cmd" ? T(e.w) : T(e.w || (e.kind === "int" ? "history.int" : e.kind === "fields" ? "history.fields" : "history.zoom"));
  function sync() {
    const u = document.getElementById("np-undo"), r = document.getElementById("np-redo");
    if (u) /** @type {HTMLButtonElement} */ (u).disabled = HistoryStack.last(stack.v, visible) < 0 && !(window.ARRANGE && ARRANGE.last() > 0);
    if (r) /** @type {HTMLButtonElement} */ (r).disabled = HistoryStack.last(stack.r, visible) < 0;
  }
  /** Records the state of p BEFORE a change. @param {any} p @param {string} kind @param {string} [w] i18n key of the label @param {any} [snap] */
  function record(p, kind, w, snap) {
    if (batch) { if (batch.ps.has(p)) return; batch.ps.add(p); }
    const before = capture(p, kind, snap);
    if (kind === "zoom") { const l = stack.v[HistoryStack.last(stack.v, e => e.p === p && e.kind === "zoom")]; if (l && l.before === before) return; }
    stack.push({ p, kind, w, before, after: null, g: batch ? batch.id : 0 }); sync();
  }
  /** One gesture = one undo step: every "int" entry recorded while fn runs (one per panel, mirrors included) is undone and redone together. @param {() => void} fn */
  function group(fn) {
    if (batch) { fn(); return; }
    batch = { id: ++HSEQ, ps: new Set() };
    try { fn(); } finally { batch = null; }
  }
  /** Records the step of a registered command (commands.js) whose state BEFORE the change is `before`; steps with the same `merge` on the same panel less than 0.8 s apart are one (a drag). @param {string} id @param {any} p @param {string} before @param {string} [merge] */
  function command(id, p, before, merge) {
    const l = stack.v[stack.v.length - 1], t = Date.now();
    if (merge && l && l.kind === "cmd" && l.cmd === id && l.p === p && l.merge === merge && t - l.t < 800) { l.t = t; stack.r.length = 0; sync(); return; }
    stack.push({ p, kind: "cmd", cmd: id, w: COMMANDS.row(id)?.name, merge: merge || "", t, before, after: null }); sync();
  }
  /** @param {(e: any) => boolean} [only] */
  function undo(only) {
    const ok = only ? (/** @type {any} */ e) => visible(e) && only(e) : visible, i = HistoryStack.last(stack.v, ok), top = i < 0 ? 0 : stack.v[i].n;
    // the last thing done was a change of the arrangement
    if (!only && window.ARRANGE && ARRANGE.last() > top) { ARRANGE.undo(); sync(); return true; }
    const e = stack.undo(ok, restore);
    if (!e) { if (!only) say(T("history.nothingToUndo")); return false; }
    if (e.g) while (stack.undo(x => x.g === e.g, restore));
    taken(e); say(T("history.undone", { what: label(e) })); sync(); return true;
  }
  /** The undo of a removal is done: its timer stops and the copy in the store is not freed. @param {any} e */
  function taken(e) { if (e.kind !== "removed") return; e.expires = null; if (e.close) e.close(); }
  /** A removal has no visible notice (the undo/redo buttons and Ctrl+Z are enough): it is announced to screen readers, and when it keeps something to free (`expires`) the removal becomes final after 6 s: the entry leaves the stack and `expires` frees what it kept. @param {any} e */
  function notice(e) {
    say(T("history.removed", { what: e.what }));
    if (!e.expires) return;
    const t = setTimeout(() => { e.close = null; const f = e.expires; e.expires = null; if (f) { stack.drop(e); sync(); f(); } }, 6000);
    e.close = () => { clearTimeout(t); e.close = null; };
  }
  /** Records a removal that can be taken back and shows its notice. o = { what, tab?, p?, data, undo, redo, expires? }: `tab` ties it to a tab (a closed panel), `p` to a live panel (the marks), `expires` runs once the notice has closed without an undo. @param {any} o */
  function removed(o) {
    const e = stack.push({ kind: "removed", what: o.what, tab: o.tab, p: o.p, data: o.data || "", before: "U" + (o.data || ""), after: null, undo: o.undo, redo: o.redo, expires: o.expires || null, close: null });
    sync(); notice(e); return e;
  }
  /** A panel the student opened: the undo closes it, the redo opens it again (the same entry as a removal with the two closures the other way round, and no notice). o = { what, tab, data, undo, redo, follows? }: `follows(previous, next)` follows a panel that was closed and opened again. @param {any} o */
  function opened(o) {
    const e = stack.push({ kind: "removed", what: o.what, tab: o.tab, data: o.data || "", before: "U" + (o.data || ""), after: null, undo: o.undo, redo: o.redo, follows: o.follows, expires: null, close: null });
    sync(); return e;
  }
  /** A closed panel is open again as another object: the entries of the old one follow it. @param {any} previous @param {any} next */
  function follow(previous, next) { stack.v.concat(stack.r).forEach(e => { if (e.p === previous && e.kind !== "removed") e.p = next; if (e.follows) e.follows(previous, next); }); sync(); }
  function redo() {
    const e = stack.redo(visible, restore);
    if (!e) { say(T("history.nothingToRedo")); return false; }
    if (e.g) while (stack.redo(x => x.g === e.g, restore));
    say(T("history.redone", { what: label(e) })); sync(); return true;
  }
  // text entry keeps the undo of the browser; a button, checkbox, range or select does not (the key must reach the stack)
  const TEXT_INPUT = "input:not([type=checkbox],[type=radio],[type=range],[type=button],[type=submit],[type=reset],[type=color],[type=file],[type=image]),textarea,[contenteditable='true'],[contenteditable='']";
  const idle = (/** @type {Element | null} */ t) => !!(t && t.closest && t.closest(TEXT_INPUT)) || !!document.querySelector("dialog[open]");
  // the panels container is hidden when every file was removed, but the key must still undo that removal
  // the one-page Data view (drop area and sample table, S.adding) hides the panels and BARRA.isDataView() is false there, yet the stack is what Ctrl+Z must reach
  const inData = () => { const d = document.getElementById("dpanels"); return !!(d && d.offsetParent) || !!(window.BARRA && BARRA.isDataView()) || (S.view === "data" && !!S.sess); };
  document.addEventListener("keydown", e => {
    const k = (e.key || "").toLowerCase(), mod = e.ctrlKey || e.metaKey;
    if (!mod || e.altKey || (k !== "z" && k !== "y") || (k === "y" && (e.shiftKey || e.metaKey))) return;
    // the browser undoes its own text
    if (idle(/** @type {Element | null} */ (e.target))) return;
    const isRedo = k === "y" || e.shiftKey;
    // always intercepted: Safari would reopen the closed tab
    if (inData()) { e.preventDefault(); if (isRedo) redo(); else undo(); return; }
    const kf = document.getElementById("kframe");
    // Disegno with the focus outside the editor
    if (kf && kf.offsetParent && window.TPDraw && TPDraw.undo) { e.preventDefault(); TPDraw.undo(isRedo); }
  });
  // the buttons of the bar do what the keys do
  const bu = document.getElementById("np-undo"), br = document.getElementById("np-redo");
  if (bu) bu.addEventListener("click", () => { undo(); });
  if (br) br.addEventListener("click", () => { redo(); });
  return { record, group, command, removed, opened, follow, undo, redo, sync, clear: () => { stack.clear(); sync(); }, stack };
})();

// thin wrappers: the callers (eventi.js, pannelli.js, dda.js, mappa.js, banco.js) keep their names
/** @param {any} p */
function pushZh(p) { if (HISTORY) HISTORY.record(p, "zoom"); }
/** @param {any} p @param {string} [snap] */
function pushIh(p, snap) { if (HISTORY) HISTORY.record(p, "int", undefined, snap); }
/** @param {any} p @param {string} [w] i18n key of the label */
function pushFields(p, w) { if (HISTORY) HISTORY.record(p, "fields", w); }
/** @param {any} p */
function undoZoom(p) { if (HISTORY) HISTORY.undo(e => e.p === p && e.kind === "zoom"); }
