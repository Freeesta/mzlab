// @ts-strict
// Loading-screen analyser: magnetic sector. Ions accelerated through the same potential U have q U = m v^2 / 2; in the field B (into the page, crosses) they follow
// a circle of radius r = m v / (q B) = sqrt(2 m U / q) / B, i.e. r ~ sqrt(m/z). Only the ion whose radius matches the geometry goes through the slit of the collector.
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.settore = c => {
    // source, field entrance, beam height, field exit, collector plate
    const { cx, TAU, line, muted } = c, xs = 22, xb = 140, yb = 96, yx = 52, yc = 26, R = [44, 36, 52], V0 = 78, T = 4.4;
    const alpha = r => Math.acos((r - (yb - yx)) / r), sx = r => xb + r * Math.sin(alpha(r)), len = r => (xb - xs) + r * alpha(r);
    return {
      n: 16, dt: 0.035, T,
      at(j, t) {
        // same energy: v ~ 1/sqrt(m) ~ 1/r
        const r = R[j], v = V0 * 44 / r, u = (((t + 0.4 * j) % T) + T) % T, s = v * u, a = alpha(r);
        if (s < xb - xs) return { x: xs + s, y: yb };
        if (s < len(r)) { const al = (s - (xb - xs)) / r; return { x: xb + r * Math.sin(al), y: yb - r + r * Math.cos(al) }; }
        // field free again: straight along the tangent
        const d = s - len(r), x = sx(r) + Math.cos(a) * d, y = yx - Math.sin(a) * d;
        return y < yc ? (Math.abs(x - 184) < 3.5 && y > yc - 12 ? { x, y } : null) : { x, y };
      },
      draw() {
        // magnetic field region
        cx.fillStyle = muted; cx.strokeStyle = muted; cx.lineWidth = 2; cx.globalAlpha = 0.14; cx.fillRect(xb, yx, 76, yb + 8 - yx); cx.globalAlpha = 0.5;
        cx.strokeRect(xb, yx, 76, yb + 8 - yx);
        // B into the page
        cx.lineWidth = 1.2; for (let i = 0; i < 4; i++) for (let k = 0; k < 3; k++) {
          const x = xb + 16 + i * 18, y = yx + 12 + k * 18; cx.beginPath(); cx.arc(x, y, 3.6, 0, TAU); cx.moveTo(x - 2.5, y - 2.5); cx.lineTo(x + 2.5, y + 2.5); cx.moveTo(x + 2.5, y - 2.5); cx.lineTo(x - 2.5, y + 2.5); cx.stroke();
        }
        // accelerating slit
        cx.globalAlpha = 0.8; cx.lineWidth = 3; cx.beginPath(); cx.moveTo(xs - 6, yb - 12); cx.lineTo(xs - 6, yb - 3); cx.moveTo(xs - 6, yb + 3); cx.lineTo(xs - 6, yb + 12);
        cx.moveTo(xs + 8, yb - 12); cx.lineTo(xs + 8, yb - 3); cx.moveTo(xs + 8, yb + 3); cx.lineTo(xs + 8, yb + 12); cx.stroke();
        // collector plate with its slit
        cx.beginPath(); cx.moveTo(xb - 40, yc); cx.lineTo(180, yc); cx.moveTo(188, yc); cx.lineTo(xb + 76, yc); cx.stroke();
        // detector
        cx.lineWidth = 4; cx.strokeStyle = line; cx.beginPath(); cx.moveTo(176, yc - 14); cx.lineTo(192, yc - 14); cx.stroke(); cx.globalAlpha = 1;
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
