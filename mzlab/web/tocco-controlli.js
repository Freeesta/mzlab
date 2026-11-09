// @ts-check
// Touch and pen outside the graphs (lazy ES module, loaded by touch.js at the first touch): a double tap is a double click, as touch.js does on the canvases.
// iOS sends no dblclick for a double tap, so renaming a panel or a trace, resetting a divider or opening a table row had no finger path.
// While the soft keyboard is up, a modal dialog sits in the part of the window the keyboard leaves.
// The controls with a double-click action have no double-tap zoom (app.css, touch-action:manipulation), so both taps arrive as clicks; elsewhere the page zooms as usual.
// A long press on the rows that have a right-click menu opens it (iPadOS has no long-press contextmenu; Android sends its own, which then wins).
const MOVE = 8, DTAP = 350, NEAR = 28, LONG = 550;
const MENU_ROWS = "#inf-tree tbody tr, #mincl tr[data-i], #flst .fgh[data-ug]";
const W = /** @type {any} */ (window);
const dblclick = (/** @type {Element} */ el, /** @type {number} */ x, /** @type {number} */ y) =>
  el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true, view: window, clientX: x, clientY: y, button: 0, buttons: 0, detail: 2 }));
const isTouch = (/** @type {PointerEvent} */ e) => e.pointerType === "touch" || e.pointerType === "pen";

// ---- a double tap = a double click, outside the graphs (touch.js does theirs) and the file list (lista-file.js pairs its own clicks)
let lastUp = 0, fired = 0;
/** @type {{ t: number, x: number, y: number, el: Element | null }} */ let tap = { t: 0, x: 0, y: 0, el: null };
/** @type {{ x: number, y: number, t: number } | null} */ let down = null;
addEventListener("pointerdown", e => { down = isTouch(e) && e.isPrimary ? { x: e.clientX, y: e.clientY, t: Date.now() } : null; if (e.pointerType === "mouse") lastUp = 0; }, true);   // a mouse click right after a touch is the mouse's
addEventListener("pointerup", e => {
  if (!down || !isTouch(e) || !e.isPrimary) return;
  const d = down; down = null;
  if (Math.hypot(e.clientX - d.x, e.clientY - d.y) > MOVE || Date.now() - d.t > 500) { tap.t = 0; return; }   // a drag or a long press is not a tap
  lastUp = Date.now();
}, true);
document.addEventListener("click", e => {
  if (!e.isTrusted || Date.now() - lastUp > 700) return;
  const el = /** @type {Element} */ (e.target);
  if (!el.closest || el.closest(".pnl canvas, #flst, input, select, textarea, option, [contenteditable]")) { tap.t = 0; return; }
  const dbl = Date.now() - tap.t < DTAP && Math.hypot(e.clientX - tap.x, e.clientY - tap.y) < NEAR && !!tap.el && (tap.el === el || tap.el.contains(el) || el.contains(tap.el));
  if (!dbl) { tap = { t: Date.now(), x: e.clientX, y: e.clientY, el }; return; }
  tap.t = 0; fired = Date.now();
  const x = e.clientX, y = e.clientY;
  setTimeout(() => { if (el.isConnected) dblclick(el, x, y); }, 0);   // after the second click has run, as the mouse does
}, true);
// the browser's own double click, when it sends one, would run the action twice
document.addEventListener("dblclick", e => { if (e.isTrusted && Date.now() - fired < 700) { e.stopImmediatePropagation(); e.preventDefault(); } }, true);

