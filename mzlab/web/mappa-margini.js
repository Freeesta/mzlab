// @ts-strict
// Bitmap size, geometry and redraw triggers of the canvases at the margins of the 2D map (sight spectrum and XIC, box extraction plots): tested in Node (tests/mappa_margini_node.js). Classic script, loaded before mappa.js.
// Everything is measured in LAYOUT pixels (clientWidth/clientHeight, never getBoundingClientRect): while a panel animates (transizioni.js plays a FLIP scale on it) the rect is scaled,
// and since the layout box does not change when the animation ends, no ResizeObserver would ever ask for the repair of a bitmap sized from it.
const MAPMARGINI = (() => {
  /** Sizes the bitmap of `c` to its layout box times the pixel ratio (only when it changed: resizing reallocates) and returns the context in CSS pixels with the layout W x H.
   * @param {any} c canvas @param {number} [ratio] devicePixelRatio */
  const sized = (c, ratio = 1) => {
    const W = c.clientWidth, H = c.clientHeight, d = Math.min(4, ratio || 1), w = Math.round(W * d), h = Math.round(H * d);
    if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
    const g = c.getContext("2d"); g.setTransform(d, 0, 0, d, 0, 0); g.clearRect(0, 0, W, H); return { g, W, H };
  };
  /** Where the drawing area of `a` starts, in the layout pixels of canvas `b` (the transform of an animating ancestor, the same for both, is divided out).
   * @param {any} a @param {any} b @returns {{ x: number, y: number }} */
  const rel = (a, b) => {
    const ra = a.getBoundingClientRect(), rb = b.getBoundingClientRect(), sx = b.offsetWidth ? rb.width / b.offsetWidth : 1, sy = b.offsetHeight ? rb.height / b.offsetHeight : 1;
    return { x: (ra.left - rb.left) / sx + a.clientLeft - b.clientLeft, y: (ra.top - rb.top) / sy + a.clientTop - b.clientTop };
  };
  /** Calls `redraw` whenever the layout box of any of `canvases` changes (also hidden -> shown: the box goes from 0 x 0 to its size) or the pixel ratio changes (zoom of the window,
   * another screen), so a margin plot never keeps a bitmap measured for another box or ratio. @param {any[]} canvases @param {() => void} redraw @returns {() => void} stops watching */
  const watch = (canvases, redraw) => {
    const stops = canvases.map(c => FRAME.ro(c, redraw));
    let live = true;
    const arm = () => {
      if (!live || typeof matchMedia !== "function") return;
      const mq = matchMedia(`(resolution: ${devicePixelRatio}dppx)`);
      mq.addEventListener("change", () => { if (!live) return; redraw(); arm(); }, { once: true });
    };
    arm();
    return () => { live = false; stops.forEach(f => f()); };
  };
  return { sized, rel, watch };
})();
