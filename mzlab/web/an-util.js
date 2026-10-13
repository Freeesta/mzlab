// @ts-strict
// Small drawing helpers shared by the loading-screen analysers (an-*.js, used by ldIon in index.html).
(function (G) {
  "use strict";
  const LDU = {
    smooth: u => { const x = Math.max(0, Math.min(1, u)); return x * x * (3 - 2 * x); },
    // a rod seen from the side: rounded bar from x0 to x1, centred on yc, height h
    rod(cx, x0, x1, yc, h) {
      const r = Math.min(h / 2, 3);
      cx.beginPath(); cx.moveTo(x0 + r, yc - h / 2); cx.lineTo(x1 - r, yc - h / 2); cx.quadraticCurveTo(x1, yc - h / 2, x1, yc - h / 2 + r);
      cx.lineTo(x1, yc + h / 2 - r); cx.quadraticCurveTo(x1, yc + h / 2, x1 - r, yc + h / 2); cx.lineTo(x0 + r, yc + h / 2);
      cx.quadraticCurveTo(x0, yc + h / 2, x0, yc + h / 2 - r); cx.lineTo(x0, yc - h / 2 + r); cx.quadraticCurveTo(x0, yc - h / 2, x0 + r, yc - h / 2); cx.closePath();
    },
    // a line from (x0,y) to (x1,y) with an open arrow head at x1 (towards the right when x1 > x0)
    arrow(cx, x0, x1, y, head = 5) {
      const d = Math.sign(x1 - x0) || 1;
      cx.beginPath(); cx.moveTo(x0, y); cx.lineTo(x1, y); cx.moveTo(x1 - d * head, y - head * 0.7); cx.lineTo(x1, y); cx.lineTo(x1 - d * head, y + head * 0.7); cx.stroke();
    },
  };
  G.LDU = LDU;
  if (typeof module !== "undefined") module.exports = LDU;
})(typeof window !== "undefined" ? window : globalThis);