// ---- a long press on a row with a menu = right click
/** @type {{ id: number, x: number, y: number, el: Element, t: number } | null} */ let lp = null;
let menuAt = 0;
document.addEventListener("pointerdown", e => {
  if (!isTouch(e) || !e.isPrimary) return;
  const el = /** @type {Element} */ (e.target), row = el.closest && el.closest(MENU_ROWS); if (!row) return;
  const x = e.clientX, y = e.clientY;
  lp = { id: e.pointerId, x, y, el, t: window.setTimeout(() => {
    if (!lp) return; lp = null; menuAt = Date.now();
    el.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, view: window, clientX: x, clientY: y, button: 2, buttons: 0 }));
    if (navigator.vibrate) { try { navigator.vibrate(15); } catch (_) { /* none */ } }
  }, LONG) };
}, true);
const lpStop = () => { if (lp) { clearTimeout(lp.t); lp = null; } };
addEventListener("pointermove", e => { if (lp && e.pointerId === lp.id && Math.hypot(e.clientX - lp.x, e.clientY - lp.y) > MOVE) lpStop(); }, true);
addEventListener("pointerup", lpStop, true); addEventListener("pointercancel", lpStop, true);
// Android's own long-press menu comes first: it is the one used; after ours, the compat click of the lift would close the menu just opened
document.addEventListener("contextmenu", e => { if (e.isTrusted && lp) lpStop(); }, true);
["mousedown", "mouseup", "click"].forEach(t => document.addEventListener(t, e => { if (e.isTrusted && Date.now() - menuAt < 700) { e.stopPropagation(); e.preventDefault(); if (t === "click") menuAt = 0; } }, true));

// ---- the labels of the spectrum at the side of the map tell m/z and S/N in a hover title (mappa.js): a tap shows it in the tooltip of the panel
let lblT = 0;
document.addEventListener("click", e => {
  const sp = /** @type {HTMLElement} */ (e.target);
  if (!sp.matches || !sp.matches(".pnl canvas.mzside") || !document.documentElement.classList.contains("ptr-touch")) return;
  sp.dispatchEvent(new MouseEvent("mousemove", { bubbles: true, clientX: e.clientX, clientY: e.clientY, view: window }));   // mappa.js writes the title of the label under the finger
  const t = sp.title, pnl = /** @type {HTMLElement} */ (sp.closest(".pnl")), tip = /** @type {HTMLElement | null} */ (pnl.querySelector(".tip")); if (!t || !tip) return;
  const r = pnl.getBoundingClientRect(); tip.textContent = t; tip.hidden = false;
  tip.style.left = Math.max(4, Math.min(pnl.clientWidth - tip.offsetWidth - 4, e.clientX - r.left - tip.offsetWidth - 10)) + "px"; tip.style.top = Math.max(4, e.clientY - r.top - 30) + "px";
  clearTimeout(lblT); lblT = window.setTimeout(() => { tip.hidden = true; }, 2800);
}, true);

// ---- the soft keyboard: iOS keeps the layout viewport and shrinks only the visual one, so a modal dialog centred in the window sat under the keyboard
// with its field and its OK button; while a keyboard is up (over 100 px gone, no pinch zoom) html.kb and --vv-top/--vv-bottom (app.css) hold it in the band left above
function keyboardBand() {
  const v = window.visualViewport; if (!v) return;
  const root = document.documentElement; let was = "";
  const fit = () => {
    const top = Math.max(0, v.offsetTop), below = Math.max(0, root.clientHeight - v.offsetTop - v.height), up = v.scale < 1.01 && below > 100;
    const now = up ? `${Math.round(top)}/${Math.round(below)}` : ""; if (now === was) return; was = now;
    root.classList.toggle("kb", up); root.style.setProperty("--vv-top", up ? top + "px" : "0px"); root.style.setProperty("--vv-bottom", up ? below + "px" : "0px");
    const f = /** @type {HTMLElement | null} */ (document.activeElement);
    if (up && f && f.closest && f.closest("dialog[open]")) FRAME.next(() => f.scrollIntoView({ block: "nearest" }));   // the dialog got shorter: its field stays in sight
  };
  v.addEventListener("resize", fit); v.addEventListener("scroll", fit); fit();
}
keyboardBand();

W.TOCCO_CONTROLLI = { fired: () => fired };
export {};
