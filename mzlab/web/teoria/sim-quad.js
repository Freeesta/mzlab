/* QqQ lab - Teoria: quadrupole mass filter.
   Physics (de Hoffmann & Stroobant; Douglas 2009): potential Phi = (U - V cos Wt)(x^2 - y^2)/r0^2,
   Mathieu form  d2u/dxi2 + (a_u - 2 q_u cos 2xi) u = 0,  xi = W t / 2,
   a_x = -a_y = 8 z e U / (m r0^2 W^2),   q_x = -q_y = 4 z e V / (m r0^2 W^2).
   Stability is decided numerically with Floquet theory: |trace of the one-period transfer matrix| < 2. */
"use strict";
const QUAD = (() => {
  const E = 1.602176634e-19, AMU = 1.66053906660e-27;
  // one RK4 step of u'' = -(a - 2 q cos 2(xi+ph)) u
  function rk4(u, v, xi, h, a, q, ph) {
    const f = (x, uu) => -(a - 2 * q * Math.cos(2 * (x + ph))) * uu;
    const k1u = v, k1v = f(xi, u);
    const k2u = v + h / 2 * k1v, k2v = f(xi + h / 2, u + h / 2 * k1u);
    const k3u = v + h / 2 * k2v, k3v = f(xi + h / 2, u + h / 2 * k2u);
    const k4u = v + h * k3v, k4v = f(xi + h, u + h * k3u);
    return [u + h / 6 * (k1u + 2 * k2u + 2 * k3u + k4u), v + h / 6 * (k1v + 2 * k2v + 2 * k3v + k4v)];
  }
  // trace of the monodromy matrix over one period (pi in xi)
  function trace(a, q, N = 56) {
    const h = Math.PI / N; let u1 = 1, v1 = 0, u2 = 0, v2 = 1, xi = 0;
    for (let i = 0; i < N; i++) { [u1, v1] = rk4(u1, v1, xi, h, a, q, 0); [u2, v2] = rk4(u2, v2, xi, h, a, q, 0); xi += h; }
    return u1 + v2;
  }
  const stableX = (a, q) => Math.abs(trace(a, q)) < 2;
  const stableY = (a, q) => Math.abs(trace(-a, -q)) < 2;
  const stable = (a, q) => stableX(a, q) && stableY(a, q);
  // stable q-interval of the operating line a = k q (k = 2U/V), searched in the first stability region
  const winCache = new Map();
  function window(k) {
    const key = k.toFixed(6); if (winCache.has(key)) return winCache.get(key);
    let lo = null, hi = null, prev = false; const dq = 0.002;
    let q0 = k > 0.28 ? 0.45 : 0.002; if (q0 > 0.01 && stable(k * q0, q0)) q0 = 0.002;
    for (let q = q0; q <= 0.95; q += dq) {
      const s = stable(k * q, q);
      if (s && !prev && lo === null) lo = q; if (!s && prev && lo !== null) { hi = q; break; } prev = s;
    }
    let res = null;
    if (lo !== null) {
      if (hi === null) hi = 0.95;
      const bis = (x0, x1, want) => { for (let i = 0; i < 30; i++) { const m = (x0 + x1) / 2; (stable(k * m, m) === want ? (x1 = m) : (x0 = m)); } return (x0 + x1) / 2; };
      const ql = lo > 0.0021 ? bis(lo - dq, lo, true) : 0, qr = bis(hi - dq, hi, false);
      res = [ql, qr];
    }
    winCache.set(key, res); return res;
  }
  const VOf = (mz, q, r0, f) => q * mz * AMU * r0 * r0 * Math.pow(2 * Math.PI * f, 2) / (4 * E);
  return { rk4, trace, stableX, stableY, stable, window, VOf, E, AMU };
})();

