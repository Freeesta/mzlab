// @ts-check
"use strict";
// Scan-by-scan walk with the arrow keys (chromatogram cursor -> linked spectrum), as smooth as possible:
//  - a cache of single scans (api/spectra, blocks of at most 60) so a step never waits for the server;
//  - frozen intensity axis of the spectrum while walking (lock button), the y axis only grows;
//  - one spectrum redraw per frame (drawSoon), the cursor line is a thin overlay (cursorLine);
//  - held key = about 15 scans/s, Maiusc = 5 scans, Space = play/pause. Classic script, loaded after explore.js.

// ---- cache of single scans: key = file | level | precursor | scan index (position in the chromatogram of the same selection)
const SC = { m: new Map(), n: new Map(), fl: new Map(), max: 400 };
const SC_AHEAD = 20;
const scBase = (k, lv, pr) => `${k}|${lv}|${pr ?? ""}`;
function scGet(k, lv, pr, i) {
  const key = scBase(k, lv, pr) + "|" + i, v = SC.m.get(key);
  if (v) { SC.m.delete(key); SC.m.set(key, v); }                  // the oldest one is the first to leave
  return v;
}
function scFetch(k, lv, pr, i0, i1) {
  const b = scBase(k, lv, pr), n = SC.n.get(b);
  i0 = Math.max(0, Math.floor(i0)); i1 = Math.floor(i1); if (n != null) i1 = Math.min(i1, n - 1);
  while (i0 <= i1 && SC.m.has(b + "|" + i0)) i0++;
  while (i1 >= i0 && SC.m.has(b + "|" + i1)) i1--;
  if (i0 > i1) return Promise.resolve();
  if (i1 - i0 > 59) i1 = i0 + 59;
  const fk = `${b}|${i0}|${i1}`; if (SC.fl.has(fk)) return SC.fl.get(fk);
  const f = E.files[k], q = `k=${k}&i0=${i0}&i1=${i1}&level=${lv}&prec=${prSplit(pr)[0]}${FT(prSplit(pr)[1])}` + MERGE();
  // high resolution: a binary block (m/z float64, intensity float32), not a JSON text of 1-3 MB; any other file keeps the JSON of always
  const pm = (window.HR && f && HR.isHr(f, lv) ? scBin(`api/scanbin?${q}`) : J(`api/spectra?${q}`)).then(j => {
    SC.n.set(b, j.n);
    for (const s of j.scans) SC.m.set(b + "|" + s.i, s);
    while (SC.m.size > SC.max) SC.m.delete(SC.m.keys().next().value);
  }).finally(() => SC.fl.delete(fk));
  SC.fl.set(fk, pm); return pm;
}
// api/scanbin: 4 bytes = length of the JSON header, the header {n, scans:[{i, sid, rt, mode, n, np}]}, then for each scan peaks (float64 m/z, float32 y) and, for profile, the line (pmz, py)
async function scBin(url) {
  const r = await fetch(url);
  if (!r.ok) { let m = new Error(I18N.t("err.http", { status: r.status })); try { const j = await r.json(); if (j && j.error) m = I18N.err(j); } catch (e) { /* not JSON */ } throw m; }
  const buf = await r.arrayBuffer(), hl = new DataView(buf).getUint32(0, true), head = JSON.parse(new TextDecoder().decode(new Uint8Array(buf, 4, hl)));
  let off = 4 + hl;
  const arr = (T, n) => { const a = Array.from(new T(buf.slice(off, off + n * T.BYTES_PER_ELEMENT))); off += n * T.BYTES_PER_ELEMENT; return a; };      // slice: an array view needs an aligned offset
  const scans = head.scans.map(m => { const mz = arr(Float64Array, m.n), y = arr(Float32Array, m.n), o = { i: m.i, sid: m.sid, rt: m.rt, mode: m.mode, mz, y, no: m.no, filter: m.filter, nl: m.nl, tic: m.tic };
    if (m.np) { o.pmz = arr(Float64Array, m.np); o.py = arr(Float32Array, m.np); } return o; });
  return { n: head.n, scans };
}
// the spectrum of scan p.si: from the cache, else one block around it in the direction of the walk, else (any problem) the window request of before
async function scData(p, k) {
  let s = scGet(k, p.level, prk(p.prec, p.filt), p.si);
  if (!s) {
    const back = p._dir < 0;
    await scFetch(k, p.level, prk(p.prec, p.filt), p.si - (back ? 40 : 2), p.si + (back ? 2 : 40)).catch(() => {});
    s = scGet(k, p.level, prk(p.prec, p.filt), p.si);
  }
  if (s && s.mz.length && s.rt >= p.r0 - 1e-6 && s.rt <= p.r1 + 1e-6) return { mz: s.mz, y: s.y, mode: s.mode, pmz: s.pmz, py: s.py, scans: 1, i0: s.i, n: SC.n.get(scBase(k, p.level, prk(p.prec, p.filt))) };
  return getSpec(k, p.r0, p.r1, p.level, prk(p.prec, p.filt), null);
}
// keep the next scans in the cache before they are needed (nothing is waited for)
function scAhead(s) {
  if (s.si == null || (s.bg !== "" && s.bg != null)) return;
  const dir = s._dir || 1;
  const pk = prk(s.prec, s.filt);
  if (SC.m.has(scBase(s.k, s.level, pk) + "|" + (s.si + dir * 12))) return;
  (dir > 0 ? scFetch(s.k, s.level, pk, s.si + 1, s.si + 40) : scFetch(s.k, s.level, pk, s.si - 40, s.si - 1)).catch(() => {});
}

