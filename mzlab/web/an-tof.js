// Loading-screen analyser: reflectron time of flight. Ions of equal charge leave the pulser with the same kinetic energy, so v ~ 1/sqrt(m); the uniform
// field of the mirror stops them at the SAME depth (set by the energy) but a heavier ion decelerates more slowly (a ~ 1/m): the flight time grows with sqrt(m/z).
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.tof = c => {
    const { cx, line } = c, M = [1, 1.6, 2.5], v0 = 260, L = 290, P = 52, a1 = v0 * v0 / (2 * P), T = 4.6;   // relative m/z, entry speed of m = 1, tube length, mirror depth, deceleration of m = 1
    const v = M.map(m => v0 / Math.sqrt(m)), tr = M.map((m, j) => 2 * v[j] * m / a1);                       // turnaround time of each ion (a = a1 / m)
    return {
      n: 14, dt: 0.03, T,
      at(j, t) {
        const u = ((t % T) + T) % T, t1 = L / v[j];
        if (u < t1) { const f = u / t1; return { x: 40 + L * f, y: 34 + 26 * f }; }
        if (u < t1 + tr[j]) { const f = (u - t1) / tr[j]; return { x: 330 + P * 4 * (f - f * f), y: 60 + 4 * f }; }
        const f = (u - t1 - tr[j]) / t1; if (f > 1) return null;
        return { x: 330 - L * f, y: 64 + 26 * f };
      },
      draw() {
        cx.strokeStyle = line; cx.lineWidth = 2;
        for (let i = 0; i < 6; i++) { const x = 338 + i * 12; cx.beginPath(); cx.moveTo(x, 22); cx.lineTo(x, 102); cx.stroke(); }   // mirror grids: uniform field
        cx.globalAlpha = 0.45; cx.lineWidth = 1.5; cx.beginPath(); cx.moveTo(34, 16); cx.lineTo(330, 16); cx.moveTo(34, 108); cx.lineTo(330, 108); cx.stroke();   // field-free tube
        cx.globalAlpha = 1; cx.lineWidth = 2; cx.beginPath(); cx.moveTo(26, 24); cx.lineTo(26, 44); cx.stroke();                    // pulser
        cx.lineWidth = 4; cx.beginPath(); cx.moveTo(30, 80); cx.lineTo(30, 100); cx.stroke();                                       // detector
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
