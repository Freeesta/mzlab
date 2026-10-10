// RT x m/z map: true zoom, sight (crosshair) with the two plots at the margins, lock, keyboard, full screen (classic script, loaded after explore.js).
// The map itself (colours, difference, 3D) is drawn by drawMap in explore.js; this file adds what lives on top of it. Hooks in explore.js:
// drawMap (zoomImg before the image is drawn, after at the end), attach (wheel, box zoom, click, right click, mouse move), ctl (decorate).
// Preferences of this browser in localStorage "qqq.mappa".
window.MAPPA = (() => {
  const T = (k, o) => I18N.t(k, o);
  const PREF_KEY = "qqq.mappa";
  const PREF = (() => { let o = {}; try { o = JSON.parse(localStorage.getItem(PREF_KEY) || "{}"); } catch (e) { /* no storage */ } return o; })();
  const prefSave = () => { try { localStorage.setItem(PREF_KEY, JSON.stringify(PREF)); } catch (e) { /* no storage */ } };
  const SIDE = 180, BOTTOM = 140, NARROW = 700, QUIET = 150;
  const st = p => p._mz || (p._mz = { lock: null, kp: null, hov: null, mir: null, reg: null, regT: 0, spec: null, xic: null, xicB: null });
  const is2d = p => p.type === "map" && p.view !== "3d" && p._a && p._a.map && !p._a.is3d;
  const isMax = p => p.el && p.el.classList.contains("max");
  const mirOn = p => { const s = st(p); return s.mir != null ? s.mir : isMax(p); };
  const wide = p => p.el && p.el.clientWidth >= NARROW;
  const point = p => { const s = st(p); return s.lock || s.kp || s.hov; };
  const isHrF = f => !!(f && window.HR && HR.isHr(f, f.lv || 1));

  // ---------------------------------------------------------------- DOM: grid with the map, the overlay (crosshair) and the two margin plots
  function init(p) {
    if (p._mzInit || !p.cv) return; p._mzInit = true;
    const cv = p.cv, grid = document.createElement("div");
    grid.className = "mzgrid"; cv.before(grid); grid.appendChild(cv);
    const ov = document.createElement("canvas"); ov.className = "mzov"; grid.appendChild(ov);
    const sp = document.createElement("canvas"); sp.className = "mzside"; sp.hidden = true; grid.appendChild(sp);
    const xc = document.createElement("canvas"); xc.className = "mzbot"; xc.hidden = true; grid.appendChild(xc);
    Object.assign(st(p), { ov, sp, xc, grid });
    cv.tabIndex = 0; cv.setAttribute("aria-label", T("mappa.aria"));
    cv.addEventListener("keydown", e => key(p, e));
    p.el.addEventListener("keydown", e => { if (isMax(p) && e.target === p.el) key(p, e); });
    // long press on a touch screen = lock the point
    let lp = 0;
    cv.addEventListener("touchstart", e => { if (e.touches.length !== 1 || !is2d(p)) return; const t = e.touches[0], r = cv.getBoundingClientRect(), px = t.clientX - r.left, py = t.clientY - r.top; clearTimeout(lp); lp = setTimeout(() => { lockAt(p, px, py); }, 550); }, { passive: true });
    ["touchend", "touchmove", "touchcancel"].forEach(n => cv.addEventListener(n, () => clearTimeout(lp), { passive: true }));
    const wrap = () => {
      if (!p._setMax || p._setMax._mz) return;
      const orig = p._setMax, f = on => { orig(on); onMax(p, on); }; f._mz = true; p._setMax = f;
    };
    wrap(); setTimeout(wrap, 0);
  }
  let fsByUs = null;
  function onMax(p, on) {
    st(p).mir = null;                                                   // the sight follows the view again: off in the page, on in full screen
    layout(p);
    if (on) {
      const de = document.documentElement;       // the whole page goes full screen (menus and dialogs, which live in the page, stay visible); the panel covers it
      if (!document.fullscreenElement && de.requestFullscreen) de.requestFullscreen().then(() => { fsByUs = p; }).catch(() => { /* iPad and others: the fixed panel is the full screen */ });
      setTimeout(() => p.cv.focus({ preventScroll: true }), 0);
    } else if (fsByUs === p && document.fullscreenElement) { fsByUs = null; document.exitFullscreen().catch(() => {}); }
  }
  document.addEventListener("fullscreenchange", () => { if (!document.fullscreenElement && fsByUs) { const p = fsByUs; fsByUs = null; if (isMax(p)) p._setMax(false); } });
  function layout(p) {
    const s = st(p); if (!s.grid) return;
    const m = mirOn(p) && is2dView(p) && wide(p);
    s.grid.classList.toggle("mir", m); s.sp.hidden = !m; s.xc.hidden = !m;
    p.el.classList.toggle("mzmir", m);
    const grow = m && !isMax(p);                                        // in the page the panel grows by the height of the XIC, and shrinks back after
    if (grow !== !!s.grown) { p.h = Math.max(200, p.h + (grow ? 1 : -1) * (BOTTOM + 80)); s.grown = grow; apply(p); if (typeof relayout === "function") relayout(); fitHost(); uiSave(); }
  }
  const is2dView = p => p.type === "map" && p.view !== "3d";

  // ---------------------------------------------------------------- the true zoom: the region in view on new bins, asked 150 ms after the last change
  function normFactor(m, norm) {
    if (norm !== "max" && norm !== "tic") return 1;
    let t = 0; for (let i = 0; i < m.length; i++) t = norm === "max" ? Math.max(t, m[i]) : t + m[i];
    return t > 0 ? (norm === "max" ? 1 : 1000) / t : 1;
  }
  // ---------------------------------------------------------------- the difference (extends the one of explore.js): computed by the server in numpy
  // with a tolerance in RT (the reference is replaced by its local maximum over +-0.1 min: background lines and small drifts cancel); what it shows
  // (all | increases only | decreases only) and the noise threshold (fraction of the 99.5th percentile) are choices of this browser
  const diffOpt = () => ({ show: PREF.show || "all", thr: PREF.thr ?? 0.02, rttol: PREF.rttol ?? 0.1 });
  const diffMap = (k, ref, lv, norm) => memo(`mzd${k}|${ref}|${lv}|${norm}|${diffOpt().rttol}`, () => J(`api/map?k=${k}&level=${lv}&ref=${ref}&norm=${norm}&rttol=${diffOpt().rttol}`).then(j => {
    const bin = atob(j.data), u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
    return new Float32Array(u.buffer);
  }));
  const getRegion = (k, lv, r, extra = "") => memo(`mzr${k}|${lv}|${r.join("|")}${extra}`, () => J(`api/map?k=${k}&level=${lv}&rt0=${r[0]}&rt1=${r[1]}&mz0=${r[2]}&mz1=${r[3]}&nrt=${r[4]}&nmz=${r[5]}${extra}`).then(j => {
    const bin = atob(j.data), u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
    return { ...j, m: new Float32Array(u.buffer) };
  }));
  function regionGrid(p, A, x0, x1, y0, y1, pw, ph) {
    const f = E.files[A.k], hr = isHrF(f), step = Math.max(scanStep(), 1e-4);
    const d = Math.min(2, devicePixelRatio || 1), nrt = Math.max(2, Math.min(1000, Math.max(Math.round(pw * d), Math.ceil((x1 - x0) / (A.rt1 - A.rt0) * A.nrt)), Math.floor((x1 - x0) / step)));   // never more bins than scans: no empty stripes
    const nmz = Math.max(2, Math.min(1000, Math.max(Math.round(ph * d), Math.ceil((y1 - y0) / A.dmz)), hr ? 1000 : Math.floor((y1 - y0) / 0.1)));  // never coarser than the whole map; low resolution: bins of at least 0.1
    return [+x0.toFixed(5), +x1.toFixed(5), +y0.toFixed(5), +y1.toFixed(5), nrt, nmz];
  }
  // called by drawMap before the image is drawn: true = the region image was drawn (otherwise the whole-map image is stretched, as before)
  function zoomImg(p, g, o) {
    const s = st(p); init(p);
    if (!p.zoom && !p.zoomY) { s.reg = null; return false; }
    const { A, B, im, x0, x1, y0, y1, pw, ph, norm } = o;
    const r = regionGrid(p, A, x0, x1, y0, y1, pw, ph), key = `${A.k}|${B ? B.k : ""}|${A.level}|${norm}|${p.scale}|${r.join("|")}|${JSON.stringify(diffOpt())}`;
    if (s.reg && s.reg.key === key && s.reg.off) { g.imageSmoothingEnabled = false; g.drawImage(s.reg.off, 0, (s.reg.mz1 - y1) / s.reg.dmz, s.reg.nrt, (y1 - y0) / s.reg.dmz, M.l, M.t, pw, ph); return true; }
    clearTimeout(s.regT);
    s.regT = setTimeout(async () => {
      try {
        const a = await getRegion(A.k, A.level, r, B ? `&ref=${B.k}&norm=${norm}&rttol=${diffOpt().rttol}` : "");
        const fa = B ? 1 : normFactor(A.m, norm), n = a.m.length, v = new Float32Array(n);
        for (let i = 0; i < n; i++) v[i] = a.m[i] * fa;                    // with a reference the server already gives A - B (normalised, tolerance in RT)
        s.reg = { key, v, nrt: a.nrt, nmz: a.nmz, rt0: a.rt0, rt1: a.rt1, mz0: a.mz0, mz1: a.mz1, dmz: a.dmz, off: paint(v, a.nrt, a.nmz, im, !!B) };
        draw(p);
      } catch (e) { /* keep the stretched whole map */ }
    }, QUIET);
    return false;
  }
  function paint(v, nrt, nmz, im, diff) {
    const off = document.createElement("canvas"); off.width = nrt; off.height = nmz;
    const ctx = off.getContext("2d"), d = ctx.createImageData(nrt, nmz), tab = diff ? LUT_DIV : LUT_SEQ;
    for (let i = 0; i < nrt; i++) for (let j = 0; j < nmz; j++) {
      const x = v[i * nmz + j], t = im.T(x), q = diff ? Math.round(127.5 + Math.sign(-x) * t * 127.5) : Math.round(t * 255), o = ((nmz - 1 - j) * nrt + i) * 4;
      d.data[o] = tab[q * 4]; d.data[o + 1] = tab[q * 4 + 1]; d.data[o + 2] = tab[q * 4 + 2]; d.data[o + 3] = 255;
    }
    ctx.putImageData(d, 0, 0); return off;
  }
  // value of the map at (rt, m/z): from the zoomed region when there is one, otherwise from the whole map
  function valueAt(p, rt, mz) {
    const s = st(p), a = p._a; if (!a || !a.mapv) return null;
    const R = s.reg && p.zoom || s.reg && p.zoomY ? s.reg : null;
    if (R) { const i = Math.floor((rt - R.rt0) / (R.rt1 - R.rt0) * R.nrt), j = Math.floor((mz - R.mz0) / R.dmz); if (i >= 0 && i < R.nrt && j >= 0 && j < R.nmz) return R.v[i * R.nmz + j]; }
    return a.mapv(rt, mz);
  }
  const binDa = p => { const s = st(p), a = p._a; if (s.reg && (p.zoom || p.zoomY)) return s.reg.dmz; return a && a.dmz ? a.dmz : 1; };

  // ---------------------------------------------------------------- after the map is drawn: overlay and margins
  function after(p) {
    init(p); layout(p);
    const s = st(p), a = p._a; if (!a || !s.ov) return;
    const r = p.cv.getBoundingClientRect(), d = Math.min(4, devicePixelRatio || 1);
    s.ov.width = Math.round(r.width * d); s.ov.height = Math.round(r.height * d); s.ov.style.width = r.width + "px"; s.ov.style.height = r.height + "px";
    s.ovg = s.ov.getContext("2d"); s.ovg.setTransform(d, 0, 0, d, 0, 0);
    overlay(p); margins(p); table(p);
  }
  function overlay(p) {
    const s = st(p), g = s.ovg, a = p._a; if (!g || !a) return;
    g.clearRect(0, 0, a.W, a.H);
    if (!is2d(p)) return;
    drawPoints(p, g);
    const pt = point(p), on = mirOn(p);
    if (pt && (on || s.lock) && pt.rt >= a.x0 && pt.rt <= a.x1 && pt.mz >= a.y0 && pt.mz <= a.y1) {
      const x = Math.round(a.X(pt.rt)) + .5, y = Math.round(a.Y(pt.mz)) + .5;
      g.save(); g.strokeStyle = css("--ink"); g.globalAlpha = s.lock ? .9 : .6; g.lineWidth = 1; if (!s.lock) g.setLineDash([4, 3]);
      g.beginPath(); g.moveTo(M.l, y); g.lineTo(a.W - M.r, y); g.moveTo(x, M.t); g.lineTo(x, a.H - M.b); g.stroke(); g.setLineDash([]);
      if (s.lock) {                                                     // the padlock next to the point
        g.globalAlpha = 1; g.fillStyle = css("--panel"); g.strokeStyle = css("--ink"); g.lineWidth = 1.4;
        const lx = Math.min(a.W - M.r - 14, x + 6), ly = Math.max(M.t + 12, y - 8);
        g.fillRect(lx, ly - 6, 10, 8); g.strokeRect(lx, ly - 6, 10, 8); g.beginPath(); g.arc(lx + 5, ly - 6, 3.2, Math.PI, 0); g.stroke();
      }
      g.restore();
    }
    readout(p);
  }
  function readout(p) {
    const pt = point(p), a = p._a; if (!a || !is2d(p)) return;
    if (!pt || !(mirOn(p) || st(p).lock)) return;
    const s = st(p), f = a.f, v = valueAt(p, pt.rt, pt.mz), sc = s.spec && s.spec.sid != null ? T("mappa.read.scan", { n: s.spec.sid + 1 }) + " · " : "";
    p.rd.textContent = `${f ? f.label + " · " : ""}${sc}RT ${pt.rt.toFixed(2)} min · m/z ${pt.mz.toFixed(mzd(p))} · ${T("mappa.read.int")} ${v == null ? "–" : fmt(v)}${s.lock ? " · " + T("mappa.read.locked") : ""}`;
  }

  // ---------------------------------------------------------------- the two plots at the margins: spectrum of the scan (m/z aligned with the map) and XIC (RT aligned)
  let mT = 0;
  function margins(p) {
    const s = st(p); if (!s.sp || s.sp.hidden || !is2d(p)) return;
    const pt = point(p); drawSide(p, pt); drawBottom(p, pt);
    clearTimeout(mT); if (!pt) return;
    mT = setTimeout(async () => {
      const q = point(p); if (!q) return;
      await Promise.all([fetchSpec(p, q.rt), fetchXic(p, q.mz)]);
      const r = point(p); drawSide(p, r); drawBottom(p, r); readout(p);
    }, 60);
  }
  const xicTol = (f, mz) => isHrF(f) ? HR.tolDa(f, mz, f.lv || 1) : 0.5;           // LR: window [n - 0.2, n + 0.8] = centre n + 0.3, +-0.5
  const xicCentre = (f, mz) => isHrF(f) ? mz : Math.round(mz) + 0.3;
  async function fetchXic(p, mz) {
    const s = st(p), a = p._a; if (!a) return null; const f = a.f, c = xicCentre(f, mz), tol = xicTol(f, mz), key = `${f.k}|${c.toFixed(4)}|${tol.toFixed(5)}`;
    if (s.xic && s.xic.key === key) return s.xic;
    const ks = [f.k, ...(a.ref ? [a.ref.k] : [])].join(",");
    try { const j = await J(`api/xic?k=${ks}&mz=${c}&tol=${tol}&level=${f.lv || 1}`); s.xic = { key, c, tol, tr: j.traces }; } catch (e) { /* nothing to show */ }
    return s.xic;
  }
  async function fetchSpec(p, rt) {
    const s = st(p), a = p._a; if (!a) return null; const f = a.f;
    try {
      const n = await memo(`mzn${f.k}|${f.lv || 1}|${rt.toFixed(4)}`, () => J(`api/nearest_scan?k=${f.k}&rt=${rt}&level=${f.lv || 1}`));
      const key = `${f.k}|${n.sid}`; if (s.spec && s.spec.key === key) return s.spec;
      const j = await memo(`mzs${key}`, () => J(`api/spectrum?k=${f.k}&rt0=${n.rt - NEAR}&rt1=${n.rt + NEAR}&level=${f.lv || 1}&bin=0.1${MERGE()}`));
      s.spec = { key, sid: n.sid, rt: n.rt, mz: j.mz, y: j.y };
    } catch (e) { /* nothing to show */ }
    return s.spec;
  }
  function canvasOf(c) {
    const r = c.getBoundingClientRect(), d = Math.min(4, devicePixelRatio || 1); c.width = Math.round(r.width * d); c.height = Math.round(r.height * d);
    const g = c.getContext("2d"); g.setTransform(d, 0, 0, d, 0, 0); return { g, W: r.width, H: r.height };
  }
  function drawSide(p, pt) {                                            // spectrum turned on its side: m/z upwards like the map, intensity to the right
    const s = st(p), a = p._a; if (s.sp.hidden || !a) return;
    const { g, W, H } = canvasOf(s.sp), ink = css("--muted"); g.clearRect(0, 0, W, H); g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink;
    const Y = v => H - M.b - (v - a.y0) / (a.y1 - a.y0) * (H - M.t - M.b), x0 = 6, x1 = W - 8;
    g.beginPath(); g.moveTo(x0 + .5, M.t); g.lineTo(x0 + .5, H - M.b); g.stroke();
    const sp = s.spec; if (!sp || !pt) { g.fillText(T("mappa.side.title"), x0 + 4, M.t + 10); return; }
    let ymax = 0; const vis = []; for (let i = 0; i < sp.mz.length; i++) if (sp.mz[i] >= a.y0 && sp.mz[i] <= a.y1) { vis.push(i); if (sp.y[i] > ymax) ymax = sp.y[i]; }
    if (!ymax) { g.fillText(T("mappa.side.empty"), x0 + 4, M.t + 10); return; }
    g.strokeStyle = a.f.color || css("--accent"); g.beginPath();
    for (const i of vis) { const y = Math.round(Y(sp.mz[i])) + .5; g.moveTo(x0, y); g.lineTo(x0 + (x1 - x0) * sp.y[i] / ymax, y); }
    g.stroke();
    const top = vis.slice().sort((u, v) => sp.y[v] - sp.y[u]), used = [];
    g.fillStyle = css("--ink"); g.textAlign = "left";
    for (const i of top) { if (used.length >= 8 || sp.y[i] < ymax * 0.05) break; const y = Y(sp.mz[i]); if (used.some(u => Math.abs(u - y) < 12)) continue; used.push(y); g.fillText(sp.mz[i].toFixed(mzd(p)), Math.min(x1 - 40, x0 + 4 + (x1 - x0) * sp.y[i] / ymax), y + 4); }
    if (pt.mz >= a.y0 && pt.mz <= a.y1) { g.strokeStyle = css("--ink"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(x0, Y(pt.mz) + .5); g.lineTo(x1, Y(pt.mz) + .5); g.stroke(); g.setLineDash([]); }
    g.fillStyle = ink; g.fillText(`${T("mappa.read.scan", { n: sp.sid + 1 })} · RT ${sp.rt.toFixed(2)}`, x0 + 4, H - M.b + 14);
  }
  function drawBottom(p, pt) {                                          // XIC under the map: the same RT axis
    const s = st(p), a = p._a; if (s.xc.hidden || !a) return;
    const { g, W, H } = canvasOf(s.xc), ink = css("--muted"), mt = 10, mb = 22; g.clearRect(0, 0, W, H); g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink;
    const X = v => M.l + (v - a.x0) / (a.x1 - a.x0) * (W - M.l - M.r);
    g.beginPath(); g.moveTo(M.l, H - mb + .5); g.lineTo(W - M.r, H - mb + .5); g.stroke();
    for (const t of nice(a.x0, a.x1, 8)) { const x = X(t); if (x < M.l - 1 || x > W - M.r + 1) continue; g.textAlign = "center"; g.fillText(String(+t.toFixed(2)), x, H - mb + 13); }
    const xi = s.xic; if (!xi || !pt) { g.textAlign = "left"; g.fillText(T("mappa.bot.title"), M.l + 4, mt + 8); return; }
    let ymax = 0; const tr = xi.tr || [];
    for (const t of tr) for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= a.x0 && t.rt[i] <= a.x1 && t.y[i] > ymax) ymax = t.y[i];
    const Y = v => H - mb - v / (ymax || 1) * (H - mt - mb);
    tr.forEach((t, n) => {
      const f = E.files[t.k]; g.strokeStyle = f ? f.color : ink; g.lineWidth = 1.2; if (n) g.setLineDash([4, 3]); g.beginPath(); let first = true;
      for (let i = 0; i < t.rt.length; i++) { if (t.rt[i] < a.x0 || t.rt[i] > a.x1) continue; const x = X(t.rt[i]), y = Y(t.y[i]); if (first) { g.moveTo(x, y); first = false; } else g.lineTo(x, y); }
      g.stroke(); g.setLineDash([]); g.lineWidth = 1;
    });
    g.fillStyle = css("--ink"); g.textAlign = "left";
    const lo = xi.c - xi.tol, hi = xi.c + xi.tol;
    g.fillText(`XIC ${lo.toFixed(isHrF(a.f) ? 4 : 1)}–${hi.toFixed(isHrF(a.f) ? 4 : 1)} · max ${fmt(ymax)}`, M.l + 4, mt + 8);
    if (pt.rt >= a.x0 && pt.rt <= a.x1) { g.strokeStyle = css("--ink"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(X(pt.rt) + .5, mt); g.lineTo(X(pt.rt) + .5, H - mb); g.stroke(); g.setLineDash([]); }
  }

  // ---------------------------------------------------------------- mouse
  function move(p, px, py) {
    if (!is2d(p)) return; const s = st(p), a = p._a;
    if (px < M.l || px > a.W - M.r || py < M.t || py > a.H - M.b) { s.hov = null; } else { s.hov = { rt: xOf(p, px), mz: a.mzAt(py) }; s.kp = null; }
    if (mirOn(p)) { p.tip.hidden = true; p.vl.hidden = true; }
    if (!s.lock) { overlay(p); margins(p); } else readout(p);
  }
  function leave(p) { const s = st(p); s.hov = null; if (!s.lock && s.ovg) { overlay(p); margins(p); } }
  function wheel(p, e, px, py) {
    if (!is2d(p) || e.ctrlKey || e.metaKey) return false;               // Ctrl/Cmd + wheel keeps the old zoom of explore.js
    e.preventDefault();
    const a = p._a, dy = e.deltaY || e.deltaX, f = Math.exp(Math.max(-60, Math.min(60, dy)) * 0.004);
    if (!p._wz || Date.now() - p._wz > 600) pushZh(p); p._wz = Date.now();
    const x = xOf(p, px), y = a.mzAt(py);
    if (!e.altKey) p.zoomY = clampView(y - (y - a.y0) * f, y + (a.y1 - y) * f, a.fullY[0], a.fullY[1]);       // Maiusc = m/z only
    if (!e.shiftKey) p.zoom = clampView(x - (x - a.x0) * f, x + (a.x1 - x) * f, a.full[0], a.full[1]);         // Alt = RT only
    draw(p); return true;
  }
  function boxZoom(p, ax, ay, bx, by) {
    const a = p._a, u = [xOf(p, ax), xOf(p, bx)].sort((m, n) => m - n), v = [a.mzAt(ay), a.mzAt(by)].sort((m, n) => m - n);
    pushZh(p);
    if (Math.abs(bx - ax) > 4) p.zoom = clampView(u[0], u[1], a.full[0], a.full[1]);
    if (Math.abs(by - ay) > 4) p.zoomY = clampView(v[0], v[1], a.fullY[0], a.fullY[1]);
    draw(p); uiSave();
  }
  function whole(p) { pushZh(p); p.zoom = null; p.zoomY = null; p.sel = null; draw(p); uiSave(); }
  function lockAt(p, px, py) { const s = st(p), a = p._a; if (!a) return; s.lock = { rt: xOf(p, px), mz: a.mzAt(py) }; s.kp = null; overlay(p); margins(p); }
  function unlock(p) { const s = st(p); s.lock = null; overlay(p); margins(p); if (!mirOn(p)) p.rd.textContent = ""; }
  // right click: lock / unlock the point; Maiusc + right click: the XIC of that m/z (the old right-click entry, also in the «⋯» menu)
  function ctx(p, e, px, py) {
    if (!is2d(p)) return false;
    e.preventDefault(); hideHover(p);
    const a = p._a;
    if (e.shiftKey) { if (px >= M.l && px <= a.W - M.r && py >= M.t && py <= a.H - M.b) openXic(null, { mz: a.mzAt(py), obs: true }); return true; }
    const s = st(p); if (s.lock) unlock(p); else if (px >= M.l && px <= a.W - M.r && py >= M.t && py <= a.H - M.b) lockAt(p, px, py);
    return true;
  }
  // the «⋯» menu: the old right-click menu, at the point of the sight (locked, keyboard or the middle of the map)
  function menuAt(p, btn) {
    const a = p._a; if (!a) return; const pt = point(p) || { rt: (a.x0 + a.x1) / 2, mz: (a.y0 + a.y1) / 2 }, r = btn.getBoundingClientRect();
    ctxFor(p, { preventDefault() {}, clientX: r.left, clientY: r.bottom + 4 }, pt.rt, a.X(pt.rt), a.Y(pt.mz));
  }

  // ---------------------------------------------------------------- keyboard (focus on the map, or the map in full screen; never inside a text field)
  function key(p, e) {
    if (p.type !== "map" || e.target.closest && e.target.closest("input,textarea,select,[contenteditable='true']")) return;
    if (e.ctrlKey || e.metaKey) return;
    const k = e.key, s = st(p), a = p._a; if (!a) return;
    const done = () => { e.preventDefault(); e.stopPropagation(); };
    if (k === "f" || k === "F") { done(); p._setMax(!isMax(p)); return; }
    if (k === "0") { done(); whole(p); return; }
    if (k === "Backspace") { done(); undoZoom(p); return; }
    if (!is2d(p)) return;
    if (k === "m" || k === "M") { done(); s.mir = !mirOn(p); layout(p); draw(p); return; }
    if (k === "l" || k === "L") { done(); if (s.lock) unlock(p); else { const q = point(p) || { rt: (a.x0 + a.x1) / 2, mz: (a.y0 + a.y1) / 2 }; s.lock = { ...q }; overlay(p); margins(p); } return; }
    if (k === "Escape" && s.lock) { done(); unlock(p); return; }
    if (k === "s" || k === "S") { done(); s.mark = !s.mark; markBtn(p); return; }
    if (k === "Delete" && s.psel != null) { done(); delPoint(p, s.psel); return; }
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(k)) return;
    done();
    const cur = point(p) || { rt: (a.x0 + a.x1) / 2, mz: (a.y0 + a.y1) / 2 }, nx = { ...cur }, dir = k === "ArrowRight" || k === "ArrowUp" ? 1 : -1;
    const set = q => { if (s.lock) s.lock = q; else s.kp = q; follow(p, q); overlay(p); margins(p); };
    if (k === "ArrowUp" || k === "ArrowDown") {
      if (e.shiftKey) { fetchSpec(p, cur.rt).then(sp => { const m = peakStep(sp, cur.mz, dir); if (m != null) set({ ...cur, mz: m }); }); return; }   // next local maximum of the spectrum above 3 x noise
      nx.mz = Math.max(a.fullY[0], Math.min(a.fullY[1], cur.mz + dir * binDa(p))); set(nx); return;
    }
    fetchXic(p, cur.mz).then(xi => {
      const t = xi && xi.tr && xi.tr[0]; if (!t || !t.rt.length) { nx.rt = Math.max(a.full[0], Math.min(a.full[1], cur.rt + dir * scanStep())); set(nx); return; }
      const i0 = nearIdx(t.rt, cur.rt);
      if (e.shiftKey) { const i = apexStep(t, i0, dir); if (i != null) set({ ...cur, rt: t.rt[i] }); return; }       // apex of the XIC in that direction
      const i = Math.max(0, Math.min(t.rt.length - 1, (Math.abs(t.rt[i0] - cur.rt) < 1e-6 ? i0 : (dir > 0 ? (t.rt[i0] > cur.rt ? i0 - 1 : i0) : (t.rt[i0] < cur.rt ? i0 + 1 : i0))) + dir));   // next scan
      set({ ...cur, rt: t.rt[i] });
    });
  }
  // the view follows the point when it leaves it
  function follow(p, q) {
    const a = p._a; let ch = false;
    if (p.zoom && (q.rt < a.x0 || q.rt > a.x1)) { const w = a.x1 - a.x0; p.zoom = clampView(q.rt - w / 2, q.rt + w / 2, a.full[0], a.full[1]); ch = true; }
    if (p.zoomY && (q.mz < a.y0 || q.mz > a.y1)) { const h = a.y1 - a.y0; p.zoomY = clampView(q.mz - h / 2, q.mz + h / 2, a.fullY[0], a.fullY[1]); ch = true; }
    if (ch) draw(p);
  }
  const madNoise = y => { const v = Array.from(y).filter(x => x > 0).sort((m, n) => m - n); if (!v.length) return 0; const med = v[v.length >> 1], d = v.map(x => Math.abs(x - med)).sort((m, n) => m - n); return 1.4826 * d[d.length >> 1]; };
  function peakStep(sp, mz, dir) {
    if (!sp || !sp.mz || !sp.mz.length) return null;
    const thr = 3 * madNoise(sp.y), n = sp.mz.length;
    const isMax = i => sp.y[i] > thr && (i === 0 || sp.y[i] >= sp.y[i - 1]) && (i === n - 1 || sp.y[i] >= sp.y[i + 1]);
    if (dir > 0) { for (let i = 0; i < n; i++) if (sp.mz[i] > mz + 1e-6 && isMax(i)) return sp.mz[i]; }
    else { for (let i = n - 1; i >= 0; i--) if (sp.mz[i] < mz - 1e-6 && isMax(i)) return sp.mz[i]; }
    return null;
  }
  function apexStep(t, i0, dir) {
    const y = t.y, n = y.length, thr = 3 * madNoise(y);
    for (let i = i0 + dir; i > 0 && i < n - 1; i += dir) if (y[i] > thr && y[i] >= y[i - 1] && y[i] >= y[i + 1] && y[i] > 0) return i;
    return null;
  }


  // ---------------------------------------------------------------- marked points (variant A, for everybody): the student marks, the program MEASURES.
  // Tool «Segna» (button or S): a click adds a numbered point on the local maximum (+-0.2 min, +-1 bin); Canc removes the chosen one; a point can be
  // dragged. Saved in the notebook (NB.mappaPunti) for each pair of files. Table under the map: numbers only, never a name or a verdict.
  const pairKey = p => { const a = p._a; if (!a || !a.f) return null; return `${a.f.file}|${a.ref ? a.ref.file : ""}|${a.f.lv || 1}`; };
  const pts = p => { const k = pairKey(p); if (!k) return []; NB.mappaPunti = NB.mappaPunti || {}; return NB.mappaPunti[k] || (NB.mappaPunti[k] = []); };
  const ptsSave = p => { const k = pairKey(p); if (k && NB.mappaPunti && !NB.mappaPunti[k].length) delete NB.mappaPunti[k]; nbSave(); };
  const ppmOf = f => isHrF(f) ? ((HR.prof(f, f.lv || 1) || {}).tol || 5) : 0;
  async function addPoint(p, px, py) {
    const a = p._a, rt = xOf(p, px), mz = a.mzAt(py);
    let at = null; try { at = await J(`api/mapsnap?k=${a.f.k}&level=${a.f.lv || 1}&rt=${rt}&mz=${mz}&dmz=${a.dmz || 1}`); } catch (e) { /* no snap */ }
    const q = at && at.rt != null ? at : { rt, mz };
    const list = pts(p); list.push({ rt: q.rt, mz: q.mz, note: "" }); st(p).psel = list.length - 1;
    ptsSave(p); overlay(p); table(p);
  }
  function nearPoint(p, px, py) {
    const a = p._a; let best = -1, bd = 81;
    pts(p).forEach((q, i) => { const d = (a.X(q.rt) - px) ** 2 + (a.Y(q.mz) - py) ** 2; if (d < bd) { bd = d; best = i; } });
    return best;
  }
  // mousedown on a point: drag it (it snaps again where it is dropped); true = explore.js does nothing else
  function down(p, e, px, py) {
    if (!is2d(p) || e.button !== 0 || e.shiftKey || e.altKey) return false;
    const i = nearPoint(p, px, py); if (i < 0) return false;
    e.preventDefault(); const s = st(p), list = pts(p), r = p.cv.getBoundingClientRect(); s.psel = i; let moved = false;
    const mv = ev => { moved = true; const x = ev.clientX - r.left, y = ev.clientY - r.top; list[i].rt = xOf(p, x); list[i].mz = p._a.mzAt(y); overlay(p); };
    const up = async ev => {
      removeEventListener("mousemove", mv); removeEventListener("mouseup", up);
      if (moved) { const a = p._a; try { const at = await J(`api/mapsnap?k=${a.f.k}&level=${a.f.lv || 1}&rt=${list[i].rt}&mz=${list[i].mz}&dmz=${a.dmz || 1}`); if (at && at.rt != null) { list[i].rt = at.rt; list[i].mz = at.mz; } } catch (e2) { /* keep where dropped */ } ptsSave(p); }
      overlay(p); table(p);
    };
    addEventListener("mousemove", mv); addEventListener("mouseup", up);
    return true;
  }
  // a click (no drag) on the map: with «Segna» on it adds a point; on a point it chooses it
  function click(p, px, py) {
    if (!is2d(p)) return false; const s = st(p), i = nearPoint(p, px, py);
    if (i >= 0) { s.psel = i; overlay(p); table(p); return true; }
    if (!s.mark) return false;
    addPoint(p, px, py); return true;
  }
  function delPoint(p, i) { const list = pts(p); if (i == null || i < 0 || i >= list.length) return; list.splice(i, 1); st(p).psel = null; ptsSave(p); overlay(p); table(p); }
  function drawPoints(p, g) {
    const a = p._a, s = st(p); g.save(); g.font = fpx(11, "bold "); g.textAlign = "center";
    pts(p).forEach((q, i) => {
      if (q.rt < a.x0 || q.rt > a.x1 || q.mz < a.y0 || q.mz > a.y1) return;
      const x = a.X(q.rt), y = a.Y(q.mz), on = s.psel === i;
      g.beginPath(); g.arc(x, y, on ? 9 : 7.5, 0, 2 * Math.PI); g.fillStyle = css("--panel"); g.globalAlpha = .9; g.fill(); g.globalAlpha = 1;
      g.lineWidth = on ? 2.4 : 1.4; g.strokeStyle = on ? css("--accent") : css("--ink"); g.stroke();
      g.fillStyle = css("--ink"); g.fillText(String(i + 1), x, y + 4);
    });
    g.restore();
  }
  // ---- the table under the map
  const COLS = ["n", "rt", "mz", "ia", "ib", "d", "ratio", "sn", "g", "dm", "note"];
  const COLK = { n: ["mappa.col.n", "mappa.col.n.title"], rt: ["mappa.col.rt", "mappa.col.rt.title"], mz: ["mappa.col.mz", "mappa.col.mz.title"], ia: ["mappa.col.ia", "mappa.col.ia.title"], ib: ["mappa.col.ib", "mappa.col.ib.title"], d: ["mappa.col.d", "mappa.col.d.title"], ratio: ["mappa.col.ratio", "mappa.col.ratio.title"], sn: ["mappa.col.sn", "mappa.col.sn.title"], g: ["mappa.col.g", "mappa.col.g.title"], dm: ["mappa.col.dm", "mappa.col.dm.title"], note: ["mappa.col.note", "mappa.col.note.title"] };     // literal keys: the i18n check finds them
  let tT = 0;
  function table(p) {
    clearTimeout(tT); tT = setTimeout(() => tableNow(p), 30);
  }
  async function tableNow(p) {
    const s = st(p), a = p._a; if (!a || !s.grid) return;
    let box = s.tab;
    const list = is2d(p) ? pts(p) : [];
    if (!list.length) { if (box) { box.remove(); s.tab = null; growTable(p, false); } return; }
    if (!box) { box = s.tab = document.createElement("div"); box.className = "mztab"; s.grid.after(box); growTable(p, true); }
    const f = a.f, ref = a.ref, mzq = list.map(q => `${(+q.rt).toFixed(4)},${(+q.mz).toFixed(5)}`).join(";");
    let rows = [];
    try { rows = (await J(`api/mappunti?k=${f.k}&level=${f.lv || 1}${ref ? "&ref=" + ref.k : ""}&ppm=${ppmOf(f)}&grp=${PREF.grp ?? 0.05}&pts=${mzq}`)).rows; } catch (e) { rows = list.map(q => ({ rt: q.rt, mz: q.mz })); }
    rows = rows.map((r, i) => ({ ...r, n: i + 1, note: list[i] ? list[i].note || "" : "" }));
    s.rows = rows;
    const so = s.sort || { c: "n", d: 1 }, val = (r, c) => r[c] == null ? -Infinity : r[c];
    const view = rows.slice().sort((u, v) => typeof val(u, so.c) === "string" ? so.d * String(val(u, so.c)).localeCompare(String(val(v, so.c))) : so.d * (val(u, so.c) - val(v, so.c)));
    const dec = mzd(p), hr = isHrF(f), num = (v, d) => v == null || !Number.isFinite(v) ? "–" : v.toFixed(d);
    const head = COLS.map(c => `<th data-c="${c}" title="${T(COLK[c][1])}">${T(COLK[c][0])}${so.c === c ? (so.d > 0 ? " ▲" : " ▼") : ""}</th>`).join("") + "<th></th>";
    box.innerHTML = `<div class="mzt-bar"><b>${T("mappa.tab.title", { n: rows.length })}</b><label class="muted" title="${T("mappa.tab.grp.title")}">${T("mappa.tab.grp")} ± <input data-t="grp" class="mzf" inputmode="decimal" value="${PREF.grp ?? 0.05}"> min</label>` +
      `<span class="sp"></span><button data-t="xl" title="${T("mappa.tab.xl.title")}">Excel</button><button data-t="cp" title="${T("mappa.tab.cp.title")}">${T("mappa.tab.cp")}</button><button data-t="clear">${T("mappa.tab.clear")}</button></div>` +
      `<div class="mzt-sc"><table><thead><tr>${head}</tr></thead><tbody>${view.map(r => `<tr data-i="${r.n - 1}" class="${s.psel === r.n - 1 ? "on" : ""}">` +
      `<td>${r.n}</td><td>${num(r.rt, 2)}</td><td>${num(r.mz, hr ? 4 : 1)}</td><td>${r.ia == null ? "–" : fmt(r.ia)}</td><td>${r.ib == null ? "–" : fmt(r.ib)}</td><td>${r.d == null ? "–" : fmt(r.d)}</td>` +
      `<td>${num(r.ratio, 2)}</td><td>${num(r.sn, 1)}</td><td>${r.g ?? "–"}</td><td>${r.dm == null ? "–" : r.dm === 0 ? "0" : `<a href="#" data-dm="${r.dm}">${(r.dm > 0 ? "+" : "") + r.dm.toFixed(hr ? 4 : 1)}</a>`}</td>` +
      `<td><input data-note="${r.n - 1}" value="${EH(r.note || "")}" placeholder="${T("mappa.tab.note")}"></td><td><button data-xic="${r.n - 1}" title="${T("mappa.tab.xic.title")}">XIC</button></td></tr>`).join("")}</tbody></table></div>`;
    box.querySelectorAll("th[data-c]").forEach(th => th.onclick = () => { const c = th.dataset.c; s.sort = { c, d: so.c === c ? -so.d : 1 }; tableNow(p); });
    box.querySelector('[data-t="grp"]').onchange = ev => { const v = parseFloat(String(ev.target.value).replace(",", ".")); if (v > 0) { PREF.grp = v; prefSave(); } tableNow(p); };
    box.querySelector('[data-t="xl"]').onclick = () => excel(p);
    box.querySelector('[data-t="cp"]').onclick = () => copyRows(p);
    box.querySelector('[data-t="clear"]').onclick = async () => { if (await yesno(T("mappa.tab.clear.ask"))) { pts(p).splice(0); ptsSave(p); overlay(p); tableNow(p); } };
    box.querySelectorAll("tbody tr").forEach(tr => {
      const i = +tr.dataset.i;
      tr.onclick = ev => { if (ev.target.closest("input,button,a")) return; const q = pts(p)[i]; if (!q) return; s.psel = i; s.lock = { rt: q.rt, mz: q.mz }; follow(p, s.lock); overlay(p); margins(p); tableNow(p); };
      tr.ondblclick = ev => { if (ev.target.closest("input,button,a")) return; const q = pts(p)[i]; if (q) spectrumAt(p, q.rt); };
    });
    box.querySelectorAll("[data-note]").forEach(inp => inp.onchange = () => { const q = pts(p)[+inp.dataset.note]; if (q) { q.note = inp.value; ptsSave(p); } });
    box.querySelectorAll("[data-xic]").forEach(b => b.onclick = () => { const q = pts(p)[+b.dataset.xic]; if (q) xicAB(p, q.mz); });
    box.querySelectorAll("[data-dm]").forEach(l => l.onclick = ev => { ev.preventDefault(); openDm(+l.dataset.dm, isHrF(f)); });
  }
  function growTable(p, on) {
    const s = st(p); if (on === !!s.tgrown || isMax(p)) return;
    s.tgrown = on; p.h = Math.max(200, p.h + (on ? 1 : -1) * 220); apply(p); if (typeof relayout === "function") relayout(); fitHost(); uiSave();
  }
  async function spectrumAt(p, rt) {
    const f = p._a.f; let r0 = rt - scanStep() / 2, r1 = rt + scanStep() / 2;
    const n = await nearScan(f.k, rt); if (n) { r0 = n.rt - NEAR; r1 = n.rt + NEAR; }
    newSpec(p, r0, r1, f.k);
  }
  // XIC of A and B overlaid: a new XIC panel with the m/z; the files of the panel are those of the tab, A and B among them
  function xicAB(p, mz) { xicDirect(mz, p); }
  // delta m: the sidebar tab of the neutral losses (filtered on |delta m|) or, for the usual adduct differences, the adducts
  const AD_DIFF = [21.9819, 17.0265, 37.9559];
  function openDm(dm, hr) {
    const v = Math.abs(dm), tol = hr ? 0.003 : 0.5, ad = dm > 0 && AD_DIFF.some(x => Math.abs(v - x) <= tol), q = String(+v.toFixed(hr ? 4 : 1));
    if (window.BARRA && BARRA.isDataView()) BARRA.setTab(ad ? "adducts" : "losses", ad ? {} : { q });
    else if (window.QQQRef) QQQRef.open(ad ? "ad" : "ls", ad ? {} : { q });
  }
  function sheetRows(p) {
    const s = st(p), hr = isHrF(p._a.f);
    return (s.rows || []).map(r => [r.n, r.rt != null ? +r.rt.toFixed(3) : "", r.mz != null ? +r.mz.toFixed(hr ? 5 : 2) : "", r.ia ?? "", r.ib ?? "", r.d ?? "", r.ratio != null ? +r.ratio.toFixed(3) : "", r.sn != null ? +r.sn.toFixed(1) : "", r.g ?? "", r.dm != null ? +r.dm.toFixed(hr ? 5 : 2) : "", r.note || ""]);
  }
  const headRow = () => COLS.map(c => T(COLK[c][0]));
  function excel(p) {
    const a = p._a, f = a.f, meta = [[T("mappa.meta.fileA"), f.label], [T("mappa.meta.fileB"), a.ref ? a.ref.label : "–"], [T("mappa.meta.level"), f.lv === 2 ? "MS2" : "MS1"],
      [T("mappa.meta.xic"), isHrF(f) ? `± ${ppmOf(f)} ppm` : "[n − 0,2; n + 0,8]"], [T("mappa.meta.rttol"), `± ${diffOpt().rttol} min`], [T("mappa.meta.grp"), `± ${PREF.grp ?? 0.05} min`],
      [T("mappa.meta.snap"), "± 0,2 min · ± 1 bin"], [T("mappa.meta.date"), new Date().toLocaleString()]];
    dlx(`${f.label}${a.ref ? "-" + a.ref.label : ""}_punti.xlsx`, [{ name: T("mappa.tab.sheet"), head: headRow(), rows: sheetRows(p), widths: [5, 9, 11, 12, 12, 12, 9, 8, 7, 10, 30] },
      { name: T("mappa.meta.sheet"), head: [T("mappa.meta.what"), T("mappa.meta.value")], rows: meta, widths: [30, 40] }]);
  }
  async function copyRows(p) {
    const txt = [headRow(), ...sheetRows(p)].map(r => r.join("\t")).join("\n");
    try { await navigator.clipboard.writeText(txt); nearMsg(T("mappa.tab.copied")); } catch (e) { nearMsg(T("mappa.tab.copyFail")); }
  }

  // ---------------------------------------------------------------- buttons in the controls of the map: «⋯» menu, sight, whole view
  function decorate(p, c) {
    if (p.type !== "map") return;
    const sp = document.createElement("span"); sp.className = "seg mzbt";
    const o = diffOpt(), sel = (v, lab) => `<option value="${v}" ${o.show === v ? "selected" : ""}>${lab}</option>`;
    const dif = p.ref !== "" && p.ref != null ? `<select data-mz="show" title="${T("mappa.show.title")}">${sel("all", T("mappa.show.all"))}${sel("up", T("mappa.show.up"))}${sel("down", T("mappa.show.down"))}</select>` +
      `<label class="muted" title="${T("mappa.thr.title")}">${T("mappa.thr")} <input data-mz="thr" class="mzf" inputmode="decimal" value="${+(o.thr * 100).toFixed(1)}"> %</label>` : "";
    sp.innerHTML = `<button data-mz="mark" class="${st(p).mark ? "on" : ""}" title="${T("mappa.btn.mark.title")}">${T("mappa.btn.mark")}</button><button data-mz="mir" class="${mirOn(p) ? "on" : ""}" title="${T("mappa.btn.mir.title")}">${T("mappa.btn.mir")}</button><button data-mz="all" title="${T("mappa.btn.all.title")}">&#10530;</button><button data-mz="menu" title="${T("mappa.btn.menu.title")}" aria-label="${T("mappa.btn.menu.title")}">&#8943;</button>`;
    c.appendChild(sp);
    if (dif) { const d2 = document.createElement("span"); d2.className = "mzdif"; d2.innerHTML = dif; c.appendChild(d2);
      d2.querySelector('[data-mz="show"]').onchange = ev => { PREF.show = ev.target.value; prefSave(); draw(p); };
      d2.querySelector('[data-mz="thr"]').onchange = ev => { const v = parseFloat(String(ev.target.value).replace(",", ".")); if (v >= 0 && v < 100) { PREF.thr = v / 100; prefSave(); } draw(p); }; }
    sp.querySelector('[data-mz="mark"]').onclick = () => { const s = st(p); s.mark = !s.mark; markBtn(p); };
    sp.querySelector('[data-mz="mir"]').onclick = ev => { const s = st(p); s.mir = !mirOn(p); ev.currentTarget.classList.toggle("on", s.mir); layout(p); draw(p); };
    sp.querySelector('[data-mz="all"]').onclick = () => whole(p);
    sp.querySelector('[data-mz="menu"]').onclick = ev => { ev.stopPropagation(); menuAt(p, ev.currentTarget); };     // the click must not reach the document (it closes #ctx)
  }

  function markBtn(p) { const b = p.el.querySelector('[data-mz="mark"]'); if (b) b.classList.toggle("on", !!st(p).mark); p.cv.style.cursor = st(p).mark ? "copy" : ""; }

  return { init, zoomImg, after, down, click, diffOpt, diffMap, pts, delPoint, move, leave, wheel, boxZoom, ctx, decorate, key, whole, prefs: PREF, prefSave, point, st, valueAt, peakStep, apexStep, madNoise };
})();

