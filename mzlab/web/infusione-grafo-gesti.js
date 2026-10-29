// @ts-strict
// Pan and zoom of the canvas of the «Fragmentation graph» with the gestures of a trackpad, a mouse and a touch screen: two fingers = scroll, pinch or Ctrl/Cmd + wheel = zoom
// around the pointer, drag = pan, pinch with two touches = zoom. The page scrolls as usual outside the canvas; inside, the gesture never scrolls the page nor zooms the browser.

/** @typedef {{ get: () => number, set: (s: number) => void }} Escala */

const LINE = 16;

/** Zooms to `scale` keeping under the point (`x`, `y`) of the viewport of `wrap` the same place of the drawing. */
function zoomAt(/** @type {HTMLElement} */ wrap, /** @type {Escala} */ sc, /** @type {number} */ scale, /** @type {number} */ x, /** @type {number} */ y) {
  const before = sc.get(); sc.set(scale); const k = sc.get() / before;
  if (k === 1) return;
  wrap.scrollLeft = (wrap.scrollLeft + x) * k - x; wrap.scrollTop = (wrap.scrollTop + y) * k - y;
}

export function wireGestures(/** @type {HTMLElement} */ wrap, /** @type {Escala} */ sc) {
  const at = (/** @type {number} */ cx, /** @type {number} */ cy) => { const r = wrap.getBoundingClientRect(); return [cx - r.left, cy - r.top]; };
  // wheel: with Ctrl/Cmd (a pinch on a trackpad arrives so) zoom, else pan; a non passive listener, only on the canvas
  wrap.addEventListener("wheel", e => {
    e.preventDefault();
    const m = e.deltaMode === 1 ? LINE : e.deltaMode === 2 ? wrap.clientHeight : 1;
    if (e.ctrlKey || e.metaKey) { const [x, y] = at(e.clientX, e.clientY); zoomAt(wrap, sc, sc.get() * Math.exp(-e.deltaY * m * (e.ctrlKey ? 0.01 : 0.002)), x, y); } else wrap.scrollBy(e.deltaX * m, e.deltaY * m);
  }, { passive: false });
  // Safari: the pinch is a gesture* event
  /** @type {number} */ let g0 = 1;
  wrap.addEventListener("gesturestart", e => { e.preventDefault(); g0 = sc.get(); });
  wrap.addEventListener("gesturechange", e => { e.preventDefault(); const [x, y] = at(/** @type {any} */ (e).clientX, /** @type {any} */ (e).clientY); zoomAt(wrap, sc, g0 * /** @type {any} */ (e).scale, x, y); });
  wrap.addEventListener("gestureend", e => e.preventDefault());
  // pointers: one = pan (after 4 px, so a click on an ion still selects it), two = pinch
  /** @type {Map<number, { x: number, y: number }>} */ const ptr = new Map();
  /** @type {{ d: number, s: number } | null} */ let pinch = null;
  let moved = false;
  const dist = () => { const [a, b] = [...ptr.values()]; return Math.hypot(a.x - b.x, a.y - b.y); };
  wrap.addEventListener("pointerdown", e => {
    if (e.pointerType === "mouse" && e.button !== 0) return;
    ptr.set(e.pointerId, { x: e.clientX, y: e.clientY }); moved = false;
    if (ptr.size === 2) pinch = { d: dist() || 1, s: sc.get() };
  });
  wrap.addEventListener("pointermove", e => {
    const p = ptr.get(e.pointerId); if (!p) return;
    const dx = e.clientX - p.x, dy = e.clientY - p.y;
    if (ptr.size === 2 && pinch) {
      p.x = e.clientX; p.y = e.clientY;
      const [a, b] = [...ptr.values()], [x, y] = at((a.x + b.x) / 2, (a.y + b.y) / 2);
      zoomAt(wrap, sc, pinch.s * dist() / pinch.d, x, y); moved = true; return;
    }
    if (!moved && Math.hypot(dx, dy) < 4) return;
    if (!moved) { moved = true; try { wrap.setPointerCapture(e.pointerId); } catch (_) { /* the pointer is already gone */ } wrap.classList.add("ig-grab"); }
    wrap.scrollLeft -= dx; wrap.scrollTop -= dy; p.x = e.clientX; p.y = e.clientY;
  });
  const up = (/** @type {PointerEvent} */ e) => { ptr.delete(e.pointerId); pinch = null; wrap.classList.remove("ig-grab"); };
  wrap.addEventListener("pointerup", up); wrap.addEventListener("pointercancel", up);
  // a drag that ended on an ion is not a click on it
  wrap.addEventListener("click", e => { if (moved) { e.stopPropagation(); e.preventDefault(); moved = false; } }, true);
}
