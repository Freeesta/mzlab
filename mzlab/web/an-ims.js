// @ts-strict
// Loading-screen analyser: drift-tube ion mobility. A gate releases a packet; in the uniform field E, set by the ring electrodes, the ions cross a buffer gas
// and reach a constant drift speed v = K E, with K ~ 1 / (collision cross-section): the compact ion arrives first, whatever its m/z. Collisions make the path zigzag.
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.ims = c => {
    const { cx, W, H, TAU, line, muted } = c, my = H / 2, x0 = 40, x1 = W - 40, hh = 34, T = 5.2, K = [1, 0.8, 0.63], Tn = 3.0;   // K of the compact, medium and extended ion
    const gas = Array.from({ length: 34 }, (_, i) => ({ x: x0 + 8 + (i * 61 % (x1 - x0 - 16)), y: my - hh + 6 + (i * 37 % (2 * hh - 12)), p: i * 1.3 }));
    return {
      n: 18, dt: 0.035, T,
      at(j, t) {
        const u = (((t + 0.15 * j) % T) + T) % T, f = u * K[j] / Tn;
        if (f > 1) return null;
        const w = 9 * Math.sqrt(f) * Math.sin(26 * u + j * 2.3), d = Math.sin(14 * u + j);                              // the packet spreads ~ sqrt(t): diffusion in the gas
        return { x: x0 + (x1 - x0) * f, y: my + w, z: d };
      },
      draw(t) {
        cx.strokeStyle = muted; cx.fillStyle = muted; cx.lineWidth = 2; cx.globalAlpha = 0.6;
        cx.beginPath(); cx.moveTo(x0, my - hh); cx.lineTo(x1, my - hh); cx.moveTo(x0, my + hh); cx.lineTo(x1, my + hh); cx.stroke();                                // tube
        cx.lineWidth = 1.5; for (let i = 0; i <= 12; i++) { const x = x0 + (x1 - x0) * i / 12; cx.beginPath(); cx.moveTo(x, my - hh - 4); cx.lineTo(x, my - hh + 5); cx.moveTo(x, my + hh - 5); cx.lineTo(x, my + hh + 4); cx.stroke(); }   // rings: uniform field
        cx.lineWidth = 3; cx.beginPath(); cx.moveTo(x0, my - hh); cx.lineTo(x0, my - 5); cx.moveTo(x0, my + 5); cx.lineTo(x0, my + hh); cx.stroke();                 // gate
        cx.beginPath(); cx.moveTo(x1 + 6, my - 14); cx.lineTo(x1 + 6, my + 14); cx.stroke();                                                                        // detector
        cx.fillStyle = line; cx.globalAlpha = 0.5; for (const g of gas) { cx.beginPath(); cx.arc(g.x + 2.5 * Math.sin(3.1 * (t || 0) + g.p), g.y + 2.5 * Math.cos(2.3 * (t || 0) + g.p), 1.3, 0, TAU); cx.fill(); }   // buffer gas
        cx.strokeStyle = line; cx.globalAlpha = 0.7; cx.lineWidth = 1.5; cx.beginPath(); cx.moveTo(x0 + 40, my - hh - 16); cx.lineTo(x1 - 40, my - hh - 16); cx.moveTo(x1 - 46, my - hh - 20); cx.lineTo(x1 - 40, my - hh - 16); cx.lineTo(x1 - 46, my - hh - 12); cx.stroke();   // E
        cx.globalAlpha = 1;
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
