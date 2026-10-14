// @ts-strict
// Loading-screen analyser: MALDI with linear TOF. A laser pulse hits the matrix crystals on the target: a plume of ions leaves the plate, the field between the plate
// and the grid gives all singly charged ions the same energy (v ~ 1/sqrt(m)), then they drift in the field-free tube: the flight time grows with sqrt(m/z).
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.maldi = c => {
    const { cx, W, H, TAU, line, muted } = c, my = H / 2, xp = 26, xg = 66, xd = W - 22, T = 5.4, M = [1, 1.6, 2.5], v1 = 150, d = xg - xp, SP = [-9, 12, -3];
    const v = M.map(m => v1 / Math.sqrt(m)), acc = M.map((m, j) => v[j] * v[j] / (2 * d)), ta = M.map((m, j) => v[j] / acc[j]);          // speed at the grid, acceleration (a ~ 1/m), time in the field
    return {
      n: 14, dt: 0.035, T,
      at(j, t) {
        const u = ((t % T) + T) % T, x = u < ta[j] ? xp + acc[j] * u * u / 2 : xg + v[j] * (u - ta[j]);
        if (x > xd) return null;
        return { x, y: my + SP[j] * (1 - (x - xp) / (xd - xp)), z: Math.sin(SP[j] + 5 * u) };                                         // the plume starts wide and converges on the detector
      },
      draw(t) {
        const u = ((t || 0) % T + T) % T;
        cx.strokeStyle = muted; cx.fillStyle = muted; cx.lineWidth = 4; cx.globalAlpha = 0.7; cx.beginPath(); cx.moveTo(xp - 3, my - 38); cx.lineTo(xp - 3, my + 38); cx.stroke();    // target plate
        cx.lineWidth = 1.5; cx.globalAlpha = 0.6; for (let i = 0; i < 5; i++) cx.fillRect(xp, my - 6 + i * 3, 3, 2);                                                           // sample + matrix crystals
        cx.lineWidth = 2; cx.beginPath(); cx.moveTo(xg, my - 38); cx.lineTo(xg, my - 5); cx.moveTo(xg, my + 5); cx.lineTo(xg, my + 38); cx.moveTo(xg + 12, my - 38); cx.lineTo(xg + 12, my - 5); cx.moveTo(xg + 12, my + 5); cx.lineTo(xg + 12, my + 38); cx.stroke();   // extraction grids
        cx.globalAlpha = 0.45; cx.beginPath(); cx.moveTo(xg + 12, my - 38); cx.lineTo(xd, my - 38); cx.moveTo(xg + 12, my + 38); cx.lineTo(xd, my + 38); cx.stroke();                      // flight tube
        cx.globalAlpha = 0.9; cx.lineWidth = 4; cx.strokeStyle = line; cx.beginPath(); cx.moveTo(xd + 6, my - 14); cx.lineTo(xd + 6, my + 14); cx.stroke();                           // detector
        cx.lineWidth = 1.5; cx.setLineDash([4, 3]); cx.beginPath(); cx.moveTo(xp + 66, 12); cx.lineTo(xp + 3, my - 3); cx.stroke(); cx.setLineDash([]);                                // laser beam at an angle
        if (u < 0.22) { cx.globalAlpha = 0.75 * (1 - u / 0.22); cx.fillStyle = line; cx.beginPath(); cx.arc(xp + 3, my, 4 + 14 * u, 0, TAU); cx.fill(); }                                   // laser flash
        cx.globalAlpha = 1;
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