// The simulations build after the first paint (the chapter text is already readable); heavy numerics must not hold DOMContentLoaded.
document.addEventListener("DOMContentLoaded", () => requestAnimationFrame(() => setTimeout(() => {
  const css = getComputedStyle(document.documentElement), col = n => css.getPropertyValue("--c" + n).trim();
  const APEX_K = 2 * 0.16784; // a/q at the apex (0.237/0.706)

  // ------------------------------------------------------------ 1) field + ion in the xy plane (animated)
  const fd = document.getElementById("sim-field");
  if (fd) {
    const v = TP.controls(fd, [
      { id: "q", label: "q (ampiezza RF)", min: 0.05, max: 1.0, step: 0.01, value: 0.5, fmt: x => TP.fmt(x, 2) },
      { id: "a", label: "a (componente continua)", min: 0, max: 0.26, step: 0.002, value: 0, fmt: x => TP.fmt(x, 3) },
      { id: "sp", label: "Velocità dell'animazione", min: 0.2, max: 3, step: 0.1, value: 1, fmt: x => TP.fmt(x, 1) + "×" },
    ], () => reset());
    const bt = TP.buttons(fd, [["play", "Pausa"], ["new", "Nuovo ione"], ["rf", "Solo RF (a = 0)"], ["apex", "Vicino al vertice"]], null, k => {
      if (k === "play") { run = !run; bt.querySelector("[data-k=play]").textContent = run ? "Pausa" : "Riprendi"; bt.querySelector("[data-k=play]").classList.remove("on"); if (run) loop(); }
      if (k === "new") reset();
      if (k === "rf") { v._set_a(0); v.a = 0; reset(); }
      if (k === "apex") { v._set_q(0.70); v.q = 0.70; v._set_a(0.232); v.a = 0.232; reset(); }
      bt.querySelectorAll("button").forEach(b => b.classList.remove("on"));
    });
    const wrap = document.createElement("div"); wrap.className = "grid2"; fd.appendChild(wrap);
    const L = document.createElement("div"), R = document.createElement("div"); wrap.append(L, R);
    const c1 = TP.canvas(L, 320), c2 = TP.canvas(R, 320);
    const out = document.createElement("div"); out.className = "readout"; fd.appendChild(out);
    let st, run = !TP.reduced(), raf = null;   // reduced motion: starts paused (▶ «Riprendi»)
    function reset() {
      const ang = Math.random() * 2 * Math.PI, rad = 0.12 + 0.1 * Math.random();
      st = { x: rad * Math.cos(ang), y: rad * Math.sin(ang), vx: 0.03 * (Math.random() - .5), vy: 0.03 * (Math.random() - .5), xi: 0, ph: Math.random() * Math.PI, trail: [], lost: false, hist: [] };
      const sx = QUAD.stableX(v.a, v.q), sy = QUAD.stableY(v.a, v.q);
      out.innerHTML = `(a, q) = (${TP.fmt(v.a, 3)}, ${TP.fmt(v.q, 2)}): direzione x <span class="${sx ? "ok" : "bad"}">${sx ? "stabile" : "instabile"}</span>, direzione y <span class="${sy ? "ok" : "bad"}">${sy ? "stabile" : "instabile"}</span>. ` +
        (sx && sy ? "Lo ione oscilla ma resta confinato: attraversa il quadrupolo." : "L'ampiezza cresce esponenzialmente: lo ione colpisce una barra e si scarica.") +
        (v.a === 0 && v.q < 0.4 ? ` Frequenza secolare ≈ β·Ω/2 con β ≈ q/√2 = ${TP.fmt(v.q / Math.SQRT2, 2)}.` : "");
      if (!raf) loop();   // paused: loop() only draws one frame
    }
    function drawField() {
      const { ctx, W, H } = c1, S = Math.min(W, H), cx = W / 2, cy = H / 2, sc = S * 0.26; // r0 in pixels
      const phi0 = v.a / 2 - v.q * Math.cos(2 * (st.xi + st.ph)); // proportional to U - V cos(Wt): U ~ a/8, V ~ q/4
      // draw at CSS-pixel resolution, then scale through drawImage
      const off = drawField.off || (drawField.off = document.createElement("canvas"));
      off.width = Math.round(W); off.height = Math.round(H);
      const o = off.getContext("2d"), id = o.createImageData(off.width, off.height), d = id.data;
      const mx = Math.max(0.05, Math.abs(phi0)) * 2.2;
      for (let j = 0; j < off.height; j++) for (let i = 0; i < off.width; i++) {
        const x = (i - cx) / sc, y = (j - cy) / sc, p = phi0 * (x * x - y * y) / mx, k = 4 * (j * off.width + i);
        const t = Math.max(-1, Math.min(1, p));
        if (t >= 0) { d[k] = 255 - 60 * t; d[k + 1] = 255 - 150 * t; d[k + 2] = 255 - 200 * t; } else { d[k] = 255 + 200 * t; d[k + 1] = 255 + 120 * t; d[k + 2] = 255 + 40 * t; }
        d[k + 3] = 255;
      }
      o.putImageData(id, 0, 0);
      ctx.clearRect(0, 0, W, H); ctx.drawImage(off, 0, 0, W, H);
      // rods (round rods, r = 1.13 r0)
      const rr = 1.13 * sc;
      [[1, 0], [-1, 0], [0, 1], [0, -1]].forEach(([dx, dy]) => {
        const pos = dx !== 0, sign = Math.sign(phi0) * (pos ? 1 : -1);
        ctx.beginPath(); ctx.arc(cx + dx * (sc + rr), cy + dy * (sc + rr), rr, 0, 7);
        ctx.fillStyle = sign > 0 ? "#c2410c" : "#2b5c8a"; ctx.globalAlpha = .85; ctx.fill(); ctx.globalAlpha = 1;
        ctx.fillStyle = "#fff"; ctx.font = "bold 18px system-ui"; ctx.textAlign = "center"; ctx.textBaseline = "middle";
        ctx.fillText(sign > 0 ? "+" : "−", cx + dx * (sc + rr * 0.7), cy + dy * (sc + rr * 0.7));
      });
      ctx.strokeStyle = "#9b978c"; ctx.setLineDash([3, 4]); ctx.beginPath(); ctx.arc(cx, cy, sc, 0, 7); ctx.stroke(); ctx.setLineDash([]);
      // trail and ion
      ctx.strokeStyle = "rgba(36,35,31,.55)"; ctx.lineWidth = 1; ctx.beginPath();
      st.trail.forEach(([x, y], i) => { const px = cx + x * sc, py = cy + y * sc; i ? ctx.lineTo(px, py) : ctx.moveTo(px, py); }); ctx.stroke();
      ctx.fillStyle = st.lost ? "#b42318" : "#24231f"; ctx.beginPath(); ctx.arc(cx + st.x * sc, cy + st.y * sc, 4.5, 0, 7); ctx.fill();
      ctx.fillStyle = "#24231f"; ctx.font = "12px system-ui"; ctx.textAlign = "left"; ctx.textBaseline = "top";
      ctx.fillStyle = "rgba(255,255,255,.85)"; ctx.fillRect(4, 3, 236, 18); ctx.fillStyle = "#24231f"; ctx.fillText("sezione x–y, cerchio tratteggiato = r₀", 8, 6);
      if (st.lost) { ctx.fillStyle = "#b42318"; ctx.fillText("ione perso sulle barre", 8, 22); }
    }
    function drawHist() {
      const n = Math.max(6, (st.hist.length ? st.hist[st.hist.length - 1][0] : 0));
      const ax = TP.axes(c2, { x0: 0, x1: n, y0: -1.2, y1: 1.2, xl: "cicli RF", yl: "spostamento / r₀", yfmt: x => TP.fmt(x, 1), xfmt: x => TP.fmt(x, 0), yticks: [-1, -0.5, 0, 0.5, 1] });
      TP.line(ax, [0, n], [1, 1], "#b42318", 1, [4, 4]); TP.line(ax, [0, n], [-1, -1], "#b42318", 1, [4, 4]);
      ax.clip();
      TP.line(ax, st.hist.map(h => h[0]), st.hist.map(h => h[1]), col(2), 1.6, TP.DASH[1]);
      TP.line(ax, st.hist.map(h => h[0]), st.hist.map(h => h[2]), col(1), 1.6);
      ax.ctx.restore();
      TP.legend(ax, [["x (barre +)", col(2), TP.DASH[1]], ["y (barre −)", col(1)]]);
    }
    function loop() {
      raf = null; if (!run) { drawField(); drawHist(); return; }
      const steps = Math.round(6 * v.sp), h = Math.PI / 60;
      for (let s = 0; s < steps && !st.lost; s++) {
        [st.x, st.vx] = QUAD.rk4(st.x, st.vx, st.xi, h, v.a, v.q, st.ph);
        [st.y, st.vy] = QUAD.rk4(st.y, st.vy, st.xi, h, -v.a, -v.q, st.ph);
        st.xi += h; st.trail.push([st.x, st.y]); if (st.trail.length > 900) st.trail.shift();
        st.hist.push([st.xi / Math.PI, st.x, st.y]); if (st.hist.length > 3000) st.hist.shift();
        if (Math.abs(st.x) > 1 || Math.abs(st.y) > 1) st.lost = true;
      }
      drawField(); drawHist();
      if (st.xi / Math.PI > 60 && !st.lost) reset();
      raf = requestAnimationFrame(loop);
    }
    // pause the animation when not visible (saves battery)
    if ("IntersectionObserver" in window) new IntersectionObserver(es => es.forEach(e => { const vis = e.isIntersecting; if (vis && run && !raf) loop(); if (!vis && raf) { cancelAnimationFrame(raf); raf = null; } })).observe(fd);
    c1.onresize = c2.onresize = () => { drawField(); drawHist(); };
    if (!run) bt.querySelector("[data-k=play]").textContent = "Riprendi";
    reset();
  }

  // ------------------------------------------------------------ 2) stability diagram + mass filter with real numbers
  const sd = document.getElementById("sim-stab");
  if (sd) {
    let zoom = false;
    const bt = TP.buttons(sd, [["full", "Prima regione di stabilità"], ["zoom", "Ingrandisci il vertice"]], "full", k => { zoom = k === "zoom"; img = null; draw(); });
    const v = TP.controls(sd, [
      { id: "ms", label: "m/z impostato (filtro)", min: 100, max: 1000, step: 1, value: 300, fmt: x => x },
      { id: "res", label: "Rapporto U/V (in % del valore al vertice)", min: 0, max: 99.95, step: 0.05, value: 99.5, fmt: x => TP.fmt(x, 2) + " %" },
      { id: "mi", label: "m/z dello ione in esame", min: 280, max: 320, step: 0.1, value: 301, fmt: x => TP.fmt(x, 1) },
      { id: "ez", label: "Energia assiale dello ione", min: 1, max: 20, step: 0.5, value: 5, unit: "eV", fmt: x => TP.fmt(x, 1) },
    ], () => draw());
    const wrap = document.createElement("div"); wrap.className = "grid2"; sd.appendChild(wrap);
    const L = document.createElement("div"), R = document.createElement("div"); wrap.append(L, R);
    const c1 = TP.canvas(L, 330), c2 = TP.canvas(R, 330);
    const out = document.createElement("div"); out.className = "readout"; sd.appendChild(out);
    const r0 = 4.0e-3, f = 1.0e6, Lrod = 0.20;
    let img = null;
    // ion m/z slider follows the set mass (+-20)
    const miInput = sd.querySelectorAll(".ctl input[type=range]")[2];
    function syncMi() { miInput.min = v.ms - 20; miInput.max = v.ms + 20; if (v.mi < v.ms - 20 || v.mi > v.ms + 20) { v._set_mi(v.ms + 1); v.mi = v.ms + 1; } }
    function region(ax, q0, q1, a0, a1) {
      const x0 = ax.X(q0), x1 = ax.X(q1), y0 = ax.Y(a1), y1 = ax.Y(a0);
      const d = window.devicePixelRatio || 1, Wp = Math.round((x1 - x0) * d), Hp = Math.round((y1 - y0) * d);
      const tmp = document.createElement("canvas"); tmp.width = Wp; tmp.height = Hp;
      const tc = tmp.getContext("2d"), id = tc.createImageData(Wp, Hp), px = id.data, step = Math.max(2, Math.round(2 * d));
      for (let j = 0; j < Hp; j += step) for (let i = 0; i < Wp; i += step) {
        const q = q0 + ((i + step / 2) / Wp) * (q1 - q0), a = a1 - ((j + step / 2) / Hp) * (a1 - a0);
        const sx = QUAD.stableX(a, q), sy = QUAD.stableY(a, q);
        // «solo x» and «solo y» are hatched (/ and \\), «stable in both» is solid: the regions differ by more than the colour
        const bandX = Math.floor((i + j) / (step * 3)) % 2 === 0, bandY = Math.floor((i - j + Wp) / (step * 3)) % 2 === 0;
        const c = sx && sy ? [43, 92, 138, 125] : sx ? [194, 65, 12, bandX ? 95 : 0] : sy ? [4, 120, 87, bandY ? 95 : 0] : [0, 0, 0, 0];
        for (let jj = 0; jj < step && j + jj < Hp; jj++) for (let ii = 0; ii < step && i + ii < Wp; ii++) { const k = 4 * ((j + jj) * Wp + i + ii); px[k] = c[0]; px[k + 1] = c[1]; px[k + 2] = c[2]; px[k + 3] = c[3]; }
      }
      tc.putImageData(id, 0, 0);
      img = { w: ax.W, zoom, tmp, x0, y0, wC: x1 - x0, hC: y1 - y0 };
    }
    function draw() {
      syncMi();
      const q0 = zoom ? 0.66 : 0, q1 = zoom ? 0.76 : 1.0, a0 = zoom ? 0.20 : 0, a1 = zoom ? 0.245 : 0.30;
      const ax = TP.axes(c1, { x0: q0, x1: q1, y0: a0, y1: a1, xl: "q", yl: "a", xfmt: x => TP.fmt(x, zoom ? 2 : 1), yfmt: x => TP.fmt(x, zoom ? 3 : 2) });
      if (!(img && img.w === ax.W && img.zoom === zoom)) region(ax, q0, q1, a0, a1);
      ax.ctx.drawImage(img.tmp, img.x0, img.y0, img.wC, img.hC);
      const k = APEX_K * v.res / 100, win = QUAD.window(k);
      // the set mass sits at the centre of the stable window (or at q = 0.706 for RF-only / no window)
      const qset = win && win[0] > 0.01 ? (win[0] + win[1]) / 2 : 0.706;
      const V = QUAD.VOf(v.ms, qset, r0, f), U = k * V / 2;
      TP.line(ax, [0, 1], [0, k], "#24231f", 1.4);
      // ions: set mass and neighbours
      const pts = [[v.ms - 1, "#9b978c"], [v.ms + 1, "#9b978c"], [v.mi, col(4)], [v.ms, "#24231f"]];
      pts.forEach(([m, c]) => { const q = qset * v.ms / m, a = k * q; if (q < q0 || q > q1) return; ax.ctx.fillStyle = c; ax.ctx.beginPath(); ax.ctx.arc(ax.X(q), ax.Y(a), m === v.mi ? 5.5 : 4, 0, 7); ax.ctx.fill(); });
      TP.label(ax, zoom ? 0.705 : 0.706, zoom ? 0.2365 : 0.237, "vertice (0,706; 0,237)", "#24231f", "center", "bottom", -8);
      TP.legend(ax, [["stabile x e y", "rgba(43,92,138,.6)"], ["solo x (a righe /)", "rgba(194,65,12,.7)", TP.DASH[1]], ["solo y (a righe \\)", "rgba(4,120,87,.7)", TP.DASH[2]]]);
      // trajectories of a bunch of ions of m/z = mi
      const qi = qset * v.ms / v.mi, ai = k * qi;
      const vz = Math.sqrt(2 * v.ez * QUAD.E / (v.mi * QUAD.AMU)), ncyc = Math.round(f * Lrod / vz);
      const N = 60, show = 6, hist = []; let ok = 0;
      for (let n = 0; n < N; n++) {
        const ang = 2 * Math.PI * Math.random(), rr = 0.15 * Math.sqrt(Math.random());
        let x = rr * Math.cos(ang), y = rr * Math.sin(ang), vx = 0.04 * (Math.random() - .5), vy = 0.04 * (Math.random() - .5), xi = 0;
        const ph = Math.random() * Math.PI, h = Math.PI / 30, steps = ncyc * 30; let alive = true; const H = [];
        for (let s = 0; s < steps; s++) {
          [x, vx] = QUAD.rk4(x, vx, xi, h, ai, qi, ph); [y, vy] = QUAD.rk4(y, vy, xi, h, -ai, -qi, ph); xi += h;
          if (n < show && s % 3 === 0) H.push([xi / Math.PI, Math.max(-1.3, Math.min(1.3, x))]);
          if (Math.abs(x) > 1 || Math.abs(y) > 1) { alive = false; if (n < show) H.push([xi / Math.PI, Math.max(-1.3, Math.min(1.3, x))]); break; }
        }
        if (alive) ok++; if (n < show) hist.push(H);
      }
      const ax2 = TP.axes(c2, { x0: 0, x1: ncyc, y0: -1.3, y1: 1.3, xl: `cicli RF percorsi (${ncyc} = 20 cm a ${TP.fmt(v.ez, 1)} eV)`, yl: "x / r₀", yticks: [-1, 0, 1], xfmt: x => TP.fmt(x, 0) });
      TP.line(ax2, [0, ncyc], [1, 1], "#b42318", 1, [4, 4]); TP.line(ax2, [0, ncyc], [-1, -1], "#b42318", 1, [4, 4]);
      ax2.clip(); hist.forEach((H, i) => TP.line(ax2, H.map(p => p[0]), H.map(p => p[1]), [col(1), col(2), col(3), col(4), col(5), col(6)][i], 1.1)); ax2.ctx.restore();
      let wtxt = "nessuna finestra stabile su questa retta (sopra il vertice)";
      if (win) { const mlo = qset * v.ms / win[1], mhi = qset * v.ms / win[0]; wtxt = win[0] < 0.01 ? `trasmette tutto sopra m/z ${TP.fmt(mlo, 1)} (solo RF: filtro passa-alto)` : `finestra stabile m/z ${TP.fmt(mlo, 2)}–${TP.fmt(mhi, 2)} (larghezza ${TP.fmt(mhi - mlo, 2)}, R ≈ ${TP.fmt(v.ms / (mhi - mlo), 0)})`; }
      out.innerHTML = `Con r₀ = 4 mm e f = 1 MHz, per centrare m/z ${v.ms} nella finestra servono <b>V = ${TP.fmt(V, 0)} V</b> (0-picco) e <b>U = ${TP.fmt(U, 1)} V</b>. Retta di lavoro: ${wtxt}. ` +
        `Ione a m/z ${TP.fmt(v.mi, 1)}: (a, q) = (${TP.fmt(ai, 4)}, ${TP.fmt(qi, 4)}), ` + `<b>${QUAD.stable(ai, qi) ? "<span class='ok'>stabile</span>" : "<span class='bad'>instabile</span>"}</b>; trasmessi <b>${Math.round(100 * ok / N)} %</b> di ${N} ioni simulati (ingresso entro 0,15 r₀, fase RF casuale).`;
    }
    c1.onresize = c2.onresize = () => { img = null; draw(); };
    draw();
  }

  // ------------------------------------------------------------ 3) scanning: building a spectrum
  const sc = document.getElementById("sim-scan");
  if (sc) {
    const IONS = [[152, 60], [200, 100], [201, 11], [300, 80], [301, 17], [302, 26], [500, 45], [501, 13], [502, 15], [800, 30], [801, 12]];
    let mode = "R";
    TP.buttons(sc, [["R", "U/V costante (R costante)"], ["D", "Δm costante (risoluzione unitaria)"], ["RF", "Solo RF (U = 0)"]], "R", k => { mode = k; draw(); });
    const v = TP.controls(sc, [
      { id: "res", label: "U/V in % del vertice (modo R costante)", min: 95, max: 99.95, step: 0.05, value: 99.8, fmt: x => TP.fmt(x, 2) + " %" },
      { id: "dm", label: "Larghezza della finestra (modo Δm costante)", min: 0.3, max: 2, step: 0.05, value: 0.7, unit: "Da", fmt: x => TP.fmt(x, 2) },
      { id: "z0", label: "Zoom su m/z", min: 150, max: 800, step: 1, value: 300, fmt: x => x },
    ], () => draw());
    const wrap = document.createElement("div"); wrap.className = "grid2"; sc.appendChild(wrap);
    const L = document.createElement("div"), R = document.createElement("div"); wrap.append(L, R);
    const c1 = TP.canvas(L, 280), c2 = TP.canvas(R, 280);
    const out = document.createElement("div"); out.className = "readout"; sc.appendChild(out);
    // table: relative window width w(k) = (qr - ql)/qc along lines below the apex
    const TAB = []; for (let i = 0; i <= 120; i++) { const k = APEX_K * (0.9 + 0.0999 * i / 120); const w = QUAD.window(k); if (w) TAB.push([k, (w[1] - w[0]) / ((w[1] + w[0]) / 2), w]); }
    // window for a requested relative width, interpolated in the table (no new stability search)
    const winForRel = rel => {
      for (let i = 0; i < TAB.length - 1; i++) if (TAB[i][1] >= rel && TAB[i + 1][1] <= rel) {
        const t = (rel - TAB[i][1]) / (TAB[i + 1][1] - TAB[i][1]), A = TAB[i][2], B = TAB[i + 1][2];
        return [TAB[i][0] + t * (TAB[i + 1][0] - TAB[i][0]), [A[0] + t * (B[0] - A[0]), A[1] + t * (B[1] - A[1])]];
      }
      const e = TAB[rel > TAB[0][1] ? 0 : TAB.length - 1]; return [e[0], e[2]];
    };
    const DEPTH = 0.0016; // relative q-depth needed for full transmission (acceptance; illustrative)
    function trans(m, s) {
      // ion of m/z m while the filter is set to s
      let k, w;
      if (mode === "RF") { k = 0; w = QUAD.window(0); } else if (mode === "R") { k = APEX_K * v.res / 100; w = QUAD.window(k); } else [k, w] = winForRel(v.dm / s);
      if (!w) return 0;
      const q = 0.706 * s / m; // the set mass sits at q = 0.706
      const qc = k === 0 ? 0.706 : (w[0] + w[1]) / 2, qq = q * qc / 0.706; // centre the window on the set mass
      if (qq <= w[0] || qq >= w[1]) return 0;
      if (k === 0) return Math.min(1, (w[1] - qq) / (DEPTH * qq));
      const d = Math.min(qq - w[0], w[1] - qq) / qq;
      return Math.min(1, d / DEPTH);
    }
    function spec(s0, s1, n) { const xs = [], ys = []; for (let i = 0; i <= n; i++) { const s = s0 + (s1 - s0) * i / n; xs.push(s); ys.push(IONS.reduce((t, [m, a]) => t + a * trans(m, s), 0)); } return [xs, ys]; }
    function fwhm(m0) { const [xs, ys] = spec(m0 - 3, m0 + 3, 1200); const one = xs.map(s => trans(m0, s)); const mx = Math.max(...one); if (!mx) return [0, 0]; const ix = one.map((y, i) => y >= mx / 2 ? i : -1).filter(i => i >= 0); return [xs[ix[ix.length - 1]] - xs[ix[0]], mx]; }
    function draw() {
      const [xs, ys] = spec(140, 820, 2400), mx = Math.max(...ys, 1);
      const ax = TP.axes(c1, { x0: 140, x1: 820, y0: 0, y1: mx * 1.1, xl: "m/z impostato durante la scansione", yl: "segnale", yfmt: () => "" });
      TP.line(ax, xs, ys, col(1), 1.4);
      const z0 = v.z0, [xz, yz] = spec(z0 - 4, z0 + 4, 900), mz = Math.max(...yz, 1);
      const ax2 = TP.axes(c2, { x0: z0 - 4, x1: z0 + 4, y0: 0, y1: mz * 1.12, xl: "m/z (zoom)", yl: "segnale", yfmt: () => "", xfmt: x => TP.fmt(x, 0) });
      TP.line(ax2, xz, yz, col(1), 2);
      IONS.filter(([m]) => m > z0 - 4 && m < z0 + 4).forEach(([m, a]) => TP.label(ax2, m, 0, String(m), "#6b675c", "center", "top", 4));
      const rows = [200, 500, 800].map(m => { const [w, h] = fwhm(m); return `m/z ${m}: FWHM ${w ? TP.fmt(w, 2) : "–"} Da, trasmissione ${TP.fmt(100 * h, 0)} %`; });
      out.innerHTML = (mode === "RF" ? "Solo RF: ogni ione sopra il taglio di bassa massa passa, il segnale è la somma di tutti: nessuna separazione. " :
        mode === "R" ? "U/V costante: la risoluzione relativa R = m/Δm è costante, quindi Δm cresce con la massa. " :
        "Δm costante: U/V è corretto a ogni massa (così lavorano gli strumenti in «risoluzione unitaria»); ad alta massa la finestra stretta costa trasmissione. ") + rows.join(" · ");
    }
    c1.onresize = c2.onresize = draw;
    draw();
  }
}, 0)));