// ---- frozen intensity axis while walking (the m/z axis is always fixed, see drawSpec). s.lock = {ymax, z (zoom when locked), k (file), fresh}; {pending:true} = to be measured at the next draw
/** @type {(...a: any[]) => void} */
function lockStart() { /* the lock never closes by itself (7 Oct 2026, Federico): only the student closes it, with the lock button */ }
// y = the highest peak of +-20 scans in the visible m/z range (from the cache) x 1.12, and it can only grow (no cut peaks)
function lockSync(p, k, x0a, x1a, ymaxA, inRange, one) {
  const lk = p.lock; if (!lk) return null;
  const zk = p.zoom ? p.zoom.join() : "";
  const nb = (lo, hi) => {
    let m = 0;
    if (one) for (let j = p.si - SC_AHEAD; j <= p.si + SC_AHEAD; j++) { const c = SC.m.get(scBase(k, p.level, prk(p.prec, p.filt)) + "|" + j); if (c) c.y.forEach((v, i) => { if (v > m && c.mz[i] >= lo && c.mz[i] <= hi) m = v; }); }
    return m * 1.12;
  };
  if (lk.ymax == null) { Object.assign(lk, { z: zk, k, pending: false, ymax: Math.max(ymaxA, nb(x0a, x1a)) }); return lk; }
  if (lk.k !== k) { lk.k = k; return lk; }
  if (zk !== lk.z) { Object.assign(lk, { z: zk, ymax: Math.max(ymaxA, nb(x0a, x1a)) }); return lk; }   // zoom changed by the student: new top for the new m/z range
  if (lk.fresh) { lk.fresh = false; lk.ymax = Math.max(lk.ymax, ymaxA, nb(x0a, x1a)); return lk; }
  const need = inRange(x0a, x1a) * 1.12; if (need > lk.ymax) lk.ymax = need;
  return lk;
}
function lockButton(p) {
  const b = p.el && p.el.querySelector('[data-a="lock"]'); if (!b) return;
  const on = !!p.lock;
  b.innerHTML = on ? IC_LOCK : IC_UNLOCK; b.classList.toggle("on", on);
  b.style.left = p.cv.offsetLeft + 8 + "px"; b.style.top = p.cv.offsetTop - 10 + "px"; b.hidden = !!p._exp;       // in the top-left margin above the y-axis label ("Intensità (cps)")
  b.title = I18N.t(on ? "lock.on.title" : "lock.off.title");
}
function toggleLock(p) {
  if (p.lock) p.lock = null;
  else { const a = p._a; p.zoomY = null; p.lock = a ? { ymax: a.ymax, z: p.zoom ? p.zoom.join() : "", k: a.data[0].f.k, fresh: p.si != null } : { pending: true }; }
  lockButton(p); draw(p); uiSave();
}

