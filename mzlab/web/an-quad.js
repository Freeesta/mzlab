// @ts-strict
// Loading-screen analyser: quadrupole mass filter. Four round rods of radius 1.1469 r0 (best approximation of the hyperbolae) at distance r0 from the axis,
// length about 50 r0; opposite rods share the same RF + DC potential, neighbouring ones the opposite sign. A stable ion crosses the rods with a slow
// secular oscillation (Mathieu: beta ~ q / sqrt 2) carrying the fast RF micromotion (amplitude ~ q/2).
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.quad = c => {
    const { cx, W, H, TAU, line, muted } = c, U = G.LDU, x0 = 24, x1 = W - 20, L = x1 - x0, r0 = L / 50, rr = 1.1469 * r0, off = r0 + rr, my = H / 2;
    // q of each ion (q ~ 1/m), axial speed (px/s, same energy: ~ 1/sqrt m), RF angular frequency
    const Q = [0.55, 0.44, 0.35], V = [88, 77, 68], OM = TAU * 3.4;
    const T = j => L / V[j] + 0.4;
    return {
      n: 26, dt: 0.04, T: 1,
      at(j, t) {
        const tt = T(j), u = ((t % tt) + tt) % tt, x = x0 + V[j] * u;
        if (x > x1) return null;
        const q = Q[j], th = q / Math.SQRT2 * OM / 2 * u, A = 0.75 * r0, mod = 1 - q / 2 * Math.cos(OM * u);
        return { x, y: my + A * Math.cos(th + j) * mod, z: Math.sin(th + j * 1.3) * mod };
      },
      draw(t) {
        cx.fillStyle = muted; cx.strokeStyle = muted; cx.lineWidth = 1.5;
        // the two rods seen at the side (same polarity)
        cx.globalAlpha = 0.6; for (const e of [-1, 1]) { U.rod(cx, x0, x1, my + e * off, 2 * rr); cx.fill(); }
        // the rod behind the axis
        cx.globalAlpha = 0.35; U.rod(cx, x0, x1, my, 2 * rr); cx.stroke();
        // RF + DC: the two phases, opposite in sign
        cx.globalAlpha = 1; cx.strokeStyle = line; cx.lineWidth = 1.5;
        const wx = W - 62, wy = 11, ph = OM * (t || 0) / 16;
        for (const e of [-1, 1]) { cx.beginPath(); for (let i = 0; i <= 40; i++) { const x = wx + i, y = wy + e * (1.5 + 4 * Math.cos(i / 40 * TAU * 2 + ph)); i ? cx.lineTo(x, y) : cx.moveTo(x, y); } cx.stroke(); }
      },
      // the rod in front of the axis, see-through
      mid() { cx.globalAlpha = 0.25; cx.fillStyle = muted; U.rod(cx, x0, x1, my, 2 * rr); cx.fill(); cx.globalAlpha = 1; },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
