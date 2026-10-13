// @ts-strict
// Loading-screen analyser: linear ion trap. Four rods in three sections; RF confines the ions radially, the higher DC on the two end sections confines
// them axially in a parabolic well, where they swing with omega_z ~ sqrt(qU/m). The centre section has a slit in one rod (radial ejection).
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.trappola = c => {
    const { cx, W, H, TAU, line, muted } = c, U = G.LDU, my = H / 2, h = 11, off = 15, g = 3;
    const xs = [22, 112, 308, W - 22];                                              // section edges: front, centre, back
    const M = [1, 1.5, 2.2], A = [62, 46, 30], WZ = 3.2, xc = (xs[1] + xs[2]) / 2;
    return {
      n: 24, dt: 0.04,
      at(j, t) {
        const m = M[j], wz = WZ / Math.sqrt(m), wr = 9.5 / m, ph = wr * t + j * 2;       // axial ~ 1/sqrt(m), radial secular ~ q ~ 1/m
        const fade = 0.8 + 0.2 * Math.cos(0.25 * t + j);                                 // buffer gas: the swing is slightly cooled and refreshed
        return { x: xc + A[j] * fade * Math.sin(wz * t + j), y: my + 4.5 * Math.cos(ph), z: Math.sin(ph) };
      },
      draw() {
        cx.fillStyle = muted; cx.strokeStyle = muted; cx.lineWidth = 1.5;
        for (let s = 0; s < 3; s++) {
          const a = xs[s] + (s ? g : 0), b = xs[s + 1] - (s < 2 ? g : 0);
          cx.globalAlpha = 0.6; U.rod(cx, a, b, my - off, h); cx.fill();
          if (s === 1) { U.rod(cx, a, xc - 3, my + off, h); cx.fill(); U.rod(cx, xc + 3, b, my + off, h); cx.fill(); }   // ejection slit
          else { U.rod(cx, a, b, my + off, h); cx.fill(); }
          cx.globalAlpha = 0.3; U.rod(cx, a, b, my, h); cx.stroke();
        }
        cx.globalAlpha = 0.8; cx.strokeStyle = line; cx.lineWidth = 1.5; cx.beginPath();                    // axial DC well: low in the centre, high at the ends
        for (let i = 0; i <= 60; i++) { const x = xs[0] + (xs[3] - xs[0]) * i / 60, d = (x - xc) / ((xs[3] - xs[0]) / 2); const y = 13 - 9 * Math.max(0, 1 - d * d * 1.2); i ? cx.lineTo(x, y) : cx.moveTo(x, y); }
        cx.stroke(); cx.globalAlpha = 1;
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
