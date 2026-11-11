// @ts-strict
// The two points where an integrated area meets its baseline (the ends of the window on the line) as 24 px handles over the chart: drag with mouse, finger
// or pen, or ← → one point (Shift: five). Where the end goes (onto a point of the trace, never past the other end or a neighbouring integration) is the
// engine's (QUALITA.windowEnd, mzlab_core::intwin, the bytes of `mzlab-cli peak-window`); this file only asks, moves the end and redraws. Loaded by barre-maniglie.js.
/** @typedef {Record<string, any>} Bag */
const EPS = 1e-6;
const lo = (/** @type {Bag} */ it) => (it.a <= it.b ? "a" : "b");
// the time an end is drawn at (the manual window keeps its exact bar times) and the baseline there, in counts of the trace
const shown = (/** @type {Bag} */ s, /** @type {Bag} */ it, /** @type {string} */ e) => (it.zero && window.INTZERO ? INTZERO.clamp(s, it[e]) : s.x[nearIdx(s.x, it[e])]);
function baseY(/** @type {Bag} */ s, /** @type {Bag} */ it, /** @type {number} */ t) {
  if (it.zero || it.bm === "zero") return 0;
  if (it.bm === "line" && it.bl && window.INTBASE) return INTBASE.lineAt(it, t);
  return it.base ? baseAt(it.base, t) : edge3(s.y, nearIdx(s.x, t));
}
// the borders of the other integrations on the trace this end may touch but not cross: [lo, hi] (null: none)
function limits(/** @type {Bag} */ p, /** @type {Bag} */ it, /** @type {string} */ e) {
  const cur = it[e], first = e === lo(it); let l = null, h = null;
  for (const o of p.ints) if (o !== it && o.key === it.key) {
    if (first && iHi(o) <= cur + EPS) l = l == null ? iHi(o) : Math.max(l, iHi(o));
    if (!first && iLo(o) >= cur - EPS) h = h == null ? iLo(o) : Math.min(h, iLo(o));
  }
  return [l, h];
}
const label = (/** @type {Bag} */ it, /** @type {string} */ e, /** @type {number} */ t) => I18N.t(e === lo(it) ? "int.end.a" : "int.end.b", { rt: t.toFixed(3) });
// one move of an end: the engine's point, then the record, the chart and the live text; the last answer wins
async function move(/** @type {Bag} */ p, /** @type {Bag} */ n, /** @type {number | null} */ at, /** @type {number} */ step) {
  const it = n._it, e = n._e, s = n._s, o = e === "a" ? "b" : "a", [l, h] = limits(p, it, e), tok = (n._tok = (n._tok || 0) + 1);
  const txt = window.QUALITA ? await QUALITA.windowEnd(s.x, it[e], it[o], at, step, l, h) : null;
  if (!txt || tok !== n._tok) return;
  const t = JSON.parse(txt).t; if (t === it[e]) return;
  it[e] = t; p._iepSay = { it, e }; draw(p);
}
function button(/** @type {Bag} */ p) {
  const n = /** @type {HTMLButtonElement & Bag} */ (document.createElement("button")); n.type = "button"; n.className = "iep"; n.setAttribute("role", "slider"); p.el.appendChild(n);
  const at = (/** @type {PointerEvent} */ ev) => xOf(p, ev.clientX - p.cv.getBoundingClientRect().left);
  const done = () => { if (intSnap(p) !== n._snap) pushIh(p, n._snap); uiSave(); };
  n.addEventListener("pointerdown", ev => { ev.preventDefault(); ev.stopPropagation(); try { n.setPointerCapture(ev.pointerId); } catch (_) { /* events still arrive */ } setActive(p); n._snap = intSnap(p); n.classList.add("on"); });
  n.addEventListener("pointermove", ev => { if (n.classList.contains("on")) move(p, n, at(ev), 0); });
  const end = async (/** @type {PointerEvent} */ ev) => { if (!n.classList.contains("on")) return; n.classList.remove("on"); if (ev.type === "pointerup") await move(p, n, at(ev), 0); done(); };
  n.addEventListener("pointerup", end); n.addEventListener("pointercancel", end);
  n.addEventListener("keydown", async ev => {
    const d = ev.key === "ArrowLeft" || ev.key === "ArrowDown" ? -1 : ev.key === "ArrowRight" || ev.key === "ArrowUp" ? 1 : 0;
    if (!d || ev.ctrlKey || ev.metaKey || ev.altKey) return;
    ev.preventDefault(); ev.stopPropagation(); setActive(p); n._snap = intSnap(p);
    await move(p, n, null, d * (ev.shiftKey ? 5 : 1)); done();
  });
  return n;
}
// the live text of the panel: the end that moved and the area it gives, once the area is known
function say(/** @type {Bag} */ p) {
  const w = p._iepSay, s = w && p._a && p._a.sr && p._a.sr.find((/** @type {Bag} */ q) => q.key === w.it.key); if (!w || !s || !Number.isFinite(w.it.area)) return;
  if (w.it.bm && w.it.bm !== "edges" && window.INTBASE && INTBASE.area(p, s, w.it).wait) return;          // the engine's area of the new window is not there yet
  let r = p.el.querySelector(".iep-live");
  if (!r) { r = document.createElement("div"); r.className = "iep-live sr-only"; r.setAttribute("role", "status"); r.setAttribute("aria-live", "polite"); p.el.appendChild(r); }
  const v = I18N.t(w.e === lo(w.it) ? "int.end.liveA" : "int.end.liveB", { rt: w.it[w.e].toFixed(3), area: I18N.sciE(w.it.area) });
  if (r.textContent !== v) r.textContent = v;
  p._iepSay = null;
}
function sync(/** @type {Bag} */ p) {
  const a = p._a, old = /** @type {Bag[]} */ (p._ep || []); p._ep = [];
  const list = a && a.sr && !a.cas && a.X && a.Y && a.U && !p._exp && p.type !== "spec" && p.type !== "map" ? p.ints : [];
  const pts = (p._bp || []).filter((/** @type {HTMLElement} */ b) => !b.hidden).map((/** @type {HTMLElement} */ b) => [parseFloat(b.style.left), parseFloat(b.style.top)]);
  for (const it of list) {
    const s = a.sr.find((/** @type {Bag} */ q) => q.key === it.key); if (!s) continue;
    for (const e of ["a", "b"]) {
      const b = old.shift() || button(p), t = shown(s, it, e); b._it = it; b._e = e; b._s = s; b.dataset.e = e === lo(it) ? "0" : "1"; p._ep.push(b);
      const px = a.X(t), py = a.Y(a.U(s, baseY(s, it, t))), x = p.cv.offsetLeft + px, y = p.cv.offsetTop + py;
      const out = px < M.l - 2 || px > a.W - M.r + 2 || py < M.t - 2 || py > a.H - M.b + 2;
      b.hidden = out || pts.some(q => Math.abs(q[0] - x) < 24 && Math.abs(q[1] - y) < 24);          // a point of a custom line sits there: that one is dragged
      b.style.left = x + "px"; b.style.top = y + "px";
      const v = label(it, e, t); b.setAttribute("aria-label", v); b.title = v;
      b.setAttribute("aria-valuenow", t.toFixed(3)); b.setAttribute("aria-valuetext", I18N.t("cursor.handle.value", { rt: t.toFixed(3) }));
      if (a.full) { b.setAttribute("aria-valuemin", a.full[0].toFixed(3)); b.setAttribute("aria-valuemax", a.full[1].toFixed(3)); }
    }
  }
  old.forEach(n => n.remove()); say(p);
}
{                                                                  // the look of the handles: a ring on the baseline (colours from the CSS variables)
  const st = document.createElement("style");
  st.textContent = ".iep{position:absolute;z-index:3;width:24px;height:24px;margin:-12px 0 0 -12px;padding:0;border:0;border-radius:50%;background:transparent;touch-action:none;cursor:ew-resize;transition:background-color .12s}" +
    ".iep::after{content:\"\";position:absolute;left:7px;top:7px;width:10px;height:10px;box-sizing:border-box;border-radius:50%;border:2px solid var(--ink);background:var(--panel)}" +
    ".iep:focus-visible{outline:2px solid var(--accent);outline-offset:1px}.iep.on{background:color-mix(in srgb,var(--accent) 25%,transparent)}" +
    "@media (prefers-reduced-motion:reduce){.iep{transition:none}}@media (forced-colors:active){.iep::after{border-color:CanvasText;background:Canvas}.iep:focus-visible{outline-color:Highlight}.iep.on{background:Highlight}}";
  document.head.appendChild(st);
}
window.INTEND = { sync, baseY, shown };
