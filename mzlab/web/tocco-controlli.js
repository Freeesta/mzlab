// @ts-check
// Touch and pen outside the graphs (lazy ES module, loaded by touch.js at the first touch): a double tap is a double click, as touch.js does on the canvases.
// iOS sends no dblclick for a double tap, so renaming a panel or a trace, resetting a divider or opening a table row had no finger path.
// The controls with a double-click action have no double-tap zoom (app.css, touch-action:manipulation), so both taps arrive as clicks; elsewhere the page zooms as usual.
const MOVE = 8, DTAP = 350, NEAR = 28;
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

W.TOCCO_CONTROLLI = { fired: () => fired };
export {};
