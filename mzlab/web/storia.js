// @ts-strict
// @ts-check
// One undo / redo stack for the Dati view: Ctrl/Cmd+Z undoes the last action of the visible tab, Shift+Ctrl/Cmd+Z or Ctrl+Y does it again.
// Classic script, loaded after picchi.js. An entry = { n, p, tipo, etichetta, prima, dopo }: a small snapshot of the touched part only
// (zoom, the JSON of p.ints, the correction fields), never traces, spectra or p._a. No persistence: reload empties it.
let HSEQ = 0;                                                             // order of the undoable actions (also used by affianca.js)

/** The stack itself (no DOM, tested in Node): at most maxN entries / maxB characters of snapshots, the oldest are dropped. */
class StoriaPila {
  constructor(maxN = 100, maxB = 2e6) { this.maxN = maxN; this.maxB = maxB; /** @type {any[]} */ this.v = []; /** @type {any[]} */ this.r = []; this.bytes = 0; }
  /** @param {any} e */
  peso(e) { return String(e.prima || "").length + String(e.dopo || "").length; }
  /** @param {any} e */
  push(e) {
    e.n = ++HSEQ; this.v.push(e); this.r.length = 0; this.bytes += this.peso(e);
    while (this.v.length > 1 && (this.v.length > this.maxN || this.bytes > this.maxB)) this.bytes -= this.peso(/** @type {any} */ (this.v.shift()));
    return e;
  }
  /** @param {any[]} l @param {(e: any) => boolean} ok */
  static ultimo(l, ok) { for (let i = l.length - 1; i >= 0; i--) if (ok(l[i])) return i; return -1; }
  /** Takes the last entry that passes ok, lets fn(e, state) restore `prima` and return the state it replaced. @param {(e: any) => boolean} ok @param {(e: any, st: any) => any} fn */
  annulla(ok, fn) {
    const i = StoriaPila.ultimo(this.v, ok); if (i < 0) return null;
    const e = this.v.splice(i, 1)[0]; this.bytes -= this.peso(e); e.dopo = fn(e, e.prima); this.r.push(e); return e;
  }
  /** @param {(e: any) => boolean} ok @param {(e: any, st: any) => any} fn */
  ripeti(ok, fn) {
    const i = StoriaPila.ultimo(this.r, ok); if (i < 0) return null;
    const e = this.r.splice(i, 1)[0]; e.prima = fn(e, e.dopo); e.n = ++HSEQ; this.v.push(e); this.bytes += this.peso(e); return e;
  }
  /** Drops an entry for good (a removal whose undo window has closed). @param {any} e */
  togli(e) { const i = this.v.indexOf(e); if (i >= 0) { this.v.splice(i, 1); this.bytes -= this.peso(e); } const j = this.r.indexOf(e); if (j >= 0) this.r.splice(j, 1); }
  svuota() { this.v = []; this.r = []; this.bytes = 0; }
}

