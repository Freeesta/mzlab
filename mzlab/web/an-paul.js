// @ts-strict
// Loading-screen analyser: Paul trap (3D quadrupole ion trap). Ring electrode r^2 = r0^2 + 2 z^2 and two end caps z^2 = z0^2 + r^2/2 with z0 = r0 / sqrt 2
// (hyperboloids). RF on the ring confines the ions in a pseudo-potential well: the secular motion along z is twice as fast as the radial one
// (omega_z = 2 omega_r, a Lissajous figure), carrying the fast RF micromotion; lighter ions (larger q) oscillate faster.
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.paul = c => {
    const { cx, W, H, TAU, line, muted } = c, mx = W / 2, my = H / 2, r0 = 38, z0 = r0 / Math.SQRT2, zr = 3.1 * z0;
    const M = [1, 1.5, 2.2], OM = TAU * 4, A = [0.62, 0.5, 0.4];                    // m/z (relative), RF angular frequency, amplitude as a fraction of r0 (radial) and z0 (axial)
    return {
      n: 24, dt: 0.04,
      at(j, t) {
        const q = 0.55 / M[j], wz = q / Math.SQRT2 * OM / 2, wr = wz / 2, mod = 1 - q / 2 * Math.cos(OM * t);       // secular beta ~ q / sqrt 2; omega_z = 2 omega_r
        const ph = wr * t + j * 2.1;
        return { x: mx + A[j] * z0 * Math.cos(wz * t + j) * (1 - q / 2 * Math.cos(OM * t) * 0.5), y: my + A[j] * r0 * Math.cos(ph) * mod, z: Math.sin(ph) };
      },
      draw(t) {
        cx.strokeStyle = muted; cx.fillStyle = muted; cx.lineWidth = 3; cx.lineCap = "round"; cx.globalAlpha = 0.65;
        for (const e of [-1, 1]) {                                                  // end caps: z = +-sqrt(z0^2 + r^2/2)
          cx.beginPath(); for (let i = -20; i <= 20; i++) { const r = i / 20 * 46, x = mx + e * Math.sqrt(z0 * z0 + r * r / 2), y = my + r; i > -20 ? cx.lineTo(x, y) : cx.moveTo(x, y); } cx.stroke();
        }
        for (const e of [-1, 1]) {                                                  // ring: r = +-sqrt(r0^2 + 2 z^2), two gaps towards the caps
          cx.beginPath(); for (let i = -20; i <= 20; i++) { const z = i / 20 * zr, x = mx + z, y = my + e * Math.sqrt(r0 * r0 + 2 * z * z); if (y > H - 4 || y < 4) continue; i > -20 ? cx.lineTo(x, y) : cx.moveTo(x, y); } cx.stroke();
        }
        cx.globalAlpha = 1; cx.strokeStyle = line; cx.lineWidth = 1.5; const wx = 14, wy = H - 12, ph = OM * (t || 0) / 16;        // RF on the ring
        cx.beginPath(); for (let i = 0; i <= 40; i++) { const x = wx + i, y = wy + 4 * Math.cos(i / 40 * TAU * 2 + ph); i ? cx.lineTo(x, y) : cx.moveTo(x, y); } cx.stroke();
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
