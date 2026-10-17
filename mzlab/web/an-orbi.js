// @ts-strict
// Loading-screen analyser: Orbitrap. The central spindle and the outer barrel are drawn from the exact equipotentials of the quadro-logarithmic
// potential  Phi(r,z) = k/2 (z^2 - r^2/2) + k/2 Rm^2 ln(r/Rm) + C.  With x = r^2/Rm^2 an equipotential through (r0, 0) is  ln x - x = A(z),
// A(z) = ln x0 - x0 - 2 z^2 / Rm^2,  solved with the Lambert W function: x = -W0(-e^A) (x < 1) or x = -W-1(-e^A) (x > 1).
(function (G) {
  "use strict";
  const EM1 = Math.exp(-1);
  function halley(w, y) {                                         // Halley iteration for w e^w = y
    for (let i = 0; i < 50; i++) {
      const ew = Math.exp(w), f = w * ew - y, d = ew * (w + 1) - (w + 2) * f / (2 * w + 2), dw = f / d;
      if (!isFinite(dw)) break;
      w -= dw; if (Math.abs(dw) <= 1e-15 * (1 + Math.abs(w))) break;
    }
    return w;
  }
  // series in p = sqrt(2(e y + 1)) around the branch point y = -1/e (sign +1: W0, -1: W-1)
  function branchPoint(y, sign) {
    const p = sign * Math.sqrt(2 * (Math.E * y + 1));
    return -1 + p - p * p / 3 + 11 / 72 * p ** 3 - 43 / 540 * p ** 4 + 769 / 17280 * p ** 5;
  }
  function lambertW0(y) {                                         // principal branch, y >= -1/e
    if (!(y >= -EM1)) return NaN;
    if (y === 0) return 0;
    if (Math.E * y + 1 < 1e-7) return branchPoint(y, 1);
    let w0;
    if (y < -0.25) w0 = branchPoint(y, 1);
    else if (y < 3) w0 = Math.log(1 + y) * (y < 0 ? 1 : 0.9);
    else { const L = Math.log(y); w0 = L - Math.log(L); }
    return halley(w0, y);
  }
  function lambertWm1(y) {                                        // lower branch, -1/e <= y < 0
    if (!(y >= -EM1) || y >= 0) return NaN;
    if (Math.E * y + 1 < 1e-7) return branchPoint(y, -1);
    let w0;
    if (y < -0.25) w0 = branchPoint(y, -1);
    else { const L1 = Math.log(-y), L2 = Math.log(-L1); w0 = L1 - L2 + L2 / L1; }
    return halley(w0, y);
  }
  // r(z) of the equipotential through (r0, 0): inner branch (x0 <= 1) for r0 < Rm, outer branch for r0 > Rm
  function radiusAt(r0, Rm, z) {
    const x0 = r0 * r0 / (Rm * Rm), A = Math.log(x0) - x0 - 2 * z * z / (Rm * Rm), y = -Math.exp(A);
    return Rm * Math.sqrt(-(x0 <= 1 ? lambertW0(y) : lambertWm1(y)));
  }
  // inverse of radiusAt
  function zAt(r0, Rm, r) { return Math.sqrt(Math.max(0, (r * r - r0 * r0) / 2 - Rm * Rm * Math.log(r / r0))); }
  // ln x - x at the equator
  function levelA(r0, Rm) { const x = r0 * r0 / (Rm * Rm); return Math.log(x) - x; }
  // Rm for which both electrodes lie on the same equipotential at z = 0 (zero voltage): (r2^2 - r1^2) / ln(r2^2 / r1^2). A working Orbitrap needs more.
  function rmEqualPotential(r1, r2) { return Math.sqrt((r2 * r2 - r1 * r1) / Math.log(r2 * r2 / (r1 * r1))); }
  // arbitrary units; r2 / r1 = 2.5; Rm > r2: the outer barrel closes towards the spindle tips
  const GEOM = { r1: 6, r2: 15, Rm: 17, rTip: 2.2 };
  // in units of omega_z
  const omega = (Rm, r) => ({ r: Math.sqrt(Math.max(0, (Rm / r) ** 2 - 2)), phi: Math.sqrt(Math.max(0, ((Rm / r) ** 2 - 1) / 2)) });

  G.LDAN = G.LDAN || {};
  G.LDAN.orbi = c => {
    const { cx, W, H, TAU, line } = c, g = GEOM, zE = zAt(g.r2, g.Rm, g.rTip), s = (H / 2 - 6) / g.r2, mx = W / 2, my = H / 2;
    const NZ = 48, zs = Array.from({ length: NZ + 1 }, (_, i) => zE * i / NZ);
    const rIn = zs.map(z => radiusAt(g.r1, g.Rm, z)), rOut = zs.map(z => radiusAt(g.r2, g.Rm, z)), gap = 0.5;
    const at0 = (arr, z) => { const f = Math.min(1, Math.abs(z) / zE) * NZ, i = Math.min(NZ - 1, Math.floor(f)); return arr[i] + (arr[i + 1] - arr[i]) * (f - i); };
    // m/z (relative), place in the gap, axial amplitude, omega_z = WZ / sqrt(m)
    const M = [1, 1.6, 2.5], F = [0.4, 0.5, 0.6], AZ = [12.5, 10, 7.5], WZ = 4.2;
    // angle phi(t): angular momentum is conserved, so the ion turns faster where the gap is narrow (omega ~ 1/r^2); one axial period is tabulated
    const tab = M.map((m, j) => {
      const wz = WZ / Math.sqrt(m), Tz = TAU / wz, N = 96, ph = [0]; let a = 0;
      for (let i = 0; i < N; i++) {
        const z = AZ[j] * Math.sin(wz * Tz * (i + 0.5) / N), r = at0(rIn, z) + F[j] * (at0(rOut, z) - at0(rIn, z));
        a += omega(g.Rm, r).phi * wz * Tz / N; ph.push(a);
      }
      return { Tz, N, ph };
    });
    const strokeCol = c.muted;
    return {
      n: 22, dt: 0.035,
      at(j, t) {
        const wz = WZ / Math.sqrt(M[j]), z = AZ[j] * Math.sin(wz * t), tb = tab[j], per = Math.floor(t / tb.Tz), u = (t - per * tb.Tz) / tb.Tz * tb.N, i = Math.min(tb.N - 1, Math.floor(u));
        const phi = per * tb.ph[tb.N] + tb.ph[i] + (tb.ph[i + 1] - tb.ph[i]) * (u - i) + j * 2.1, rr = at0(rIn, z) + F[j] * (at0(rOut, z) - at0(rIn, z));
        return { x: mx + s * z, y: my + s * rr * Math.cos(phi), z: Math.sin(phi) };
      },
      // outer electrode: two halves with a slit at z = 0 (the ions come in there)
      draw() {
        cx.strokeStyle = strokeCol; cx.globalAlpha = 0.55; cx.lineWidth = 3; cx.lineJoin = "round"; cx.lineCap = "round";
        for (const d of [-1, 1]) for (const e of [-1, 1]) {
          cx.beginPath(); cx.moveTo(mx + d * gap * s, my + e * s * rOut[0]);
          for (let i = 0; i <= NZ; i++) if (zs[i] >= gap) cx.lineTo(mx + d * s * zs[i], my + e * s * rOut[i]);
          cx.stroke();
        }
        cx.globalAlpha = 1;
      },
      // central electrode: the exact spindle, widest at the equator, thin at the tips
      mid() {
        cx.fillStyle = strokeCol; cx.beginPath();
        for (let i = NZ; i >= 0; i--) cx.lineTo(mx - s * zs[i], my - s * rIn[i]);
        for (let i = 1; i <= NZ; i++) cx.lineTo(mx + s * zs[i], my - s * rIn[i]);
        for (let i = NZ; i >= 0; i--) cx.lineTo(mx + s * zs[i], my + s * rIn[i]);
        for (let i = 1; i <= NZ; i++) cx.lineTo(mx - s * zs[i], my + s * rIn[i]);
        cx.closePath(); cx.fill();
      },
    };
  };
  const api = { lambertW0, lambertWm1, radiusAt, zAt, levelA, rmEqualPotential, omega, GEOM };
  G.LDAN_ORBI = api;
  if (typeof module !== "undefined") module.exports = api;
})(typeof window !== "undefined" ? window : globalThis);
