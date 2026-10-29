// @ts-strict
// @ts-noany
// RT x m/z map: true zoom, sight (crosshair) with the two plots at the margins, lock, keyboard, in-page expand (classic script, loaded after explore.js).
// The map itself (colours, difference, 3D) is drawn by drawMap in explore.js; this file adds what lives on top of it. Hooks in explore.js:
// drawMap (zoomImg before the image is drawn, after at the end), attach (wheel, box zoom, click, right click, mouse move), ctl (decorate).
// Preferences of this browser in localStorage "qqq.mappa".
window.MAPPA = (() => {
  /** @typedef {{ preventDefault: () => void, clientX: number, clientY: number, kbd?: boolean, lp?: boolean }} MenuEv  a (real or synthetic) event that opens the context menu */
  /** @param {string} k @param {Record<string, any>} [o] */
  const T = (k, o) => I18N.t(k, o);
  // grafici.js clampView returns number[]; the zoom of a panel is a pair
  const clampV = (/** @type {number} */ z0, /** @type {number} */ z1, /** @type {number} */ f0, /** @type {number} */ f1) => /** @type {[number, number]} */ (clampView(z0, z1, f0, f1));
  const PREF_KEY = NOMI.KEY("mappa");
  const PREF = (() => { return lsObj(PREF_KEY); })();
  const prefSave = () => { try { localStorage.setItem(PREF_KEY, JSON.stringify(PREF)); } catch (e) { /* no storage */ } };
  const GRP0 = 0.02, SHAPES = ["circle", "square", "triangle", "diamond"], GLYPH = ["●", "■", "▲", "◆"];
  // literal keys: the i18n check finds them     // RT window of a group (min); the shape of a group's points, by turns
  const SHAPE_KEY = { circle: "mappa.shape.circle", square: "mappa.shape.square", triangle: "mappa.shape.triangle", diamond: "mappa.shape.diamond" };
  /** @type {Record<string, { lab: string, limit: number, bands: number, quota: number }>} */ const DENS = { few: { lab: "mappa.dens.few", limit: 4, bands: 2, quota: 1 }, mid: { lab: "mappa.dens.mid", limit: 8, bands: 4, quota: 1 }, many: { lab: "mappa.dens.many", limit: 14, bands: 6, quota: 2 } }, BOTTOM = 140, NARROW = 700, QUIET = 150;
  const st = (/** @type {Pannello} */ p) => p._mz || (p._mz = { lock: null, kp: null, hov: null, mir: null, reg: null, regT: 0, spec: null, xic: null, xicB: null });
  const is2d = (/** @type {Pannello} */ p) => p.type === "map" && p._a && p._a.map;
  const isMax = (/** @type {Pannello} */ p) => p.el && p.el.classList.contains("max");
  // «How it is extracted» always uses the two plots at the margins
  const mirOn = (/** @type {Pannello} */ p) => { const s = st(p); return s.ex ? true : s.mir != null ? s.mir : isMax(p); };
  const wide = (/** @type {Pannello} */ p) => p.el && p.el.clientWidth >= NARROW;
  // frz: the point of the right click, held while the menu is open
  const point = (/** @type {Pannello} */ p) => { const s = st(p); return s.lock || s.frz || s.kp || s.hov; };
  const isHrF = (/** @type {MzFile} */ f) => !!(f && window.HR && HR.isHr(f, f.lv || 1));

  // the first time a 2D map is opened, the 3D viewer is fetched when the browser is idle (same guards as the other preloads in shell.js): the switch to 3D is then immediate
  let pre3d = false;
  function preload3d() {
    if (pre3d) return; pre3d = true;
    const c = /** @type {any} */ (navigator).connection || {};
    if (c.saveData || /2g|3g/.test(c.effectiveType || "")) return;
    const go = () => { const l = document.createElement("link"); l.rel = "modulepreload"; l.href = "static/mappa3d.js"; document.head.appendChild(l); };
    /** @type {any} */ (window).requestIdleCallback ? /** @type {any} */ (window).requestIdleCallback(go, { timeout: 8000 }) : setTimeout(go, 2000);
  }

  // ---------------------------------------------------------------- DOM: grid with the map, the overlay (crosshair) and the two margin plots
  function init(/** @type {Pannello} */ p) {
    if (p._mzInit || !p.cv) return; p._mzInit = true;
    preload3d(); /** @type {any} */ (window).MAPPA_WARM?.();
    const cv = /** @type {HTMLCanvasElement} */ (p.cv), grid = document.createElement("div");
    grid.className = "mzgrid"; cv.before(grid); grid.appendChild(cv);
    const ov = document.createElement("canvas"); ov.className = "mzov"; ov.setAttribute("aria-hidden", "true"); grid.appendChild(ov);
    const sp = document.createElement("canvas"); sp.className = "mzside"; sp.hidden = true; sp.setAttribute("role", "img"); grid.appendChild(sp);
    const xc = document.createElement("canvas"); xc.className = "mzbot"; xc.hidden = true; grid.appendChild(xc);
    Object.assign(st(p), { ov, sp, xc, grid });
    // the tooltip of a label: its m/z and its S/N (intensity over the noise around it)
    sp.addEventListener("mousemove", ev => {
      const r = sp.getBoundingClientRect(), x = ev.clientX - r.left, y = ev.clientY - r.top, s = st(p), l = (s.lbls || []).find((/** @type {Record<string, any>} */ o) => x >= o.x - 2 && x <= o.x + o.w + 2 && y >= o.y - 2 && y <= o.y + o.h + 2), q = l && s.snr ? s.snr.get(l.i) : null;
      sp.title = q != null ? T("mappa.lbl.tip", { mz: l.t, snr: q >= 100 ? Math.round(q) : q.toFixed(1) }) : "";
    });
    cv.tabIndex = 0; cv.setAttribute("aria-label", T("mappa.aria"));
    cv.addEventListener("keydown", e => key(p, e));
    document.addEventListener("keyup", e => { if (p.cv.isConnected) shiftUp(p, e); }, { signal: p._ac.signal });
    p.el.addEventListener("keydown", e => { if (isMax(p) && e.target === p.el) key(p, e); });
    const tbs = p.el.querySelector(".tbs"); tbs.insertBefore(navBar(p), tbs.children[1] || null); navSync(p); touchInit(p, cv);
    const ro = FRAME.ro(cv, () => { if (mirOn(p)) { overlay(p); margins(p); } });
    // the margin plots follow their own box too: it changes without the map's (the panel grows by the XIC, a hidden canvas is shown)
    const roMargins = MAPMARGINI.watch([sp, xc], () => { if (mirOn(p)) margins(p); });
    st(p).ro = p._ro = () => { ro(); roMargins(); };
    const wrap = () => {
      if (!p._setMax || p._setMax._mz) return;
      const orig = p._setMax, f = (/** @type {boolean} */ on) => { orig(on); onMax(p, on); }; f._mz = true; p._setMax = f;
    };
    wrap(); setTimeout(wrap, 0);
  }
  function onMax(/** @type {Pannello} */ p, /** @type {boolean} */ on) {
    // the sight follows the view again: off in the page, on when expanded; the expand itself is the shared in-page one (pannelli.js `_setMax`)
    st(p).mir = null;
    layout(p);
    if (on) setTimeout(() => p.cv.focus({ preventScroll: true }), 0);
  }
  // the button shows the real state of the sight (on when expanded, off in the page, until it is clicked or M is pressed)
  function mirBtn(/** @type {Pannello} */ p) {
    const b = p.el && p.el.querySelector('[data-mz="mir"]'); if (!b) return;
    const on = !!mirOn(p); b.classList.toggle("on", on); b.setAttribute("aria-pressed", String(on));
  }
  function layout(/** @type {Pannello} */ p) {
    mirBtn(p); const s = st(p); if (!s.grid) return;
    const m = mirOn(p) && is2dView(p) && wide(p);
    s.grid.classList.toggle("mir", m); s.sp.hidden = !m; s.xc.hidden = !m;
    p.el.classList.toggle("mzmir", m);
    // in the page the panel grows by the height of the XIC, and shrinks back after
    const grow = m && !isMax(p);
    if (grow !== !!s.grown) { p.h = Math.max(200, p.h + (grow ? 1 : -1) * (BOTTOM + 80)); s.grown = grow; apply(p); if (typeof relayout === "function") relayout(); fitHost(); uiSave(); }
  }
  const is2dView = (/** @type {Pannello} */ p) => p.type === "map";

  // ---------------------------------------------------------------- the true zoom: the region in view on new bins, asked 150 ms after the last change
  function normFactor(/** @type {ArrayLike<number>} */ m, /** @type {string} */ norm) {
    if (norm !== "max" && norm !== "tic") return 1;
    let t = 0; for (let i = 0; i < m.length; i++) t = norm === "max" ? Math.max(t, m[i]) : t + m[i];
    return t > 0 ? (norm === "max" ? 1 : 1000) / t : 1;
  }
  // ---------------------------------------------------------------- the difference (extends the one of explore.js): computed by the server in numpy
  // with a tolerance in RT (the reference is replaced by its local maximum over +-0.1 min: background lines and small drifts cancel); what it shows
  // (all | increases only | decreases only) and the noise threshold (fraction of the 99.5th percentile) are choices of this browser
  const diffOpt = (diff = true) => ({ show: PREF.neg === false ? "up" : "all", thr: PREF.thr ?? (diff ? 0.02 : 0.004), rttol: PREF.rttol ?? 0.1, fix: !!PREF.fix });
  const diffMap = (/** @type {number} */ k, /** @type {number | null} */ ref, /** @type {number} */ lv, /** @type {string} */ norm) => memo(`mzd${k}|${ref}|${lv}|${norm}|${diffOpt().rttol}`, () => J(`api/map?k=${k}&level=${lv}&ref=${ref}&norm=${norm}&rttol=${diffOpt().rttol}`).then((/** @type {Record<string, any>} */ j) => {
    return MAPDATI.b64f32(j.data);
  }));
  // regions held by the page: the oldest ones leave the cache of explore.js when they pass a byte budget (smaller on small devices)
  const REG = { map: new Map(), bytes: 0, max: ((/** @type {any} */ (navigator)).deviceMemory || 4) <= 2 ? 48e6 : 96e6, ms: 0 };
  function regionKeep(/** @type {string} */ key, /** @type {number} */ bytes) {
    REG.map.set(key, bytes); REG.bytes += bytes;
    for (const [k, b] of REG.map) { if (REG.bytes <= REG.max || k === key) break; REG.map.delete(k); REG.bytes -= b; CACHE.delete(k); }
  }
  const getRegion = (/** @type {number} */ k, /** @type {number} */ lv, /** @type {number[]} */ r, extra = "") => {
    const key = `mzr${k}|${lv}|${r.join("|")}${extra}`, t0 = performance.now();
    return memo(key, () => J(`api/map?k=${k}&level=${lv}&rt0=${r[0]}&rt1=${r[1]}&mz0=${r[2]}&mz1=${r[3]}&nrt=${r[4]}&nmz=${r[5]}${extra}`).then((/** @type {Record<string, any>} */ j) => {
      const m = MAPDATI.b64f32(j.data);
      REG.ms = performance.now() - t0; regionKeep(key, m.byteLength);
      return { ...j, m };
    }));
  };
  // a fast route is asked soon after the last change, a slow one only after the pause
  const quiet = () => REG.ms && REG.ms < 30 ? 40 : QUIET;
  function regionGrid(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ A, /** @type {number} */ x0, /** @type {number} */ x1, /** @type {number} */ y0, /** @type {number} */ y1, /** @type {number} */ pw, /** @type {number} */ ph) {
    const f = E.files[A.k], hr = isHrF(f), step = Math.max(scanStep(), 1e-4);
    // never more bins than scans: no empty stripes
    const d = Math.min(2, devicePixelRatio || 1), nrt = Math.max(2, Math.min(1000, Math.max(Math.round(pw * d), Math.ceil((x1 - x0) / (A.rt1 - A.rt0) * A.nrt)), Math.floor((x1 - x0) / step)));
    // never coarser than the whole map; low resolution: bins of at least 0.1
    const nmz = Math.max(2, Math.min(1000, Math.max(Math.round(ph * d), Math.ceil((y1 - y0) / A.dmz)), hr ? 1000 : Math.floor((y1 - y0) / 0.1)));
    return [+x0.toFixed(5), +x1.toFixed(5), +y0.toFixed(5), +y1.toFixed(5), nrt, nmz];
  }
  // The region is asked with a margin (a quarter of the view on each side): a pan or a small zoom inside it is already sharp and asks nothing.
  // SLACK must exceed 1 + 2 * MARGIN: at strong zoom the bins are capped by the scan step, so the wide region has the view's bin count and is exactly that much coarser
  const MARGIN = 0.25, SLACK = 1 + 2 * MARGIN + 0.05;
  function regionWide(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ A, /** @type {number} */ x0, /** @type {number} */ x1, /** @type {number} */ y0, /** @type {number} */ y1, /** @type {number} */ pw, /** @type {number} */ ph) {
    const dx = (x1 - x0) * MARGIN, dy = (y1 - y0) * MARGIN, k = 1 + 2 * MARGIN;
    return regionGrid(p, A, Math.max(A.rt0, x0 - dx), Math.min(A.rt1, x1 + dx), Math.max(A.mz0, y0 - dy), Math.min(A.mz0 + A.nmz * A.dmz, y1 + dy), pw * k, ph * k);
  }
  // the region held covers the view with bins at most SLACK times coarser than the view needs
  const regionFits = (/** @type {Record<string, any>} */ R, /** @type {unknown} */ pk, /** @type {number[]} */ r, /** @type {number} */ x0, /** @type {number} */ x1, /** @type {number} */ y0, /** @type {number} */ y1) => R && R.pk === pk && R.off && x0 >= R.rt0 - 1e-9 && x1 <= R.rt1 + 1e-9 && y0 >= R.mz0 - 1e-9 && y1 <= R.mz1 + 1e-9
    && (R.rt1 - R.rt0) / R.nrt <= (x1 - x0) / r[4] * SLACK && R.dmz <= (y1 - y0) / r[5] * SLACK;
  // called by drawMap before the image is drawn: true = the region image was drawn (otherwise the whole-map image is stretched, as before)
  function zoomImg(/** @type {Pannello} */ p, /** @type {CanvasRenderingContext2D} */ g, /** @type {Record<string, any>} */ o) {
    const s = st(p); init(p);
    if (!p.zoom && !p.zoomY) { s.reg = null; return false; }
    const { A, B, im, x0, x1, y0, y1, pw, ph, norm } = o;
    const r = regionGrid(p, A, x0, x1, y0, y1, pw, ph), w = regionWide(p, A, x0, x1, y0, y1, pw, ph);
    const pk = `${A.k}|${B ? B.k : ""}|${A.level}|${norm}|${liscia() ? "s" : "d"}|${p.scale}${p.scale === "gamma" ? p.gamma || 0.6 : ""}|${mapPal()}|${mapBg(p)}|${JSON.stringify(diffOpt(!!B))}`;
    const R = s.reg;
    const put = (/** @type {Record<string, any>} */ R) => { g.imageSmoothingEnabled = liscia(); g.drawImage(R.off, (x0 - R.rt0) / (R.rt1 - R.rt0) * R.nrt, (R.mz1 - y1) / R.dmz, (x1 - x0) / (R.rt1 - R.rt0) * R.nrt, (y1 - y0) / R.dmz, M.l, M.t, pw, ph); return true; };
    if (regionFits(R, pk, r, x0, x1, y0, y1)) return put(R);
    clearTimeout(s.regT); const seq = s.regSeq = (s.regSeq || 0) + 1;
    s.regT = setTimeout(async () => {
      try {
        const a = await getRegion(A.k, A.level, w, (B ? `&ref=${B.k}&norm=${norm}&rttol=${diffOpt().rttol}` : "") + mapBg(p));
        // a newer view was asked meanwhile: this one is not painted
        if (seq !== s.regSeq) return;
        const fa = B ? 1 : normFactor(A.m, norm), n = a.m.length, v = new Float32Array(n);
        // with a reference the server already gives A - B (normalised, tolerance in RT)
        for (let i = 0; i < n; i++) v[i] = a.m[i] * fa;
        s.reg = { pk, v, nrt: a.nrt, nmz: a.nmz, rt0: a.rt0, rt1: a.rt1, mz0: a.mz0, mz1: a.mz1, dmz: a.dmz, off: paint(liscia() ? splatOf(A, v, a) : v, a.nrt, a.nmz, im, !!B) };
        draw(p);
      } catch (e) { /* keep the stretched whole map */ }
    }, quiet());
    // meanwhile a coarser region that covers the view is better than the stretched map
    return !!(R && R.pk === pk && R.off && R.rt0 <= x0 + 1e-9 && R.rt1 >= x1 - 1e-9 && R.mz0 <= y0 + 1e-9 && R.mz1 >= y1 - 1e-9) && put(R);
  }
  // «Liscia» (the default) or «Dettaglio»: only how the zoomed region is drawn; the values the sight and the XIC read are the ones of the matrix
  const liscia = () => PREF.rend !== "detail", HR_FWHM_PPM = 15, LR_FWHM = 0.7, FWHM = 2.3548;
  // a gaussian as wide as the m/z resolution of the instrument and the spacing of the scans
  function splatOf(/** @type {Record<string, any>} */ A, /** @type {ArrayLike<number>} */ v, /** @type {Record<string, any>} */ a) {
    const hr = isHrF(E.files[A.k]), mid = (a.mz0 + a.mz1) / 2, wm = hr ? mid * HR_FWHM_PPM * 1e-6 : LR_FWHM;
    return MAPREND.splat(v, a.nrt, a.nmz, scanStep() / ((a.rt1 - a.rt0) / a.nrt), wm / FWHM / a.dmz);
  }
  function paint(/** @type {ArrayLike<number>} */ v, /** @type {number} */ nrt, /** @type {number} */ nmz, /** @type {{ T: (x: number) => number }} */ im, /** @type {boolean} */ diff) {
    const off = document.createElement("canvas"); off.width = nrt; off.height = nmz;
    const ctx = /** @type {CanvasRenderingContext2D} */ (off.getContext("2d")), d = ctx.createImageData(nrt, nmz), tab = diff ? LUT_DIV : lutSeq(mapPal());
    // one 32-bit write per pixel, the same bytes as the table
    const px = new Uint32Array(d.data.buffer), tab32 = new Uint32Array(tab.buffer, tab.byteOffset, 256);
    for (let i = 0; i < nrt; i++) for (let j = 0; j < nmz; j++) {
      const x = v[i * nmz + j], t = im.T(x), q = diff ? Math.round(127.5 + Math.sign(-x) * t * 127.5) : Math.round(t * 255);
      px[(nmz - 1 - j) * nrt + i] = tab32[q];
    }
    ctx.putImageData(d, 0, 0); return off;
  }
  // value of the map at (rt, m/z): from the zoomed region when there is one, otherwise from the whole map
  function valueAt(/** @type {Pannello} */ p, /** @type {number} */ rt, /** @type {number} */ mz) {
    const s = st(p), a = p._a; if (!a || !a.mapv) return null;
    const R = s.reg && p.zoom || s.reg && p.zoomY ? s.reg : null;
    if (R) { const i = Math.floor((rt - R.rt0) / (R.rt1 - R.rt0) * R.nrt), j = Math.floor((mz - R.mz0) / R.dmz); if (i >= 0 && i < R.nrt && j >= 0 && j < R.nmz) return R.v[i * R.nmz + j]; }
    return a.mapv(rt, mz);
  }
  const binDa = (/** @type {Pannello} */ p) => { const s = st(p), a = p._a; if (s.reg && (p.zoom || p.zoomY)) return s.reg.dmz; return a && a.dmz ? a.dmz : 1; };

  // ---------------------------------------------------------------- after the map is drawn: overlay and margins
  function after(/** @type {Pannello} */ p) {
    init(p); layout(p);
    const s = st(p), a = p._a; if (!a || !s.ov) return;
    const r = p.cv.getBoundingClientRect(), d = Math.min(4, devicePixelRatio || 1);
    s.ov.width = Math.round(r.width * d); s.ov.height = Math.round(r.height * d); s.ov.style.width = r.width + "px"; s.ov.style.height = r.height + "px";
    s.ovg = s.ov.getContext("2d"); s.ovg.setTransform(d, 0, 0, d, 0, 0);
    overlay(p); margins(p); table(p);
  }
  function overlay(/** @type {Pannello} */ p) {
    const s = st(p), g = s.ovg, a = p._a; if (!g || !a) return;
    g.clearRect(0, 0, a.W, a.H);
    if (!is2d(p)) return;
    drawPoints(p, g);
    // axis lock (asse.js): the line along which the gesture may move
    if (s.guide) {
      const q = s.guide; g.save(); g.strokeStyle = ecss("--accent"); g.lineWidth = 1.5; g.setLineDash([6, 4]); g.beginPath();
      if (q.ax === "x") { g.moveTo(M.l, q.y + .5); g.lineTo(a.W - M.r, q.y + .5); } else { g.moveTo(q.x + .5, M.t); g.lineTo(q.x + .5, a.H - M.b); }
      g.stroke(); g.restore();
    }
    if (s.ex && window.MAPEX) MAPEX.overlay(p, g);
    if (s.feat && window.MAPEX) MAPEX.featOverlay(p, g);
    const pt = point(p), on = mirOn(p) && !s.ex; s.dot = null;
    if (pt && (on || s.lock) && pt.rt >= a.x0 && pt.rt <= a.x1 && pt.mz >= a.y0 && pt.mz <= a.y1) {
      const x = Math.round(a.X(pt.rt)) + .5, y = Math.round(a.Y(pt.mz)) + .5;
      g.save(); g.strokeStyle = ecss("--ink"); g.globalAlpha = s.lock ? .9 : .6; g.lineWidth = 1; if (!s.lock) g.setLineDash([4, 3]);
      g.beginPath(); g.moveTo(M.l, y); g.lineTo(a.W - M.r, y); g.moveTo(x, M.t); g.lineTo(x, a.H - M.b); g.stroke(); g.setLineDash([]);
      if (s.lock) lockDot(g, a.X(pt.rt), a.Y(pt.mz), s);                  // the locked point: a small dot exactly on it
      g.restore();
    }
    readout(p); navSync(p);
  }
  // the mark of a locked point: a dot with a dark outline and an accent fill, centred on (x, y); s.dot keeps the centre for the tests
  function lockDot(/** @type {CanvasRenderingContext2D} */ g, /** @type {number} */ x, /** @type {number} */ y, /** @type {Record<string, any>} */ s) {
    g.globalAlpha = 1; g.beginPath(); g.arc(x, y, 4, 0, 2 * Math.PI); g.fillStyle = ecss("--accent"); g.fill(); g.lineWidth = 1.6; g.strokeStyle = ecss("--ink"); g.stroke(); s.dot = { x, y };
  }
  function readout(/** @type {Pannello} */ p) {
    // Map 2D has no header readout text to prevent 2-row wrapping
  }

  // ---------------------------------------------------------------- the two plots at the margins: spectrum of the scan (m/z aligned with the map) and XIC (RT aligned)
  // Asked only for what the point needs and the page does not hold: the scan is found in the page on the RT axis of the file (/api/scan_axis, once), the
  // spectrum comes from /api/spectra by position and only when the scan changes, the XIC only when the window changes. One request at a time, the newest
  // point wins (MIRINO.latest, at most once per frame); what has not arrived yet is a preview read from the matrix of the map, marked as such.
  // spectra and XIC under a memory budget; RT axes; requests in flight
  const SIGHT = MIRINO.lru(MIRINO.budget()), AX = new Map(), FLY = new Map();
  // same moments as the cache of explore.js
  { const clear = CACHE.clear.bind(CACHE); CACHE.clear = () => { clear(); SIGHT.clear(); AX.clear(); FLY.clear(); }; }
  const dropFile = (/** @type {number} */ k) => { SIGHT.drop(`s${k}|`); SIGHT.drop(`x${k}|`); SIGHT.drop(`x${k}+`); SIGHT.dropIf((/** @type {string} */ key) => key[0] === "s" ? key.includes(`-${k}|`) : key.includes(`+${k}|`)); for (const key of [...AX.keys()]) if (key.startsWith(`${k}|`)) AX.delete(key); };
  // with ?perf: the cost of the exact answer of each stop of the sight (tools/bench_mirino.py)
  const MEAS = /[?&]perf\b/.test(location.search), meas = /** @type {Record<string, number>[]} */ (window.__mir = []);
  const once = (/** @type {string} */ key, /** @type {() => Promise<any>} */ fn) => { if (!FLY.has(key)) FLY.set(key, fn().finally(() => FLY.delete(key))); return FLY.get(key); };
  // {rt, sid} of the scans of the level; null until it has arrived
  const axisOf = (/** @type {MzFile} */ f) => {
    const key = `${f.k}|${f.lv || 1}`, e = AX.get(key); if (e) return e.v;
    const q = /** @type {any} */ ({ v: null }); AX.set(key, q);
    once("a" + key, () => J(`api/scan_axis?k=${f.k}&level=${f.lv || 1}`)).then((/** @type {Record<string, any>} */ j) => { q.v = { rt: Float64Array.from(j.rt), sid: j.sid }; ask(null); }, () => AX.delete(key));
    return null;
  };
  const axisWait = async (/** @type {MzFile} */ f) => { const key = `${f.k}|${f.lv || 1}`; if (!AX.has(key)) axisOf(f); try { await FLY.get("a" + key); } catch (e) { /* no axis */ } return (AX.get(key) || {}).v || null; };
  const specKey = (/** @type {MzFile} */ f, /** @type {Record<string, any>} */ ax, /** @type {number} */ i) => `s${f.k}|${f.lv || 1}|${ax.sid[i]}`;
  const xicKey = (/** @type {MzFile} */ f, /** @type {Record<string, any>} */ a, /** @type {number} */ mz) => `x${f.k}${a.ref ? "+" + a.ref.k : ""}|${xicCentre(f, mz).toFixed(4)}|${xicTol(f, mz).toFixed(5)}|${f.lv || 1}`;
  const specSize = (/** @type {Record<string, any>} */ sp) => (sp.mz.length + sp.y.length + (sp.pmz ? sp.pmz.length + sp.py.length : 0)) * 8 + 200, xicSize = (/** @type {Record<string, any>} */ xi) => (xi.tr || []).reduce((/** @type {number} */ n, /** @type {Record<string, any>} */ t) => n + (t.rt.length + t.y.length) * 8 + 100, 200);
  // what is already in the page for the point: the exact spectrum / XIC, or null
  // the scan of A (and of B, in a difference) nearest to the point: {f, ax, i, r, rx, j} or null while an RT axis is not in the page
  function scansAt(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ pt) {
    const a = p._a, f = a && a.f; if (!f || !pt) return null;
    const ax = axisOf(f), r = a.ref || null, rx = r ? axisOf(r) : null; if (!ax || (r && !rx)) return null;
    const i = MIRINO.nearest(ax.rt, pt.rt), j = rx ? MIRINO.nearest(rx.rt, pt.rt) : -1; if (i < 0 || (r && j < 0)) return null;
    return { f, ax, i, r, rx, j };
  }
  const specKeyAt = (/** @type {Record<string, any>} */ c) => specKey(c.f, c.ax, c.i) + (c.r ? `-${c.r.k}|${c.rx.sid[c.j]}` : "");
  function specFor(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ pt) {
    const s = st(p), c = scansAt(p, pt); if (!c) return null;
    const key = specKeyAt(c); if (s.spec && s.spec.key === key) return s.spec;
    const sp = SIGHT.get(key); if (sp) s.spec = sp; return sp || null;
  }
  function xicFor(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ pt) {
    const s = st(p), a = p._a, f = a && a.f; if (!f || !pt) return null;
    const key = xicKey(f, a, pt.mz); if (s.xic && s.xic.key === key) return s.xic;
    const xi = SIGHT.get(key); if (xi) s.xic = xi; return xi || null;
  }
  async function fetchXic(/** @type {Pannello} */ p, /** @type {number} */ mz) {
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
  // one scan by position: the same arrays as /api/spectrum on a window holding only that scan
  const getScan = (/** @type {MzFile} */ f, /** @type {Record<string, any>} */ ax, /** @type {number} */ i) => {
    const key = specKey(f, ax, i), hit = SIGHT.get(key); if (hit) return Promise.resolve(hit);
    return once(key, async () => {
      const j = await J(`api/spectra?k=${f.k}&i0=${i}&i1=${i}&level=${f.lv || 1}&bin=0.1${MERGE()}`), n = j.scans && j.scans[0];
      if (!n) throw new Error("no scan");
      const r = { key, sid: n.sid, rt: n.rt, mz: n.mz, y: n.y, pmz: n.pmz, py: n.py }; SIGHT.set(key, r, specSize(r)); return r;
    });
  };
  async function fetchSpec(/** @type {Pannello} */ p, /** @type {number} */ rt) {
    const s = st(p), a = p._a; if (!a) return null; const f = a.f, r = a.ref || null;
    try {
      const ax = await axisWait(f), rx = r ? await axisWait(r) : null, i = ax ? MIRINO.nearest(ax.rt, rt) : -1, j = rx ? MIRINO.nearest(rx.rt, rt) : -1;
      if (i < 0 || (r && j < 0)) return s.spec;
      const key = specKeyAt({ f, ax, i, r, rx, j }); if (s.spec && s.spec.key === key) return s.spec;
      const hit = SIGHT.get(key); if (hit) return (s.spec = hit);
      if (!r) { s.spec = await getScan(f, ax, i); return s.spec; }
      // A − B over every m/z: peaks of A and B closer than the XIC window are one
      const [sa, sb] = await Promise.all([getScan(f, ax, i), getScan(r, rx, j)]);
      const d = MIRINO.diffPeaks(sa.mz, sa.y, sb.mz, sb.y, (/** @type {number} */ mz) => xicTol(f, mz));
      const sp = { key, sid: sa.sid, rt: sa.rt, rtb: sb.rt, mz: d.mz, y: d.y, signed: true }; SIGHT.set(key, sp, specSize(sp)); s.spec = sp;
    } catch (e) { /* nothing to show */ }
    return s.spec;
  }
  // after the exact answer, the scans next to it in the direction of the movement are asked in the background (one request, only when nothing else is waiting)
  function prefetch(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ pt) {
    const s = st(p), a = p._a, f = a && a.f, ax = f && axisOf(f); if (!ax || !pt || ask.pending()) return;
    const i = MIRINO.nearest(ax.rt, pt.rt), dir = s.dir || 1, want = [i + dir, i + 2 * dir].filter(j => j >= 0 && j < ax.rt.length && !SIGHT.get(specKey(f, ax, j)));
    if (!want.length) return;
    const lo = Math.min(...want), hi = Math.max(...want), lv = f.lv || 1;
    (window.requestIdleCallback || (g => setTimeout(g, 120)))(() => {
      if (ask.pending() || p._a !== a) return;
      J(`api/spectra?k=${f.k}&i0=${lo}&i1=${hi}&level=${lv}&bin=0.1${MERGE()}`).then((/** @type {Record<string, any>} */ j) => {
        for (const n of j.scans || []) { const key = specKey(f, ax, n.i); if (!SIGHT.get(key)) { const r = { key, sid: n.sid, rt: n.rt, mz: n.mz, y: n.y, pmz: n.pmz, py: n.py }; SIGHT.set(key, r, specSize(r)); } }
      }).catch(() => { /* a prefetch is never needed */ });
    });
  }
  // preview: the row / column of the matrix of the map (the region on finer bins when the zoom has one) at the point; null for a difference map
  function preview(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ pt) {
    const s = st(p), a = p._a, R = s.reg && (p.zoom || p.zoomY) ? s.reg : null, m = R ? { v: R.v, nrt: R.nrt, nmz: R.nmz, rt0: R.rt0, rt1: R.rt1, mz0: R.mz0, dmz: R.dmz } : a && a.mat;
    if (!m || !pt) return null;
    const i = Math.floor((pt.rt - m.rt0) / (m.rt1 - m.rt0) * m.nrt), j = Math.floor((pt.mz - m.mz0) / m.dmz);
    const out = {};
    if (i >= 0 && i < m.nrt) { const c = MIRINO.column(m.v, m.nmz, i, m.mz0, m.dmz); out.spec = { mz: c.x, y: c.y, rt: pt.rt, signed: !!a.ref }; }
    if (j >= 0 && j < m.nmz) { const r = MIRINO.trace(m.v, m.nrt, m.nmz, j, m.rt0, m.rt1); out.xic = { c: pt.mz, tol: m.dmz / 2, tr: [{ k: a.f.k, rt: r.x, y: r.y }], signed: !!a.ref }; }
    return out;
  }
  // exact where the page has it, a preview for the rest
  function show(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ pt) {
    const s = st(p); if (s.sp.hidden && s.xc.hidden) return;
    const sp = specFor(p, pt), xi = xicFor(p, pt), pv = !sp || !xi ? preview(p, pt) : null;
    if (MEAS && pv) window.__mirPrev = (window.__mirPrev || 0) + 1;
    // what the two plots hold (read by e2e_mappa_mirino)
    s.shown = { pt, sp: sp ? sp.key : null, xi: xi ? xi.key : null };
    drawSide(p, pt, sp || (pv && pv.spec) || s.spec, !sp); drawBottom(p, pt, xi || (pv && pv.xic) || s.xic, !xi);
    return sp && xi;
  }
  const ask = MIRINO.latest(() => { const p = ask.panel; return p ? exact(p) : null; });
  const ticKey = (/** @type {Record<string, any>} */ a) => `${a.f.k}|${a.ref ? a.ref.k : ""}|${a.f.lv || 1}`;
  // A − B of the TIC of the two files (once per pair): the overall trace of the difference
  async function fetchTic(/** @type {Pannello} */ p) {
    const s = st(p), a = p._a; if (!a || !a.ref) return;
    const key = ticKey(a); if (s.tic && s.tic.key === key) return;
    try {
      const lv = a.f.lv || 1, [ca, cb] = await Promise.all([getChrom(a.f.k, "tic", lv), getChrom(a.ref.k, "tic", lv)]), d = MIRINO.diffTrace(ca.rt, ca.y, cb.rt, cb.y);
      if (p._a && ticKey(p._a) === key) s.tic = { key, rt: d.rt, y: d.y };
    } catch (e) { /* no TIC */ }
  }
  async function exact(/** @type {Pannello} */ p) {
    const q = point(p); if (!q) return;
    const s = st(p), t1 = performance.now();
    await Promise.all([fetchSpec(p, q.rt), fetchXic(p, q.mz), fetchTic(p)]);
    const t2 = performance.now(), r = point(p); if (!r || !p._a) return;
    // an answer for a point the sight has left is not drawn: the newest point is already queued
    const done = show(p, r); readout(p);
    if (done) { if (MEAS && s.t0) meas.push({ fetch: t2 - t1, draw: performance.now() - t2, total: performance.now() - s.t0 }); s.t0 = 0; prefetch(p, r); }
  }
  function margins(/** @type {Pannello} */ p) {
    const s = st(p); if (!s.sp || s.sp.hidden && s.xc.hidden || !is2d(p)) return;
    if (s.ex) { if (window.MAPEX) MAPEX.margins(p); return; }
    const pt = point(p); if (!pt) { drawSide(p, null); drawBottom(p, null); return; }
    if (s.lastRt != null) s.dir = pt.rt > s.lastRt ? 1 : pt.rt < s.lastRt ? -1 : s.dir; s.lastRt = pt.rt;
    if (show(p, pt)) { if (MEAS) meas.push({ fetch: 0, draw: 0, total: s.t0 ? performance.now() - s.t0 : 0 }); s.t0 = 0; return; }
    if (!s.t0) s.t0 = performance.now();
    ask.panel = p; ask(pt);
  }
  // pointer events come faster than frames
  const frame = (/** @type {Pannello} */ p) => { const s = st(p); if (!s.raf) s.raf = FRAME.next(() => { s.raf = 0; const t = performance.now(); overlay(p); margins(p); if (MEAS) (window.__mirFrame || (window.__mirFrame = [])).push(performance.now() - t); }); };
  // LR: window [n - 0.2, n + 0.8] = centre n + 0.3, +-0.5
  const xicTol = (/** @type {MzFile} */ f, /** @type {number} */ mz) => isHrF(f) ? HR.tolDa(f, mz, f.lv || 1) : 0.5;
  const xicCentre = (/** @type {MzFile} */ f, /** @type {number} */ mz) => isHrF(f) ? mz : Math.round(mz) + 0.3;
  const canvasOf = (/** @type {Record<string, any>} */ c) => MAPMARGINI.sized(c, devicePixelRatio);
  // m/z labels of the sideways spectrum: beyond the tip of the bar (fixed gap) with a halo of the panel colour; if they do not fit they go above the bar,
  // to the left of the tip, so the bar never crosses its own text. The most intense peaks come first, a label never sits on another (by S/N, see MAPREND.pick). s.lbls keeps the boxes.
  function sideLabels(/** @type {CanvasRenderingContext2D} */ g, /** @type {number} */ W, /** @type {number[]} */ order, /** @type {(i: number) => Record<string, any>} */ get, /** @type {number} */ x0, /** @type {Record<string, any>} */ s, limit = 8) {
    /** @type {Record<string, any>[]} */ const out = []; s.lbls = out; g.textAlign = "left"; g.lineJoin = "round"; g.lineWidth = 3; g.strokeStyle = ecss("--panel"); g.fillStyle = ecss("--ink");
    for (const i of order) {
      if (out.length >= limit) break; const q = get(i);
      const w = g.measureText(q.t).width, right = q.tip + 5 + w <= W - 2, x = right ? q.tip + 5 : Math.max(x0 + 2, q.tip - 3 - w), by = right ? q.y + 4 : q.y - 3, bx = { x, y: by - 9, w, h: 11 };
      if (out.some(o => bx.x < o.x + o.w + 2 && o.x < bx.x + bx.w + 2 && bx.y < o.y + o.h + 1 && o.y < bx.y + bx.h + 1)) continue;
      g.strokeText(q.t, x, by); g.fillText(q.t, x, by); out.push({ t: q.t, ...bx, ly: q.y, tip: q.tip, right, i: q.i });
    }
    g.lineWidth = 1;
  }
  // Profile line of a scan (pmz/py, ascending m/z): the slice inside [lo, hi] plus one neighbour per side, and its highest point.
  function profRange(/** @type {Record<string, any>} */ sp, /** @type {number} */ lo, /** @type {number} */ hi) {
    const m = sp.pmz, n = m.length; let a = 0, b = n;
    while (a < b) { const h = (a + b) >> 1; if (m[h] < lo) a = h + 1; else b = h; }
    let c = a; b = n; while (c < b) { const h = (c + b) >> 1; if (m[h] <= hi) c = h + 1; else b = h; }
    const i0 = Math.max(0, a - 1), i1 = Math.min(n, c + 1); let max = 0; for (let i = i0; i < i1; i++) if (sp.py[i] > max) max = sp.py[i];
    return { i0, i1, max };
  }
  // Continuous area turned on its side: every point while there is at most one per pixel row (zoomed in), otherwise the highest of each row (min-max decimation: no peak top is lost).
  function profSide(/** @type {CanvasRenderingContext2D} */ g, /** @type {Record<string, any>} */ sp, /** @type {Record<string, any>} */ pr, /** @type {(v: number) => number} */ Y, /** @type {number} */ x0, /** @type {number} */ x1, /** @type {number} */ ymax, /** @type {string} */ col) {
    const m = sp.pmz, v = sp.py, X = (/** @type {number} */ u) => x0 + (x1 - x0) * u / ymax;
    // the zeros put back at both sides of a gap of exactly two steps land on the same m/z: one point there
    /** @type {number[]} */ const idx = []; for (let i = pr.i0; i < pr.i1; i++) if (i === pr.i0 || m[i] !== m[i - 1]) idx.push(i);
    const n = idx.length;   // m/z ascends and Y descends, so the pixel rows come in order: one pass keeps the highest value of each row
    /** @type {number[][]} */ const rows = []; for (const i of idx) { const r = Math.round(Y(m[i])), last = rows[rows.length - 1]; if (last && last[0] === r) { if (v[i] > last[1]) last[1] = v[i]; } else rows.push([r, v[i]]); }
    const pts = rows.length >= n ? idx.map(i => [Y(m[i]), v[i]]) : rows;
    const trace = () => { g.beginPath(); g.moveTo(X(pts[0][1]), pts[0][0]); for (const q of pts) g.lineTo(X(q[1]), q[0]); };
    trace(); g.lineTo(x0, pts[pts.length - 1][0]); g.lineTo(x0, pts[0][0]); g.closePath();
    const al = g.globalAlpha; g.globalAlpha = al * .25; g.fillStyle = col; g.fill(); g.globalAlpha = al; trace();
    return { pts: pts.length, full: n };
  }
  // spectrum turned on its side: m/z upwards like the map, intensity to the right
  /** @param {Pannello} p @param {Record<string, any> | null} pt @param {Record<string, any>} [sp] @param {boolean} [prev] */
  function drawSide(p, pt, sp, prev) {
    const s = st(p), a = p._a; if (s.sp.hidden || !a) return;
    const { g, W, H } = canvasOf(s.sp), ink = ecss("--muted"); g.clearRect(0, 0, W, H); g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink;
    const o = MAPMARGINI.rel(p.cv, s.sp);
    const mapTop = o.y + M.t, mapBot = o.y + p.cv.clientHeight - M.b, mapH = mapBot - mapTop;
    const Y = (/** @type {number} */ v) => mapBot - (v - a.y0) / (a.y1 - a.y0) * mapH, x0 = 6, x1 = W - 8;
    g.beginPath(); g.moveTo(x0 + .5, mapTop); g.lineTo(x0 + .5, mapBot); g.stroke();
    if (!sp || !pt) { g.fillText(T("mappa.side.title"), x0 + 4, Math.max(10, mapTop + 10)); return; }
    // difference A − B: bars right (more in A) and left (more in B) of a zero line
    const sg = !!sp.signed, ab = (/** @type {number} */ v) => sg ? Math.abs(v) : v;
    let ymax = 0; const vis = /** @type {number[]} */ ([]); for (let i = 0; i < sp.mz.length; i++) if (sp.mz[i] >= a.y0 && sp.mz[i] <= a.y1) { vis.push(i); if (ab(sp.y[i]) > ymax) ymax = ab(sp.y[i]); }
    // profile scan: the continuous line (pmz/py) is drawn, the peaks (mz/y) only label it
    const pr = !prev && !sg && sp.pmz && sp.py ? profRange(sp, a.y0, a.y1) : null;
    if (pr) ymax = Math.max(ymax, pr.max);
    if (!ymax) { g.fillText(T("mappa.side.empty"), x0 + 4, Math.max(10, mapTop + 10)); return; }
    const xz = sg ? (x0 + x1) / 2 : x0, len = (/** @type {number} */ v) => (sg ? (x1 - x0) / 2 : x1 - x0) * v / ymax;
    if (window.TDATI) { s.sp.setAttribute("aria-label", T("mappa.side.title")); TDATI.addButton(s.sp, () => ({ title: T(sg ? "mappa.side.diff" : "mappa.side.title"), head: ["m/z", T("tdati.col.int")], rows: vis.slice(0, 300).map(i => [TDATI.num(sp.mz[i], 4), TDATI.num(sp.y[i], 1)]) }), true); }
    g.strokeStyle = a.f.color || ecss("--accent"); g.globalAlpha = prev ? .5 : 1;
    if (sg) {
      g.beginPath(); g.moveTo(xz + .5, mapTop); g.lineTo(xz + .5, mapBot); g.strokeStyle = ink; g.stroke();
      for (const [sgn, col] of /** @type {[number, string][]} */ ([[1, "--diff-pos"], [-1, "--diff-neg"]])) { g.strokeStyle = css(col); g.beginPath(); for (const i of vis) if (sp.y[i] * sgn > 0) { const y = Math.round(Y(sp.mz[i])) + .5; g.moveTo(xz, y); g.lineTo(xz + len(sp.y[i]), y); } g.stroke(); }
    } else {
    g.beginPath();
    if (pr) { const r = profSide(g, sp, pr, Y, x0, x1, ymax, a.f.color || ecss("--accent")); s.sp.dataset.pts = r.pts; s.sp.dataset.full = r.full; } else { delete s.sp.dataset.pts; }
    if (!pr)
    for (const i of vis) { const y = Math.round(Y(sp.mz[i])) + .5; g.moveTo(x0, y); g.lineTo(x0 + (x1 - x0) * sp.y[i] / ymax, y); }
    g.stroke();
    }
    g.globalAlpha = 1;
    // a preview has bins, not peaks: no m/z labels
    if (prev) { g.fillStyle = ink; g.fillText(T("mappa.preview"), x0 + 4, Math.min(H - 4, mapBot + 14)); return; }
    const dn = DENS[PREF.dens] || DENS.mid, cen = xicCentre(a.f, pt.mz), tol = xicTol(a.f, pt.mz);
    // best S/N of each slice of the window; the peak of the sight always
    const kept = MAPREND.pick(sp.mz, sp.y, { lo: a.y0, hi: a.y1, bands: dn.bands, quota: dn.quota, act: [cen - tol, cen + tol] });
    s.snr = new Map(kept.all.map((/** @type {Record<string, any>} */ c) => [c.i, c.snr]));
    sideLabels(g, W, kept.map((/** @type {Record<string, any>} */ c) => c.i), (/** @type {number} */ i) => ({ y: Y(sp.mz[i]), tip: sg ? xz + Math.max(0, len(sp.y[i])) : x0 + (x1 - x0) * sp.y[i] / ymax, t: sp.mz[i].toFixed(mzd(p)), i }), x0, s, dn.limit);
    if (pt.mz >= a.y0 && pt.mz <= a.y1) { g.strokeStyle = ecss("--ink"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(x0, Y(pt.mz) + .5); g.lineTo(x1, Y(pt.mz) + .5); g.stroke(); g.setLineDash([]); }
    g.fillStyle = ink; g.fillText(sg ? `A − B · RT ${sp.rt.toFixed(2)}` : `${T("mappa.read.scan", { n: sp.sid + 1 })} · RT ${sp.rt.toFixed(2)}`, x0 + 4, Math.min(H - 4, mapBot + 14));
  }
  /** @param {Pannello} p @param {Record<string, any> | null} pt @param {Record<string, any>} [xi] @param {boolean} [prev] */
  function drawBottom(p, pt, xi, prev) {                                          // XIC under the map: the same RT axis
    const s = st(p), a = p._a; if (s.xc.hidden || !a) return;
    const { g, W, H } = canvasOf(s.xc), ink = ecss("--muted"), mt = 10, mb = 22; g.clearRect(0, 0, W, H); g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink;
    const o = MAPMARGINI.rel(p.cv, s.xc);
    const ml = o.x + M.l, mr = o.x + p.cv.clientWidth - M.r, pw = mr - ml;
    const X = (/** @type {number} */ v) => ml + (v - a.x0) / (a.x1 - a.x0) * pw;
    g.beginPath(); g.moveTo(ml, H - mb + .5); g.lineTo(mr, H - mb + .5); g.stroke();
    XICDRAW.axis(g, X, a.x0, a.x1, { base: H - mb, labelY: H - mb + 13, lo: 1, hi: W - 1, color: ink, font: fpx(11) });
    if (!xi || !pt) { g.textAlign = "left"; g.fillText(T("mappa.bot.title"), ml + 4, mt + 8); return; }
    let tr = xi.tr || [], ymax = 0, sg = false;
    // preview of a difference: the matrix already holds A − B
    if (a.ref && xi.signed) sg = true;
    else if (a.ref && tr.length >= 2) { const d = MIRINO.diffTrace(tr[0].rt, tr[0].y, tr[1].rt, tr[1].y); tr = [{ k: tr[0].k, rt: d.rt, y: d.y, label: "A − B" }]; sg = true; }
    // A − B of the TIC, on its own scale, behind the XIC
    const tic = sg && s.tic && s.tic.key === ticKey(a) ? s.tic : null;
    for (const t of tr) for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= a.x0 && t.rt[i] <= a.x1 && Math.abs(t.y[i]) > ymax) ymax = Math.abs(t.y[i]);
    if (window.TDATI) { s.xc.setAttribute("aria-label", T("mappa.bot.title")); TDATI.series(s.xc, { title: T("mappa.bot.title"), xl: T("tdati.col.rt"), yl: T("tdati.col.int"), series: tr.map((/** @type {Record<string, any>} */ t) => ({ x: t.rt, y: t.y, name: t.label || t.name })).concat(tic ? [{ x: tic.rt, y: tic.y, name: "TIC A − B" }] : []) }, true); }
    // zero always sits on the axis; 8% headroom above the highest point keeps the apex label inside the box
    const lo0 = sg ? -ymax : 0, Y = (/** @type {number} */ v) => H - mb - (v - lo0) / ((sg ? 2 * ymax : ymax * 1.08) || 1) * (H - mt - mb);
    // intensity axis, like the other chromatograms
    g.beginPath(); g.moveTo(ml + .5, mt); g.lineTo(ml + .5, H - mb + .5); g.stroke();
    if (ymax) { g.textAlign = "right"; for (const t of nice(lo0, sg ? ymax : ymax, Math.max(2, Math.floor((H - mt - mb) / 28)))) { const y = Math.round(Y(t)) + .5; g.beginPath(); g.moveTo(ml - 4, y); g.lineTo(ml, y); g.stroke(); g.fillText(fmtA(t), ml - 6, y + 3.5); } }
    if (sg) {
      const y0 = Y(0); g.beginPath(); g.moveTo(ml, y0 + .5); g.lineTo(mr, y0 + .5); g.stroke();
      // TIC A − B: thin dashed line with its own scale (written beside the label)
      if (tic) {
        const tm = maxOf(tic.y.map(Math.abs)) || 1; g.strokeStyle = ink; g.setLineDash([2, 3]); g.beginPath(); let f1 = true;
        for (let i = 0; i < tic.rt.length; i++) { if (tic.rt[i] < a.x0 || tic.rt[i] > a.x1) continue; const x = X(tic.rt[i]), y = y0 - tic.y[i] / tm * (y0 - mt); if (f1) { g.moveTo(x, y); f1 = false; } else g.lineTo(x, y); }
        g.stroke(); g.setLineDash([]);
      }
      const t = tr[0];
      // area between the curve and zero: vermilion where A is higher, blue where B is
      for (const [sgn, col] of /** @type {[number, string][]} */ ([[1, "--diff-pos"], [-1, "--diff-neg"]])) {
        g.save(); g.beginPath(); g.rect(ml, sgn > 0 ? mt : y0, pw, sgn > 0 ? y0 - mt : H - mb - y0); g.clip();
        g.fillStyle = css(col); g.strokeStyle = css(col); g.globalAlpha = prev ? .35 : .55; g.beginPath(); g.moveTo(X(Math.max(a.x0, t.rt[0] ?? a.x0)), y0);
        let lx = ml; for (let i = 0; i < t.rt.length; i++) { if (t.rt[i] < a.x0 || t.rt[i] > a.x1) continue; lx = X(t.rt[i]); g.lineTo(lx, Y(t.y[i])); }
        g.lineTo(lx, y0); g.closePath(); g.fill(); g.globalAlpha = 1; g.lineWidth = 1.2; g.stroke(); g.restore();
      }
      g.lineWidth = 1;
    } else
    tr.forEach((/** @type {Record<string, any>} */ t, /** @type {number} */ n) => {
      // drawn like the XIC of the lens: filled trace, no apex lost, the number on the apex of the first one
      const f = E.files[t.k], v = XICDRAW.slice(/** @type {{ rt: number[], y: number[] }} */ (t), a.x0, a.x1), b = XICDRAW.apex(v); if (!v.rt.length) return;
      g.globalAlpha = prev ? .5 : 1; XICDRAW.trace(g, v, X, Y, H - mb, pw, f ? f.color : ink, n ? [4, 3] : undefined);
      if (!n && !prev && XICDRAW.clearPeak(v, b)) { g.font = fpx(11); XICDRAW.apexLabel(g, v, b, X(v.rt[b]), Y(v.y[b]), ml, mr, ecss("--ink")); } g.globalAlpha = 1;
    });
    g.fillStyle = ecss("--ink"); g.textAlign = "left";
    const lo = xi.c - xi.tol, hi = xi.c + xi.tol;
    g.fillText(prev ? `XIC · ${T("mappa.preview")}` : `XIC${sg ? " A − B" : ""} ${lo.toFixed(isHrF(a.f) ? 4 : 1)}–${hi.toFixed(isHrF(a.f) ? 4 : 1)} · max ${fmt(ymax)}${tic ? " · " + T("mappa.bot.tic") : ""}`, ml + 4, mt + 8);
    if (pt.rt >= a.x0 && pt.rt <= a.x1) { g.strokeStyle = ecss("--ink"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(X(pt.rt) + .5, mt); g.lineTo(X(pt.rt) + .5, H - mb); g.stroke(); g.setLineDash([]); }
  }

  // ---------------------------------------------------------------- mouse
  // Shift held while the pointer moves with no button down: the sight is held on the line or the column picked by the first pixels (same m/z or same RT),
  // so the spectrum and the XIC at the margins follow a straight line; releasing Shift frees it. s.sl = { ax, rt, mz, x, y } (anchor point and pixel); s.lpx = last pixel
  function shiftSight(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ s, /** @type {number} */ px, /** @type {number} */ py) {
    if (!s.sl) { const h = s.hov || xy2pt(p, px, py); s.sl = { ax: null, rt: h.rt, mz: h.mz, x: s.lpx ? s.lpx[0] : px, y: s.lpx ? s.lpx[1] : py }; }
    const q = s.sl, dx = px - q.x, dy = py - q.y;
    if (!q.ax && Math.max(Math.abs(dx), Math.abs(dy)) >= ASSE.THR) q.ax = Math.abs(dx) >= Math.abs(dy) ? "x" : "y";
    return q.ax === "x" ? { rt: xOf(p, px), mz: q.mz } : q.ax === "y" ? { rt: q.rt, mz: p._a.mzAt(py) } : { rt: q.rt, mz: q.mz };
  }
  const xy2pt = (/** @type {Pannello} */ p, /** @type {number} */ px, /** @type {number} */ py) => ({ rt: xOf(p, px), mz: p._a.mzAt(py) });
  function move(/** @type {Pannello} */ p, /** @type {number} */ px, /** @type {number} */ py, /** @type {MouseEvent} */ e) {
    if (!is2d(p)) return; const s = st(p), a = p._a;
    if (s.feat && window.MAPEX && MAPEX.featHover(p, px, py)) return;
    if (s.ex && window.MAPEX && MAPEX.hover(p, px, py)) return;
    // the menu is open: the sight stays on the clicked point
    if (s.frz) return;
    const held = !!(e && e.shiftKey && !e.buttons && mirOn(p) && !s.lock);
    if (!held) s.sl = null;
    if (px < M.l || px > a.W - M.r || py < M.t || py > a.H - M.b) { s.hov = null; }
    else { s.hov = held ? shiftSight(p, s, px, py) : xy2pt(p, px, py); s.kp = null; }
    s.lpx = [px, py];
    if (mirOn(p)) { p.tip.hidden = true; p.vl.hidden = true; }
    if (!s.lock) frame(p); else readout(p);
  }
  function guide(/** @type {Pannello} */ p, /** @type {Record<string, any> | null} */ q) { const s = st(p); if (!q && !s.guide) return; s.guide = q || null; overlay(p); }
  // Shift released: the sight is free again, at the pointer
  function shiftUp(/** @type {Pannello} */ p, /** @type {KeyboardEvent} */ e) {
    const s = st(p); if (e.key !== "Shift" || !s.sl) return;
    s.sl = null; if (s.hov && s.lpx && !s.frz) s.hov = xy2pt(p, s.lpx[0], s.lpx[1]);
    frame(p);
  }
  function leave(/** @type {Pannello} */ p) { const s = st(p); if (s.frz) return; s.sl = null; s.hov = null; if (!s.lock && s.ovg) frame(p); }
  function wheel(/** @type {Pannello} */ p, /** @type {WheelEvent} */ e, /** @type {number} */ px, /** @type {number} */ py) {
    // Ctrl/Cmd + wheel (a pinch on the trackpad) keeps the zoom of explore.js
    if (!is2d(p) || e.ctrlKey || e.metaKey) return false;
    // the wheel alone scrolls the page, as over every other chart
    if (!e.shiftKey && !e.altKey) return false;
    e.preventDefault();
    const a = p._a;
    if (!p._wz || Date.now() - p._wz > 600) pushZh(p); p._wz = Date.now();
    const dy = e.deltaY || e.deltaX, f = Math.exp(Math.max(-60, Math.min(60, dy)) * 0.004);
    // Maiusc = RT only, Alt = m/z only
    scaleView(p, f, { rt: xOf(p, px), mz: a.mzAt(py) }, e.shiftKey ? "x" : e.altKey ? "y" : "xy");
    draw(p); return true;
  }
  function boxZoom(/** @type {Pannello} */ p, /** @type {number} */ ax, /** @type {number} */ ay, /** @type {number} */ bx, /** @type {number} */ by) {
    const a = p._a, u = [xOf(p, ax), xOf(p, bx)].sort((m, n) => m - n), v = [a.mzAt(ay), a.mzAt(by)].sort((m, n) => m - n);
    pushZh(p);
    if (Math.abs(bx - ax) > 4) p.zoom = clampV(u[0], u[1], a.full[0], a.full[1]);
    if (Math.abs(by - ay) > 4) p.zoomY = clampV(v[0], v[1], a.fullY[0], a.fullY[1]);
    draw(p); uiSave();
  }
  // double click / double tap: zoomed = whole view (undoable); whole map = the spectrum at that RT
  function dbl(/** @type {Pannello} */ p, /** @type {number} */ px) {
    const a = p._a; if (!a || !a.map || p.zoom || p.zoomY) { whole(p); return; }
    const rt = a.x0 + (px - M.l) / (a.W - M.l - M.r) * (a.x1 - a.x0); spectrumAt(p, Math.max(a.full[0], Math.min(a.full[1], rt)), a.f.k);
  }
  function whole(/** @type {Pannello} */ p) { pushZh(p); p.zoom = null; p.zoomY = null; p.sel = null; draw(p); uiSave(); }
  function unlock(/** @type {Pannello} */ p) { const s = st(p); s.lock = null; overlay(p); margins(p); }
  // right click: lock / unlock the point; Maiusc + right click: the XIC of that m/z (the old right-click entry)
  function ctx(/** @type {Pannello} */ p, /** @type {MouseEvent} */ e, /** @type {number} */ px, /** @type {number} */ py) {
    if (!is2d(p)) return false;
    e.preventDefault(); hideHover(p);
    const a = p._a, inside = px >= M.l && px <= a.W - M.r && py >= M.t && py <= a.H - M.b;
    if (e.shiftKey) { if (inside) openXic(null, { mz: a.mzAt(py), obs: true }); return true; }
    // the long press has just opened the menu (Android also sends contextmenu)
    if (Date.now() - (st(p).lpAt || 0) < 900) return true;
    showMenu(ad2(p, inside ? { rt: xOf(p, px), mz: a.mzAt(py) } : null), e); return true;
  }
  // the menu of the Menu key / Shift+F10: the right-click menu, at the point of the sight (locked, keyboard or the middle of the map)
  function menuAt(/** @type {Pannello} */ p, /** @type {HTMLElement} */ btn, /** @type {boolean} */ kbd) {
    const a = p._a; if (!a) return; const pt = point(p) || { rt: (a.x0 + a.x1) / 2, mz: (a.y0 + a.y1) / 2 }, r = btn.getBoundingClientRect();
    showMenu(ad2(p, pt), { preventDefault() {}, clientX: r.left + (kbd ? r.width / 2 : 0), clientY: kbd ? r.top + r.height / 2 : r.bottom + 4, kbd: !!kbd });
  }

  // ---------------------------------------------------------------- keyboard (focus on the map, or the expanded map; never inside a text field)
  function key(/** @type {Pannello} */ p, /** @type {KeyboardEvent} */ e) {
    if (p.type !== "map" || /** @type {Element} */ (e.target).closest && /** @type {Element} */ (e.target).closest("input,textarea,select,[contenteditable='true']")) return;
    if (e.ctrlKey || e.metaKey) return;
    const k = e.key, s = st(p), a = p._a; if (!a) return;
    const done = () => { e.preventDefault(); e.stopPropagation(); };
    if (k === "f" || k === "F") { done(); p._setMax(!isMax(p)); return; }
    if (k === "0") { done(); whole(p); return; }
    if (k === "Backspace") { done(); whole(p); return; }
    if (!is2d(p)) return;
    // X / Y latch the axis of the gestures, Esc frees it
    if (ASSE.key(e)) { done(); overlay(p); return; }
    if (s.ex && window.MAPEX && MAPEX.key(p, e)) { done(); return; }
    if (k === "m" || k === "M") { done(); s.mir = !mirOn(p); layout(p); draw(p); return; }
    if (k === "l" || k === "L") { done(); ad2(p, null).toggleLock(); return; }
    if (k === "Escape") { if (s.lock) { done(); unlock(p); navSync(p); } return; }       // Esc: unlock
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
    const set = (/** @type {Record<string, any>} */ q) => { if (s.lock) s.lock = q; else s.kp = q; follow(p, q); overlay(p); margins(p); };
    // the locked axis keeps the other one still
    if (ASSE.fix === "x" ? k === "ArrowUp" || k === "ArrowDown" : ASSE.fix === "y" && (k === "ArrowLeft" || k === "ArrowRight")) return;
    if (k === "ArrowUp" || k === "ArrowDown") {
      // next local maximum of the spectrum above 3 x noise
      if (e.shiftKey) { fetchSpec(p, cur.rt).then(sp => { const m = peakStep(sp, cur.mz, dir); if (m != null) set({ ...cur, mz: m }); }); return; }
      nx.mz = Math.max(a.fullY[0], Math.min(a.fullY[1], cur.mz + dir * binDa(p))); set(nx); return;
    }
    fetchXic(p, cur.mz).then(xi => {
      const t = xi && xi.tr && xi.tr[0]; if (!t || !t.rt.length) { nx.rt = Math.max(a.full[0], Math.min(a.full[1], cur.rt + dir * scanStep())); set(nx); return; }
      const i0 = nearIdx(t.rt, cur.rt);
      // apex of the XIC in that direction
      if (e.shiftKey) { const i = apexStep(t, i0, dir); if (i != null) set({ ...cur, rt: t.rt[i] }); return; }
      // next scan
      const i = Math.max(0, Math.min(t.rt.length - 1, (Math.abs(t.rt[i0] - cur.rt) < 1e-6 ? i0 : (dir > 0 ? (t.rt[i0] > cur.rt ? i0 - 1 : i0) : (t.rt[i0] < cur.rt ? i0 + 1 : i0))) + dir));
      set({ ...cur, rt: t.rt[i] });
    });
  }
  // the view follows the point when it leaves it
  function follow(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ q) {
    const a = p._a; let ch = false;
    if (p.zoom && (q.rt < a.x0 || q.rt > a.x1)) { const w = a.x1 - a.x0; p.zoom = clampV(q.rt - w / 2, q.rt + w / 2, a.full[0], a.full[1]); ch = true; }
    if (p.zoomY && (q.mz < a.y0 || q.mz > a.y1)) { const h = a.y1 - a.y0; p.zoomY = clampV(q.mz - h / 2, q.mz + h / 2, a.fullY[0], a.fullY[1]); ch = true; }
    if (ch) draw(p);
  }
  const madNoise = MAPDATI.madNoise;
  function peakStep(/** @type {Record<string, any>} */ sp, /** @type {number} */ mz, /** @type {number} */ dir) {
    if (!sp || !sp.mz || !sp.mz.length) return null;
    const thr = 3 * madNoise(sp.y), n = sp.mz.length;
    const isMax = (/** @type {number} */ i) => sp.y[i] > thr && (i === 0 || sp.y[i] >= sp.y[i - 1]) && (i === n - 1 || sp.y[i] >= sp.y[i + 1]);
    if (dir > 0) { for (let i = 0; i < n; i++) if (sp.mz[i] > mz + 1e-6 && isMax(i)) return sp.mz[i]; }
    else { for (let i = n - 1; i >= 0; i--) if (sp.mz[i] < mz - 1e-6 && isMax(i)) return sp.mz[i]; }
    return null;
  }
  function apexStep(/** @type {Record<string, any>} */ t, /** @type {number} */ i0, /** @type {number} */ dir) {
    const y = t.y, n = y.length, thr = 3 * madNoise(y);
    for (let i = i0 + dir; i > 0 && i < n - 1; i += dir) if (y[i] > thr && y[i] >= y[i - 1] && y[i] >= y[i + 1] && y[i] > 0) return i;
    return null;
  }


  // ---------------------------------------------------------------- marked points (variant A, for everybody): the student marks, the program MEASURES.
  // Tool «Segna» (button or S): a click adds a numbered point on the local maximum (+-0.2 min, +-1 bin); Canc removes the chosen one; a point can be
  // dragged. Saved in the notebook (NB.mappaPunti) for each pair of files. Table under the map: numbers only, never a name or a verdict.
  const pairKey = (/** @type {Pannello} */ p) => { const a = p._a; if (!a || !a.f) return null; return `${a.f.file}|${a.ref ? a.ref.file : ""}|${a.f.lv || 1}`; };
  const pts = (/** @type {Pannello} */ p) => { const k = pairKey(p); if (!k) return []; NB.mappaPunti = NB.mappaPunti || {}; return NB.mappaPunti[k] || (NB.mappaPunti[k] = []); };
  // the marks of the map (add, move, delete, note, reference) are one undoable action (comandi.js): the snapshot is the list of marks of the pair of files
  COMANDI.gestore("map.marks", {
    leggi: (/** @type {Pannello} */ p) => JSON.stringify(pts(p)),
    applica: (/** @type {Pannello} */ p, /** @type {string} */ snap) => { const l = pts(p); l.splice(0, l.length, ...JSON.parse(snap)); st(p).psel = null; ptsSave(p); overlay(p); table(p); },
  });
  const ptsSave = (/** @type {Pannello} */ p) => { const k = pairKey(p); if (k && NB.mappaPunti && !NB.mappaPunti[k].length) delete NB.mappaPunti[k]; nbSave(); };
  const ppmOf = (/** @type {MzFile} */ f) => isHrF(f) ? ((HR.prof(f, f.lv || 1) || {}).tol || 5) : 0;
  // the apex near (rt, m/z) (Rust, `api/mapsnap`); null when the option «snap» is off or no peak is near
  // the RT window is about 5 px of the current zoom (never wider than 0.2 min, never narrower than 1.5 scans), so the apex is the one under the click and not a taller neighbour
  /** @param {Pannello} p @param {number} rt @param {number} mz @param {number} [dmz] @param {number} [drt] */
  async function snapAt(p, rt, mz, dmz, drt) {
    if (PREF.snap === false) return null; const a = p._a;
    if (drt == null) drt = Math.min(0.2, Math.max(5 * Math.abs(a.x1 - a.x0) / (Math.abs(a.X(a.x1) - a.X(a.x0)) || 1), 1.5 * scanStep()));
    try { const at = await J(`api/mapsnap?k=${a.f.k}&level=${a.f.lv || 1}&rt=${rt}&mz=${mz}&dmz=${dmz || a.dmz || 1}&drt=${drt}`); return at && at.rt != null ? at : null; } catch (e) { return null; }
  }
  async function addPoint(/** @type {Pannello} */ p, /** @type {number} */ px, /** @type {number} */ py) {
    const a = p._a, rt = xOf(p, px), mz = a.mzAt(py);
    const q = (await snapAt(p, rt, mz)) || { rt, mz };
    const list = pts(p); COMANDI.fai("map.marks", p, () => list.push({ rt: q.rt, mz: q.mz, note: "", shape: shapeNew(list, q.rt) })); st(p).psel = list.length - 1;
    ptsSave(p); overlay(p); table(p);
  }
  // from the 3D viewer: a point at (rt, mz), snapped to the local maximum like a click on the map (dmz = width of the cell, in Da)
  async function addPointAt(/** @type {Pannello} */ p, /** @type {number} */ rt, /** @type {number} */ mz, /** @type {number} */ dmz) {
    const a = p._a; if (!a || !a.f) return;
    const q = (await snapAt(p, rt, mz, dmz || 1, 0.2)) || { rt, mz };
    const list = pts(p); COMANDI.fai("map.marks", p, () => list.push({ rt: q.rt, mz: q.mz, note: "", shape: shapeNew(list, q.rt) })); st(p).psel = list.length - 1;
    ptsSave(p); overlay(p); table(p);
  }
  // index of the marked point under (rt, m/z), -1 if none
  const markedAt = (/** @type {Pannello} */ p, /** @type {Record<string, any>} */ q) => p._a ? nearPoint(p, p._a.X(q.rt), p._a.Y(q.mz)) : -1;
  function nearPoint(/** @type {Pannello} */ p, /** @type {number} */ px, /** @type {number} */ py) {
    const a = p._a; let best = -1, bd = 81;
    pts(p).forEach((/** @type {Record<string, any>} */ q, /** @type {number} */ i) => { const d = (a.X(q.rt) - px) ** 2 + (a.Y(q.mz) - py) ** 2; if (d < bd) { bd = d; best = i; } });
    return best;
  }
  // mousedown on a point: drag it (it snaps again where it is dropped); true = explore.js does nothing else
  function down(/** @type {Pannello} */ p, /** @type {MouseEvent} */ e, /** @type {number} */ px, /** @type {number} */ py) {
    if (!is2d(p) || e.button !== 0 || e.shiftKey || e.altKey) return false;
    if (st(p).ex && window.MAPEX && MAPEX.down(p, e, px, py)) return true;       // the box of «How it is extracted»
    const i = nearPoint(p, px, py); if (i < 0) return false;
    e.preventDefault(); const s = st(p), list = pts(p), r = p.cv.getBoundingClientRect(); s.psel = i; let moved = false; const step = COMANDI.inizia("map.marks", p);
    // Ctrl/⌘ held or X / Y: the point moves along one axis only (asse.js)
    const a0 = p._a, q0 = { rt: list[i].rt, mz: list[i].mz }, g = ASSE.gest(); let ax = /** @type {string | null} */ (null), wait = false;
    const mv = (/** @type {MouseEvent} */ ev) => {
      moved = true; const x = ev.clientX - r.left, y = ev.clientY - r.top, c = ASSE.clip(g, ev, x - px, y - py); ax = c.ax; wait = !ax && (!!ASSE.fix || ASSE.held(ev));
      list[i].rt = ax === "y" || wait ? q0.rt : xOf(p, x); list[i].mz = ax === "x" || wait ? q0.mz : p._a.mzAt(y); guide(p, ax ? { ax, x: a0.X(q0.rt), y: a0.Y(q0.mz) } : null); overlay(p);
    };
    const up = async (/** @type {MouseEvent} */ ev) => {
      removeEventListener("mousemove", mv); removeEventListener("mouseup", up); guide(p, null);
      if (moved) { const at = await snapAt(p, list[i].rt, list[i].mz); if (at) { if (ax !== "y") list[i].rt = at.rt; if (ax !== "x") list[i].mz = at.mz; } ptsSave(p); step.fine(); }
      overlay(p); table(p);
    };
    addEventListener("mousemove", mv); addEventListener("mouseup", up);
    return true;
  }
  // a click (no drag) on the map: with «Segna» on it adds a point; on a point it chooses it
  function click(/** @type {Pannello} */ p, /** @type {number} */ px, /** @type {number} */ py) {
    if (!is2d(p)) return false; const s = st(p), i = nearPoint(p, px, py);
    // a marker of mzFinder: the box goes there
    if (s.feat && window.MAPEX && MAPEX.featClick(p, px, py)) return true;
    if (i >= 0) { s.psel = i; overlay(p); table(p); return true; }
    if (!s.mark) return false;
    addPoint(p, px, py); return true;
  }
  function delPoint(/** @type {Pannello} */ p, /** @type {number} */ i) { const list = pts(p); if (i == null || i < 0 || i >= list.length) return; COMANDI.fai("map.marks", p, () => list.splice(i, 1)); st(p).psel = null; ptsSave(p); overlay(p); table(p); }
  // tolerance of the RT groups: the student's value, else about one peak width (5 scans, between 0.02 and 0.15 min)
  const grpTol = () => PREF.grp ?? Math.min(0.15, Math.max(GRP0, +(5 * scanStep()).toFixed(3)));
  const MX = { st, is2d, pts, ptsSave, overlay, margins, follow, grow, ppmOf, diffOpt, isHrF, PREF, grpTol, prefSave, SHAPES, GLYPH, SHAPE_KEY, spectrumAt }, { grpShape, shapeNew, drawPoints, shapePath, ...forms } = MAPFORME(MX), { table, tableNow } = MAPTAB(MX, { ...forms, grpShape });
  /** @param {Pannello} p @param {number} rt @param {number} [k] */
  async function spectrumAt(p, rt, k) {
    const f = k != null && E.files[k] ? E.files[k] : p._a.f;
    if (p._a.pda) { await uvOpen(p, f.k, rt, rt); return; }        // the PDA map: its spectrum is the UV one (uv-spettro.js)
    let r0 = rt - scanStep() / 2, r1 = rt + scanStep() / 2;
    const n = await nearScan(f.k, rt); if (n) { r0 = n.rt - NEAR; r1 = n.rt + NEAR; }
    newSpec(p, r0, r1, f.k);
  }

  // ---------------------------------------------------------------- navigation: ONE list of commands (CMDS) for the right-click menu of the 2D map and of the 3D viewer,
  // the bar on the map, the command palette (barra.js reads CMDS) and the sheet of gestures. An «adapter» says where a command acts:
  //   { kind: "2d" | "3d", p, pt() -> {rt, mz, v?} | null, locked(), toggleLock(), resetAll(), zoomBy(f), view?(n), after?() }
  const SV = (/** @type {string} */ d) => `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${d}</svg>`;
  /** @type {Record<string, string>} */
  const ICO = {
    xic: '<path d="M2 13h12"/><path d="M3 12c2-7 3-7 5 0s3 3 5-3"/>', spec: '<path d="M3 13V8M6 13V4M9 13V9M12 13V6"/>',
    both: '<path d="M2 13h5"/><path d="M2.5 12c1-6 2-6 3 0M9 13V8M12 13V5"/>',
    mark: '<circle cx="8" cy="8" r="5.2"/><circle cx="8" cy="8" r="1.6"/>', lock: '<rect x="3.5" y="7" width="9" height="6.5" rx="1.2"/><path d="M5.5 7V5a2.5 2.5 0 015 0v2"/>', unlock: '<rect x="3.5" y="7" width="9" height="6.5" rx="1.2"/><path d="M5.5 7V5a2.5 2.5 0 014.8-1"/>',
    // the corners of the «whole view» button of every panel
    home: '<g transform="translate(1 1) scale(1.0714)"><path d="M1 5V1h4M9 1h4v4M13 9v4H9M5 13H1V9"/></g>',
     rx: '<path d="M2 8h12M2 5.5v5M14 5.5v5"/>', ry: '<path d="M8 2v12M5.5 2h5M5.5 14h5"/>',
    sel: '<rect x="3" y="3" width="10" height="10" rx="1" stroke-dasharray="2.5 2"/><path d="M8 6v4M6 8h4"/>', plus: '<path d="M8 3v10M3 8h10"/>', minus: '<path d="M3 8h10"/>',
    copy: '<rect x="5.5" y="5.5" width="8" height="8" rx="1.2"/><path d="M10.5 5.5V3.7a1.2 1.2 0 00-1.2-1.2H3.7a1.2 1.2 0 00-1.2 1.2v5.6a1.2 1.2 0 001.2 1.2h1.8"/>',
    help: '<circle cx="8" cy="8" r="6"/><path d="M6.2 6.3a1.9 1.9 0 113 1.5c-.8.5-1.2.9-1.2 1.8M8 11.7v.1"/>',
    feat: '<circle cx="5" cy="10" r="2.2"/><circle cx="11" cy="5" r="2.2"/><path d="M2.5 13.5h11"/>', learn: '<rect x="2.5" y="2.5" width="6.5" height="6.5" rx="1"/><path d="M9 9l4.5 4.5M5.8 9v4.5h4M9 5.8h4.5"/>', top: '<rect x="3" y="3" width="10" height="10" rx="1"/><path d="M3 8h10M8 3v10"/>', side: '<path d="M2 13h12"/><path d="M3 12l2.5-7 2.5 4 2-6 3 9"/>'
  };
  // where a command acts: the locked point, otherwise the one under the pointer when the menu was opened, otherwise the sight
  function ad2(/** @type {Pannello} */ p, /** @type {Record<string, any> | null} */ at) {
    const s = st(p), a = () => p._a, mid = () => ({ rt: (a().x0 + a().x1) / 2, mz: (a().y0 + a().y1) / 2 });
    const pt = () => { const q = s.lock || at || point(p); return q ? { ...q, v: valueAt(p, q.rt, q.mz) } : null; };
    return {
      vista: "map", kind: "2d", p, pt, locked: () => !!s.lock,
      toggleLock: () => { if (s.lock) unlock(p); else { s.lock = { ...(at || point(p) || mid()) }; s.kp = null; overlay(p); margins(p); } navSync(p); },
      marked: () => { const q = pt(); return !!q && markedAt(p, q) >= 0; },
      toggleMark: () => { const q = pt(); if (!q) return; const i = markedAt(p, q); if (i >= 0) delPoint(p, i); else addPointAt(p, q.rt, q.mz, p._a.dmz); },
      resetAll: () => whole(p),
      zoomBy: (/** @type {number} */ f) => zoomBy(p, f, s.lock || at || point(p))
    };
  }
  // a finger gesture has just moved the map: the mouse events that follow it are not clicks or boxes
  const touched = (/** @type {Pannello} */ p) => Date.now() - (st(p).gestAt || 0) < 700;
  const ptText = (/** @type {Record<string, any> | null} */ q) => q ? `RT ${q.rt.toFixed(2)} min · m/z ${q.mz.toFixed(3)}${q.v != null ? " · " + T("mappa.read.int") + " " + fmt(q.v) : ""}` : "";
  function extract(/** @type {Record<string, any>} */ ad, /** @type {string} */ what) {
    const q = ad.pt(); if (!q) { nearMsg(T("mapnav.nopt")); return; }
    if (what !== "spec") { if (ad.p._a.pda) wlOpen(ad.p, +q.mz.toFixed(1), +q.mz.toFixed(1), ad.p._a.f.k); else xicDirect(q.mz, ad.p); }      // on the PDA map the vertical axis is the wavelength (nm)
    if (what !== "xic") { const p = ad.p; if (ad.kind === "2d" && p.sel && p.sel[1] > p.sel[0]) newSpec(p, p.sel[0], p.sel[1], p._a.f.k); else spectrumAt(p, q.rt, ad.k); }
    if (ad.after) ad.after();
  }
  // the commands are actions of the registry azioni/1 (azioni.js), views "map" (2D) and "map3d" (the 3D viewer); the context of an action is the adapter
  const V2 = ["map", "map3d"];
  const nameOf = (/** @type {string} */ id, /** @type {Record<string, any>} */ ad) => T(id === "maplock" && ad.locked() || id === "mapmark" && ad.marked() ? `cmd.${id}.off.name` : `cmd.${id}.name`);
  const cmd = (/** @type {string} */ id, /** @type {string} */ gruppo, /** @type {string} */ ic, /** @type {string[]} */ viste, /** @type {Record<string, any>} */ o) => AZIONI.registra({ id, gruppo, ic, viste, nome: (/** @type {Record<string, any>} */ ad) => nameOf(id, ad), tip: () => T(`cmd.${id}.tip`), svg: (/** @type {Record<string, any>} */ ad) => SV(id === "maplock" && ad.locked() ? ICO.unlock : ICO[ic]), ...o });
  // with an RT stretch selected (Alt + drag) the spectrum is the average over it
  const short = (/** @type {string} */ id) => ({ voce: (/** @type {Record<string, any>} */ ad) => id === "mapspec" && ad.kind === "2d" && ad.p.sel ? T("cmd.mapspec.menuAvg") : id === "mapxic" && ad.p._a?.pda ? T("cmd.mapxic.menuWl") : T(`cmd.${id}.menu`) });
  const hasPt = (/** @type {Record<string, any>} */ ad) => !!ad.pt();
  const CMDS = [
    cmd("mapxic", "extract", "xic", V2, { ...short("mapxic"), disponibile: hasPt, esegui: (/** @type {Record<string, any>} */ ad) => extract(ad, "xic") }),
    cmd("mapspec", "extract", "spec", V2, { ...short("mapspec"), disponibile: hasPt, esegui: (/** @type {Record<string, any>} */ ad) => extract(ad, "spec") }),
    cmd("maplock", "lock", "lock", V2, { attiva: (/** @type {Record<string, any>} */ ad) => ad.locked(), esegui: (/** @type {Record<string, any>} */ ad) => ad.toggleLock() }),
    cmd("mapmark", "lock", "mark", ["map"], { disponibile: hasPt, attiva: (/** @type {Record<string, any>} */ ad) => ad.marked(), esegui: (/** @type {Record<string, any>} */ ad) => ad.toggleMark() }),
    cmd("mapreset", "view", "home", V2, { esegui: (/** @type {Record<string, any>} */ ad) => ad.resetAll() }),
    cmd("map3top", "view", "top", ["map3d"], { esegui: (/** @type {Record<string, any>} */ ad) => ad.view(1) }),
    cmd("map3rt", "view", "side", ["map3d"], { esegui: (/** @type {Record<string, any>} */ ad) => ad.view(2) }),
    cmd("map3mz", "view", "spec", ["map3d"], { esegui: (/** @type {Record<string, any>} */ ad) => ad.view(3) }),
    cmd("mapzin", "zoom", "plus", V2, { menu: false, esegui: (/** @type {Record<string, any>} */ ad) => ad.zoomBy(0.7) }),
    cmd("mapzout", "zoom", "minus", V2, { menu: false, esegui: (/** @type {Record<string, any>} */ ad) => ad.zoomBy(1 / 0.7) }),
    cmd("mapfeat", "learn", "feat", ["map"], { menu: false, visibile: (/** @type {Record<string, any>} */ ad) => featAvail(ad.p), attiva: (/** @type {Record<string, any>} */ ad) => !!st(ad.p).feat, esegui: (/** @type {Record<string, any>} */ ad) => featToggle(ad.p) }),
    cmd("mapcopy", "copy", "copy", V2, { disponibile: hasPt, esegui: async (/** @type {Record<string, any>} */ ad) => { const q = ad.pt(); if (!q) return; try { await navigator.clipboard.writeText(ptText(q)); nearMsg(T("mapnav.copied")); } catch (e) { nearMsg(T("mapnav.copyFail")); } } }),
    cmd("maphelp", "help", "help", V2, { esegui: (/** @type {Record<string, any>} */ ad) => help(ad.kind) })
  ];
  const runCmd = (/** @type {string} */ id, /** @type {Record<string, any>} */ ad) => AZIONI.esegui(id, /** @type {{ vista: string, [k: string]: any }} */ (ad));
  // the items of the right-click menu (shared component: menu() in explore.js): where the commands act, then the actions of the view
  function itemsFor(/** @type {Record<string, any>} */ ad) {
    const q = ad.pt(), hdr = ad.locked() ? T("mapnav.where.locked", { at: ptText(q) }) : q ? T("mapnav.where", { at: ptText(q) }) : T("mapnav.where.none");
    return [{ label: hdr, dim: true }, "-", ...AZIONI.per(/** @type {{ vista: string, [k: string]: any }} */ (ad))];
  }
  // while the menu is open the sight (spectrum, XIC) is held on the point of the click; it is let go when the menu closes (a choice such as Lock has already set s.lock)
  function holdSight(/** @type {Record<string, any>} */ ad) {
    if (ad.kind !== "2d") return; const s = st(ad.p), q = ad.pt(), m = Q("#ctx"); if (!q || s.lock) return;
    s.frz = { rt: q.rt, mz: q.mz }; s.kp = null; frame(ad.p);
    const mo = new MutationObserver(() => { if (!m.hidden) return; mo.disconnect(); s.frz = null; frame(ad.p); navSync(ad.p); });
    mo.observe(m, { attributes: true, attributeFilter: ["hidden"] });
  }
  function showMenu(/** @type {Record<string, any>} */ ad, /** @type {MenuEv} */ ev) { if (ad.host) ad.host(); menu(ev, itemsFor(ad)); holdSight(ad); }
  // zoom around a point (default: the middle of the view); axes "x" = RT, "y" = m/z
  function scaleView(/** @type {Pannello} */ p, /** @type {number} */ f, /** @type {Record<string, any> | null} */ c, /** @type {string} */ axes) {
    const a = p._a, x = c ? c.rt : (a.x0 + a.x1) / 2, y = c ? c.mz : (a.y0 + a.y1) / 2;
    if (axes.includes("y")) p.zoomY = clampV(y - (y - a.y0) * f, y + (a.y1 - y) * f, a.fullY[0], a.fullY[1]);
    if (axes.includes("x")) p.zoom = clampV(x - (x - a.x0) * f, x + (a.x1 - x) * f, a.full[0], a.full[1]);
  }
  function zoomBy(/** @type {Pannello} */ p, /** @type {number} */ f, /** @type {Record<string, any> | null} */ c) {
    const a = p._a; if (!a) return; if (c && (c.rt < a.x0 || c.rt > a.x1 || c.mz < a.y0 || c.mz > a.y1)) c = null;
    pushZh(p); scaleView(p, f, c, "xy"); draw(p); uiSave();
  }
  // the picture follows the finger / the pointer by (dx, dy) pixels
  function panPx(/** @type {Pannello} */ p, /** @type {number} */ dx, /** @type {number} */ dy) {
    const a = p._a, w = a.W - M.l - M.r, h = a.H - M.t - M.b, sx = (a.x1 - a.x0) / w * dx, sy = (a.y1 - a.y0) / h * dy;
    p.zoom = clampV(a.x0 - sx, a.x1 - sx, a.full[0], a.full[1]); p.zoomY = clampV(a.y0 + sy, a.y1 + sy, a.fullY[0], a.fullY[1]);
  }
  // scale by f around the previous midpoint, then follow the midpoint to where it is now
  function pinch(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ last, /** @type {Record<string, any>} */ c, /** @type {number} */ f) {
    const a = p._a, w = a.W - M.l - M.r, h = a.H - M.t - M.b, rt = xOf(p, last.x), mz = a.mzAt(last.y);
    const x0 = rt - (rt - a.x0) * f, x1 = rt + (a.x1 - rt) * f, y0 = mz - (mz - a.y0) * f, y1 = mz + (a.y1 - mz) * f, sx = (x1 - x0) / w * (c.x - last.x), sy = (y1 - y0) / h * (c.y - last.y);
    p.zoom = clampV(x0 - sx, x1 - sx, a.full[0], a.full[1]); p.zoomY = clampV(y0 + sy, y1 + sy, a.fullY[0], a.fullY[1]);
  }
  // ---- the bar on the map: reset, zoom, lock, gestures (the same commands)
  // 2D: the reset is the «fit» button of the panel head
  /** @type {Record<string, string[]>} */ const BAR = { "2d": ["mapzin", "mapzout", "mapfeat", "maphelp"], "3d": ["mapreset", "mapzin", "mapzout", "maplock", "maphelp"] };
  // the buttons sit in the toolbar of the panel (2D: the head, with the same .bt look as the others) or in the header of the side column (3D); nothing floats over the data
  function barFor(/** @type {() => Record<string, any>} */ getAd, kind = "2d") {
    const b = document.createElement("span"); b.className = "tbg mznav"; b.setAttribute("role", "toolbar"); b.setAttribute("aria-label", T("mapnav.bar"));
    b.innerHTML = BAR[kind].map((/** @type {string} */ id) => `<button type="button"${kind === "2d" ? ' class="bt"' : ""} data-nav="${id}" title="${EH(T(`cmd.${id}.tip`))}" aria-label="${EH(T(`cmd.${id}.name`))}"${id === "maplock" || id === "mapfeat" ? ' aria-pressed="false"' : ""}>${SV(ICO[/** @type {any} */ (AZIONI.voce(id)).ic])}</button>`).join("");
    b.addEventListener("click", e => { const x = /** @type {any} */ (e.target).closest("button[data-nav]"); if (x) { e.stopPropagation(); runCmd(x.dataset.nav, getAd()); } });
    b.addEventListener("mousemove", e => e.stopPropagation());
    lockBtn(b, false); return b;
  }
  // the padlock button shows the state: closed + pressed = locked
  function lockBtn(/** @type {HTMLElement} */ bar, /** @type {boolean} */ on) {
    const b = bar && bar.querySelector('[data-nav="maplock"]'); if (!b) return;
    b.setAttribute("aria-pressed", String(on)); b.classList.toggle("on", on); b.innerHTML = SV(on ? ICO.lock : ICO.unlock);
    const nm = T(on ? "cmd.maplock.off.name" : "cmd.maplock.name"); b.title = nm; b.setAttribute("aria-label", nm);
  }
  const navBar = (/** @type {Pannello} */ p) => { const b = barFor(() => ad2(p, null)); st(p).nav = b; return b; };
  const navSync = (/** @type {Pannello} */ p) => { const s = st(p); lockBtn(s.nav, !!s.lock); const f = s.nav && s.nav.querySelector('[data-nav="mapfeat"]'); if (f) { f.hidden = !featAvail(p); f.setAttribute("aria-pressed", String(!!s.feat)); f.classList.toggle("on", !!s.feat); } };
  // ---- «Correzione» of the map: one at a time (none | a file | the background of a stretch | the automatic background), like the chromatograms and the spectrum.
  // p.corr: undefined = an old panel (converted at the first draw from p.ref and from the old global switch PREF.bg), "" = follow the reference of the session, "none", "file" (p.ref), "tract" (p.bgr = [rt0, rt1]), "auto".
  function corrInit(/** @type {Pannello} */ p) { if (p.corr === undefined) p.corr = p.ref !== "" && p.ref != null ? "file" : PREF.bg ? "auto" : ""; }
  function corrState(/** @type {Pannello} */ p) {
    corrInit(p); const se = p.corr === "" && window.SUBREF ? SUBREF.eff(p) : null;
    return { now: p.corr === "file" && p.ref !== "" && p.ref != null ? "f" + p.ref : se ? "f" + se.k : p.corr === "tract" ? "tract" : p.corr === "auto" ? "auto" : "" };
  }
  function setCorr(/** @type {Pannello} */ p, /** @type {string} */ v) {
    // the shared menu calls the automatic background «snip»
    corrInit(p); if (v === "snip") v = "auto";
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
  function corrQ(/** @type {Pannello} */ p) {
    corrInit(p);
    if (p.corr === "auto") return "&bg=1";
    if (p.corr === "tract" && p.bgr) return `&bgrt0=${+(+p.bgr[0]).toFixed(4)}&bgrt1=${+(+p.bgr[1]).toFixed(4)}`;
    return "";
  }
  // «Background of a stretch» chosen with no stretch yet: the next stretch selected with Alt + drag is the one
  function corrPick(/** @type {Pannello} */ p) {
    if (!p._pickBg || !p.sel || !(p.sel[1] > p.sel[0])) return false;
    p.bgr = [...p.sel]; p.sel = null; p._pickBg = false; ctl(p); uiSave(); return true;
  }
  // what is not at its default in «Options»: chips next to the button
  function optChips(/** @type {Pannello} */ p) {
    const o = [], d = diffOpt(corrState(p).now[0] === "f");
    if (p.scale && p.scale !== "lin") o.push(p.scale === "gamma" ? `${T("ctl.map.scale.gamma")} ${p.gamma || 0.6}` : T(`ctl.map.scale.${p.scale}`));
    if (PREF.pal === "cividis") o.push("cividis"); if (PREF.fix && corrState(p).now[0] !== "f") o.push(T("mappa.fix"));
    if (PREF.thr != null && Math.abs(PREF.thr - (corrState(p).now[0] === "f" ? 0.02 : 0.004)) > 1e-9) o.push(`${T("mappa.thr")} ${+(PREF.thr * 100).toFixed(1)} %`);
    if (corrState(p).now[0] === "f" && p.norm && p.norm !== "abs") o.push(T(`ctl.map.norm.${p.norm}`));
    if (d.show === "up" && corrState(p).now[0] === "f") o.push(T("mappa.show.upOnly"));
    return o;
  }
  // ---- the box «How it is extracted» (opened by the markers of mzFinder; the module loads the first time)
  async function explain(/** @type {Pannello} */ p, /** @type {boolean} */ on) {
    const s = st(p); if (on === undefined) on = !s.ex;
    if (!window.MAPEX) { try { window.MAPEX = await import("./mappa-estrai.js"); } catch (e) { console.error(e); nearMsg(T("mapex.err")); return; } }
    if (on) MAPEX.start(p); else MAPEX.stop(p);
    navSync(p);
  }
  // only the high-resolution world, only when mzFinder holds a result
  const featAvail = (/** @type {Pannello} */ p) => !!(p._a && p._a.f && window.MAPFEAT && window.HR && HR.isHr(p._a.f, p._a.f.lv || 1) && (MAPFEAT.features(p._a.f) || []).length);
  async function featToggle(/** @type {Pannello} */ p) {
    const s = st(p); if (!s.feat && !featAvail(p)) return;
    if (!window.MAPEX) { try { window.MAPEX = await import("./mappa-estrai.js"); } catch (e) { console.error(e); return; } }
    s.feat = !s.feat; if (!s.feat && window.MAPEX) MAPEX.featClear(p);
    navSync(p); frame(p);
  }
  // the panel is px taller while a block under the map is shown (0 = gone); the amount may change
  function grow(/** @type {Pannello} */ p, /** @type {string} */ key, /** @type {number} */ px) {
    const s = st(p), k = "g" + key, prev = s[k] || 0; if (px === prev || isMax(p)) return;
    s[k] = px; p.h = Math.max(200, p.h + px - prev); apply(p); if (typeof relayout === "function") relayout(); fitHost(); uiSave();
  }
  // ---- sheet of the gestures (one key per row: action|mouse|trackpad|touch|keyboard)
  /** @type {Record<string, string[]>} */ const HELP = { "2d": ["zoom", "zoomax", "box", "pan", "reset", "undo", "sight", "lock", "menu", "extract", "interval", "mark", "axis", "full"], "3d": ["rot", "pan", "zoom", "reset", "views", "pick", "menu", "cur", "axis"] };
  function help(/** @type {string} */ kind) {
    const rows = HELP[kind].map((/** @type {string} */ r) => I18N.t(`mapnav.h.${kind}.${r}`).split("|").map((/** @type {string} */ x) => EH(x.trim()) || "–"));
    const head = ["act", "mouse", "pad", "touch", "key"].map(k => `<th scope="col">${I18N.t(`mapnav.h.${k}`)}</th>`).join("");
    big(I18N.t(`mapnav.h.title.${kind}`), `<table class="mzhelp"><thead><tr>${head}</tr></thead><tbody>${rows.map((/** @type {string[]} */ r) => `<tr><th scope="row">${r[0]}</th>${r.slice(1).map((/** @type {string} */ c) => `<td>${c}</td>`).join("")}</tr>`).join("")}</tbody></table><p class="muted">${I18N.t(`mapnav.h.note.${kind}`)}</p>`);
  }
  // the 2D map of the active tab the palette acts on
  const activeMap = () => { const q = E.active; if (q && q.type === "map" && is2d(q) && E.panels.includes(q)) return q; return (typeof tabPanels === "function" ? tabPanels() : []).find((/** @type {Pannello} */ x) => x.type === "map" && is2d(x)) || null; };
  const paletteOk = (/** @type {Record<string, any>} */ c) => { const p = activeMap(); return !!p && c.viste.includes("map") && (!c.visibile || c.visibile(ad2(p, null))); };
  const paletteRun = (/** @type {string} */ id) => { const p = activeMap(); if (p) runCmd(id, ad2(p, null)); };
  // touch: one finger draws the zoom box, two fingers move and pinch, a double tap = whole view, a long press = the menu
  function touchInit(/** @type {Pannello} */ p, /** @type {HTMLCanvasElement} */ cv) {
    cv.style.touchAction = "none";
    /** @type {any} */ let g = null;
    let lastTap = /** @type {{ t: number, x: number, y: number } | null} */ (null), lp = 0;
    const at = (/** @type {Touch} */ t, /** @type {DOMRect} */ r) => ({ x: t.clientX - r.left, y: t.clientY - r.top });
    const cen = (/** @type {ArrayLike<Touch>} */ ts) => { const r = cv.getBoundingClientRect(), a = at(ts[0], r), b = ts[1] ? at(ts[1], r) : a; return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, d: Math.hypot(a.x - b.x, a.y - b.y), cx: (ts[0].clientX + (ts[1] || ts[0]).clientX) / 2, cy: (ts[0].clientY + (ts[1] || ts[0]).clientY) / 2 }; };
    // the box of the finger: the same element as the mouse box (eventi.js)
    const rubber = (/** @type {any} */ a, /** @type {any} */ b = a) => {
      const zr = /** @type {HTMLElement | null} */ (p.el.querySelector(".zr")); if (!zr) return;
      if (!a) { zr.hidden = true; return; }
      zr.hidden = false; zr.style.left = cv.offsetLeft + Math.min(a.x, b.x) + "px"; zr.style.width = Math.abs(b.x - a.x) + "px"; zr.style.top = cv.offsetTop + Math.min(a.y, b.y) + "px"; zr.style.height = Math.abs(b.y - a.y) + "px";
    };
    const inside = (/** @type {Record<string, any>} */ q) => { const a = p._a; return a && q.x >= M.l && q.x <= a.W - M.r && q.y >= M.t && q.y <= a.H - M.b; };
    cv.addEventListener("touchstart", e => {
      if (!is2d(p)) return; clearTimeout(lp); const c = cen(e.touches);
      if (e.touches.length >= 2) { g = { two: true, last: c, moved: true }; pushZh(p); return; }
      g = { two: false, last: c, start: c, moved: false, t0: Date.now() };
      // a finger on the box moves the box, not the map
      if (st(p).ex && window.MAPEX && inside(c)) g.box = MAPEX.touch(p, c.x, c.y);
      lp = setTimeout(() => { if (g && !g.moved) { g.moved = true; st(p).lpAt = Date.now(); showMenu(ad2(p, inside(c) ? { rt: xOf(p, c.x), mz: p._a.mzAt(c.y) } : null), { preventDefault() {}, clientX: c.cx, clientY: c.cy, lp: true }); } }, 550);
    }, { passive: true });
    cv.addEventListener("touchmove", e => {
      if (!g || !is2d(p)) return; const c = cen(e.touches), a = p._a;
      if (!g.moved && Math.hypot(c.x - g.start.x, c.y - g.start.y) < 8) return;
      if (g.box && e.touches.length === 1) { g.moved = true; st(p).gestAt = Date.now(); clearTimeout(lp); e.preventDefault(); g.box(c.x, c.y); return; }
      g.moved = true; st(p).gestAt = Date.now(); clearTimeout(lp); e.preventDefault();
      // one finger draws the zoom box
      if (!g.two && e.touches.length === 1) { g.rb = true; g.last = c; rubber(g.start, c); return; }
      if (g.two && e.touches.length >= 2 && g.last.d > 0 && c.d > 0) pinch(p, g.last, c, g.last.d / c.d);
      else panPx(p, c.x - g.last.x, c.y - g.last.y);
      g.last = c; draw(p);
    }, { passive: false });
    const end = (/** @type {TouchEvent} */ e) => {
      clearTimeout(lp); if (!g) return; const moved = g.moved, two = g.two, rb = g.rb && g.start, last = g.last; g = null;
      if (rb) { rubber(null); if (Math.abs(last.x - rb.x) > 4 || Math.abs(last.y - rb.y) > 4) boxZoom(p, rb.x, rb.y, last.x, last.y); st(p).gestAt = Date.now(); if (e.cancelable) e.preventDefault(); uiSave(); return; }
      if (e.touches.length) { if (e.cancelable) e.preventDefault(); g = { two: e.touches.length > 1, last: cen(e.touches), start: cen(e.touches), moved: true }; return; }
      // no mouse events after a gesture (they would be a box zoom)
      if (moved) { st(p).gestAt = Date.now(); if (e.cancelable) e.preventDefault(); uiSave(); return; }
      const now = Date.now(), c = e.changedTouches[0], r = cv.getBoundingClientRect(), q = at(c, r);
      if (lastTap && now - lastTap.t < 350 && Math.hypot(q.x - lastTap.x, q.y - lastTap.y) < 30) { lastTap = null; if (e.cancelable) e.preventDefault(); st(p).dtap = now; dbl(p, q.x); navSync(p); return; }
      lastTap = { t: now, x: q.x, y: q.y };
    };
    cv.addEventListener("touchend", end); cv.addEventListener("touchcancel", () => { clearTimeout(lp); g = null; });
  }

  // ---------------------------------------------------------------- buttons in the controls of the map: mark, snap, sight, 3D
  function decorate(/** @type {Pannello} */ p, /** @type {Record<string, any>} */ c) {
    if (p.type !== "map") return;
    const hasRef = corrState(p).now[0] === "f", o = diffOpt(hasRef), pal = PREF.pal === "cividis" ? "cividis" : "viridis";
    const opt = (/** @type {string} */ v, /** @type {string} */ cur, /** @type {string} */ lab) => `<option value="${v}" ${cur === v ? "selected" : ""}>${lab}</option>`;
    const col = c.querySelector('[data-mzslot="col"]'), dif = c.querySelector('[data-mzslot="diff"]');
    if (col) col.outerHTML = `<select data-mz="pal" title="${T("mappa.pal.title")}" aria-label="${T("mappa.pal.title")}">${opt("viridis", pal, T("mappa.pal.viridis"))}${opt("cividis", pal, T("mappa.pal.cividis"))}</select>` +
      `<label class="muted" title="${T("mappa.thr.title")}">${T("mappa.thr")} <input data-mz="thr" class="mzf" inputmode="decimal" value="${+(o.thr * 100).toFixed(1)}"> %</label>` +
      `<select data-mz="rend" title="${T("mappa.rend.title")}" aria-label="${T("mappa.rend.title")}">${opt("smooth", liscia() ? "smooth" : "detail", T("mappa.rend.smooth"))}${opt("detail", liscia() ? "smooth" : "detail", T("mappa.rend.detail"))}</select>` +
      `<select data-mz="dens" title="${T("mappa.dens.title")}" aria-label="${T("mappa.dens.title")}">${["few", "mid", "many"].map(k => opt(k, DENS[PREF.dens] ? PREF.dens : "mid", T(DENS[k].lab))).join("")}</select>`;
    if (dif) dif.outerHTML = !hasRef ? `<label class="muted" title="${T("mappa.fix.title")}"><input type="checkbox" data-mz="fix"${PREF.fix ? " checked" : ""}> ${T("mappa.fix")}</label>` : hasRef ? `<label class="muted" title="${T("mappa.show.title")}"><input type="checkbox" data-mz="neg"${PREF.neg === false ? "" : " checked"}> ${T("mappa.show.neg")}</label>` : "";
    const g = document.createElement("span"); g.className = "mzg"; g.setAttribute("role", "group"); g.setAttribute("aria-label", T("ctl.map.grp.tools"));
    g.innerHTML = `<span class="seg mzbt"><button data-mz="mark" class="${st(p).mark ? "on" : ""}" title="${T("mappa.btn.mark.title")}">${T("mappa.btn.mark")}</button><button data-mz="snap" class="${PREF.snap === false ? "" : "on"}" aria-pressed="${PREF.snap !== false}" title="${T("mappa.btn.snap.title")}">${T("mappa.btn.snap")}</button><button data-mz="mir" class="${mirOn(p) ? "on" : ""}" aria-pressed="${!!mirOn(p)}" title="${T("mappa.btn.mir.title")}">${T("mappa.btn.mir")}</button></span>` +
      `<button data-o="view" data-v="3d" title="${T("ctl.map.open3d.title")}" aria-label="${T("ctl.map.open3d.title")}">${T("ctl.map.open3d")}</button>`;
    c.appendChild(g);
    const Q = (/** @type {string} */ k) => /** @type {Record<string, any>} */ (c.querySelector(`[data-mz="${k}"]`)), num = (/** @type {HTMLInputElement} */ el) => numIt(el.value);
    Q("pal").onchange = (/** @type {Event & { target: HTMLInputElement }} */ ev) => { PREF.pal = ev.target.value; prefSave(); draw(p); };
    Q("rend").onchange = (/** @type {Event & { target: HTMLInputElement }} */ ev) => { PREF.rend = ev.target.value; prefSave(); draw(p); };
    Q("dens").onchange = (/** @type {Event & { target: HTMLInputElement }} */ ev) => { PREF.dens = ev.target.value; prefSave(); const sp = st(p).shown; if (sp && p._a) show(p, sp.pt); };
    Q("thr").onchange = (/** @type {Event & { target: HTMLInputElement }} */ ev) => { const v = num(ev.target); if (v >= 0 && v < 100) { PREF.thr = v / 100; prefSave(); } draw(p); };
    if (!hasRef) Q("fix").onchange = (/** @type {Event & { target: HTMLInputElement }} */ ev) => { PREF.fix = ev.target.checked; prefSave(); draw(p); };
    if (hasRef) Q("neg").onchange = (/** @type {Event & { target: HTMLInputElement }} */ ev) => { PREF.neg = ev.target.checked; prefSave(); draw(p); };
    const dc = c.querySelector("details.mzcol"); if (dc) dc.ontoggle = () => { p._colOpen = dc.open; };
    Q("mark").onclick = () => { const s = st(p); s.mark = !s.mark; markBtn(p); };
    Q("snap").onclick = (/** @type {Event & { currentTarget: HTMLElement }} */ ev) => { PREF.snap = PREF.snap === false; prefSave(); ev.currentTarget.classList.toggle("on", PREF.snap !== false); ev.currentTarget.setAttribute("aria-pressed", String(PREF.snap !== false)); };
    Q("mir").onclick = () => { const s = st(p); s.mir = !mirOn(p); layout(p); draw(p); };
  }

  function markBtn(/** @type {Pannello} */ p) { const b = p.el.querySelector('[data-mz="mark"]'); if (b) b.classList.toggle("on", !!st(p).mark); p.cv.style.cursor = st(p).mark ? "copy" : ""; }

    // «Open in 3D»: the viewer is loaded the first time it is asked for
  // what the student sees when the 3D viewer cannot open: the real cause in a sentence, what to try, and the technical lines (copyable); the 2D map is untouched
  function failure3d(/** @type {string} */ kind, /** @type {any} */ detail, /** @type {Record<string, any>} */ diag) {
    const k = ["nowebgl2", "shader", "import", "lost"].includes(kind) ? kind : "error", d = { kind, detail, ...diag }, esc = (/** @type {unknown} */ x) => EH(String(x == null ? "" : x));
    const tech = Object.entries(d).filter(([, v]) => v !== undefined && v !== "").map(([a, v]) => `${a}: ${typeof v === "object" ? JSON.stringify(v) : v}`).join("\n");
    // a half-built viewer must not stay on the page
    document.querySelectorAll("dialog.m3d").forEach(x => x.remove());
    big(T("m3d.err.title"), `<p>${I18N.t(`m3d.err.${k}`)}</p>${k === "nowebgl2" ? `<p>${T("m3d.err.how")}</p>` : ""}<p>${T("m3d.err.2d")}</p><pre class="m3d-tech" tabindex="0" aria-label="${T("m3d.err.tech")}">${esc(tech)}</pre>`);
  }
  async function open3d(/** @type {PannelloMappa} */ p, /** @type {HTMLElement} */ opener) {
    let mod; if (window.CAPACITA && !CAPACITA.serve("mappa3d", opener)) return; try { mod = await import("./mappa3d.js"); } catch (e_) { const e = /** @type {Lanciato} */ (e_); console.error(e); return failure3d("import", String(e && e.message || e), {}); }
    try { mod.open(p, opener); } catch (e_) { const e = /** @type {Lanciato} */ (e_);
      const k = e && e.kind ? e.kind : "error"; console.error("mzLab 3D:", k, e && e.detail || e, e && e.diag || "");
      failure3d(k, e && e.detail != null ? e.detail : String(e && e.message || e), e && e.diag || {});
    }
  }

  return { smooth: liscia, guide, dbl, open3d, addPointAt, grpShape, shapePath, spectrumAt, init, zoomImg, after, down, click, diffOpt, diffMap, pts, delPoint, move, leave, wheel, boxZoom, ctx, decorate, key, whole, prefs: PREF, prefSave, point, st, valueAt, CMDS, ICO, SV, corrState, corrInit, setCorr, corrQ, corrPick, optChips, explain, grow, redraw: frame, layoutNow: layout, touched, barFor, lockBtn, ptText, ad2, itemsFor, showMenu, runCmd, help, paletteOk, paletteRun, extract, peakStep, apexStep, madNoise, dropFile };
})();
if (window.BARRA && BARRA.late) BARRA.late();

// ---- panels: grips on the top and bottom edge (mouse, touch and pen), double click = default height; the width is always the column's
(() => {
  const H0 = 300, HMIN = 190, h0 = (/** @type {Pannello} */ p) => p.type === "map" ? 360 : H0;
  const css = document.createElement("style");
  css.textContent = "#dpanels .pnl{resize:none;max-width:100%}.pnl .pgrip{position:absolute;left:12px;right:12px;height:24px;z-index:5;cursor:ns-resize;touch-action:none}" +
    // 24 px box as before (WCAG 2.5.8, axe target-size) but clipped so that only 5 px of it, along the panel edge, take the pointer: the header stays draggable
    ".pnl .pgrip.t{top:-11px;clip-path:inset(0 0 8px 0)}.pnl .pgrip.b{bottom:-11px;clip-path:inset(8px 0 0 0)}.pnl .pgrip.t::after{top:12px}.pnl .pgrip.b::after{top:8px}.pnl .pgrip::after{content:'';position:absolute;left:50%;width:34px;height:4px;margin-left:-17px;border-radius:2px;background:var(--line);opacity:0;transition:opacity .15s}" +
    ".pnl:hover .pgrip::after,.pnl .pgrip:focus-visible::after,.pnl .pgrip.on::after{opacity:1}.pnl .pgrip:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}@media (pointer:coarse){.pnl .pgrip::after{opacity:.8}}" +
    "@media (prefers-reduced-motion:reduce){.pnl .pgrip::after{transition:none}}";
  document.head.appendChild(css);
  const host = () => document.getElementById("dpanels");
  const pOf = (/** @type {HTMLElement} */ el) => E.panels.find((/** @type {Pannello} */ q) => q.el === el);
  const simple = (/** @type {Pannello} */ p) => !(window.DDA && (DDA.docked(p) || E.panels.some((/** @type {Pannello} */ q) => q.dock === p.id && DDA.docked(q))));
  const above = (/** @type {Pannello} */ p) => { let best = null; for (const q of E.panels) if (q !== p && q.full && simple(q) && q.y < p.y && (!best || q.y > best.y)) best = q; return best; };
  const done = (/** @type {Pannello} */ p) => { if (p.full && typeof relayout === "function") { LAYOUT.altezza(p, E.panels); relayout(); } if (typeof fitHost === "function") fitHost(); if (typeof uiSave === "function") uiSave(); };
  function setH(/** @type {Pannello} */ p, /** @type {number} */ h) { p.h = Math.max(HMIN, Math.round(h)); apply(p); if (p.el) p.el.querySelectorAll(".pgrip").forEach((/** @type {Element} */ g) => g.setAttribute("aria-valuenow", String(p.h))); }
  function grip(/** @type {HTMLElement} */ el, /** @type {string} */ side) {
    const g = document.createElement("div"); g.className = "pgrip " + side; g.tabIndex = 0; g.setAttribute("role", "separator"); g.setAttribute("aria-orientation", "horizontal");
    // a focusable separator needs its value (the height of the panel)
    { const p0 = pOf(el); g.setAttribute("aria-valuemin", String(HMIN)); g.setAttribute("aria-valuemax", "2000"); g.setAttribute("aria-valuenow", String(p0 ? p0.h : el.offsetHeight)); }
    g.setAttribute("aria-label", I18N.t(side === "t" ? "mappa.rz.top" : "mappa.rz.bot")); g.title = I18N.t("mappa.rz.title");
    g.addEventListener("pointerdown", ev => {
      const p = pOf(el); if (!p || el.classList.contains("max") || ev.button > 0) return;
      ev.preventDefault(); ev.stopPropagation(); g.setPointerCapture(ev.pointerId); g.classList.add("on");
      const y0 = ev.clientY, h0 = p.h, top0 = p.y, bot0 = p.y + p.h, up = side === "t" ? above(p) : null, uh0 = up ? up.h : 0;
      const mv = (/** @type {PointerEvent} */ e) => {
        const d = e.clientY - y0;
        if (side === "b") setH(p, h0 + d);
        // splitter: what one gains the other loses
        else if (up) { const k = Math.max(-(h0 - HMIN), Math.min(uh0 - HMIN, d)); setH(up, uh0 + k); setH(p, h0 - k); p.y = up.y + up.h + (top0 - (up.y + uh0)); apply(p); }
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
  function wire(/** @type {Node} */ n) { const el = /** @type {HTMLElement & { _pgrip?: boolean }} */ (n); if (!el.classList || !el.classList.contains("pnl") || el._pgrip) return; el._pgrip = true; grip(el, "t"); grip(el, "b"); }
  function clamp() {
    const h = host(); if (!h) return; const w = h.clientWidth; if (w < 100) return;
    for (const p of E.panels) if (p.el && !p.el.classList.contains("max") && simple(p) && p.x + p.w > w + 1) { p.w = Math.max(300, w - p.x); if (p.x + p.w > w + 1) p.x = Math.max(0, w - p.w); apply(p); }
  }
  function start() {
    const h = host(); if (!h) return;
    h.querySelectorAll(".pnl").forEach(wire);
    new MutationObserver(ms => { for (const m of ms) m.addedNodes.forEach(wire); }).observe(h, { childList: true });
    addEventListener("resize", () => FRAME.next(clamp));
    FRAME.ro(h, () => FRAME.next(clamp));
  }
  window.MAPPA_PNL = { clamp, setH };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start); else start();
})();