// ---- panels: grips on the top and bottom edge (mouse, touch and pen), double click = default height; the width is always the column's
(() => {
  const H0 = 300, HMIN = 190;
  const css = document.createElement("style");
  css.textContent = "#dpanels .pnl{resize:none;max-width:100%}.pnl .pgrip{position:absolute;left:12px;right:12px;height:10px;z-index:5;cursor:ns-resize;touch-action:none}" +
    ".pnl .pgrip.t{top:-3px}.pnl .pgrip.b{bottom:-3px}.pnl .pgrip::after{content:'';position:absolute;left:50%;top:3px;width:34px;height:4px;margin-left:-17px;border-radius:2px;background:var(--line);opacity:0;transition:opacity .15s}" +
    ".pnl:hover .pgrip::after,.pnl .pgrip:focus-visible::after,.pnl .pgrip.on::after{opacity:1}.pnl .pgrip:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.pnl .pgrip{height:22px}.pnl .pgrip.t{top:-10px}.pnl .pgrip.b{bottom:-10px}.pnl .pgrip::after{opacity:.8;top:9px}}" +
    "@media (prefers-reduced-motion:reduce){.pnl .pgrip::after{transition:none}}";
  document.head.appendChild(css);
  const host = () => document.getElementById("dpanels");
  const pOf = el => E.panels.find(q => q.el === el);
  const simple = p => !(window.DDA && (DDA.docked(p) || E.panels.some(q => q.dock === p.id && DDA.docked(q))));
  const above = p => { let best = null; for (const q of E.panels) if (q !== p && q.full && simple(q) && q.y < p.y && (!best || q.y > best.y)) best = q; return best; };
  const done = p => { if (p.full && typeof relayout === "function") relayout(); if (typeof fitHost === "function") fitHost(); if (typeof uiSave === "function") uiSave(); };
  function setH(p, h) { p.h = Math.max(HMIN, Math.round(h)); apply(p); }
  function grip(el, side) {
    const g = document.createElement("div"); g.className = "pgrip " + side; g.tabIndex = 0; g.setAttribute("role", "separator"); g.setAttribute("aria-orientation", "horizontal");
    g.setAttribute("aria-label", I18N.t(side === "t" ? "mappa.rz.top" : "mappa.rz.bot")); g.title = I18N.t("mappa.rz.title");
    g.addEventListener("pointerdown", ev => {
      const p = pOf(el); if (!p || el.classList.contains("max") || ev.button > 0) return;
      ev.preventDefault(); ev.stopPropagation(); g.setPointerCapture(ev.pointerId); g.classList.add("on");
      const y0 = ev.clientY, h0 = p.h, top0 = p.y, bot0 = p.y + p.h, up = side === "t" ? above(p) : null, uh0 = up ? up.h : 0;
      const mv = e => {
        const d = e.clientY - y0;
        if (side === "b") setH(p, h0 + d);
        else if (up) { const k = Math.max(-(h0 - HMIN), Math.min(uh0 - HMIN, d)); setH(up, uh0 + k); setH(p, h0 - k); p.y = up.y + up.h + (top0 - (up.y + uh0)); apply(p); }   // splitter: what one gains the other loses
        else { const h = Math.max(HMIN, Math.min(bot0, h0 - d)); p.h = h; p.y = bot0 - h; apply(p); }
        if (typeof fitHost === "function") fitHost();
      };
      const end = () => { g.removeEventListener("pointermove", mv); g.removeEventListener("pointerup", end); g.removeEventListener("pointercancel", end); g.classList.remove("on"); done(p); };
      g.addEventListener("pointermove", mv); g.addEventListener("pointerup", end); g.addEventListener("pointercancel", end);
    });
    g.addEventListener("dblclick", ev => { const p = pOf(el); if (!p) return; ev.stopPropagation(); if (side === "t" && !p.full) { const b = p.y + p.h; setH(p, H0); p.y = Math.max(0, b - p.h); apply(p); } else setH(p, H0); done(p); });
    g.addEventListener("keydown", ev => {
      const p = pOf(el); if (!p || !["ArrowUp", "ArrowDown", "Home"].includes(ev.key)) return;
      ev.preventDefault(); ev.stopPropagation(); const k = ev.shiftKey ? 60 : 20;
      if (ev.key === "Home") setH(p, H0); else setH(p, p.h + (ev.key === "ArrowDown" ? k : -k));
      done(p);
    });
    el.appendChild(g);
  }
  function wire(el) { if (!el.classList || !el.classList.contains("pnl") || el._pgrip) return; el._pgrip = true; grip(el, "t"); grip(el, "b"); }
  function clamp() {
    const h = host(); if (!h) return; const w = h.clientWidth; if (w < 100) return;
    for (const p of E.panels) if (p.el && !p.el.classList.contains("max") && simple(p) && p.x + p.w > w + 1) { p.w = Math.max(300, w - p.x); if (p.x + p.w > w + 1) p.x = Math.max(0, w - p.w); apply(p); }
  }
  function start() {
    const h = host(); if (!h) return;
    h.querySelectorAll(".pnl").forEach(wire);
    new MutationObserver(ms => { for (const m of ms) m.addedNodes.forEach(wire); }).observe(h, { childList: true });
    addEventListener("resize", () => requestAnimationFrame(clamp));
    new ResizeObserver(() => requestAnimationFrame(clamp)).observe(h);
  }
  window.MAPPA_PNL = { clamp, setH };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start); else start();
})();

