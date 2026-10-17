// @ts-check
// Legend of a graph: never on top of the data. Of the four corners of the plot area it takes the one with the least ink under it
// (peaks, lines, labels are read from the canvas); if none is free the intensity axis gets headroom (p._legPad) and the graph is drawn again once.
const LEGENDE_INK = 3;                                  // dots of ink tolerated under the legend box
// the box is copied here: the graph canvas itself is never read back
/** @type {CanvasRenderingContext2D | null} */ let legScratch = null;
/** Dots of ink (not transparent) of the canvas in the box x, y, w, h (CSS pixels of the canvas). */
function legInk(/** @type {any} */ p, x, y, w, h) {
  const cv = p.cv, s = cv.width / (cv.offsetWidth || 1), bw = Math.max(1, Math.round(w * s)), bh = Math.max(1, Math.round(h * s));
  try {
    if (!legScratch) legScratch = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
    const g = legScratch; if (!g) return 0;
    g.canvas.width = bw; g.canvas.height = bh;
    g.drawImage(cv, Math.max(0, Math.round(x * s)), Math.max(0, Math.round(y * s)), bw, bh, 0, 0, bw, bh);
    const d = g.getImageData(0, 0, bw, bh).data;
    let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i] > 40) n++;
    return n / (s * s);
  } catch (e) { return 0; }
}
/** Places the legend element `el` of panel `p` (corner order: top right, top left, bottom right, bottom left); returns true when it had to ask for headroom. */
function legPlace(/** @type {any} */ p, /** @type {HTMLElement} */ el) {
  const cv = p.cv, W = cv.offsetWidth, H = cv.offsetHeight, w = el.offsetWidth, h = el.offsetHeight, m = 6;
  const set = (/** @type {boolean} */ right, /** @type {boolean} */ top) => {
    if (right) { el.style.right = p.el.clientWidth - cv.offsetLeft - W + M.r + m + "px"; el.style.left = "auto"; }
    else { el.style.left = cv.offsetLeft + M.l + m + "px"; el.style.right = "auto"; }
    el.style.top = cv.offsetTop + (top ? M.t + 5 : H - M.b - h - 5) + "px";
  };
  set(true, true);
  if (!p._a || !w || !h || p.type === "map") return false;
  const key = JSON.stringify([p.zoom, p.zoomY, W, H, w, h, p.type, p._a.x0, p._a.x1]);
  if (p._legKey !== key) { p._legKey = key; p._legPad = 0; }
  const xr = W - M.r - m - w, xl = M.l + m, yt = M.t + 5, yb = H - M.b - h - 5;
  /** @type {[boolean, boolean, number][]} */
  const cand = [[true, true, legInk(p, xr, yt, w, h)], [false, true, legInk(p, xl, yt, w, h)], [true, false, legInk(p, xr, yb, w, h)], [false, false, legInk(p, xl, yb, w, h)]];
  let best = cand[0];
  for (const c of cand) if (c[2] < best[2]) best = c;
  set(best[0], best[1]);
  if (best[2] <= LEGENDE_INK || p._legPad) return false;
  // no free corner: room above the data, once per view
  p._legPad = Math.min(0.4, (h + 12) / Math.max(60, H - M.t - M.b));
  return true;
}
const LEGENDE = { place: legPlace };
