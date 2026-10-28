// @ts-strict
"use strict";
// Lens of the mass spectrum: passing over a peak (or choosing it with the keys or a tap) shows a small XIC of that ion, one file, with the tolerance of the
// button «XIC» (xic-spettro.js) and the line of the scan the spectrum shows. Transient: no state, no area, no integration. It sits in a fixed place, a top corner
// of the plot (the other one when the peak or the pointer is in it; the whole width, narrow plots), never at the pointer. A click or Enter on the lens
// makes it the XIC panel of the button (XICBTN.commit). Values come from api/xicbin as the engine gives them; the drawing keeps min and max of every pixel column.
// Classic helpers used by name: xicBin, J, UIP, E, M, ecss, fmt, ESC, TDATI, XICBTN.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  const W = 176, H = 96, EDGE = 2, CLEAR = 12, CACHE_MAX = 64;      // no title row and no DOM axis: the canvas holds the plot and its ticks; ion and file are the tooltip
  /** @typedef {{ rt: number[], y: number[] }} Trace */
  /** @typedef {{ m: number, req: { f: any, args: any[], text: string, canale?: string | null }, key: string }} Want */
  /** @type {WeakMap<any, any>} */
  const ST = new WeakMap();
  /** @type {Map<string, Trace>} least recently used last */
  const LRU = new Map();
  /** @type {any} */ let cur = null;                            // the panel whose lens is open
  let busy = false, seq = 0;
  /** @type {Want | null} */ let want = null;
  /** @type {any} */ let live = null;

  /** @param {any} p */
  function st(p) {
    let s = ST.get(p);
    if (s) return s;
    const el = document.createElement("div");
    el.className = "xl"; el.hidden = true; el.tabIndex = 0; el.setAttribute("role", "button");
    el.innerHTML = '<canvas aria-hidden="true"></canvas>';
    const cv = /** @type {HTMLCanvasElement} */ (el.querySelector("canvas")), dpr = window.devicePixelRatio || 1;
    cv.height = Math.round((H - 2) * dpr); cv.style.height = H - 2 + "px";
    s = { el, cv, m: null, key: "", dis: null, sticky: false, scan: "", tr: null, raf: 0, said: "", wired: false, text: "", file: "", w: W, ptr: null };
    ST.set(p, s); fit(s, W);
    p.cv.after(el);
    el.addEventListener("click", () => commitOf(p));
    el.addEventListener("keydown", e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); commitOf(p); } });
    el.addEventListener("mouseleave", e => { if (!s.sticky && !(e.relatedTarget instanceof Node && p.cv.contains(e.relatedTarget)) && e.relatedTarget !== p.cv) close(p); });
    TDATI.attach(el, () => tableOf(s));
    return s;
  }
  /** the width of the lens and of its canvas (the whole plot width on a narrow plot) @param {any} s @param {number} w */
  function fit(s, w) {
    if (s.w === w && s.cv.width) return;
    const dpr = window.devicePixelRatio || 1; s.w = w;
    s.el.style.width = w + "px"; s.cv.width = Math.round(w * dpr); s.cv.style.width = w + "px";
  }
  /**
   * The fixed place of the lens: the top right corner of the plot, the top left one when the peak (its apex) or the pointer would be under it, the whole width
   * below the top edge when the plot is too narrow for two lenses. Nothing moves in the layout: it floats over the plot.
   * @param {any} p @param {any} s @param {number} m @param {{ mz: number[], y: number[] }} d0
   */
  function place(p, s, m, d0) {
    const a = p._a, plotW = a.W - M.l - M.r;
    let left = 0, w = W;
    if (plotW < 2 * W + 3 * EDGE + CLEAR) { w = Math.max(W, p.cv.offsetWidth - 2 * EDGE); left = EDGE; }
    else {
      let j = -1; d0.mz.forEach((v, i) => { if (Math.abs(v - m) < 0.6 && (j < 0 || Math.abs(v - m) < Math.abs(d0.mz[j] - m))) j = i; });
      /** @type {[number, number][]} */ const pts = [[a.X(m), j < 0 ? 0 : a.Y(d0.y[j])]];
      if (s.ptr) pts.push(s.ptr);
      /** @param {number} x0 */
      const hit = x0 => pts.some(([x, y]) => x >= x0 - CLEAR && x <= x0 + W + CLEAR && y <= EDGE + H + CLEAR);
      const right = a.W - M.r - W - EDGE, lft = M.l + EDGE;
      left = !hit(right) || hit(lft) ? right : lft;
    }
    fit(s, w);
    s.el.style.left = p.cv.offsetLeft + left + "px"; s.el.style.top = p.cv.offsetTop + EDGE + "px";
  }
  /** @param {any} p */
  function commitOf(p) { const s = ST.get(p); if (!s || s.m == null) return; const m = s.m; close(p); XICBTN.commit(p, m); }

  // ---------------------------------------------------------------- data: one request in flight, the last wish wins, 64 traces kept
  /** @param {string} key @returns {Trace | undefined} */
  function cached(key) { const t = LRU.get(key); if (t) { LRU.delete(key); LRU.set(key, t); } return t; }
  /** @param {string} key @param {Trace} t */
  function keep(key, t) { LRU.delete(key); LRU.set(key, t); while (LRU.size > CACHE_MAX) LRU.delete(/** @type {string} */ (LRU.keys().next().value)); }
  /** asks for the trace of the wish; a vanished wish discards the answer @param {any} p */
  function pump(p) {
    if (busy || !want || cur !== p) return;
    const w = want, s = ST.get(p), t = cached(w.key);
    if (t) { show(p, s, w, t); return; }
    busy = true; const my = ++seq;
    const [mz, tol] = w.req.args, f = w.req.f;
    const can = w.req.canale;      // inside the channel the spectrum follows (canali.js), otherwise the whole level
    (can ? J(`api/xic?k=${f.k}&mz=${mz}&tol=${tol}&level=${f.lv}&canale=${encodeURIComponent(can)}`).then(j => j.traces ? j.traces[0] : j) : xicBin(`api/xicbin?k=${f.k}&mz=${mz}&tol=${tol}&level=${f.lv}`).then(j => j.traces[0])).then(tr => {
      busy = false; if (!tr) return;
      keep(w.key, tr);
      if (my === seq && want === w && cur === p) show(p, s, w, tr, true); else pump(p);
    }, () => { busy = false; if (want === w) close(p); });
  }
  /** a fresh response is drawn at once (it is not in the pointer handler, and waiting for the next frame would add one to the preview); a cached trace waits for the frame @param {any} p @param {any} s @param {Want} w @param {Trace} t @param {boolean} [now] */
  function show(p, s, w, t, now) {
    s.tr = t; s.m = w.m; s.text = w.req.text; s.file = w.req.f.label || "";
    place(p, s, w.m, p._a.data[0].d);
    s.el.title = withFile(s);
    s.el.hidden = false;
    FRAME.cancel(s.raf);
    if (now) paint(p, s); else s.raf = FRAME.next(() => paint(p, s));
  }

  // ---------------------------------------------------------------- drawing
  /** the apex of a trace (first maximum) @param {Trace} t */
  const apex = XICDRAW.apex;
  /** the apex is worth a time label: at least 3 times the median and wider than one point (more than one point at half height) @param {Trace} t @param {number} b */
  function clearPeak(t, b) {
    const top = t.y[b]; if (!(top > 0) || t.rt.length < 3) return false;
    const med = [...t.y].sort((u, v) => u - v)[t.y.length >> 1];
    return top >= 3 * med && t.y.filter(v => v >= top / 2).length > 1;
  }
  /** @param {any} p @param {any} s */
  function paint(p, s) {
    const t = /** @type {Trace | null} */ (s.tr); if (!t || cur !== p) return;
    const cv = /** @type {HTMLCanvasElement} */ (s.cv), g = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d")), dpr = cv.width / s.w;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    const b = apex(t);
    s.cv.title = "";
    if (!t.rt.length) { g.clearRect(0, 0, s.w, H); g.fillStyle = ecss("--muted"); g.font = "10px sans-serif"; g.fillText(T("xiclens.empty"), 6, H / 2); return; }
    const label = clearPeak(t, b);
    XICDRAW.draw(g, s.w, H - 2, t, { line: ecss("--line"), ink: ecss("--ink"), muted: ecss("--muted"), accent: ecss("--accent"), scanRt: scanRt(p), label });
    if (label) s.cv.title = T("xiclens.rt");
    const msg = T("xiclens.say", { ion: withFile(s), rt: t.rt[b].toFixed(2), y: fmt(t.y[b]) });
    s.el.setAttribute("aria-label", msg + " " + T("xiclens.keys"));
    if (msg !== s.said) { s.said = msg; announce(msg, s.sticky); }
  }
  /** the ion with its file, for the screen reader and the table (the file is cut with an ellipsis on the lens) @param {any} s */
  const withFile = s => s.file ? `${s.text} · ${s.file}` : s.text;
  /** the RT of the scan the spectrum shows @param {any} p */
  const scanRt = p => p.r0 != null && p.r1 != null ? (p.r0 + p.r1) / 2 : null;
  /** one short sentence for the screen reader, only for a peak chosen with the keys or a tap (hovering would read at every move) @param {string} msg @param {boolean} on */
  function announce(msg, on) {
    if (!on) return;
    if (!live) { live = document.createElement("div"); live.className = "sr-only"; live.setAttribute("role", "status"); live.setAttribute("aria-live", "polite"); document.body.appendChild(live); }
    live.textContent = msg;
  }
  /** the table of the data: RT and intensity of the trace as the engine gave it @param {any} s */
  function tableOf(s) {
    const t = /** @type {Trace | null} */ (s.tr); if (!t) return null;
    return { title: T("xiclens.table", { ion: withFile(s) }), head: [T("xiclens.col.rt"), T("xiclens.col.y")], rows: t.rt.map((r, i) => [TDATI.num(r, 4), TDATI.num(t.y[i], 1)]), rowHeader: false };
  }

  // ---------------------------------------------------------------- hooks (eventi.js, xic-spettro.js)
  /** the scan window and the zoom of the spectrum: when they change the lens goes @param {any} p */
  const viewKey = p => `${p.r0}|${p.r1}|${p._a ? p._a.x0 + "|" + p._a.x1 + "|" + p._a.W : ""}`;
  /** @param {any} p @param {number} m @param {boolean} sticky */
  function open(p, m, sticky) {
    const req = XICBTN.lensArgs(p, m); if (!req) return;
    const s = st(p);
    if (!s.wired) { s.wired = true; p.cv.addEventListener("wheel", () => close(p), { passive: true }); p.cv.addEventListener("contextmenu", () => close(p)); }
    const key = `${req.f.k}|${req.f.lv}|${req.args.join("|")}|${req.canale || ""}`;
    if (cur && cur !== p) close(cur);
    cur = p; s.sticky = sticky; s.scan = viewKey(p);
    if (!want || want.key !== key) { want = { m, req, key }; s.said = ""; }
    pump(p);
  }
  /** the pointer moves over the spectrum: the lens follows the peak under it, only when the peak changes @param {any} p @param {number} px @param {number} py @param {boolean} dragging */
  function move(p, px, py, dragging) {
    if (!UIP.xicLens || p._exp || dragging || p.rul || (p.meas && p.meas.ref != null) || !p._a) { if (cur === p && !ST.get(p)?.sticky) close(p); return; }
    const a = p._a, inBox = px >= M.l && px <= a.W - M.r && py >= M.t && py <= a.H - M.b;
    const s = ST.get(p), over = s && !s.el.hidden && px >= s.el.offsetLeft - p.cv.offsetLeft && px <= s.el.offsetLeft - p.cv.offsetLeft + s.w && py >= s.el.offsetTop - p.cv.offsetTop && py <= s.el.offsetTop - p.cv.offsetTop + H;
    if (over) return;
    if (s) s.ptr = [px, py];
    const m = inBox ? XICBTN.target(p, px) : null;
    if (m == null) { if (s && s.dis != null) s.dis = null; if (cur === p && !(s && s.sticky)) close(p); return; }
    if (s && s.dis === m) return;
    if (cur === p && s && s.m === m && !s.el.hidden) return;
    if (cur === p && want && want.m === m) return;
    open(p, m, false);
  }
  /** the pointer left the spectrum: the lens stays only when it went onto the lens @param {any} p @param {MouseEvent} e */
  function leave(p, e) {
    const s = ST.get(p); if (!s) return;
    s.dis = null;
    if (e.relatedTarget instanceof Node && s.el.contains(e.relatedTarget)) return;
    if (!s.sticky) close(p);
  }
  /** a peak chosen with the keys, a tap or a click: its lens stays until Esc, another peak, a new scan or a zoom @param {any} p @param {number} m */
  function showChosen(p, m) {
    if (!UIP.xicLens || p._exp) return;
    const s = ST.get(p);
    if (s && cur === p && (s.m === m || (want && want.m === m))) return;            // the lens of the hover already shows it: a click keeps it as it is
    const s2 = st(p); s2.ptr = null;
    open(p, m, true);
  }
  /** @param {any} p */
  function sync(p) {
    const s = ST.get(p); if (!s || s.el.hidden) return;
    if (s.scan !== viewKey(p)) close(p);
  }
  /** @param {any} p @param {boolean} [esc] Esc: the same peak does not reopen it until the pointer goes to another one */
  function close(p, esc) {
    const s = ST.get(p); if (!s) return;
    s.dis = esc ? (s.m ?? (want && want.m)) : null; FRAME.cancel(s.raf);
    if (!s.el.hidden) { const had = s.el.contains(document.activeElement); s.el.hidden = true; if (had) p.cv.focus({ preventScroll: true }); }
    s.sticky = false; s.m = null; s.tr = null;
    if (cur === p) { cur = null; want = null; seq++; }
  }
  const closeAll = () => { if (cur) close(cur); };

  ESC.registra({ id: "xic-lente", prio: 28, aperto: () => !!cur && !!ST.get(cur) && !ST.get(cur).el.hidden, chiudi: () => { if (cur) close(cur, true); } });
  /** @type {any} */ (window).XICLENS = { move, leave, show: showChosen, sync, close, closeAll, columns: XICDRAW.columns, apex, clearPeak, cacheSize: () => LRU.size, clear: () => LRU.clear(), trace: (/** @type {any} */ p) => { const s = ST.get(p); return s && !s.el.hidden ? s.tr : null; }, shown: (/** @type {any} */ p) => { const s = ST.get(p); return s && !s.el.hidden ? s.m : null; } };
})();