// ---- cheap redraws
// several requests in a frame = one drawing; while a drawing is still waiting for its data the next one is remembered (one, not a queue) and made when it ends
function drawSoon(p) {
  if (p._ds) return;
  p._ds = FRAME.next(async () => {
    p._ds = 0; if (p._dr) { p._dq = true; return; }
    p._dr = true; try { await draw(p); } finally { p._dr = false; if (p._dq) { p._dq = false; drawSoon(p); } }
  });
}
// one pace for the held arrow and for Space: STEP_MS between scans on a fixed timetable (not "at least that long since the last frame"), a late frame does not make a burst
const STEP_MS = 66;
const paceDue = (o, now) => { if (now - o.t < STEP_MS) return false; o.t = now - o.t > 4 * STEP_MS ? now : o.t + STEP_MS; return true; };
function cursorLine(p) {                                           // the vertical cursor of a chromatogram: a thin overlay, not part of the canvas, with a dot on top that can be dragged
  const c = p.cl, a = p._a, h = p.ch; if (!c) return;
  if (p.tl && p.cur != null && !p._cs && isLine(p)) tabPanels(p.tab).forEach(q => { if (q !== p && q.tl && isLine(q) && q.cur !== p.cur) { q.cur = p.cur; q._cs = true; cursorLine(q); q._cs = false; } });      // one cursor for the linked chromatograms
  if (!a || !a.X || p.type === "spec" || p.type === "map" || p.cur == null || p.cur < a.x0 || p.cur > a.x1) { c.hidden = true; if (h) h.hidden = true; return; }
  c.hidden = false; c.style.left = p.cv.offsetLeft + a.X(p.cur) + "px"; c.style.top = p.cv.offsetTop + M.t + "px"; c.style.height = a.H - M.t - M.b + "px";
  if (!h) return;
  h.hidden = !!p._exp; h.style.left = c.style.left; h.style.top = p.cv.offsetTop + M.t + "px";
  const t = I18N.t("cursor.handle"); h.setAttribute("aria-label", t); h.title = t;
  if (a.full) { h.setAttribute("aria-valuemin", a.full[0].toFixed(2)); h.setAttribute("aria-valuemax", a.full[1].toFixed(2)); }
  h.setAttribute("aria-valuenow", p.cur.toFixed(2)); h.setAttribute("aria-valuetext", I18N.t("cursor.handle.value", { rt: p.cur.toFixed(2) }) + (window.GRADIENTE && p.overlay ? GRADIENTE.cursorText(p) : ""));
}

