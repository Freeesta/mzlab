// @ts-strict
// @ts-check
// Axis lock for the gestures of the maps (2D panel and 3D viewer). While a gesture runs, Shift held (or the latch X / Y) constrains it to ONE direction:
// "x" = only RT changes, "y" = only m/z changes. Held: the axis is chosen by the dominant direction of the first pixels (THR) and stays until release.
// Shift already STARTS a pan (Shift + drag): a pan is locked with Ctrl/Cmd + Shift instead. Latched: X (RT only) / Y (m/z only), also with Shift, toggle it,
// the same key again or Esc frees it; it also filters the arrows. Classic script, loaded before eventi.js.
const ASSE = (() => {
  const THR = 4;                                                   // px of movement before the held lock picks its axis
  /** @type {"x" | "y" | null} */ let fix = null;
  /** @param {string} k @param {any} [v] */ const T = (k, v) => I18N.t(k, v);
  /** @param {{ ctrlKey?: boolean, metaKey?: boolean, shiftKey?: boolean } | null | undefined} e @param {boolean} [pan] a pan (Shift + drag) needs Ctrl/Cmd on top */
  const held = (e, pan) => !!(e && (pan ? e.ctrlKey || e.metaKey : e.shiftKey));
  /** @param {string} m */
  const say = m => {
    let a = document.getElementById("sb-live");
    if (!a) { a = document.createElement("div"); a.id = "sb-live"; a.setAttribute("role", "status"); a.setAttribute("aria-live", "polite"); a.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap"; document.body.appendChild(a); }
    a.textContent = ""; setTimeout(() => { /** @type {HTMLElement} */ (a).textContent = m; }, 30);
  };
  /** The state of one gesture: the axis chosen so far. @param {boolean} [pan] */
  const gest = pan => ({ pan: !!pan, ax: /** @type {"x" | "y" | null} */ (null) });
  /** Constrains a movement (dx, dy in px since the start of the gesture or since the last step): returns what the gesture may use.
   *  @param {{ pan: boolean, ax: "x" | "y" | null }} g @param {{ ctrlKey?: boolean, metaKey?: boolean, shiftKey?: boolean }} e @param {number} dx @param {number} dy @param {number} [tx] @param {number} [ty] total movement since the start (the held lock decides on it) */
  function clip(g, e, dx, dy, tx = dx, ty = dy) {
    if (fix) g.ax = fix;
    else if (!held(e, g.pan)) { g.ax = null; return { dx, dy, ax: null }; }
    else if (!g.ax && Math.max(Math.abs(tx), Math.abs(ty)) >= THR) g.ax = Math.abs(tx) >= Math.abs(ty) ? "x" : "y";
    return g.ax === "x" ? { dx, dy: 0, ax: "x" } : g.ax === "y" ? { dx: 0, dy, ax: "y" } : { dx: 0, dy: 0, ax: null };       // held, axis not chosen yet: the gesture waits
  }
  /** Short text for the readout of the panel: the axis that is locked now ("" if none). @param {{ ax: "x" | "y" | null } | null} [g] */
  const tag = g => { const a = fix || (g && g.ax); return a ? " · " + T(a === "x" ? "asse.x" : "asse.y") : ""; };
  /** @param {"x" | "y" | null} m */
  function set(m) { fix = m; say(T(m === "x" ? "asse.on.x" : m === "y" ? "asse.on.y" : "asse.off")); document.dispatchEvent(new CustomEvent("asse")); }
  /** The latch keys of the maps; true = the key was ours. @param {KeyboardEvent} e */
  function key(e) {
    if (e.ctrlKey || e.metaKey || e.altKey) return false;                  // Shift + X / Y also latch
    const k = e.key;
    if (k === "x" || k === "X") { set(fix === "x" ? null : "x"); return true; }
    if (k === "y" || k === "Y") { set(fix === "y" ? null : "y"); return true; }
    if (k === "Escape" && fix) { set(null); return true; }
    return false;
  }
  return { gest, clip, tag, key, set, get fix() { return fix; }, held, THR };
})();