// ---- 3D surface (no libraries, Canvas 2D). Cells: at most 300 x 300 (the maximum of the bins each one covers, so no peak disappears), then a
// Gaussian smoothing, the noise threshold and the scale (linear | square root | logarithmic). Lambert shading with a light that follows the camera,
// viridis or cividis, contour lines on the floor, axes with labels. Back-to-front order of the grid (no sorting); while turning, a coarser grid.
(() => {
  const T = (k, o) => I18N.t(k, o), D3 = { thr: 0.02, sig: 1, pal: "viridis" };
  const P3 = () => { const q = MAPPA.prefs; return { thr: q.t3thr ?? D3.thr, sig: q.t3sig ?? D3.sig, pal: q.t3pal || D3.pal }; };
  const ANCH = {
    viridis: [[68, 1, 84], [70, 50, 126], [54, 92, 141], [39, 127, 142], [31, 161, 135], [74, 194, 109], [159, 218, 58], [253, 231, 37]],
    cividis: [[0, 32, 76], [31, 55, 108], [70, 77, 110], [105, 100, 112], [139, 125, 119], [177, 153, 117], [217, 184, 100], [254, 232, 56]]
  };
  const LUTS = {};
  const pal = name => LUTS[name] || (LUTS[name] = (() => {
    const A = ANCH[name] || ANCH.viridis, L = new Uint8ClampedArray(256 * 3);
    for (let i = 0; i < 256; i++) { const t = i / 255 * (A.length - 1), k = Math.min(A.length - 2, Math.floor(t)), f = t - k; for (let c = 0; c < 3; c++) L[i * 3 + c] = A[k][c] + (A[k + 1][c] - A[k][c]) * f; }
    return L;
  })());
  const scaleT = (r, sc) => sc === "lin" ? r : sc === "log" ? Math.log10(1 + 1000 * r) / 3.0004 : Math.sqrt(r);
  function smooth(Z, nx, ny, s) {
    if (!(s > 0.05)) return Z;
    const r = Math.ceil(3 * s), k = new Float32Array(2 * r + 1); let sum = 0;
    for (let i = -r; i <= r; i++) { k[i + r] = Math.exp(-i * i / (2 * s * s)); sum += k[i + r]; }
    for (let i = 0; i < k.length; i++) k[i] /= sum;
    const tmp = new Float32Array(Z.length), out = new Float32Array(Z.length);
    for (let a = 0; a < nx; a++) for (let b = 0; b < ny; b++) { let acc = 0; for (let t = -r; t <= r; t++) acc += k[t + r] * Z[a * ny + Math.min(ny - 1, Math.max(0, b + t))]; tmp[a * ny + b] = acc; }
    for (let a = 0; a < nx; a++) for (let b = 0; b < ny; b++) { let acc = 0; for (let t = -r; t <= r; t++) acc += k[t + r] * tmp[Math.min(nx - 1, Math.max(0, a + t)) * ny + b]; out[a * ny + b] = acc; }
    return out;
  }
  function gridOf(p, A, B, im, x0, x1, y0, y1) {
    const o = P3(), { nrt, nmz, rt0, rt1, mz0, dmz } = A;
    const ia = Math.max(0, Math.floor((x0 - rt0) / (rt1 - rt0) * nrt)), ib = Math.min(nrt, Math.max(ia + 1, Math.ceil((x1 - rt0) / (rt1 - rt0) * nrt)));
    const ja = Math.max(0, Math.floor((y0 - mz0) / dmz)), jb = Math.min(nmz, Math.max(ja + 1, Math.ceil((y1 - mz0) / dmz)));
    const key = `${im.key}|${p.scale}|${ia},${ib},${ja},${jb}|${o.thr}|${o.sig}`;
    if (p._g3 && p._g3.key === key && p._g3.v === im.v) return p._g3;
    const MAXC = 300, sx = Math.max(1, Math.ceil((ib - ia) / MAXC)), sy = Math.max(1, Math.ceil((jb - ja) / MAXC));
    const nx = Math.max(2, Math.ceil((ib - ia) / sx)), ny = Math.max(2, Math.ceil((jb - ja) / sy)), v = im.v;
    let R = new Float32Array(nx * ny);
    for (let a = 0; a < nx; a++) for (let b = 0; b < ny; b++) {
      let best = 0; const i1 = Math.min(ib, ia + (a + 1) * sx), j1 = Math.min(jb, ja + (b + 1) * sy);
      for (let i = ia + a * sx; i < i1; i++) for (let j = ja + b * sy; j < j1; j++) { const q = v[i * nmz + j]; if (Math.abs(q) > Math.abs(best)) best = q; }
      R[a * ny + b] = best;
    }
    R = smooth(R, nx, ny, o.sig);
    const nz = []; for (let i = 0; i < R.length; i++) if (R[i] !== 0) nz.push(Math.abs(R[i]));
    nz.sort((u, w) => u - w);
    const ref = nz.length ? nz[Math.min(nz.length - 1, Math.floor(nz.length * 0.995))] || nz[nz.length - 1] : 1;
    const Z = new Float32Array(nx * ny); let zmin = 0, zmax = 0, raw = 0;
    for (let i = 0; i < R.length; i++) {
      const r = Math.min(1, Math.abs(R[i]) / ref), z = r < o.thr ? 0 : Math.sign(R[i]) * scaleT(r, p.scale);
      Z[i] = z; if (z < zmin) zmin = z; if (z > zmax) zmax = z; if (Math.abs(R[i]) > raw) raw = Math.abs(R[i]);
    }
    if (!B) zmin = 0; if (zmax === zmin) zmax = zmin + 1;
    return (p._g3 = { key, v: im.v, nx, ny, Z, zmin, zmax, raw, ref, ia, ja, sx, sy });
  }
  function contours(g, G, st, P, levels) {
    const { nx, ny, Z } = G;
    for (const L of levels) {
      g.beginPath();
      for (let a = 0; a + st < nx; a += st) for (let b = 0; b + st < ny; b += st) {
        const a1 = Math.min(nx - 1, a + st), b1 = Math.min(ny - 1, b + st);
        const c = [Z[a * ny + b], Z[a1 * ny + b], Z[a1 * ny + b1], Z[a * ny + b1]], ix = [[a, b], [a1, b], [a1, b1], [a, b1]], pts = [];
        for (let e = 0; e < 4; e++) {
          const u = c[e], w = c[(e + 1) % 4]; if ((u >= L) === (w >= L)) continue;
          const t = (L - u) / (w - u), p0 = ix[e], p1 = ix[(e + 1) % 4];
          pts.push(P(p0[0] + (p1[0] - p0[0]) * t, p0[1] + (p1[1] - p0[1]) * t, 0));
        }
        if (pts.length >= 2) { g.moveTo(pts[0][0], pts[0][1]); g.lineTo(pts[1][0], pts[1][1]); }
        if (pts.length === 4) { g.moveTo(pts[2][0], pts[2][1]); g.lineTo(pts[3][0], pts[3][1]); }
      }
      g.stroke();
    }
  }
  function draw3d(p, g, W, H, A, B, im, f, rf, x0, x1, y0, y1, scaleTxt) {
    const o = P3(), G = gridOf(p, A, B, im, x0, x1, y0, y1), { nx, ny, Z, zmin, zmax } = G;
    const rot = !!p._rot, s = rot ? Math.max(1, Math.ceil(Math.max(nx, ny) / 110)) : 1;
    const az = (p.az ?? 25) * Math.PI / 180, el = (p.elv ?? 38) * Math.PI / 180, ca = Math.cos(az), sa = Math.sin(az), ce = Math.cos(el), se = Math.sin(el);
    const zs = 0.55 / Math.max(zmax - Math.min(zmin, 0), 1e-9);
    const asp = Math.min(3, Math.max(1, (W - M.l - M.r) / Math.max(1, H - M.t - M.b) * 1.1));
    const pr = (u, w, z) => { u *= asp; const xr = u * ca - w * sa, d = u * sa + w * ca; return [xr, z * zs * ce + d * se, d * ce - z * zs * se]; };
    const corners = []; for (const u of [-.5, .5]) for (const w of [-.5, .5]) for (const z of [zmin, zmax]) corners.push(pr(u, w, z));
    const bx0 = Math.min(...corners.map(c => c[0])), bx1 = Math.max(...corners.map(c => c[0])), by0 = Math.min(...corners.map(c => c[1])), by1 = Math.max(...corners.map(c => c[1]));
    const pw = W - M.l - M.r - 40, ph = H - M.t - M.b - 30, sc = Math.min(pw / (bx1 - bx0), ph / (by1 - by0));
    const ox = M.l + 20 + (pw - (bx1 - bx0) * sc) / 2 - bx0 * sc, oy = M.t + 8 + (ph - (by1 - by0) * sc) / 2 + by1 * sc;
    const Pm = (u, w, z) => { const r = pr(u, w, z); return [ox + r[0] * sc, oy - r[1] * sc, r[2]]; };
    const PV = (a, b, z) => Pm(a / (nx - 1) - .5, b / (ny - 1) - .5, z);          // grid vertex (fractional indices allowed)
    g.clearRect(0, 0, W, H);
    const ink = css("--muted"), strong = css("--ink"); g.strokeStyle = ink; g.fillStyle = ink; g.lineWidth = 1; g.font = fpx(12);
    const fl = [[-.5, -.5], [.5, -.5], [.5, .5], [-.5, .5]].map(([u, w]) => Pm(u, w, 0));
    g.fillStyle = "rgba(120,120,120,.08)"; g.beginPath(); fl.forEach((q, i) => i ? g.lineTo(q[0], q[1]) : g.moveTo(q[0], q[1])); g.closePath(); g.fill(); g.stroke();
    // floor grid at the ticks and contour lines of the surface
    const r0 = nice(x0, x1, 5).filter(t => t >= x0 && t <= x1), m0 = nice(y0, y1, 5).filter(t => t >= y0 && t <= y1);
    g.save(); g.globalAlpha = .25; g.beginPath();
    for (const t of r0) { const u = (t - x0) / (x1 - x0) - .5, q = Pm(u, -.5, 0), q2 = Pm(u, .5, 0); g.moveTo(q[0], q[1]); g.lineTo(q2[0], q2[1]); }
    for (const t of m0) { const w = (t - y0) / (y1 - y0) - .5, q = Pm(-.5, w, 0), q2 = Pm(.5, w, 0); g.moveTo(q[0], q[1]); g.lineTo(q2[0], q2[1]); }
    g.stroke(); g.restore();
    if (!rot) { g.save(); g.strokeStyle = ink; g.globalAlpha = .55; g.lineWidth = .7; contours(g, G, Math.max(1, Math.ceil(Math.max(nx, ny) / 120)), (a, b, z) => PV(a, b, z), zmax > 0 ? [.15, .35, .6, .85].map(k => k * zmax) : []); g.restore(); }
    // surface: far cells first
    const lx = -.5, ld = -.45, lz = .75, ln = Math.hypot(lx, ld, lz), Lu = (lx * ca + ld * sa) / ln, Lw = (-lx * sa + ld * ca) / ln, Lz = lz / ln;
    const du = asp / (nx - 1), dw = 1 / (ny - 1), L = pal(o.pal), tab = B ? LUT_DIV : null;
    const na = Math.floor((nx - 1) / s), nb = Math.floor((ny - 1) / s), aD = sa > 0, bD = ca > 0;
    g.lineWidth = .5;
    for (let ia = 0; ia < na; ia++) {
      const a = (aD ? na - 1 - ia : ia) * s;
      for (let ib = 0; ib < nb; ib++) {
        const b = (bD ? nb - 1 - ib : ib) * s, a1 = Math.min(nx - 1, a + s), b1 = Math.min(ny - 1, b + s);
        const z00 = Z[a * ny + b], z10 = Z[a1 * ny + b], z11 = Z[a1 * ny + b1], z01 = Z[a * ny + b1];
        if (z00 === 0 && z10 === 0 && z11 === 0 && z01 === 0) continue;
        const dzu = ((z10 + z11) - (z00 + z01)) / 2 * zs / (du * (a1 - a)), dzw = ((z01 + z11) - (z00 + z10)) / 2 * zs / (dw * (b1 - b));
        const nn = Math.hypot(dzu, dzw, 1), lam = Math.max(0, (-dzu * Lu - dzw * Lw + Lz) / nn), k = .42 + .7 * lam, zc = (z00 + z10 + z11 + z01) / 4;
        let r, gg, bl;
        if (B) { const q = Math.round(127.5 - Math.max(-1, Math.min(1, zc)) * 127.5); r = tab[q * 4]; gg = tab[q * 4 + 1]; bl = tab[q * 4 + 2]; }
        else { const q = Math.round(Math.min(1, Math.max(0, zc)) * 255) * 3; r = L[q]; gg = L[q + 1]; bl = L[q + 2]; }
        const col = `rgb(${Math.min(255, r * k | 0)},${Math.min(255, gg * k | 0)},${Math.min(255, bl * k | 0)})`;
        const c0 = PV(a, b, z00), c1 = PV(a1, b, z10), c2 = PV(a1, b1, z11), c3 = PV(a, b1, z01);
        g.fillStyle = col; g.beginPath(); g.moveTo(c0[0], c0[1]); g.lineTo(c1[0], c1[1]); g.lineTo(c2[0], c2[1]); g.lineTo(c3[0], c3[1]); g.closePath(); g.fill();
        if (!rot) { g.strokeStyle = col; g.stroke(); }
      }
    }
    // axes: ticks on the two floor edges nearest to the viewer, z axis
    g.strokeStyle = ink; g.fillStyle = ink; g.lineWidth = 1; g.textAlign = "center"; g.font = fpx(12);
    const wn = pr(0, -.5, 0)[2] < pr(0, .5, 0)[2] ? -.5 : .5, un = pr(-.5, 0, 0)[2] < pr(.5, 0, 0)[2] ? -.5 : .5;
    g.beginPath(); fl.forEach((q, i) => i ? g.lineTo(q[0], q[1]) : g.moveTo(q[0], q[1])); g.closePath(); g.stroke();
    for (const t of r0) { const u = (t - x0) / (x1 - x0) - .5, q = Pm(u, wn, 0), e2 = Pm(u, wn * 1.06, 0); g.beginPath(); g.moveTo(q[0], q[1]); g.lineTo(e2[0], e2[1]); g.stroke(); g.fillText(String(+t.toFixed(2)), e2[0], e2[1] + (wn < 0 ? 12 : -4)); }
    for (const t of m0) { const w = (t - y0) / (y1 - y0) - .5, q = Pm(un, w, 0), e2 = Pm(un * 1.05, w, 0); g.beginPath(); g.moveTo(q[0], q[1]); g.lineTo(e2[0], e2[1]); g.stroke(); g.textAlign = un < 0 ? "right" : "left"; g.fillText(String(Math.round(t)), e2[0] + (un < 0 ? -3 : 3), e2[1] + 4); g.textAlign = "center"; }
    const tm = Pm(0, wn * 1.3, 0), ym = Pm(un * 1.28, 0, 0), zb = Pm(-.5, -.5, B ? zmin : 0), zt = Pm(-.5, -.5, zmax);
    g.fillStyle = strong; g.fillText("RT (min)", tm[0], tm[1] + (wn < 0 ? 18 : -14)); g.fillText("m/z", ym[0], ym[1] + 4); g.fillStyle = ink;
    g.beginPath(); g.moveTo(zb[0], zb[1]); g.lineTo(zt[0], zt[1]); g.stroke(); g.textAlign = "right"; g.fillText(fmt(G.raw * (G.zmax >= 0 ? 1 : -1)), zt[0] - 4, zt[1] + 4); g.fillText(B ? "0" : "0", zb[0] - 4, zb[1] + 4);
    g.textAlign = "left"; g.fillStyle = strong; g.fillText(T("map.height3d", { scale: scaleTxt }), M.l, M.t + 8); g.fillStyle = ink;
    // centres for the pointer (about 80 x 80)
    const cs = Math.max(1, Math.ceil(Math.max(nx, ny) / 80)), centres = [];
    for (let a = 0; a < nx; a += cs) for (let b = 0; b < ny; b += cs) { const q = PV(a, b, Z[a * ny + b]); centres.push({ x: q[0], y: q[1], depth: q[2], a, b, v: Z[a * ny + b], rt: A.rt0 + (G.ia + (a + .5) * G.sx) / A.nrt * (A.rt1 - A.rt0), mz: A.mz0 + (G.ja + (b + .5) * G.sy) * A.dmz }); }
    // the line the student clicked: from the floor to the surface, with RT and m/z
    const pin = p._pin3;
    if (pin && pin.rt >= x0 && pin.rt <= x1 && pin.mz >= y0 && pin.mz <= y1) {
      const a = Math.round(((pin.rt - A.rt0) / (A.rt1 - A.rt0) * A.nrt - G.ia) / G.sx - .5), b = Math.round(((pin.mz - A.mz0) / A.dmz - G.ja) / G.sy - .5);
      if (a >= 0 && a < nx && b >= 0 && b < ny) {
        const z = Z[a * ny + b], q0 = PV(a, b, 0), q1 = PV(a, b, z);
        g.save(); g.strokeStyle = strong; g.lineWidth = 1.6; g.setLineDash([5, 3]); g.beginPath(); g.moveTo(q0[0], q0[1]); g.lineTo(q1[0], q1[1]); g.stroke(); g.setLineDash([]);
        g.fillStyle = strong; g.beginPath(); g.arc(q1[0], q1[1], 3.5, 0, 6.3); g.fill();
        const txt = T("mappa.v3.pin", { rt: pin.rt.toFixed(2), mz: pin.mz.toFixed(mzd(p)) }), tw = g.measureText(txt).width + 10, tx = Math.min(W - tw - 4, Math.max(4, q1[0] - tw / 2)), ty = Math.max(M.t + 18, q1[1] - 14);
        g.fillStyle = css("--panel"); g.globalAlpha = .92; g.fillRect(tx, ty - 13, tw, 18); g.globalAlpha = 1; g.fillStyle = strong; g.textAlign = "left"; g.fillText(txt, tx + 5, ty); g.restore();
      }
    }
    const hov = (px, py) => {
      let best = null, bd = 18 * 18;
      for (const c of centres) { const d = (c.x - px) ** 2 + (c.y - py) ** 2; if (d < bd || (best && d < bd + 40 && c.depth < best.depth)) { bd = Math.min(bd, d); best = c; } }
      if (!best) return null;
      return { px: best.x, rt: null, novl: true, html: T("map.hov", { rt: best.rt.toFixed(2), mz: best.mz.toFixed(mzd(p)), what: T(B ? "map.hov.diff" : "map.hov.mean"), v: (B && best.v > 0 ? "+" : "") + fmt(best.v * G.ref) }) };
    };
    p._a = { x0, x1, y0, y1, X: v2 => M.l + (v2 - x0) / (x1 - x0) * (W - M.l - M.r), Y: () => 0, W, H, full: [A.rt0, A.rt1], fullY: [A.mz0, A.mz0 + A.nmz * A.dmz], f, mzAt: () => (y0 + y1) / 2, map: true, is3d: true, hov, centres };
  }
  const nearest = (p, px, py) => { let best = null, bd = 22 * 22; for (const c of (p._a && p._a.centres) || []) { const d = (c.x - px) ** 2 + (c.y - py) ** 2; if (d < bd || (best && d < bd + 40 && c.depth < best.depth)) { bd = Math.min(bd, d); best = c; } } return best; };
  // a click without moving (the drag rotates): the line with RT and m/z
  function click3(p, d) {
    if (!d || p.az !== d.az || p.elv !== d.elv) return false;
    const c = nearest(p, d.x0, d.y0); if (!c) return false;
    p._pin3 = { rt: c.rt, mz: c.mz };
    draw(p); return true;
  }
  // double click: the spectrum at that RT
  async function dbl3(p, px, py) {
    const c = nearest(p, px, py); if (!c) return;
    const a = p._a, k = p.k != null ? p.k : (a && a.f ? a.f.k : 0), f = E.files[k]; if (!f || f.kind === "mrm") return;
    let r0 = c.rt - scanStep() / 2, r1 = c.rt + scanStep() / 2, at = c.rt;
    const n = await nearScan(k, c.rt); if (n) { r0 = n.rt - NEAR; r1 = n.rt + NEAR; at = n.rt; }
    p.sel = null; p.cur = at; newSpec(p, r0, r1, k); draw(p);
  }
  function view3(p, az, elv) { p.az = az; p.elv = elv; draw(p); if (typeof uiSave === "function") uiSave(); }
  function decorate3(p, c) {
    const o = P3(), sp = document.createElement("span"); sp.className = "seg mzbt";
    sp.innerHTML = `<button data-v3="top" title="${T("mappa.v3.top.title")}">${T("mappa.v3.top")}</button><button data-v3="rt" title="${T("mappa.v3.rt.title")}">${T("mappa.v3.rt")}</button><button data-v3="mz" title="${T("mappa.v3.mz.title")}">${T("mappa.v3.mz")}</button><button data-v3="iso" title="${T("mappa.v3.iso.title")}">${T("mappa.v3.iso")}</button>`;
    c.appendChild(sp);
    const op = (v, lab) => `<option value="${v}" ${o.pal === v ? "selected" : ""}>${lab}</option>`, d2 = document.createElement("span"); d2.className = "mzdif";
    d2.innerHTML = `<select data-v3="pal" title="${T("mappa.v3.pal.title")}">${op("viridis", "viridis")}${op("cividis", "cividis")}</select>` +
      `<label class="muted" title="${T("mappa.v3.sig.title")}">${T("mappa.v3.sig")} <input data-v3="sig" class="mzf" inputmode="decimal" value="${o.sig}"></label>` +
      `<label class="muted" title="${T("mappa.v3.thr.title")}">${T("mappa.v3.thr")} <input data-v3="thr" class="mzf" inputmode="decimal" value="${+(o.thr * 100).toFixed(1)}"> %</label>`;
    c.appendChild(d2);
    const Q = s => c.querySelector(`[data-v3="${s}"]`), num = el => parseFloat(String(el.value).replace(",", "."));
    Q("top").onclick = () => setMapView(p, "2d");
    Q("rt").onclick = () => view3(p, 0, 0);          // seen from the side along m/z: the outline is the chromatogram
    Q("mz").onclick = () => view3(p, 90, 0);         // seen from the side along RT: the outline is the spectrum
    Q("iso").onclick = () => view3(p, 25, 38);
    Q("pal").onchange = e => { MAPPA.prefs.t3pal = e.target.value; MAPPA.prefSave(); draw(p); };
    Q("sig").onchange = e => { const v = num(e.target); if (v >= 0 && v <= 4) { MAPPA.prefs.t3sig = v; MAPPA.prefSave(); } draw(p); };
    Q("thr").onchange = e => { const v = num(e.target); if (v >= 0 && v < 100) { MAPPA.prefs.t3thr = v / 100; MAPPA.prefSave(); } draw(p); };
  }
  const dec0 = MAPPA.decorate, aft0 = MAPPA.after;
  Object.assign(MAPPA, {
    draw3d, click3, dbl3,
    decorate: (p, c) => p.type === "map" && p.view === "3d" ? decorate3(p, c) : dec0(p, c),
    after: p => {
      aft0(p);
      if (p._a && p._a.is3d) { const b = p.leg && p.leg.querySelector(".cbar"); if (b && !(p.ref !== "" && p.ref != null)) { const L = pal(P3().pal), st = [0, 36, 73, 109, 146, 182, 219, 255].map(i => `rgb(${L[i * 3]},${L[i * 3 + 1]},${L[i * 3 + 2]})`); b.style.background = `linear-gradient(90deg,${st.join(",")})`; } }
    }
  });
})();
