// @ts-strict
// Loading-screen analyser: triple quadrupole (Q1 - collision cell q2 - Q3). Three rod sets of the same type (the length is shortened to fit the screen, about
// 28 r0 instead of 50): Q1 selects the precursor (a wrong m/z has an unstable trajectory: the swing grows until it hits a rod), q2 is a gas-filled RF-only cell
// where the precursor breaks into lighter fragments, Q3 lets through only one fragment m/z; what is left reaches the detector.
(function (G) {
  "use strict";
  G.LDAN = G.LDAN || {};
  G.LDAN.qqq = c => {
    const { cx, H, TAU, line, muted } = c, U = G.LDU, my = H / 2, r0 = 4.4, rr = 1.1469 * r0, off = r0 + rr;
    // Q1, q2, Q3 edges; detector; period of the loop (the crossing is at 130 px/s)
    const S = [[18, 138], [156, 258], [276, 396]], xd = 400, P = 3.6;
    const gas = Array.from({ length: 16 }, (_, i) => ({ x: S[1][0] + 8 + (i * 37 % 82), y: my - 17 + (i * 53 % 34), p: i * 1.7 }));
    return {
      n: 24, dt: 0.04, T: 1,
      // j = 0: passes everything; 1: wrong precursor, lost in Q1; 2: its fragment is lost in Q3
      at(j, t) {
        const u = (((t + j * 0.9) % P) + P) % P, x = 24 + 130 * u;
        if (x > xd) return null;
        // unstable: the swing grows exponentially
        const bad = j === 1 ? x - S[0][0] : j === 2 && x > S[2][0] ? x - S[2][0] : 0, a = 0.28 * Math.exp(bad / (j === 1 ? 34 : 20));
        if (bad && a >= 1) return null;                                               // it touches a rod
        const th = x * (j === 2 && x > S[2][0] ? 0.08 : 0.055) + j * 1.9;
        // after the collision: lighter fragment
        return { x, y: my + a * r0 * Math.cos(th) * (1 - 0.18 * Math.cos(x * 0.9)), z: Math.sin(th), s: j !== 1 && x > S[1][0] + 56 ? 0.7 : 1 };
      },
      draw(t) {
        cx.fillStyle = muted; cx.strokeStyle = muted; cx.lineWidth = 1.5;
        for (const [a, b] of S) for (const e of [-1, 1]) { cx.globalAlpha = 0.6; U.rod(cx, a, b, my + e * off, 2 * rr); cx.fill(); }
        // collision cell housing
        cx.strokeStyle = line; cx.globalAlpha = 0.7; cx.beginPath(); cx.rect(S[1][0] - 6, my - 26, S[1][1] - S[1][0] + 12, 52); cx.stroke();
        cx.fillStyle = line; cx.globalAlpha = 0.5;
        // collision gas
        for (const q of gas) { cx.beginPath(); cx.arc(q.x + 2 * Math.sin(2.1 * (t || 0) + q.p), q.y + 2 * Math.cos(1.7 * (t || 0) + q.p), 1.4, 0, TAU); cx.fill(); }
        // detector
        cx.globalAlpha = 1; cx.lineWidth = 4; cx.strokeStyle = muted; cx.beginPath(); cx.moveTo(xd + 6, my - 14); cx.lineTo(xd + 6, my + 14); cx.stroke();
      },
    };
  };
})(typeof window !== "undefined" ? window : globalThis);