const STORIA = typeof document === "undefined" ? null : (() => {
  const pila = new StoriaPila(), KEYS = ["bk", "bg", "fondo", "smooth", "corr", "ref", "bgr"];
  /** @param {string} k @param {any} [v] */ const T = (k, v) => I18N.t(k, v);
  /** The live region of the bar (barra.js creates the same one). @param {string} m */
  const say = m => {
    let a = document.getElementById("sb-live");
    if (!a) { a = document.createElement("div"); a.id = "sb-live"; a.setAttribute("role", "status"); a.setAttribute("aria-live", "polite"); a.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap"; document.body.appendChild(a); }
    a.textContent = ""; setTimeout(() => { /** @type {HTMLElement} */ (a).textContent = m; }, 30);
  };
  const fullscreen = () => E.panels.find((/** @type {any} */ q) => q.el && q.el.classList.contains("max"));
  /** Visible now: the panel is open, in the visible tab, and in full screen only that panel. @param {any} e */
  const visibile = e => { if (e.tipo === "tolto" && e.tab !== undefined) return !fullscreen() && e.tab === E.tab;      // a removed file or closed panel belongs to a tab, not to a live panel
    if (!E.panels.includes(e.p)) return false; const m = fullscreen(); return m ? e.p === m : (e.p.tab || E.tab) === E.tab; };
  /** @param {any} p @param {string} tipo @param {any} [snap] */
  const cattura = (p, tipo, snap) => tipo === "int" ? (snap ?? intSnap(p))
    : tipo === "campi" ? JSON.stringify(Object.fromEntries(KEYS.filter(k => p[k] !== undefined).map(k => [k, p[k]])))
    : JSON.stringify({ zoom: p.zoom || null, zoomY: p.zoomY || null });
  /** Puts a snapshot back and returns the one it replaced. @param {any} e @param {string} st */
  const ripristina = (e, st) => {
    if (e.tipo === "tolto") { if (st.charAt(0) === "U") { e.undo(); return "R"; } e.redo(); return "U" + e.dati; }     // prima = "U" + data (its weight), dopo = "R"
    const p = e.p, cur = cattura(p, e.tipo), o = JSON.parse(st);
    if (e.tipo === "int") { p.ints = o; p.isel = null; }
    else if (e.tipo === "campi") { KEYS.forEach(k => { if (k in o) p[k] = o[k]; else delete p[k]; }); if (typeof ctl === "function") ctl(p); if (typeof syncTb === "function") syncTb(p); }
    else { p.zoom = o.zoom; p.zoomY = o.zoomY; if (p.type === "spec") p.lock = null; }
    draw(p); uiSave(); return cur;
  };
  const etichetta = (/** @type {any} */ e) => e.tipo === "tolto" ? e.what : T(e.w || (e.tipo === "int" ? "storia.int" : e.tipo === "campi" ? "storia.campi" : "storia.zoom"));
  function sync() {
    const u = document.getElementById("np-undo"), r = document.getElementById("np-redo");
    if (u) /** @type {HTMLButtonElement} */ (u).disabled = StoriaPila.ultimo(pila.v, visibile) < 0 && !(window.AFFIANCA && AFFIANCA.ultimo() > 0);
    if (r) /** @type {HTMLButtonElement} */ (r).disabled = StoriaPila.ultimo(pila.r, visibile) < 0;
  }
  /** Records the state of p BEFORE a change. @param {any} p @param {string} tipo @param {string} [w] i18n key of the label @param {any} [snap] */
  function registra(p, tipo, w, snap) {
    const prima = cattura(p, tipo, snap);
    if (tipo === "zoom") { const l = pila.v[StoriaPila.ultimo(pila.v, e => e.p === p && e.tipo === "zoom")]; if (l && l.prima === prima) return; }
    pila.push({ p, tipo, w, prima, dopo: null }); sync();
  }
  /** @param {(e: any) => boolean} [solo] */
  function annulla(solo) {
    const ok = solo ? (/** @type {any} */ e) => visibile(e) && solo(e) : visibile, i = StoriaPila.ultimo(pila.v, ok), top = i < 0 ? 0 : pila.v[i].n;
    if (!solo && window.AFFIANCA && AFFIANCA.ultimo() > top) { AFFIANCA.annulla(); sync(); return true; }      // the last thing done was a change of the arrangement
    const e = pila.annulla(ok, ripristina);
    if (!e) { if (!solo) say(T("storia.niente")); return false; }
    presa(e); say(T("storia.undone", { what: etichetta(e) })); sync(); return true;
  }
  /** The undo of a removal is done: its notice goes away and the copy in the store is not freed. @param {any} e */
  function presa(e) { if (e.tipo !== "tolto") return; e.scade = null; if (e.chiudi) e.chiudi(); }
  /** Takes back one removal (the button of its notice). @param {any} e */
  function annullaVoce(e) {
    const r = pila.annulla(x => x === e, ripristina); if (!r) return false;
    presa(r); say(T("storia.undone", { what: etichetta(r) })); sync(); return true;
  }
  /** The notice «Removed: x · Undo» (6 s, role=status); when it closes by itself the removal is final: the entry leaves the stack and `scade` frees what the removal kept. @param {any} e */
  function avviso(e) {
    let box = document.getElementById("tolto-box");
    if (!box) { box = document.createElement("div"); box.id = "tolto-box"; box.setAttribute("role", "status"); box.setAttribute("aria-live", "polite"); document.body.appendChild(box); }
    const n = document.createElement("div"), b = document.createElement("button"), t = document.createElement("span");
    n.className = "tolto"; t.textContent = T("storia.tolto", { what: e.what }); b.type = "button"; b.textContent = T("storia.annulla"); b.onclick = () => { annullaVoce(e); };
    n.append(t, b); box.appendChild(n);
    e.chiudi = window.avvisoAuto ? avvisoAuto(n, { ms: 6000, x: false, onClose: () => { e.chiudi = null; const f = e.scade; e.scade = null; if (f) { pila.togli(e); sync(); f(); } } }) : () => n.remove();
    if (!window.avvisoAuto) setTimeout(() => { if (e.chiudi) e.chiudi(); }, 6000);
  }
  /** Records a removal that can be taken back and shows its notice. o = { what, tab?, p?, dati, undo, redo, scade? }: `tab` ties it to a tab (a file, a closed panel), `p` to a live panel (the marks), `scade` runs once the notice has closed without an undo. @param {any} o */
  function tolto(o) {
    const e = pila.push({ tipo: "tolto", what: o.what, tab: o.tab, p: o.p, dati: o.dati || "", prima: "U" + (o.dati || ""), dopo: null, undo: o.undo, redo: o.redo, scade: o.scade || null, chiudi: null });
    sync(); avviso(e); return e;
  }
  /** A closed panel is open again as another object: the entries of the old one follow it. @param {any} vecchio @param {any} nuovo */
  function segui(vecchio, nuovo) { pila.v.concat(pila.r).forEach(e => { if (e.p === vecchio && e.tipo !== "tolto") e.p = nuovo; }); sync(); }
  function ripeti() {
    const e = pila.ripeti(visibile, ripristina);
    if (!e) { say(T("storia.nienteRipeti")); return false; }
    say(T("storia.redone", { what: etichetta(e) })); sync(); return true;
  }
  const idle = (/** @type {Element | null} */ t) => !!(t && t.closest && t.closest("input,textarea,select,[contenteditable='true'],[contenteditable='']")) || !!document.querySelector("dialog[open]");
  const inDati = () => { const d = document.getElementById("dpanels"); return !!(d && d.offsetParent); };
  document.addEventListener("keydown", e => {
    const k = e.key.toLowerCase(), mod = e.ctrlKey || e.metaKey;
    if (!mod || e.altKey || (k !== "z" && k !== "y") || (k === "y" && (e.shiftKey || e.metaKey))) return;
    if (idle(/** @type {Element | null} */ (e.target))) return;                                     // the browser undoes its own text
    const redo = k === "y" || e.shiftKey;
    if (inDati()) { e.preventDefault(); if (redo) ripeti(); else annulla(); return; }                // always intercepted: Safari would reopen the closed tab
    const kf = document.getElementById("kframe");
    if (kf && kf.offsetParent && window.TPDraw && TPDraw.undo) { e.preventDefault(); TPDraw.undo(redo); }   // Disegno with the focus outside the editor
  });
  return { registra, tolto, segui, annulla, ripeti, sync, svuota: () => { pila.svuota(); sync(); }, pila };
})();

// thin wrappers: the callers (eventi.js, pannelli.js, dda.js, mappa.js, banco.js) keep their names
/** @param {any} p */
function pushZh(p) { if (STORIA) STORIA.registra(p, "zoom"); }
/** @param {any} p @param {string} [snap] */
function pushIh(p, snap) { if (STORIA) STORIA.registra(p, "int", undefined, snap); }
/** @param {any} p @param {string} [w] i18n key of the label */
function pushCampi(p, w) { if (STORIA) STORIA.registra(p, "campi", w); }
/** @param {any} p */
function undoZoom(p) { if (STORIA) STORIA.annulla(e => e.p === p && e.tipo === "zoom"); }
