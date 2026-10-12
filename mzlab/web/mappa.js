// RT x m/z map: true zoom, sight (crosshair) with the two plots at the margins, lock, keyboard, full screen (classic script, loaded after explore.js).
// The map itself (colours, difference, 3D) is drawn by drawMap in explore.js; this file adds what lives on top of it. Hooks in explore.js:
// drawMap (zoomImg before the image is drawn, after at the end), attach (wheel, box zoom, click, right click, mouse move), ctl (decorate).
// Preferences of this browser in localStorage "qqq.mappa".
window.MAPPA = (() => {
  const T = (k, o) => I18N.t(k, o);
  const PREF_KEY = "qqq.mappa";
  const PREF = (() => { return lsObj(PREF_KEY); })();
  const prefSave = () => { try { localStorage.setItem(PREF_KEY, JSON.stringify(PREF)); } catch (e) { /* no storage */ } };
  const BOTTOM = 140, NARROW = 700, QUIET = 150;
  const st = p => p._mz || (p._mz = { lock: null, kp: null, hov: null, mir: null, reg: null, regT: 0, spec: null, xic: null, xicB: null });
  const is2d = p => p.type === "map" && p._a && p._a.map;
  const isMax = p => p.el && p.el.classList.contains("max");
  const mirOn = p => { const s = st(p); return s.ex ? true : s.mir != null ? s.mir : isMax(p); };       // «How it is extracted» always uses the two plots at the margins
  const wide = p => p.el && p.el.clientWidth >= NARROW;
  const point = p => { const s = st(p); return s.lock || s.frz || s.kp || s.hov; };       // frz: the point of the right click, held while the menu is open
  const isHrF = f => !!(f && window.HR && HR.isHr(f, f.lv || 1));

  // ---------------------------------------------------------------- DOM: grid with the map, the overlay (crosshair) and the two margin plots
  function init(p) {
    if (p._mzInit || !p.cv) return; p._mzInit = true;
    const cv = p.cv, grid = document.createElement("div");
    grid.className = "mzgrid"; cv.before(grid); grid.appendChild(cv);
    const ov = document.createElement("canvas"); ov.className = "mzov"; ov.setAttribute("aria-hidden", "true"); grid.appendChild(ov);
    const sp = document.createElement("canvas"); sp.className = "mzside"; sp.hidden = true; sp.setAttribute("role", "img"); grid.appendChild(sp);
    const xc = document.createElement("canvas"); xc.className = "mzbot"; xc.hidden = true; grid.appendChild(xc);
    Object.assign(st(p), { ov, sp, xc, grid });
    cv.tabIndex = 0; cv.setAttribute("aria-label", T("mappa.aria"));
    cv.addEventListener("keydown", e => key(p, e));
    p.el.addEventListener("keydown", e => { if (isMax(p) && e.target === p.el) key(p, e); });
    const tbs = p.el.querySelector(".tbs"); tbs.insertBefore(navBar(p), tbs.children[1] || null); navSync(p); touchInit(p, cv);
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
  const is2dView = p => p.type === "map";

  // ---------------------------------------------------------------- the true zoom: the region in view on new bins, asked 150 ms after the last change
  function normFactor(m, norm) {
    if (norm !== "max" && norm !== "tic") return 1;
    let t = 0; for (let i = 0; i < m.length; i++) t = norm === "max" ? Math.max(t, m[i]) : t + m[i];
    return t > 0 ? (norm === "max" ? 1 : 1000) / t : 1;
  }
  // ---------------------------------------------------------------- the difference (extends the one of explore.js): computed by the server in numpy
  // with a tolerance in RT (the reference is replaced by its local maximum over +-0.1 min: background lines and small drifts cancel); what it shows
  // (all | increases only | decreases only) and the noise threshold (fraction of the 99.5th percentile) are choices of this browser
  const diffOpt = (diff = true) => ({ show: PREF.show || "all", thr: PREF.thr ?? (diff ? 0.02 : 0.004), rttol: PREF.rttol ?? 0.1 });
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
    const r = regionGrid(p, A, x0, x1, y0, y1, pw, ph), key = `${A.k}|${B ? B.k : ""}|${A.level}|${norm}|${p.scale}${p.scale === "gamma" ? p.gamma || 0.6 : ""}|${mapPal()}|${mapBg(p)}|${r.join("|")}|${JSON.stringify(diffOpt(!!B))}`;
    if (s.reg && s.reg.key === key && s.reg.off) { g.imageSmoothingEnabled = false; g.drawImage(s.reg.off, 0, (s.reg.mz1 - y1) / s.reg.dmz, s.reg.nrt, (y1 - y0) / s.reg.dmz, M.l, M.t, pw, ph); return true; }
    clearTimeout(s.regT);
    s.regT = setTimeout(async () => {
      try {
        const a = await getRegion(A.k, A.level, r, (B ? `&ref=${B.k}&norm=${norm}&rttol=${diffOpt().rttol}` : "") + mapBg(p));
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
    const ctx = off.getContext("2d"), d = ctx.createImageData(nrt, nmz), tab = diff ? LUT_DIV : lutSeq(mapPal());
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
    if (s.ex && window.MAPEX) MAPEX.overlay(p, g);
    if (s.feat && window.MAPEX) MAPEX.featOverlay(p, g);
    const pt = point(p), on = mirOn(p) && !s.ex; s.dot = null;
    if (pt && (on || s.lock) && pt.rt >= a.x0 && pt.rt <= a.x1 && pt.mz >= a.y0 && pt.mz <= a.y1) {
      const x = Math.round(a.X(pt.rt)) + .5, y = Math.round(a.Y(pt.mz)) + .5;
      g.save(); g.strokeStyle = css("--ink"); g.globalAlpha = s.lock ? .9 : .6; g.lineWidth = 1; if (!s.lock) g.setLineDash([4, 3]);
      g.beginPath(); g.moveTo(M.l, y); g.lineTo(a.W - M.r, y); g.moveTo(x, M.t); g.lineTo(x, a.H - M.b); g.stroke(); g.setLineDash([]);
      if (s.lock) lockDot(g, a.X(pt.rt), a.Y(pt.mz), s);                  // the locked point: a small dot exactly on it
      g.restore();
    }
    readout(p); navSync(p);
  }
  // the mark of a locked point: a dot with a dark outline and an accent fill, centred on (x, y); s.dot keeps the centre for the tests
  function lockDot(g, x, y, s) {
    g.globalAlpha = 1; g.beginPath(); g.arc(x, y, 4, 0, 2 * Math.PI); g.fillStyle = css("--accent"); g.fill(); g.lineWidth = 1.6; g.strokeStyle = css("--ink"); g.stroke(); s.dot = { x, y };
  }
  function readout(p) {
    const pt = point(p), a = p._a; if (!a || !is2d(p)) return;
    if (!pt || !(mirOn(p) || st(p).lock)) return;
    const s = st(p), f = a.f, v = valueAt(p, pt.rt, pt.mz), sp = specFor(p, pt), sc = sp && sp.sid != null ? T("mappa.read.scan", { n: sp.sid + 1 }) + " · " : "";
    p.rd.textContent = `${f ? f.label + " · " : ""}${sc}RT ${pt.rt.toFixed(2)} min · m/z ${pt.mz.toFixed(mzd(p))} · ${T("mappa.read.int")} ${v == null ? "–" : fmt(v)}${s.lock ? " · " + T("mappa.read.locked") : ""}`;
  }

  // ---------------------------------------------------------------- the two plots at the margins: spectrum of the scan (m/z aligned with the map) and XIC (RT aligned)
  // Asked only for what the point needs and the page does not hold: the scan is found in the page on the RT axis of the file (/api/scan_axis, once), the
  // spectrum comes from /api/spectra by position and only when the scan changes, the XIC only when the window changes. One request at a time, the newest
  // point wins (MIRINO.latest, at most once per frame); what has not arrived yet is a preview read from the matrix of the map, marked as such.
  const SIGHT = MIRINO.lru(MIRINO.budget()), AX = new Map(), FLY = new Map();      // spectra and XIC under a memory budget; RT axes; requests in flight
  { const clear = CACHE.clear.bind(CACHE); CACHE.clear = () => { clear(); SIGHT.clear(); AX.clear(); FLY.clear(); }; }   // same moments as the cache of explore.js
  const dropFile = k => { SIGHT.drop(`s${k}|`); SIGHT.drop(`x${k}|`); SIGHT.drop(`x${k}+`); for (const key of [...AX.keys()]) if (key.startsWith(`${k}|`)) AX.delete(key); };
  const MEAS = /[?&]perf\b/.test(location.search), meas = window.__mir = [];     // with ?perf: the cost of the exact answer of each stop of the sight (tools/bench_mirino.py)
  const once = (key, fn) => { if (!FLY.has(key)) FLY.set(key, fn().finally(() => FLY.delete(key))); return FLY.get(key); };
  const axisOf = f => {                                                  // {rt, sid} of the scans of the level; null until it has arrived
    const key = `${f.k}|${f.lv || 1}`, e = AX.get(key); if (e) return e.v;
    const q = { v: null }; AX.set(key, q);
    once("a" + key, () => J(`api/scan_axis?k=${f.k}&level=${f.lv || 1}`)).then(j => { q.v = { rt: Float64Array.from(j.rt), sid: j.sid }; ask(null); }, () => AX.delete(key));
    return null;
  };
  const axisWait = async f => { const key = `${f.k}|${f.lv || 1}`; if (!AX.has(key)) axisOf(f); try { await FLY.get("a" + key); } catch (e) { /* no axis */ } return (AX.get(key) || {}).v || null; };
  const specKey = (f, ax, i) => `s${f.k}|${f.lv || 1}|${ax.sid[i]}`;
  const xicKey = (f, a, mz) => `x${f.k}${a.ref ? "+" + a.ref.k : ""}|${xicCentre(f, mz).toFixed(4)}|${xicTol(f, mz).toFixed(5)}|${f.lv || 1}`;
  const specSize = sp => (sp.mz.length + sp.y.length) * 8 + 200, xicSize = xi => (xi.tr || []).reduce((n, t) => n + (t.rt.length + t.y.length) * 8 + 100, 200);
  // what is already in the page for the point: the exact spectrum / XIC, or null
  function specFor(p, pt) {
    const s = st(p), a = p._a, f = a && a.f; if (!f || !pt) return null;
    const ax = axisOf(f); if (!ax) return null;
    const i = MIRINO.nearest(ax.rt, pt.rt); if (i < 0) return null;
    const key = specKey(f, ax, i); if (s.spec && s.spec.key === key) return s.spec;
    const sp = SIGHT.get(key); if (sp) s.spec = sp; return sp || null;
  }
  function xicFor(p, pt) {
    const s = st(p), a = p._a, f = a && a.f; if (!f || !pt) return null;
    const key = xicKey(f, a, pt.mz); if (s.xic && s.xic.key === key) return s.xic;
    const xi = SIGHT.get(key); if (xi) s.xic = xi; return xi || null;
  }
  async function fetchXic(p, mz) {
    const s = st(p), a = p._a; if (!a) return null; const f = a.f, c = xicCentre(f, mz), tol = xicTol(f, mz), key = xicKey(f, a, mz);
    if (s.xic && s.xic.key === key) return s.xic;
    const hit = SIGHT.get(key); if (hit) return (s.xic = hit);
    const ks = [f.k, ...(a.ref ? [a.ref.k] : [])].join(",");
    try {
      const xi = await once(key, async () => { const j = await J(`api/xic?k=${ks}&mz=${c}&tol=${tol}&level=${f.lv || 1}`); const r = { key, c, tol, tr: j.traces }; SIGHT.set(key, r, xicSize(r)); return r; });
      if (p._a && p._a.f === f) s.xic = xi;
    } catch (e) { /* nothing to show */ }
    return s.xic;
  }
  async function fetchSpec(p, rt) {
    const s = st(p), a = p._a; if (!a) return null; const f = a.f, lv = f.lv || 1;
    try {
      const ax = await axisWait(f), i = ax ? MIRINO.nearest(ax.rt, rt) : -1;
      if (i < 0) return s.spec;
      const key = specKey(f, ax, i); if (s.spec && s.spec.key === key) return s.spec;
      const hit = SIGHT.get(key); if (hit) return (s.spec = hit);
      const sp = await once(key, async () => {                          // one scan by position: the same arrays as /api/spectrum on a window holding only that scan
        const j = await J(`api/spectra?k=${f.k}&i0=${i}&i1=${i}&level=${lv}&bin=0.1${MERGE()}`), n = j.scans && j.scans[0];
        if (!n) throw new Error("no scan");
        const r = { key, sid: n.sid, rt: n.rt, mz: n.mz, y: n.y }; SIGHT.set(key, r, specSize(r)); return r;
      });
      s.spec = sp;
    } catch (e) { /* nothing to show */ }
    return s.spec;
  }
  // after the exact answer, the scans next to it in the direction of the movement are asked in the background (one request, only when nothing else is waiting)
  function prefetch(p, pt) {
    const s = st(p), a = p._a, f = a && a.f, ax = f && axisOf(f); if (!ax || !pt || ask.pending()) return;
    const i = MIRINO.nearest(ax.rt, pt.rt), dir = s.dir || 1, want = [i + dir, i + 2 * dir].filter(j => j >= 0 && j < ax.rt.length && !SIGHT.get(specKey(f, ax, j)));
    if (!want.length) return;
    const lo = Math.min(...want), hi = Math.max(...want), lv = f.lv || 1;
    (window.requestIdleCallback || (g => setTimeout(g, 120)))(() => {
      if (ask.pending() || p._a !== a) return;
      J(`api/spectra?k=${f.k}&i0=${lo}&i1=${hi}&level=${lv}&bin=0.1${MERGE()}`).then(j => {
        for (const n of j.scans || []) { const key = specKey(f, ax, n.i); if (!SIGHT.get(key)) { const r = { key, sid: n.sid, rt: n.rt, mz: n.mz, y: n.y }; SIGHT.set(key, r, specSize(r)); } }
      }).catch(() => { /* a prefetch is never needed */ });
    });
  }
  // preview: the row / column of the matrix of the map (the region on finer bins when the zoom has one) at the point; null for a difference map
  function preview(p, pt) {
    const s = st(p), a = p._a, R = s.reg && (p.zoom || p.zoomY) ? s.reg : null, m = R ? { v: R.v, nrt: R.nrt, nmz: R.nmz, rt0: R.rt0, rt1: R.rt1, mz0: R.mz0, dmz: R.dmz } : a && a.mat;
    if (!m || !pt) return null;
    const i = Math.floor((pt.rt - m.rt0) / (m.rt1 - m.rt0) * m.nrt), j = Math.floor((pt.mz - m.mz0) / m.dmz);
    const out = {};
    if (i >= 0 && i < m.nrt) { const c = MIRINO.column(m.v, m.nmz, i, m.mz0, m.dmz); out.spec = { mz: c.x, y: c.y, rt: pt.rt }; }
    if (j >= 0 && j < m.nmz) { const r = MIRINO.trace(m.v, m.nrt, m.nmz, j, m.rt0, m.rt1); out.xic = { c: pt.mz, tol: m.dmz / 2, tr: [{ k: a.f.k, rt: r.x, y: r.y }] }; }
    return out;
  }
  function show(p, pt) {                                                 // exact where the page has it, a preview for the rest
    const s = st(p); if (s.sp.hidden && s.xc.hidden) return;
    const sp = specFor(p, pt), xi = xicFor(p, pt), pv = !sp || !xi ? preview(p, pt) : null;
    if (MEAS && pv) window.__mirPrev = (window.__mirPrev || 0) + 1;
    s.shown = { pt, sp: sp ? sp.key : null, xi: xi ? xi.key : null };                 // what the two plots hold (read by e2e_mappa_mirino)
    drawSide(p, pt, sp || (pv && pv.spec) || s.spec, !sp); drawBottom(p, pt, xi || (pv && pv.xic) || s.xic, !xi);
    return sp && xi;
  }
  const ask = MIRINO.latest(() => { const p = ask.panel; return p ? exact(p) : null; });
  async function exact(p) {
    const q = point(p); if (!q) return;
    const s = st(p), t1 = performance.now();
    await Promise.all([fetchSpec(p, q.rt), fetchXic(p, q.mz)]);
    const t2 = performance.now(), r = point(p); if (!r || !p._a) return;
    const done = show(p, r); readout(p);                                 // an answer for a point the sight has left is not drawn: the newest point is already queued
    if (done) { if (MEAS && s.t0) meas.push({ fetch: t2 - t1, draw: performance.now() - t2, total: performance.now() - s.t0 }); s.t0 = 0; prefetch(p, r); }
  }
  function margins(p) {
    const s = st(p); if (!s.sp || s.sp.hidden && s.xc.hidden || !is2d(p)) return;
    if (s.ex) { if (window.MAPEX) MAPEX.margins(p); return; }
    const pt = point(p); if (!pt) { drawSide(p, null); drawBottom(p, null); return; }
    if (s.lastRt != null) s.dir = pt.rt > s.lastRt ? 1 : pt.rt < s.lastRt ? -1 : s.dir; s.lastRt = pt.rt;
    if (show(p, pt)) { if (MEAS) meas.push({ fetch: 0, draw: 0, total: s.t0 ? performance.now() - s.t0 : 0 }); s.t0 = 0; return; }
    if (!s.t0) s.t0 = performance.now();
    ask.panel = p; ask(pt);
  }
  const frame = p => { const s = st(p); if (!s.raf) s.raf = requestAnimationFrame(() => { s.raf = 0; const t = performance.now(); overlay(p); margins(p); if (MEAS) (window.__mirFrame || (window.__mirFrame = [])).push(performance.now() - t); }); };      // pointer events come faster than frames
  const xicTol = (f, mz) => isHrF(f) ? HR.tolDa(f, mz, f.lv || 1) : 0.5;           // LR: window [n - 0.2, n + 0.8] = centre n + 0.3, +-0.5
  const xicCentre = (f, mz) => isHrF(f) ? mz : Math.round(mz) + 0.3;
  function canvasOf(c) {
    const r = c.getBoundingClientRect(), d = Math.min(4, devicePixelRatio || 1), w = Math.round(r.width * d), h = Math.round(r.height * d);
    if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }      // resizing reallocates the bitmap: only when the panel changed size
    const g = c.getContext("2d"); g.setTransform(d, 0, 0, d, 0, 0); g.clearRect(0, 0, r.width, r.height); return { g, W: r.width, H: r.height };
  }
  // m/z labels of the sideways spectrum: beyond the tip of the bar (fixed gap) with a halo of the panel colour; if they do not fit they go above the bar,
  // to the left of the tip, so the bar never crosses its own text. The most intense peaks come first, a label never sits on another (at most 8). s.lbls keeps the boxes.
  function sideLabels(g, W, order, get, x0, s) {
    const out = []; s.lbls = out; g.textAlign = "left"; g.lineJoin = "round"; g.lineWidth = 3; g.strokeStyle = css("--panel"); g.fillStyle = css("--ink");
    for (const i of order) {
      if (out.length >= 8) break; const q = get(i);
      const w = g.measureText(q.t).width, right = q.tip + 5 + w <= W - 2, x = right ? q.tip + 5 : Math.max(x0 + 2, q.tip - 3 - w), by = right ? q.y + 4 : q.y - 3, bx = { x, y: by - 9, w, h: 11 };
      if (out.some(o => bx.x < o.x + o.w + 2 && o.x < bx.x + bx.w + 2 && bx.y < o.y + o.h + 1 && o.y < bx.y + bx.h + 1)) continue;
      g.strokeText(q.t, x, by); g.fillText(q.t, x, by); out.push({ t: q.t, ...bx, ly: q.y, tip: q.tip, right });
    }
    g.lineWidth = 1;
  }
  function drawSide(p, pt, sp, prev) {                                            // spectrum turned on its side: m/z upwards like the map, intensity to the right
    const s = st(p), a = p._a; if (s.sp.hidden || !a) return;
    const { g, W, H } = canvasOf(s.sp), ink = css("--muted"); g.clearRect(0, 0, W, H); g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink;
    const Y = v => H - M.b - (v - a.y0) / (a.y1 - a.y0) * (H - M.t - M.b), x0 = 6, x1 = W - 8;
    g.beginPath(); g.moveTo(x0 + .5, M.t); g.lineTo(x0 + .5, H - M.b); g.stroke();
    if (!sp || !pt) { g.fillText(T("mappa.side.title"), x0 + 4, M.t + 10); return; }
    let ymax = 0; const vis = []; for (let i = 0; i < sp.mz.length; i++) if (sp.mz[i] >= a.y0 && sp.mz[i] <= a.y1) { vis.push(i); if (sp.y[i] > ymax) ymax = sp.y[i]; }
    if (!ymax) { g.fillText(T("mappa.side.empty"), x0 + 4, M.t + 10); return; }
    if (window.TDATI) { s.sp.setAttribute("aria-label", T("mappa.side.title")); TDATI.addButton(s.sp, () => ({ title: T("mappa.side.title"), head: ["m/z", T("tdati.col.int")], rows: vis.slice(0, 300).map(i => [TDATI.num(sp.mz[i], 4), TDATI.num(sp.y[i], 1)]) })); }
    g.strokeStyle = a.f.color || css("--accent"); g.globalAlpha = prev ? .5 : 1; g.beginPath();
    for (const i of vis) { const y = Math.round(Y(sp.mz[i])) + .5; g.moveTo(x0, y); g.lineTo(x0 + (x1 - x0) * sp.y[i] / ymax, y); }
    g.stroke(); g.globalAlpha = 1;
    if (prev) { g.fillStyle = ink; g.fillText(T("mappa.preview"), x0 + 4, H - M.b + 14); return; }       // a preview has bins, not peaks: no m/z labels
    const top = vis.slice().sort((u, v) => sp.y[v] - sp.y[u]);
    sideLabels(g, W, top.filter(i => sp.y[i] >= ymax * 0.05), i => ({ y: Y(sp.mz[i]), tip: x0 + (x1 - x0) * sp.y[i] / ymax, t: sp.mz[i].toFixed(mzd(p)) }), x0, s);
    if (pt.mz >= a.y0 && pt.mz <= a.y1) { g.strokeStyle = css("--ink"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(x0, Y(pt.mz) + .5); g.lineTo(x1, Y(pt.mz) + .5); g.stroke(); g.setLineDash([]); }
    g.fillStyle = ink; g.fillText(`${T("mappa.read.scan", { n: sp.sid + 1 })} · RT ${sp.rt.toFixed(2)}`, x0 + 4, H - M.b + 14);
  }
  function drawBottom(p, pt, xi, prev) {                                          // XIC under the map: the same RT axis
    const s = st(p), a = p._a; if (s.xc.hidden || !a) return;
    const { g, W, H } = canvasOf(s.xc), ink = css("--muted"), mt = 10, mb = 22; g.clearRect(0, 0, W, H); g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink;
    const X = v => M.l + (v - a.x0) / (a.x1 - a.x0) * (W - M.l - M.r);
    g.beginPath(); g.moveTo(M.l, H - mb + .5); g.lineTo(W - M.r, H - mb + .5); g.stroke();
    for (const t of nice(a.x0, a.x1, 8)) { const x = X(t); if (x < M.l - 1 || x > W - M.r + 1) continue; g.textAlign = "center"; g.fillText(String(+t.toFixed(2)), x, H - mb + 13); }
    if (!xi || !pt) { g.textAlign = "left"; g.fillText(T("mappa.bot.title"), M.l + 4, mt + 8); return; }
    let ymax = 0; const tr = xi.tr || [];
    for (const t of tr) for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= a.x0 && t.rt[i] <= a.x1 && t.y[i] > ymax) ymax = t.y[i];
    if (window.TDATI) { s.xc.setAttribute("aria-label", T("mappa.bot.title")); TDATI.series(s.xc, { title: T("mappa.bot.title"), xl: T("tdati.col.rt"), yl: T("tdati.col.int"), series: tr.map(t => ({ x: t.rt, y: t.y, name: t.label || t.name })) }); }
    const Y = v => H - mb - v / (ymax || 1) * (H - mt - mb);
    tr.forEach((t, n) => {
      const f = E.files[t.k]; g.strokeStyle = f ? f.color : ink; g.lineWidth = 1.2; g.globalAlpha = prev ? .5 : 1; if (n) g.setLineDash([4, 3]); g.beginPath(); let first = true;
      for (let i = 0; i < t.rt.length; i++) { if (t.rt[i] < a.x0 || t.rt[i] > a.x1) continue; const x = X(t.rt[i]), y = Y(t.y[i]); if (first) { g.moveTo(x, y); first = false; } else g.lineTo(x, y); }
      g.stroke(); g.setLineDash([]); g.lineWidth = 1; g.globalAlpha = 1;
    });
    g.fillStyle = css("--ink"); g.textAlign = "left";
    const lo = xi.c - xi.tol, hi = xi.c + xi.tol;
    g.fillText(prev ? `XIC · ${T("mappa.preview")}` : `XIC ${lo.toFixed(isHrF(a.f) ? 4 : 1)}–${hi.toFixed(isHrF(a.f) ? 4 : 1)} · max ${fmt(ymax)}`, M.l + 4, mt + 8);
    if (pt.rt >= a.x0 && pt.rt <= a.x1) { g.strokeStyle = css("--ink"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(X(pt.rt) + .5, mt); g.lineTo(X(pt.rt) + .5, H - mb); g.stroke(); g.setLineDash([]); }
  }

  // ---------------------------------------------------------------- mouse
  function move(p, px, py) {
    if (!is2d(p)) return; const s = st(p), a = p._a;
    if (s.feat && window.MAPEX && MAPEX.featHover(p, px, py)) return;
    if (s.ex && window.MAPEX && MAPEX.hover(p, px, py)) return;
    if (s.frz) return;                                                  // the menu is open: the sight stays on the clicked point
    if (px < M.l || px > a.W - M.r || py < M.t || py > a.H - M.b) { s.hov = null; } else { s.hov = { rt: xOf(p, px), mz: a.mzAt(py) }; s.kp = null; }
    if (mirOn(p)) { p.tip.hidden = true; p.vl.hidden = true; }
    if (!s.lock) frame(p); else readout(p);
  }
  function leave(p) { const s = st(p); if (s.frz) return; s.hov = null; if (!s.lock && s.ovg) frame(p); }
  function wheel(p, e, px, py) {
    if (!is2d(p) || e.ctrlKey || e.metaKey) return false;               // Ctrl/Cmd + wheel (a pinch on the trackpad) keeps the zoom of explore.js
    e.preventDefault();
    const a = p._a;
    if (!p._wz || Date.now() - p._wz > 600) pushZh(p); p._wz = Date.now();
    if (!e.shiftKey && !e.altKey && Math.abs(e.deltaX) > Math.abs(e.deltaY)) {          // trackpad, two fingers sideways: move along RT
      panPx(p, -e.deltaX, 0); draw(p); return true;
    }
    const dy = e.deltaY || e.deltaX, f = Math.exp(Math.max(-60, Math.min(60, dy)) * 0.004);
    scaleView(p, f, { rt: xOf(p, px), mz: a.mzAt(py) }, e.shiftKey ? "x" : e.altKey ? "y" : "xy");       // Maiusc = RT only, Alt = m/z only
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
  function unlock(p) { const s = st(p); s.lock = null; overlay(p); margins(p); if (!mirOn(p)) p.rd.textContent = ""; }
  // right click: lock / unlock the point; Maiusc + right click: the XIC of that m/z (the old right-click entry, also in the «⋯» menu)
  function ctx(p, e, px, py) {
    if (!is2d(p)) return false;
    e.preventDefault(); hideHover(p);
    const a = p._a, inside = px >= M.l && px <= a.W - M.r && py >= M.t && py <= a.H - M.b;
    if (e.shiftKey) { if (inside) openXic(null, { mz: a.mzAt(py), obs: true }); return true; }
    if (Date.now() - (st(p).lpAt || 0) < 900) return true;             // the long press has just opened the menu (Android also sends contextmenu)
    showMenu(ad2(p, inside ? { rt: xOf(p, px), mz: a.mzAt(py) } : null), e); return true;
  }
  // the «⋯» menu: the old right-click menu, at the point of the sight (locked, keyboard or the middle of the map)
  function menuAt(p, btn, kbd) {
    const a = p._a; if (!a) return; const pt = point(p) || { rt: (a.x0 + a.x1) / 2, mz: (a.y0 + a.y1) / 2 }, r = btn.getBoundingClientRect();
    showMenu(ad2(p, pt), { preventDefault() {}, clientX: r.left + (kbd ? r.width / 2 : 0), clientY: kbd ? r.top + r.height / 2 : r.bottom + 4, kbd: !!kbd });
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
    if (s.ex && window.MAPEX && MAPEX.key(p, e)) { done(); return; }
    if (k === "m" || k === "M") { done(); s.mir = !mirOn(p); layout(p); draw(p); return; }
    if (k === "l" || k === "L") { done(); ad2(p, null).toggleLock(); return; }
    if (k === "Escape") { if (s.lock) { done(); unlock(p); navSync(p); } else if (!isMax(p) && p.zh && p.zh.length) { done(); undoZoom(p); } return; }       // Esc: unlock, or take back the last zoom
    if (k === "Home") { done(); whole(p); return; }
    if (k === "+" || k === "=") { done(); ad2(p, null).zoomBy(0.7); return; }
    if (k === "-" || k === "_") { done(); ad2(p, null).zoomBy(1 / 0.7); return; }
    if (k === "?") { done(); help("2d"); return; }
    if (k === "ContextMenu" || (k === "F10" && e.shiftKey)) { done(); menuAt(p, p.cv, true); return; }
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
  // from the 3D viewer: a point at (rt, mz), snapped to the local maximum like a click on the map (dmz = width of the cell, in Da)
  async function addPointAt(p, rt, mz, dmz) {
    const a = p._a; if (!a || !a.f) return;
    let at = null; try { at = await J(`api/mapsnap?k=${a.f.k}&level=${a.f.lv || 1}&rt=${rt}&mz=${mz}&dmz=${dmz || 1}`); } catch (e) { /* no snap */ }
    const q = at && at.rt != null ? at : { rt, mz };
    const list = pts(p); list.push({ rt: q.rt, mz: q.mz, note: "" }); st(p).psel = list.length - 1;
    ptsSave(p); overlay(p); table(p);
  }
  const markedAt = (p, q) => p._a ? nearPoint(p, p._a.X(q.rt), p._a.Y(q.mz)) : -1;      // index of the marked point under (rt, m/z), -1 if none
  function nearPoint(p, px, py) {
    const a = p._a; let best = -1, bd = 81;
    pts(p).forEach((q, i) => { const d = (a.X(q.rt) - px) ** 2 + (a.Y(q.mz) - py) ** 2; if (d < bd) { bd = d; best = i; } });
    return best;
  }
  // mousedown on a point: drag it (it snaps again where it is dropped); true = explore.js does nothing else
  function down(p, e, px, py) {
    if (!is2d(p) || e.button !== 0 || e.shiftKey || e.altKey) return false;
    if (st(p).ex && window.MAPEX && MAPEX.down(p, e, px, py)) return true;       // the box of «How it is extracted»
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
    if (s.feat && window.MAPEX && MAPEX.featClick(p, px, py)) return true;       // a marker of mzFinder: the box goes there
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
    const ri = list.findIndex(q => q.ref);                      // the point the student chose as the reference of Δm (none: the most intense of each group)
    if (ri >= 0 && rows[ri] && rows[ri].mz != null) rows.forEach(r => { r.dm = r.mz == null ? null : r.mz - rows[ri].mz; r.isref = r.n - 1 === ri; });
    s.rows = rows;
    const so = s.sort || { c: "n", d: 1 }, val = (r, c) => r[c] == null ? -Infinity : r[c];
    const view = rows.slice().sort((u, v) => typeof val(u, so.c) === "string" ? so.d * String(val(u, so.c)).localeCompare(String(val(v, so.c))) : so.d * (val(u, so.c) - val(v, so.c)));
    const dec = mzd(p), hr = isHrF(f), num = (v, d) => v == null || !Number.isFinite(v) ? "–" : v.toFixed(d);
    const head = COLS.map(c => `<th data-c="${c}" title="${T(COLK[c][1])}">${T(COLK[c][0])}${so.c === c ? (so.d > 0 ? " ▲" : " ▼") : ""}</th>`).join("") + `<th title="${T("mappa.tab.ref.title")}">${T("mappa.tab.ref")}</th><th></th>`;
    box.innerHTML = `<div class="mzt-bar"><b>${T("mappa.tab.title", { n: rows.length })}</b><label class="muted" title="${T("mappa.tab.grp.title")}">${T("mappa.tab.grp")} ± <input data-t="grp" class="mzf" inputmode="decimal" value="${PREF.grp ?? 0.05}"> min</label>` +
      `<span class="sp"></span><button data-t="xl" title="${T("mappa.tab.xl.title")}">Excel</button><button data-t="cp" title="${T("mappa.tab.cp.title")}">${T("mappa.tab.cp")}</button><button data-t="clear">${T("mappa.tab.clear")}</button></div>` +
      `<div class="mzt-sc"><table><thead><tr>${head}</tr></thead><tbody>${view.map(r => `<tr data-i="${r.n - 1}" class="${s.psel === r.n - 1 ? "on" : ""}">` +
      `<td>${r.n}</td><td>${num(r.rt, 2)}</td><td>${num(r.mz, hr ? 4 : 1)}</td><td>${r.ia == null ? "–" : fmt(r.ia)}</td><td>${r.ib == null ? "–" : fmt(r.ib)}</td><td>${r.d == null ? "–" : fmt(r.d)}</td>` +
      `<td>${num(r.ratio, 2)}</td><td>${num(r.sn, 1)}</td><td>${r.g ?? "–"}</td><td>${r.dm == null ? "–" : r.dm === 0 ? "0" : `<a href="#" data-dm="${r.dm}">${(r.dm > 0 ? "+" : "") + r.dm.toFixed(hr ? 4 : 1)}</a>`}</td>` +
      `<td><input data-note="${r.n - 1}" value="${EH(r.note || "")}" placeholder="${T("mappa.tab.note")}"></td><td><button data-ref="${r.n - 1}" aria-pressed="${r.isref ? "true" : "false"}" title="${T("mappa.tab.ref.pick", { n: r.n })}" aria-label="${T("mappa.tab.ref.pick", { n: r.n })}">${r.isref ? "●" : "○"}</button></td><td><button data-xic="${r.n - 1}" title="${T("mappa.tab.xic.title")}">XIC</button></td></tr>`).join("")}</tbody></table></div>`;
    box.querySelectorAll("th[data-c]").forEach(th => th.onclick = () => { const c = th.dataset.c; s.sort = { c, d: so.c === c ? -so.d : 1 }; tableNow(p); });
    box.querySelector('[data-t="grp"]').onchange = ev => { const v = numIt(ev.target.value); if (v > 0) { PREF.grp = v; prefSave(); } tableNow(p); };
    box.querySelector('[data-t="xl"]').onclick = () => excel(p);
    box.querySelector('[data-t="cp"]').onclick = () => copyRows(p);
    box.querySelector('[data-t="clear"]').onclick = async () => { if (await yesno(T("mappa.tab.clear.ask"))) { pts(p).splice(0); ptsSave(p); overlay(p); tableNow(p); } };
    box.querySelectorAll("tbody tr").forEach(tr => {
      const i = +tr.dataset.i;
      tr.onclick = ev => { if (ev.target.closest("input,button,a")) return; const q = pts(p)[i]; if (!q) return; s.psel = i; s.lock = { rt: q.rt, mz: q.mz }; follow(p, s.lock); overlay(p); margins(p); tableNow(p); };
      tr.ondblclick = ev => { if (ev.target.closest("input,button,a")) return; const q = pts(p)[i]; if (q) spectrumAt(p, q.rt); };
    });
    box.querySelectorAll("[data-note]").forEach(inp => inp.onchange = () => { const q = pts(p)[+inp.dataset.note]; if (q) { q.note = inp.value; ptsSave(p); } });
    box.querySelectorAll("[data-ref]").forEach(b => b.onclick = () => { const i = +b.dataset.ref, was = !!(pts(p)[i] || {}).ref; pts(p).forEach(q => { delete q.ref; }); if (!was && pts(p)[i]) pts(p)[i].ref = true; ptsSave(p); tableNow(p); });
    box.querySelectorAll("[data-xic]").forEach(b => b.onclick = () => { const q = pts(p)[+b.dataset.xic]; if (q) xicAB(p, q.mz); });
    box.querySelectorAll("[data-dm]").forEach(l => l.onclick = ev => { ev.preventDefault(); openDm(+l.dataset.dm, isHrF(f)); });
  }
  function growTable(p, on) {
    const s = st(p); if (on === !!s.tgrown || isMax(p)) return;
    s.tgrown = on; p.h = Math.max(200, p.h + (on ? 1 : -1) * 220); apply(p); if (typeof relayout === "function") relayout(); fitHost(); uiSave();
  }
  async function spectrumAt(p, rt, k) {
    const f = k != null && E.files[k] ? E.files[k] : p._a.f; let r0 = rt - scanStep() / 2, r1 = rt + scanStep() / 2;
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
      [T("mappa.meta.snap"), "± 0.2 min · ± 1 bin"], [T("mappa.meta.date"), new Date().toLocaleString(LOCALE)]];
    dlx(`${f.label}${a.ref ? "-" + a.ref.label : ""}_punti.xlsx`, [{ name: T("mappa.tab.sheet"), head: headRow(), rows: sheetRows(p), widths: [5, 9, 11, 12, 12, 12, 9, 8, 7, 10, 30] },
      { name: T("mappa.meta.sheet"), head: [T("mappa.meta.what"), T("mappa.meta.value")], rows: meta, widths: [30, 40] }]);
  }
  async function copyRows(p) {
    const txt = [headRow(), ...sheetRows(p)].map(r => r.join("\t")).join("\n");
    try { await navigator.clipboard.writeText(txt); nearMsg(T("mappa.tab.copied")); } catch (e) { nearMsg(T("mappa.tab.copyFail")); }
  }

  // ---------------------------------------------------------------- navigation: ONE list of commands (CMDS) for the right-click menu of the 2D map and of the 3D viewer,
  // the bar on the map, the command palette (barra.js reads CMDS) and the sheet of gestures. An «adapter» says where a command acts:
  //   { kind: "2d" | "3d", p, pt() -> {rt, mz, v?} | null, locked(), toggleLock(), resetAll(), zoomBy(f), view?(n), after?() }
  const SV = d => `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${d}</svg>`;
  const ICO = {
    xic: '<path d="M2 13h12"/><path d="M3 12c2-7 3-7 5 0s3 3 5-3"/>', spec: '<path d="M3 13V8M6 13V4M9 13V9M12 13V6"/>',
    both: '<path d="M2 13h5"/><path d="M2.5 12c1-6 2-6 3 0M9 13V8M12 13V5"/>',
    mark: '<circle cx="8" cy="8" r="5.2"/><circle cx="8" cy="8" r="1.6"/>', lock: '<rect x="3.5" y="7" width="9" height="6.5" rx="1.2"/><path d="M5.5 7V5a2.5 2.5 0 015 0v2"/>', unlock: '<rect x="3.5" y="7" width="9" height="6.5" rx="1.2"/><path d="M5.5 7V5a2.5 2.5 0 014.8-1"/>',
    home: '<g transform="translate(1 1) scale(1.0714)"><path d="M1 5V1h4M9 1h4v4M13 9v4H9M5 13H1V9"/></g>',       // the corners of the «whole view» button of every panel
     rx: '<path d="M2 8h12M2 5.5v5M14 5.5v5"/>', ry: '<path d="M8 2v12M5.5 2h5M5.5 14h5"/>',
    sel: '<rect x="3" y="3" width="10" height="10" rx="1" stroke-dasharray="2.5 2"/><path d="M8 6v4M6 8h4"/>', plus: '<path d="M8 3v10M3 8h10"/>', minus: '<path d="M3 8h10"/>',
    copy: '<rect x="5.5" y="5.5" width="8" height="8" rx="1.2"/><path d="M10.5 5.5V3.7a1.2 1.2 0 00-1.2-1.2H3.7a1.2 1.2 0 00-1.2 1.2v5.6a1.2 1.2 0 001.2 1.2h1.8"/>',
    help: '<circle cx="8" cy="8" r="6"/><path d="M6.2 6.3a1.9 1.9 0 113 1.5c-.8.5-1.2.9-1.2 1.8M8 11.7v.1"/>',
    feat: '<circle cx="5" cy="10" r="2.2"/><circle cx="11" cy="5" r="2.2"/><path d="M2.5 13.5h11"/>', learn: '<rect x="2.5" y="2.5" width="6.5" height="6.5" rx="1"/><path d="M9 9l4.5 4.5M5.8 9v4.5h4M9 5.8h4.5"/>', top: '<rect x="3" y="3" width="10" height="10" rx="1"/><path d="M3 8h10M8 3v10"/>', side: '<path d="M2 13h12"/><path d="M3 12l2.5-7 2.5 4 2-6 3 9"/>'
  };
  // where a command acts: the locked point, otherwise the one under the pointer when the menu was opened, otherwise the sight
  function ad2(p, at) {
    const s = st(p), a = () => p._a, mid = () => ({ rt: (a().x0 + a().x1) / 2, mz: (a().y0 + a().y1) / 2 });
    const pt = () => { const q = s.lock || at || point(p); return q ? { ...q, v: valueAt(p, q.rt, q.mz) } : null; };
    return {
      kind: "2d", p, pt, locked: () => !!s.lock,
      toggleLock: () => { if (s.lock) unlock(p); else { s.lock = { ...(at || point(p) || mid()) }; s.kp = null; overlay(p); margins(p); } navSync(p); },
      marked: () => { const q = pt(); return !!q && markedAt(p, q) >= 0; },
      toggleMark: () => { const q = pt(); if (!q) return; const i = markedAt(p, q); if (i >= 0) delPoint(p, i); else addPointAt(p, q.rt, q.mz, p._a.dmz); },
      resetAll: () => whole(p),
      zoomBy: f => zoomBy(p, f, s.lock || at || point(p))
    };
  }
  const touched = p => Date.now() - (st(p).gestAt || 0) < 700;           // a finger gesture has just moved the map: the mouse events that follow it are not clicks or boxes
  const ptText = q => q ? `RT ${q.rt.toFixed(2)} min · m/z ${q.mz.toFixed(3)}${q.v != null ? " · " + T("mappa.read.int") + " " + fmt(q.v) : ""}` : "";
  function extract(ad, what) {
    const q = ad.pt(); if (!q) { nearMsg(T("mapnav.nopt")); return; }
    if (what !== "spec") xicDirect(q.mz, ad.p);
    if (what !== "xic") { const p = ad.p; if (ad.kind === "2d" && p.sel && p.sel[1] > p.sel[0]) newSpec(p, p.sel[0], p.sel[1], p._a.f.k); else spectrumAt(p, q.rt, ad.k); }
    if (ad.after) ad.after();
  }
  const CMDS = [
    { id: "mapxic", g: "extract", ic: "xic", short: true, kinds: ["2d", "3d"], ok: ad => !!ad.pt(), run: ad => extract(ad, "xic") },
    { id: "mapspec", g: "extract", ic: "spec", short: true, kinds: ["2d", "3d"], ok: ad => !!ad.pt(), run: ad => extract(ad, "spec") },
    { id: "maplock", g: "lock", ic: "lock", kinds: ["2d", "3d"], on: ad => ad.locked(), run: ad => ad.toggleLock() },
    { id: "mapmark", g: "lock", ic: "mark", kinds: ["2d"], ok: ad => !!ad.pt(), on: ad => ad.marked(), run: ad => ad.toggleMark() },
    { id: "mapreset", g: "view", ic: "home", kinds: ["2d", "3d"], run: ad => ad.resetAll() },
    { id: "map3top", g: "view", ic: "top", kinds: ["3d"], run: ad => ad.view(1) },
    { id: "map3rt", g: "view", ic: "side", kinds: ["3d"], run: ad => ad.view(2) },
    { id: "map3mz", g: "view", ic: "spec", kinds: ["3d"], run: ad => ad.view(3) },
    { id: "mapzin", g: "zoom", ic: "plus", kinds: ["2d", "3d"], bar: true, menu: false, run: ad => ad.zoomBy(0.7) },
    { id: "mapzout", g: "zoom", ic: "minus", kinds: ["2d", "3d"], bar: true, menu: false, run: ad => ad.zoomBy(1 / 0.7) },
    { id: "mapfeat", g: "learn", ic: "feat", kinds: ["2d"], menu: false, vis: ad => featAvail(ad.p), on: ad => !!st(ad.p).feat, run: ad => featToggle(ad.p) },
    { id: "mapcopy", g: "copy", ic: "copy", kinds: ["2d", "3d"], ok: ad => !!ad.pt(), run: async ad => { const q = ad.pt(); if (!q) return; try { await navigator.clipboard.writeText(ptText(q)); nearMsg(T("mapnav.copied")); } catch (e) { nearMsg(T("mapnav.copyFail")); } } },
    { id: "maphelp", g: "help", ic: "help", kinds: ["2d", "3d"], run: ad => help(ad.kind) }
  ];
  const cmdOf = id => CMDS.find(c => c.id === id);
  const nameOf = (c, ad) => T(c.id === "maplock" && ad.locked() || c.id === "mapmark" && ad.marked() ? `cmd.${c.id}.off.name` : `cmd.${c.id}.name`);
  const runCmd = (id, ad) => { const c = cmdOf(id); if (c && (!c.ok || c.ok(ad))) return c.run(ad); };
  // the items of the right-click menu (shared component: menu() in explore.js)
  function itemsFor(ad) {
    const items = [], q = ad.pt(), hdr = ad.locked() ? T("mapnav.where.locked", { at: ptText(q) }) : q ? T("mapnav.where", { at: ptText(q) }) : T("mapnav.where.none");
    items.push({ label: hdr, dim: true }, "-");
    let last = null;
    for (const c of CMDS) {
      if (!c.kinds.includes(ad.kind) || c.menu === false || c.vis && !c.vis(ad)) continue;
      if (last && c.g !== last) items.push("-");
      last = c.g;
      const on = c.on ? c.on(ad) : null, dis = c.ok && !c.ok(ad);
      const lab = c.id === "mapspec" && ad.kind === "2d" && ad.p.sel ? T("cmd.mapspec.menuAvg") : c.short ? T(`cmd.${c.id}.menu`) : nameOf(c, ad);       // with an RT stretch selected (Alt + drag) the spectrum is the average over it
      items.push({ label: lab, icon: SV(c.id === "maplock" && on ? ICO.unlock : ICO[c.ic]), aria: nameOf(c, ad), tip: T(`cmd.${c.id}.tip`), check: on, dim: dis, fn: () => runCmd(c.id, ad) });
    }
    return items;
  }
  // while the menu is open the sight (spectrum, XIC) is held on the point of the click; it is let go when the menu closes (a choice such as Lock has already set s.lock)
  function holdSight(ad) {
    if (ad.kind !== "2d") return; const s = st(ad.p), q = ad.pt(), m = Q("#ctx"); if (!q || s.lock) return;
    s.frz = { rt: q.rt, mz: q.mz }; s.kp = null; frame(ad.p);
    const mo = new MutationObserver(() => { if (!m.hidden) return; mo.disconnect(); s.frz = null; frame(ad.p); navSync(ad.p); });
    mo.observe(m, { attributes: true, attributeFilter: ["hidden"] });
  }
  function showMenu(ad, ev) { if (ad.host) ad.host(); menu(ev, itemsFor(ad)); holdSight(ad); }
  // zoom around a point (default: the middle of the view); axes "x" = RT, "y" = m/z
  function scaleView(p, f, c, axes) {
    const a = p._a, x = c ? c.rt : (a.x0 + a.x1) / 2, y = c ? c.mz : (a.y0 + a.y1) / 2;
    if (axes.includes("y")) p.zoomY = clampView(y - (y - a.y0) * f, y + (a.y1 - y) * f, a.fullY[0], a.fullY[1]);
    if (axes.includes("x")) p.zoom = clampView(x - (x - a.x0) * f, x + (a.x1 - x) * f, a.full[0], a.full[1]);
  }
  function zoomBy(p, f, c) {
    const a = p._a; if (!a) return; if (c && (c.rt < a.x0 || c.rt > a.x1 || c.mz < a.y0 || c.mz > a.y1)) c = null;
    pushZh(p); scaleView(p, f, c, "xy"); draw(p); uiSave();
  }
  function panPx(p, dx, dy) {                                           // the picture follows the finger / the pointer by (dx, dy) pixels
    const a = p._a, w = a.W - M.l - M.r, h = a.H - M.t - M.b, sx = (a.x1 - a.x0) / w * dx, sy = (a.y1 - a.y0) / h * dy;
    p.zoom = clampView(a.x0 - sx, a.x1 - sx, a.full[0], a.full[1]); p.zoomY = clampView(a.y0 + sy, a.y1 + sy, a.fullY[0], a.fullY[1]);
  }
  function pinch(p, last, c, f) {                                      // scale by f around the previous midpoint, then follow the midpoint to where it is now
    const a = p._a, w = a.W - M.l - M.r, h = a.H - M.t - M.b, rt = xOf(p, last.x), mz = a.mzAt(last.y);
    const x0 = rt - (rt - a.x0) * f, x1 = rt + (a.x1 - rt) * f, y0 = mz - (mz - a.y0) * f, y1 = mz + (a.y1 - mz) * f, sx = (x1 - x0) / w * (c.x - last.x), sy = (y1 - y0) / h * (c.y - last.y);
    p.zoom = clampView(x0 - sx, x1 - sx, a.full[0], a.full[1]); p.zoomY = clampView(y0 + sy, y1 + sy, a.fullY[0], a.fullY[1]);
  }
  // ---- the bar on the map: reset, zoom, lock, gestures (the same commands)
  const BAR = { "2d": ["mapzin", "mapzout", "maplock", "mapfeat", "maphelp"], "3d": ["mapreset", "mapzin", "mapzout", "maplock", "maphelp"] };      // 2D: the reset is the «fit» button of the panel head
  // the buttons sit in the toolbar of the panel (2D: the head, with the same .bt look as the others) or in the header of the side column (3D); nothing floats over the data
  function barFor(getAd, kind = "2d") {
    const b = document.createElement("span"); b.className = "tbg mznav"; b.setAttribute("role", "toolbar"); b.setAttribute("aria-label", T("mapnav.bar"));
    b.innerHTML = BAR[kind].map(id => `<button type="button"${kind === "2d" ? ' class="bt"' : ""} data-nav="${id}" title="${EH(T(`cmd.${id}.tip`))}" aria-label="${EH(T(`cmd.${id}.name`))}"${id === "maplock" || id === "mapfeat" ? ' aria-pressed="false"' : ""}>${SV(ICO[cmdOf(id).ic])}</button>`).join("");
    b.addEventListener("click", e => { const x = e.target.closest("button[data-nav]"); if (x) { e.stopPropagation(); runCmd(x.dataset.nav, getAd()); } });
    b.addEventListener("mousemove", e => e.stopPropagation());
    lockBtn(b, false); return b;
  }
  function lockBtn(bar, on) {                                           // the padlock button shows the state: closed + pressed = locked
    const b = bar && bar.querySelector('[data-nav="maplock"]'); if (!b) return;
    b.setAttribute("aria-pressed", String(on)); b.classList.toggle("on", on); b.innerHTML = SV(on ? ICO.lock : ICO.unlock);
    const nm = T(on ? "cmd.maplock.off.name" : "cmd.maplock.name"); b.title = nm; b.setAttribute("aria-label", nm);
  }
  const navBar = p => { const b = barFor(() => ad2(p, null)); st(p).nav = b; return b; };
  const navSync = p => { const s = st(p); lockBtn(s.nav, !!s.lock); const f = s.nav && s.nav.querySelector('[data-nav="mapfeat"]'); if (f) { f.hidden = !featAvail(p); f.setAttribute("aria-pressed", String(!!s.feat)); f.classList.toggle("on", !!s.feat); } };
  // ---- «Correzione» of the map: one at a time (none | a file | the background of a stretch | the automatic background), like the chromatograms and the spectrum.
  // p.corr: undefined = an old panel (converted at the first draw from p.ref and from the old global switch PREF.bg), "" = follow the reference of the session, "none", "file" (p.ref), "tract" (p.bgr = [rt0, rt1]), "auto".
  function corrInit(p) { if (p.corr === undefined) p.corr = p.ref !== "" && p.ref != null ? "file" : PREF.bg ? "auto" : ""; }
  function corrState(p) {
    corrInit(p); const se = p.corr === "" && window.SUBREF ? SUBREF.eff(p) : null;
    return { now: p.corr === "file" && p.ref !== "" && p.ref != null ? "f" + p.ref : se ? "f" + se.k : p.corr === "tract" ? "tract" : p.corr === "auto" ? "auto" : "" };
  }
  function setCorr(p, v) {
    corrInit(p); if (v === "snip") v = "auto";                                // the shared menu calls the automatic background «snip»
    if (v === "") { p.corr = "none"; p.ref = ""; p.bgr = null; }
    else if (v[0] === "f") { p.corr = "file"; p.ref = +v.slice(1); p.bgr = null; }
    else if (v === "auto") { p.corr = "auto"; p.ref = ""; p.bgr = null; }
    else if (v === "tract") {
      p.corr = "tract"; p.ref = "";
      if (p.sel && p.sel[1] > p.sel[0]) { p.bgr = [...p.sel]; p.sel = null; p._pickBg = false; } else if (!p.bgr) { p._pickBg = true; nearMsg(T("mapcorr.pick")); }
    }
    ctl(p); draw(p); uiSave();
  }
  // the request suffix of the correction that is not a file: the automatic background or the mean of the chosen stretch
  function corrQ(p) {
    corrInit(p);
    if (p.corr === "auto") return "&bg=1";
    if (p.corr === "tract" && p.bgr) return `&bgrt0=${+(+p.bgr[0]).toFixed(4)}&bgrt1=${+(+p.bgr[1]).toFixed(4)}`;
    return "";
  }
  function corrPick(p) {                                                // «Background of a stretch» chosen with no stretch yet: the next stretch selected with Alt + drag is the one
    if (!p._pickBg || !p.sel || !(p.sel[1] > p.sel[0])) return false;
    p.bgr = [...p.sel]; p.sel = null; p._pickBg = false; ctl(p); uiSave(); return true;
  }
  // what is not at its default in «Options»: chips next to the button
  function optChips(p) {
    const o = [], d = diffOpt(corrState(p).now[0] === "f");
    if (p.scale && p.scale !== "lin") o.push(p.scale === "gamma" ? `${T("ctl.map.scale.gamma")} ${p.gamma || 0.6}` : T(`ctl.map.scale.${p.scale}`));
    if (PREF.pal === "cividis") o.push("cividis");
    if (PREF.thr != null && Math.abs(PREF.thr - (corrState(p).now[0] === "f" ? 0.02 : 0.004)) > 1e-9) o.push(`${T("mappa.thr")} ${+(PREF.thr * 100).toFixed(1)} %`);
    if (corrState(p).now[0] === "f" && p.norm && p.norm !== "abs") o.push(T(`ctl.map.norm.${p.norm}`));
    if (d.show !== "all" && corrState(p).now[0] === "f") o.push(T(`mappa.show.${d.show}`));
    return o;
  }
  // ---- the box «How it is extracted» (opened by the markers of mzFinder; the module loads the first time)
  async function explain(p, on) {
    const s = st(p); if (on === undefined) on = !s.ex;
    if (!window.MAPEX) { try { window.MAPEX = await import("./mappa-estrai.js"); } catch (e) { console.error(e); nearMsg(T("mapex.err")); return; } }
    if (on) MAPEX.start(p); else MAPEX.stop(p);
    navSync(p);
  }
  const featAvail = p => !!(p._a && p._a.f && window.MAPFEAT && window.HR && HR.isHr(p._a.f, p._a.f.lv || 1) && (MAPFEAT.features(p._a.f) || []).length);       // only the high-resolution world, only when mzFinder holds a result
  async function featToggle(p) {
    const s = st(p); if (!s.feat && !featAvail(p)) return;
    if (!window.MAPEX) { try { window.MAPEX = await import("./mappa-estrai.js"); } catch (e) { console.error(e); return; } }
    s.feat = !s.feat; if (!s.feat && window.MAPEX) MAPEX.featClear(p);
    navSync(p); frame(p);
  }
  function grow(p, key, px) {                                           // the panel is px taller while a block under the map is shown (0 = gone); the amount may change
    const s = st(p), k = "g" + key, prev = s[k] || 0; if (px === prev || isMax(p)) return;
    s[k] = px; p.h = Math.max(200, p.h + px - prev); apply(p); if (typeof relayout === "function") relayout(); fitHost(); uiSave();
  }
  // ---- sheet of the gestures (one key per row: action|mouse|trackpad|touch|keyboard)
  const HELP = { "2d": ["zoom", "zoomax", "box", "pan", "reset", "undo", "sight", "lock", "menu", "extract", "interval", "mark", "full"], "3d": ["rot", "pan", "zoom", "reset", "views", "pick", "menu", "cur"] };
  function help(kind) {
    const rows = HELP[kind].map(r => I18N.t(`mapnav.h.${kind}.${r}`).split("|").map(x => EH(x.trim()) || "–"));
    const head = ["act", "mouse", "pad", "touch", "key"].map(k => `<th scope="col">${I18N.t(`mapnav.h.${k}`)}</th>`).join("");
    big(I18N.t(`mapnav.h.title.${kind}`), `<table class="mzhelp"><thead><tr>${head}</tr></thead><tbody>${rows.map(r => `<tr><th scope="row">${r[0]}</th>${r.slice(1).map(c => `<td>${c}</td>`).join("")}</tr>`).join("")}</tbody></table><p class="muted">${I18N.t(`mapnav.h.note.${kind}`)}</p>`);
  }
  // the 2D map of the active tab the palette acts on
  const activeMap = () => { const q = E.active; if (q && q.type === "map" && is2d(q) && E.panels.includes(q)) return q; return (typeof tabPanels === "function" ? tabPanels() : []).find(x => x.type === "map" && is2d(x)) || null; };
  const paletteOk = c => { const p = activeMap(); return !!p && c.kinds.includes("2d") && (!c.vis || c.vis(ad2(p, null))); };
  const paletteRun = id => { const p = activeMap(); if (p) runCmd(id, ad2(p, null)); };
  // touch: one finger moves the map, two fingers move and pinch, a double tap = whole view, a long press = the menu
  function touchInit(p, cv) {
    cv.style.touchAction = "none";
    let g = null, lastTap = null, lp = 0;
    const at = (t, r) => ({ x: t.clientX - r.left, y: t.clientY - r.top });
    const cen = ts => { const r = cv.getBoundingClientRect(), a = at(ts[0], r), b = ts[1] ? at(ts[1], r) : a; return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, d: Math.hypot(a.x - b.x, a.y - b.y), cx: (ts[0].clientX + (ts[1] || ts[0]).clientX) / 2, cy: (ts[0].clientY + (ts[1] || ts[0]).clientY) / 2 }; };
    const inside = q => { const a = p._a; return a && q.x >= M.l && q.x <= a.W - M.r && q.y >= M.t && q.y <= a.H - M.b; };
    cv.addEventListener("touchstart", e => {
      if (!is2d(p)) return; clearTimeout(lp); const c = cen(e.touches);
      if (e.touches.length >= 2) { g = { two: true, last: c, moved: true }; pushZh(p); return; }
      g = { two: false, last: c, start: c, moved: false, t0: Date.now() };
      if (st(p).ex && window.MAPEX && inside(c)) g.box = MAPEX.touch(p, c.x, c.y);       // a finger on the box moves the box, not the map
      lp = setTimeout(() => { if (g && !g.moved) { g.moved = true; st(p).lpAt = Date.now(); showMenu(ad2(p, inside(c) ? { rt: xOf(p, c.x), mz: p._a.mzAt(c.y) } : null), { preventDefault() {}, clientX: c.cx, clientY: c.cy, lp: true }); } }, 550);
    }, { passive: true });
    cv.addEventListener("touchmove", e => {
      if (!g || !is2d(p)) return; const c = cen(e.touches), a = p._a;
      if (!g.moved && Math.hypot(c.x - g.start.x, c.y - g.start.y) < 8) return;
      if (g.box && e.touches.length === 1) { g.moved = true; st(p).gestAt = Date.now(); clearTimeout(lp); e.preventDefault(); g.box(c.x, c.y); return; }
      if (!g.two && !g.moved) pushZh(p);
      g.moved = true; st(p).gestAt = Date.now(); clearTimeout(lp); e.preventDefault();
      if (g.two && e.touches.length >= 2 && g.last.d > 0 && c.d > 0) pinch(p, g.last, c, g.last.d / c.d);
      else panPx(p, c.x - g.last.x, c.y - g.last.y);
      g.last = c; draw(p);
    }, { passive: false });
    const end = e => {
      clearTimeout(lp); if (!g) return; const moved = g.moved, two = g.two; g = null;
      if (e.touches.length) { if (e.cancelable) e.preventDefault(); g = { two: e.touches.length > 1, last: cen(e.touches), start: cen(e.touches), moved: true }; return; }
      if (moved) { st(p).gestAt = Date.now(); if (e.cancelable) e.preventDefault(); uiSave(); return; }       // no mouse events after a gesture (they would be a box zoom)
      const now = Date.now(), c = e.changedTouches[0], r = cv.getBoundingClientRect(), q = at(c, r);
      if (lastTap && now - lastTap.t < 350 && Math.hypot(q.x - lastTap.x, q.y - lastTap.y) < 30) { lastTap = null; if (e.cancelable) e.preventDefault(); st(p).dtap = now; whole(p); navSync(p); return; }
      lastTap = { t: now, x: q.x, y: q.y };
    };
    cv.addEventListener("touchend", end); cv.addEventListener("touchcancel", () => { clearTimeout(lp); g = null; });
  }

  // ---------------------------------------------------------------- buttons in the controls of the map: «⋯» menu, sight, whole view
  function decorate(p, c) {
    if (p.type !== "map") return;
    const hasRef = corrState(p).now[0] === "f", o = diffOpt(hasRef), pal = PREF.pal === "cividis" ? "cividis" : "viridis";
    const opt = (v, cur, lab) => `<option value="${v}" ${cur === v ? "selected" : ""}>${lab}</option>`;
    const col = c.querySelector('[data-mzslot="col"]'), dif = c.querySelector('[data-mzslot="diff"]');
    if (col) col.outerHTML = `<select data-mz="pal" title="${T("mappa.pal.title")}" aria-label="${T("mappa.pal.title")}">${opt("viridis", pal, T("mappa.pal.viridis"))}${opt("cividis", pal, T("mappa.pal.cividis"))}</select>` +
      `<label class="muted" title="${T("mappa.thr.title")}">${T("mappa.thr")} <input data-mz="thr" class="mzf" inputmode="decimal" value="${+(o.thr * 100).toFixed(1)}"> %</label>`;
    if (dif) dif.outerHTML = hasRef ? `<select data-mz="show" title="${T("mappa.show.title")}">${opt("all", o.show, T("mappa.show.all"))}${opt("up", o.show, T("mappa.show.up"))}${opt("down", o.show, T("mappa.show.down"))}</select>` : "";
    const g = document.createElement("span"); g.className = "mzg"; g.setAttribute("role", "group"); g.setAttribute("aria-label", T("ctl.map.grp.tools"));
    g.innerHTML = `<span class="seg mzbt"><button data-mz="mark" class="${st(p).mark ? "on" : ""}" title="${T("mappa.btn.mark.title")}">${T("mappa.btn.mark")}</button><button data-mz="mir" class="${mirOn(p) ? "on" : ""}" title="${T("mappa.btn.mir.title")}">${T("mappa.btn.mir")}</button><button data-mz="menu" title="${T("mappa.btn.menu.title")}" aria-label="${T("mappa.btn.menu.title")}">&#8943;</button></span>` +
      `<button data-o="view" data-v="3d" title="${T("ctl.map.open3d.title")}" aria-label="${T("ctl.map.open3d.title")}">${T("ctl.map.open3d")}</button>`;
    c.appendChild(g);
    const Q = k => c.querySelector(`[data-mz="${k}"]`), num = el => numIt(el.value);
    Q("pal").onchange = ev => { PREF.pal = ev.target.value; prefSave(); draw(p); };
    Q("thr").onchange = ev => { const v = num(ev.target); if (v >= 0 && v < 100) { PREF.thr = v / 100; prefSave(); } draw(p); };
    if (hasRef) Q("show").onchange = ev => { PREF.show = ev.target.value; prefSave(); draw(p); };
    const dc = c.querySelector("details.mzcol"); if (dc) dc.ontoggle = () => { p._colOpen = dc.open; };
    Q("mark").onclick = () => { const s = st(p); s.mark = !s.mark; markBtn(p); };
    Q("mir").onclick = ev => { const s = st(p); s.mir = !mirOn(p); ev.currentTarget.classList.toggle("on", s.mir); layout(p); draw(p); };
    Q("menu").onclick = ev => { ev.stopPropagation(); menuAt(p, ev.currentTarget); };     // the click must not reach the document (it closes #ctx)
  }

  function markBtn(p) { const b = p.el.querySelector('[data-mz="mark"]'); if (b) b.classList.toggle("on", !!st(p).mark); p.cv.style.cursor = st(p).mark ? "copy" : ""; }

    // «Open in 3D»: the viewer is loaded the first time it is asked for
  // what the student sees when the 3D viewer cannot open: the real cause in a sentence, what to try, and the technical lines (copyable); the 2D map is untouched
  function failure3d(kind, detail, diag) {
    const k = ["nowebgl2", "shader", "import", "lost"].includes(kind) ? kind : "error", d = { kind, detail, ...diag }, esc = x => EH(String(x == null ? "" : x));
    const tech = Object.entries(d).filter(([, v]) => v !== undefined && v !== "").map(([a, v]) => `${a}: ${typeof v === "object" ? JSON.stringify(v) : v}`).join("\n");
    document.querySelectorAll("dialog.m3d").forEach(x => x.remove());            // a half-built viewer must not stay on the page
    big(T("m3d.err.title"), `<p>${I18N.t(`m3d.err.${k}`)}</p>${k === "nowebgl2" ? `<p>${T("m3d.err.how")}</p>` : ""}<p>${T("m3d.err.2d")}</p><pre class="m3d-tech" tabindex="0" aria-label="${T("m3d.err.tech")}">${esc(tech)}</pre>`);
  }
  async function open3d(p, opener) {
    let mod; try { mod = await import("./mappa3d.js"); } catch (e) { console.error(e); return failure3d("import", String(e && e.message || e), {}); }
    try { mod.open(p, opener); } catch (e) {
      const k = e && e.kind ? e.kind : "error"; console.error("mzLab 3D:", k, e && e.detail || e, e && e.diag || "");
      failure3d(k, e && e.detail != null ? e.detail : String(e && e.message || e), e && e.diag || {});
    }
  }

  return { open3d, addPointAt, spectrumAt, init, zoomImg, after, down, click, diffOpt, diffMap, pts, delPoint, move, leave, wheel, boxZoom, ctx, decorate, key, whole, prefs: PREF, prefSave, point, st, valueAt, CMDS, ICO, SV, corrState, corrInit, setCorr, corrQ, corrPick, optChips, explain, grow, redraw: frame, layoutNow: layout, touched, barFor, lockBtn, ptText, ad2, itemsFor, showMenu, runCmd, help, paletteOk, paletteRun, extract, peakStep, apexStep, madNoise, dropFile };
})();

// ---- panels: grips on the top and bottom edge (mouse, touch and pen), double click = default height; the width is always the column's
(() => {
  const H0 = 300, HMIN = 190, h0 = p => p.type === "map" ? 360 : H0;
  const css = document.createElement("style");
  css.textContent = "#dpanels .pnl{resize:none;max-width:100%}.pnl .pgrip{position:absolute;left:12px;right:12px;height:10px;z-index:5;cursor:ns-resize;touch-action:none}" +
    ".pnl .pgrip.t{top:-3px}.pnl .pgrip.b{bottom:-3px}.pnl .pgrip::after{content:'';position:absolute;left:50%;top:3px;width:34px;height:4px;margin-left:-17px;border-radius:2px;background:var(--line);opacity:0;transition:opacity .15s}" +
    ".pnl:hover .pgrip::after,.pnl .pgrip:focus-visible::after,.pnl .pgrip.on::after{opacity:1}.pnl .pgrip:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.pnl .pgrip{height:24px}.pnl .pgrip.t{top:-11px}.pnl .pgrip.b{bottom:-11px}.pnl .pgrip::after{opacity:.8;top:10px}}" +
    "@media (prefers-reduced-motion:reduce){.pnl .pgrip::after{transition:none}}";
  document.head.appendChild(css);
  const host = () => document.getElementById("dpanels");
  const pOf = el => E.panels.find(q => q.el === el);
  const simple = p => !(window.DDA && (DDA.docked(p) || E.panels.some(q => q.dock === p.id && DDA.docked(q))));
  const above = p => { let best = null; for (const q of E.panels) if (q !== p && q.full && simple(q) && q.y < p.y && (!best || q.y > best.y)) best = q; return best; };
  const done = p => { if (p.full && typeof relayout === "function") relayout(); if (typeof fitHost === "function") fitHost(); if (typeof uiSave === "function") uiSave(); };
  function setH(p, h) { p.h = Math.max(HMIN, Math.round(h)); apply(p); if (p.el) p.el.querySelectorAll(".pgrip").forEach(g => g.setAttribute("aria-valuenow", String(p.h))); }
  function grip(el, side) {
    const g = document.createElement("div"); g.className = "pgrip " + side; g.tabIndex = 0; g.setAttribute("role", "separator"); g.setAttribute("aria-orientation", "horizontal");
    { const p0 = pOf(el); g.setAttribute("aria-valuemin", String(HMIN)); g.setAttribute("aria-valuemax", "2000"); g.setAttribute("aria-valuenow", String(p0 ? p0.h : el.offsetHeight)); }          // a focusable separator needs its value (the height of the panel)
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
    g.addEventListener("dblclick", ev => { const p = pOf(el); if (!p) return; ev.stopPropagation(); if (side === "t" && !p.full) { const b = p.y + p.h; setH(p, h0(p)); p.y = Math.max(0, b - p.h); apply(p); } else setH(p, h0(p)); done(p); });
    g.addEventListener("keydown", ev => {
      const p = pOf(el); if (!p || !["ArrowUp", "ArrowDown", "Home"].includes(ev.key)) return;
      ev.preventDefault(); ev.stopPropagation(); const k = ev.shiftKey ? 60 : 20;
      if (ev.key === "Home") setH(p, h0(p)); else setH(p, p.h + (ev.key === "ArrowDown" ? k : -k));
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
