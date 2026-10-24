// @ts-check
"use strict";
// Lens of the mass spectrum: passing over a peak (or choosing it with the keys or a tap) shows a small XIC of that ion, one file, with the tolerance of the
// button «XIC» (xic-spettro.js) and the line of the scan the spectrum shows. Transient: no state, no area, no integration. A click or Enter on the lens
// makes it the XIC panel of the button (XICBTN.commit). Values come from api/xicbin as the engine gives them; the drawing keeps min and max of every pixel column.
// Classic helpers used by name: xicBin, UIP, E, M, ecss, fmt, ESC, TDATI, XICBTN.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  const W = 176, H = 100, LAB = 16, GAP = 10, CACHE_MAX = 64;
  /** @typedef {{ rt: number[], y: number[] }} Trace */
  /** @typedef {{ m: number, req: { f: any, args: any[], text: string }, key: string }} Want */
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
    el.innerHTML = '<div class="xl-t"></div><canvas aria-hidden="true"></canvas>';
    const cv = /** @type {HTMLCanvasElement} */ (el.querySelector("canvas")), dpr = window.devicePixelRatio || 1;
    cv.width = Math.round(W * dpr); cv.height = Math.round((H - LAB) * dpr); cv.style.width = W + "px"; cv.style.height = H - LAB + "px";
    s = { el, cv, lab: el.firstElementChild, m: null, key: "", dis: null, sticky: false, scan: "", tr: null, raf: 0, said: "", wired: false, text: "" };
    ST.set(p, s);
    p.cv.after(el);
    el.addEventListener("click", () => commitOf(p));
    el.addEventListener("keydown", e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); commitOf(p); } });
    el.addEventListener("mouseleave", e => { if (!s.sticky && !(e.relatedTarget instanceof Node && p.cv.contains(e.relatedTarget)) && e.relatedTarget !== p.cv) close(p); });
    TDATI.attach(el, () => tableOf(s));
    return s;
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
    xicBin(`api/xicbin?k=${f.k}&mz=${mz}&tol=${tol}&level=${f.lv}`).then(j => j.traces[0]).then(tr => {
      busy = false; if (!tr) return;
      keep(w.key, tr);
      if (my === seq && want === w && cur === p) show(p, s, w, tr, true); else pump(p);
    }, () => { busy = false; if (want === w) close(p); });
  }
  /** a fresh response is drawn at once (it is not in the pointer handler, and waiting for the next frame would add one to the preview); a cached trace waits for the frame @param {any} p @param {any} s @param {Want} w @param {Trace} t @param {boolean} [now] */
  function show(p, s, w, t, now) {
    s.tr = t; s.m = w.m; s.text = w.req.text;
    const a = p._a, px = a && a.X ? a.X(w.m) : M.l;
    const right = px + GAP + W <= a.W - M.r, left = right ? px + GAP : Math.max(M.l, px - GAP - W);
    // level with the top of the peak, so that the pointer reaches the lens by a horizontal move that crosses no other peak
    const d0 = a.data[0].d; let j = -1;
    d0.mz.forEach((/** @type {number} */ v, /** @type {number} */ i) => { if (Math.abs(v - w.m) < 0.6 && (j < 0 || Math.abs(v - w.m) < Math.abs(d0.mz[j] - w.m))) j = i; });
    const lo = M.t + 4, hi = Math.max(lo, a.H - M.b - H), top = Math.min(hi, Math.max(lo, (j < 0 ? lo : a.Y(d0.y[j])) - 24));
    s.el.style.left = p.cv.offsetLeft + left + "px"; s.el.style.top = p.cv.offsetTop + top + "px";
    s.el.hidden = false;
    FRAME.cancel(s.raf);
    if (now) paint(p, s); else s.raf = FRAME.next(() => paint(p, s));
  }

  // ---------------------------------------------------------------- drawing
  /** the apex of a trace (first maximum) @param {Trace} t */
  function apex(t) { let b = 0; for (let i = 1; i < t.y.length; i++) if (t.y[i] > t.y[b]) b = i; return b; }
  /** the points to draw for pixel columns: first, lowest, highest and last of every column, in the order of the data (no apex is lost) @param {Trace} t @param {number} pw @returns {number[]} */
  function columns(t, pw) {
    const n = t.rt.length, x0 = t.rt[0], span = t.rt[n - 1] - x0 || 1, out = [];
    let c = -1, f = 0, lo = 0, hi = 0;
    /** @param {number} l */
    const flush = l => { for (const i of [...new Set([f, lo, hi, l])].sort((u, v) => u - v)) out.push(i); };
    for (let i = 0; i < n; i++) {
      const k = Math.min(pw - 1, Math.floor((t.rt[i] - x0) / span * (pw - 1)));
      if (k !== c) { if (c >= 0) flush(i - 1); c = k; f = lo = hi = i; }
      else { if (t.y[i] < t.y[lo]) lo = i; if (t.y[i] > t.y[hi]) hi = i; }
    }
    if (c >= 0) flush(n - 1);
    return out;
  }
  /** @param {any} p @param {any} s */
  function paint(p, s) {
    const t = /** @type {Trace | null} */ (s.tr); if (!t || cur !== p) return;
    s.lab.textContent = s.text;
    const cv = /** @type {HTMLCanvasElement} */ (s.cv), g = /** @type {CanvasRenderingContext2D} */ (cv.getContext("2d")), dpr = cv.width / W, h = H - LAB;
    g.setTransform(dpr, 0, 0, dpr, 0, 0); g.clearRect(0, 0, W, h);
    const n = t.rt.length, L = 6, R = 6, B = 14, Tp = 6, pw = W - L - R, ph = h - B - Tp;
    g.font = "10px sans-serif"; g.textBaseline = "alphabetic";
    g.strokeStyle = ecss("--line"); g.lineWidth = 1; g.beginPath(); g.moveTo(L, Tp + ph + .5); g.lineTo(L + pw, Tp + ph + .5); g.stroke();
    if (!n) { g.fillStyle = ecss("--muted"); g.fillText(T("xiclens.empty"), L, Tp + ph / 2); return; }
    const x0 = t.rt[0], x1 = t.rt[n - 1], span = x1 - x0 || 1;
    let ym = 0; for (let i = 0; i < n; i++) if (t.y[i] > ym) ym = t.y[i];
    const X = (/** @type {number} */ v) => L + (v - x0) / span * pw, Y = (/** @type {number} */ v) => Tp + ph - (ym > 0 ? v / ym : 0) * ph;
    g.fillStyle = ecss("--muted"); g.textAlign = "left"; g.fillText(String(+x0.toFixed(1)), L, h - 2); g.textAlign = "right"; g.fillText(String(+x1.toFixed(1)) + " min", L + pw, h - 2);
    const rt = scanRt(p);
    if (rt != null && rt >= x0 && rt <= x1) {
      g.save(); g.strokeStyle = ecss("--ink"); g.setLineDash([3, 3]); g.beginPath(); g.moveTo(X(rt), Tp); g.lineTo(X(rt), Tp + ph); g.stroke(); g.restore();
    }
    g.strokeStyle = ecss("--accent"); g.lineWidth = 1.5; g.lineJoin = "round"; g.beginPath();
    columns(t, pw).forEach((i, k) => { if (k) g.lineTo(X(t.rt[i]), Y(t.y[i])); else g.moveTo(X(t.rt[i]), Y(t.y[i])); });
    g.stroke();
    const b = apex(t);
    g.beginPath(); g.arc(X(t.rt[b]), Y(t.y[b]), 3, 0, 7); g.lineWidth = 2; g.strokeStyle = ecss("--panel"); g.stroke(); g.fillStyle = ecss("--accent"); g.fill();
    const msg = T("xiclens.say", { ion: s.text, rt: t.rt[b].toFixed(2), y: fmt(t.y[b]) });
    s.el.setAttribute("aria-label", msg + " " + T("xiclens.keys"));
    if (msg !== s.said) { s.said = msg; announce(msg, s.sticky); }
  }
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
    return { title: T("xiclens.table", { ion: s.text }), head: [T("xiclens.col.rt"), T("xiclens.col.y")], rows: t.rt.map((r, i) => [TDATI.num(r, 4), TDATI.num(t.y[i], 1)]), rowHeader: false };
  }

  // ---------------------------------------------------------------- hooks (eventi.js, xic-spettro.js)
  /** the scan window and the zoom of the spectrum: when they change the lens goes @param {any} p */
  const viewKey = p => `${p.r0}|${p.r1}|${p._a ? p._a.x0 + "|" + p._a.x1 + "|" + p._a.W : ""}`;
  /** @param {any} p @param {number} m @param {boolean} sticky */
  function open(p, m, sticky) {
    const req = XICBTN.lensArgs(p, m); if (!req) return;
    const s = st(p);
    if (!s.wired) { s.wired = true; p.cv.addEventListener("wheel", () => close(p), { passive: true }); }
    const key = `${req.f.k}|${req.f.lv}|${req.args.join("|")}`;
    if (cur && cur !== p) close(cur);
    cur = p; s.sticky = sticky; s.scan = viewKey(p);
    if (!want || want.key !== key) { want = { m, req, key }; s.said = ""; }
    pump(p);
  }
  /** the pointer moves over the spectrum: the lens follows the peak under it, only when the peak changes @param {any} p @param {number} px @param {number} py @param {boolean} dragging */
  function move(p, px, py, dragging) {
    if (!UIP.xicLens || p._exp || dragging || p.rul || (p.meas && p.meas.ref != null) || !p._a) { if (cur === p && !ST.get(p)?.sticky) close(p); return; }
    const a = p._a, inBox = px >= M.l && px <= a.W - M.r && py >= M.t && py <= a.H - M.b;
    const s = ST.get(p), over = s && !s.el.hidden && px >= s.el.offsetLeft - p.cv.offsetLeft && px <= s.el.offsetLeft - p.cv.offsetLeft + W && py >= s.el.offsetTop - p.cv.offsetTop && py <= s.el.offsetTop - p.cv.offsetTop + H;
    if (over) return;
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
  /** @type {any} */ (window).XICLENS = { move, leave, show: showChosen, sync, close, closeAll, columns, apex, cacheSize: () => LRU.size, clear: () => LRU.clear(), trace: (/** @type {any} */ p) => { const s = ST.get(p); return s && !s.el.hidden ? s.tr : null; }, shown: (/** @type {any} */ p) => { const s = ST.get(p); return s && !s.el.hidden ? s.m : null; } };
})();
