// Loading-screen animation: electrospray source. The liquid leaves the capillary as a Taylor cone (half-angle 49.3 deg), a jet breaks into charged droplets,
// the droplets lose solvent (they shrink) and the gas-phase ions follow the field lines into the orifice of the sampling cone, behind a curtain plate.
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.esi = c => {
    const { cx, W, H, line, muted, TAU } = c, my = H / 2, xt = 74, xo = W - 52, Ln = xo - xt, T = 3.6, hw = 9;
    const AMP = [34, -27, 13], PH = [0.4, 2.2, 4.1];
    const cone = hw / Math.tan(49.3 * Math.PI / 180);                              // length of the Taylor cone for a mouth of half-width hw
    return {
      n: 16, dt: 0.035, T,
      at(j, t) {
        const u = (((t + j * 0.8) % T) + T) % T, f = u / T, e = f * f * (3 - 2 * f), s = 2.1 - 1.2 * Math.min(1, e / 0.55);   // droplet -> bare ion
        const spread = AMP[j] * 4 * e * (1 - e) * Math.cos(PH[j] + 5 * e);        // the plume opens, then the field lines close it on the orifice
        return { x: xt + cone + (Ln - cone) * e, y: my + spread, z: Math.sin(PH[j] + 5 * e), s };
      },
      draw() {
        cx.strokeStyle = muted; cx.fillStyle = muted; cx.lineWidth = 2; cx.globalAlpha = 0.7;
        cx.beginPath(); cx.moveTo(8, my - hw - 4); cx.lineTo(xt - 20, my - hw - 4); cx.lineTo(xt, my - hw); cx.lineTo(xt, my + hw); cx.lineTo(xt - 20, my + hw + 4); cx.lineTo(8, my + hw + 4); cx.stroke();   // capillary
        cx.globalAlpha = 0.4; cx.beginPath(); cx.moveTo(xt, my - hw); cx.lineTo(xt + cone, my); cx.lineTo(xt, my + hw); cx.closePath(); cx.fill();                                                    // Taylor cone
        cx.globalAlpha = 0.7;
        cx.beginPath(); cx.moveTo(xo + 36, 14); cx.lineTo(xo + 4, my - 6); cx.lineTo(xo + 4, my - 3); cx.moveTo(xo + 36, H - 14); cx.lineTo(xo + 4, my + 6); cx.lineTo(xo + 4, my + 3); cx.stroke();   // sampling cone
        cx.strokeStyle = line; cx.lineWidth = 1.5; cx.globalAlpha = 0.6; cx.setLineDash([3, 4]);
        cx.beginPath(); cx.moveTo(xo - 22, 12); cx.lineTo(xo - 22, my - 8); cx.moveTo(xo - 22, my + 8); cx.lineTo(xo - 22, H - 12); cx.stroke(); cx.setLineDash([]);                                 // curtain plate
        cx.strokeStyle = muted; cx.lineWidth = 1.5; cx.globalAlpha = 0.5;
        for (let i = 0; i < 3; i++) { const x = 22 + i * 14; cx.beginPath(); cx.moveTo(x - 3, my - 22); cx.lineTo(x + 3, my - 22); cx.moveTo(x, my - 25); cx.lineTo(x, my - 19); cx.stroke(); }   // charge at the capillary
        cx.globalAlpha = 1;
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
