// @ts-strict
// Bitmap size and redraw trigger of the canvases at the margins of the 2D map (sight spectrum and XIC): tested in Node (tests/mappa_margini_node.js). Classic script, loaded before mappa.js.
const MAPMARGINI = (() => {
  /** Sizes the bitmap of `c` to its measured box times the pixel ratio (only when it changed: resizing reallocates) and returns the context in CSS pixels with the measured W x H.
   * @param {any} c canvas @param {number} [ratio] devicePixelRatio */
  const sized = (c, ratio = 1) => {
    const r = c.getBoundingClientRect(), d = Math.min(4, ratio || 1), w = Math.round(r.width * d), h = Math.round(r.height * d);
    if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
    const g = c.getContext("2d"); g.setTransform(d, 0, 0, d, 0, 0); g.clearRect(0, 0, r.width, r.height); return { g, W: r.width, H: r.height };
  };
  /** Calls `redraw` whenever the box of any of `canvases` changes (also hidden -> shown: the box goes from 0 x 0 to its size), so a margin plot never keeps a bitmap measured in another box.
   * @param {any[]} canvases @param {() => void} redraw @returns {() => void} stops watching */
  const watch = (canvases, redraw) => {
    const stops = canvases.map(c => FRAME.ro(c, redraw));
    return () => stops.forEach(f => f());
  };
  return { sized, watch };
})();
