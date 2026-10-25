// @ts-strict
// Images of a drawn fragmentation graph: a stand-alone SVG (computed styles written on the elements, so no stylesheet is needed) and a PNG drawn
// from it with a canvas. Used by the Flow graph view; the stand-alone HTML of mzlab-cli carries the same logic in its page script.

const STILI = ["fill", "stroke", "stroke-width", "stroke-dasharray", "opacity", "font-family", "font-size", "font-weight", "font-style", "text-anchor", "dominant-baseline", "paint-order"];

/** A copy of `svg` that draws the same without the page's CSS. @param {SVGSVGElement} svg */
export function svgAutonomo(svg) {
  const copia = /** @type {SVGSVGElement} */ (svg.cloneNode(true));
  const da = [svg, ...svg.querySelectorAll("*")], a = [copia, ...copia.querySelectorAll("*")];
  da.forEach((e, k) => {
    const cs = getComputedStyle(e), el = /** @type {SVGElement} */ (a[k]);
    for (const p of STILI) { const v = cs.getPropertyValue(p); if (v) el.style.setProperty(p, v); }
  });
  const box = svg.getBoundingClientRect(), vb = svg.viewBox && svg.viewBox.baseVal;
  copia.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  copia.setAttribute("width", String(vb && vb.width ? vb.width : Math.round(box.width)));
  copia.setAttribute("height", String(vb && vb.height ? vb.height : Math.round(box.height)));
  return new XMLSerializer().serializeToString(copia);
}

/** A PNG blob of the SVG text, at `scala` times its size on a white ground. @param {string} testo @param {number} [scala] @returns {Promise<Blob>} */
export function pngDa(testo, scala = 2) {
  return new Promise((ok, no) => {
    const img = new Image(), url = URL.createObjectURL(new Blob([testo], { type: "image/svg+xml" }));
    img.onload = () => {
      const c = document.createElement("canvas");
      c.width = Math.round(img.width * scala); c.height = Math.round(img.height * scala);
      const x = /** @type {CanvasRenderingContext2D} */ (c.getContext("2d"));
      x.fillStyle = "#ffffff"; x.fillRect(0, 0, c.width, c.height); x.drawImage(img, 0, 0, c.width, c.height);
      URL.revokeObjectURL(url);
      c.toBlob(b => (b ? ok(b) : no(new Error("png"))), "image/png");
    };
    img.onerror = () => { URL.revokeObjectURL(url); no(new Error("svg")); };
    img.src = url;
  });
}
