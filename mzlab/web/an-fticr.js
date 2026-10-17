// @ts-strict
// Loading-screen analyser: FT-ICR cell. A strong static field B along the axis makes every ion turn at the cyclotron frequency omega_c = qB/m (lighter ions
// faster); the two end plates trap them axially. An RF pulse on the excitation plates (top and bottom) opens the orbit coherently; the detection plates
// (front and back) then see the image current, which a Fourier transform turns into a spectrum.
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.fticr = c => {
    const { cx, W, H, line, muted } = c, U = G.LDU, my = H / 2, mx = W / 2, hl = 56, hh = 40, T = 5.4;
    // m/z (relative), excited radius, omega_c of m = 1, axial trapping swing
    const M = [1, 1.45, 2.1], R = [28, 23, 18], WC = 13, WT = 1.5;
    return {
      n: 22, dt: 0.03, T,
      at(j, t) {
        // excitation, free orbit, then it relaxes
        const u = ((t % T) + T) % T, rad = 4 + (R[j] - 4) * U.smooth(u / 1.4) * (1 - U.smooth((u - 4.4) / 0.9)), ph = WC / M[j] * t + j * 2.3;
        return { x: mx + 20 * (j - 1) * 0.5 + 14 * Math.sin(WT / Math.sqrt(M[j]) * t + j), y: my + rad * Math.cos(ph), z: Math.sin(ph) };
      },
      draw() {
        cx.strokeStyle = muted; cx.fillStyle = muted; cx.lineWidth = 3; cx.lineCap = "round";
        // excitation plates
        cx.globalAlpha = 0.65; for (const e of [-1, 1]) { cx.beginPath(); cx.moveTo(mx - hl + 8, my + e * hh); cx.lineTo(mx + hl - 8, my + e * hh); cx.stroke(); }
        // detection plates (front, back)
        cx.globalAlpha = 0.35; cx.beginPath(); cx.rect(mx - hl + 8, my - hh, 2 * hl - 16, 2 * hh); cx.stroke();
        // trapping plates
        cx.globalAlpha = 0.65; for (const e of [-1, 1]) { cx.beginPath(); cx.moveTo(mx + e * hl, my - hh); cx.lineTo(mx + e * hl, my - 6); cx.moveTo(mx + e * hl, my + 6); cx.lineTo(mx + e * hl, my + hh); cx.stroke(); }
        // B along the axis
        cx.globalAlpha = 0.5; cx.strokeStyle = line; cx.lineWidth = 1.5;
        for (const e of [-1, 1]) { U.arrow(cx, 14, mx - hl - 8, my + e * 50, 5); U.arrow(cx, mx + hl + 8, W - 14, my + e * 50, 5); }
        cx.globalAlpha = 1;
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