// ---- keys: ← → one scan (Maiusc: five), held = ~15 scans/s, Space = play/pause, Esc = stop, ? = list of the shortcuts
const KEY = { dir: 0, n: 1, t0: 0, t: 0, raf: 0, p: null };
const PLAY = { p: null, dir: 1, sel: null, t: 0, raf: 0 };
// the chromatogram that walks: the active one, or the one a linked spectrum follows (after zooming in the spectrum the arrows still walk)
const walkTarget = (play) => {
  const fp = fsPanel();
  let ap = fp || E.active;                                          // full screen: only that panel can walk
  if (!fp && ap && ap.type === "spec" && ap.link != null) ap = E.panels.find(q => q.id === ap.link) || ap;
  return ap && E.panels.includes(ap) && ap._a && ap._a.sr && (ap.cur != null || (play && ap.sel)) ? ap : null;
};
function playStop() { FRAME.cancel(PLAY.raf); PLAY.raf = 0; PLAY.p = null; }
function playStart(p) {
  const sel = p.sel && p.sel[1] > p.sel[0] ? [p.sel[0], p.sel[1]] : null;
  if (sel && (p.cur == null || p.cur < sel[0] || p.cur > sel[1])) p.cur = sel[0];
  PLAY.p = p; PLAY.dir = 1; PLAY.sel = sel; PLAY.t = 0;     // t = 0: the first step is due at once
  const tick = now => {
    PLAY.raf = 0; const q = PLAY.p; if (!q || S.view !== "data" || !E.panels.includes(q)) return playStop();
    if (paceDue(PLAY, now)) {                                        // ~15 scans per second, like the held arrow
      const sl = PLAY.sel;
      if (sl) { if (PLAY.dir > 0 && q.cur >= sl[1] - 1e-9) PLAY.dir = -1; else if (PLAY.dir < 0 && q.cur <= sl[0] + 1e-9) PLAY.dir = 1; }
      if (!stepScan(q, PLAY.dir, { keepSel: !!sl })) { if (sl) PLAY.dir = -PLAY.dir; else return playStop(); }
    }
    PLAY.raf = FRAME.next(tick);
  };
  PLAY.raf = FRAME.next(tick);
}
function keyLoop() {
  if (KEY.raf) return;
  const tick = now => {
    KEY.raf = 0; if (!KEY.dir || !KEY.p) return;
    if (now - KEY.t0 >= 350 && paceDue(KEY, now)) { if (!stepScan(KEY.p, KEY.dir * KEY.n)) { KEY.dir = 0; return; } }
    KEY.raf = FRAME.next(tick);
  };
  KEY.raf = FRAME.next(tick);
}
document.addEventListener("keyup", e => { if (e.key === " " && PLAY.space) { PLAY.space = false; e.preventDefault(); } });   // Space on a focused button would also click it on release
const typingNow = () => /INPUT|SELECT|TEXTAREA/.test(document.activeElement?.tagName || "") || !!Q("dialog[open]");
document.addEventListener("keydown", e => {
  if (S.view !== "data") return;
  if (e.key === "Escape") { playStop(); KEY.dir = 0; return; }
  if (!E.files.length || typingNow() || e.ctrlKey || e.metaKey || e.altKey) return;
  if (e.key === "?") { e.preventDefault(); shortcuts(); return; }
  if (e.key === " ") {
    const ap = walkTarget(true); if (!ap) return;
    e.preventDefault(); PLAY.space = true; if (e.repeat) return;
    if (PLAY.p) playStop(); else playStart(ap);
    return;
  }
  const dir = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0; if (!dir) return;
  if (window.DDA && DDA.key(e, dir)) return;                          // the MS2 spectrum of a DDA trio is active: arrows = the next / previous MS2
  e.preventDefault();
  const ap = walkTarget(false);                                      // active panel with a cursor: arrows = previous/next scan; otherwise arrows change file
  if (!ap) { if (!fsPanel()) goFile(dir); return; }
  if (e.repeat) return;                                              // the automatic repeat of the system is ignored: the loop below paces a held key
  playStop();
  KEY.dir = dir; KEY.n = e.shiftKey ? 5 : 1; KEY.p = ap; KEY.t0 = KEY.t = performance.now();
  if (stepScan(ap, dir * KEY.n)) keyLoop(); else KEY.dir = 0;
});
document.addEventListener("keyup", e => {
  if ((e.key === "ArrowRight" && KEY.dir > 0) || (e.key === "ArrowLeft" && KEY.dir < 0)) KEY.dir = 0;
  if (e.key === " " && walkTarget(true) && S.view === "data" && !typingNow()) e.preventDefault();   // a focused button must not be "clicked" by the Space that started the play
});
addEventListener("blur", () => { KEY.dir = 0; });
document.addEventListener("mousedown", () => playStop(), true);      // a click stops the play

function shortcuts() {
  info(HELP.scorrimento[1]);
}
