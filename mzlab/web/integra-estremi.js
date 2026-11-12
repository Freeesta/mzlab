// @ts-strict
// The two points where an integrated area meets its baseline (the ends of the window on the line) as 24 px handles over the chart: drag with mouse, finger
// or pen, or ← → one point (Shift: five). Where the end goes (onto a point of the trace, never past the other end or a neighbouring integration) is the
// engine's (QUALITA.windowEnd, mzlab_core::intwin, the bytes of `mzlab-cli peak-window`); this file only asks, moves the end and redraws. Loaded by barre-maniglie.js.
// On SIM/MRM panels the ends are the two anchors of the integration (`it.snap`): each on the trace, at zero or where the user put it (manual: ↑ ↓ or a drag
// off the line); S or the menu (context menu, Shift+F10) sets the mode. picchi.js (snapBase) turns the two modes into the baseline.
/** @typedef {Record<string, any>} Bag */
const EPS = 1e-6;
const lo = (/** @type {Bag} */ it) => (it.a <= it.b ? "a" : "b");
// the time an end is drawn at (the manual window keeps its exact bar times) and the baseline there, in counts of the trace
const shown = (/** @type {Bag} */ s, /** @type {Bag} */ it, /** @type {string} */ e) => (it.zero && typeof INTZERO !== "undefined" ? INTZERO.clamp(s, it[e]) : s.x[nearIdx(s.x, it[e])]);
function baseY(/** @type {Bag} */ s, /** @type {Bag} */ it, /** @type {number} */ t) {
  const q = it.snap && (Math.abs(t - shown(s, it, "a")) < EPS ? it.snap.a : Math.abs(t - shown(s, it, "b")) < EPS ? it.snap.b : null);   // an anchor: its own mode, even before the next drawing
  if (q) return q === "zero" ? 0 : q === "man" ? +((it.my || {})[t === shown(s, it, "a") ? "a" : "b"] || 0) : edge3(s.y, nearIdx(s.x, t));
  if (it.zero || it.bm === "zero") return 0;
  if (it.bm === "line" && it.bl && typeof INTBASE !== "undefined") return INTBASE.lineAt(it, t);
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
const SNAPS = ["curve", "zero", "man"];
const label = (/** @type {Bag} */ it, /** @type {string} */ e, /** @type {number} */ t, /** @type {number} */ y) => it.snap
  ? I18N.t(e === lo(it) ? "int.anchor.a" : "int.anchor.b", { rt: t.toFixed(3), y: I18N.sciE(y), m: I18N.t(`int.snap.${it.snap[e]}`) })
  : I18N.t(e === lo(it) ? "int.end.a" : "int.end.b", { rt: t.toFixed(3) });
// y on the canvas -> counts of the trace (U is linear: s.off + v / s.sc)
const raw = (/** @type {Bag} */ p, /** @type {Bag} */ s, /** @type {number} */ py) => (p._a.yinv(py) - s.off) * s.sc;
// the mode of one anchor; a manual one keeps the level it has now, the stamp makes it the mode a new integration inherits
function setMode(/** @type {Bag} */ p, /** @type {Bag} */ n, /** @type {string} */ q, /** @type {number | null} */ y) {
  const it = n._it, e = n._e, s = n._s;
  if (q === "man") it.my = { ...(it.my || {}), [e]: y == null ? baseY(s, it, shown(s, it, e)) : y };
  if (it.snap[e] !== q) { it.snap = { ...it.snap, [e]: q }; it.st = E.seq++; }
}
// one move of an end: the engine's point, then the record, the chart and the live text; the last answer wins
async function move(/** @type {Bag} */ p, /** @type {Bag} */ n, /** @type {number | null} */ at, /** @type {number} */ step, dirty = false) {
  const it = n._it, e = n._e, s = n._s, o = e === "a" ? "b" : "a", [l, h] = limits(p, it, e), tok = (n._tok = (n._tok || 0) + 1);
  const txt = window.QUALITA ? await QUALITA.windowEnd(s.x, it[e], it[o], at, step, l, h) : null;
  if (!txt || tok !== n._tok) return;
  const t = JSON.parse(txt).t; if (t === it[e]) { if (dirty) draw(p); return; }
  it[e] = t; p._iepSay = { it, e }; draw(p);
}
function button(/** @type {Bag} */ p) {
  const n = /** @type {HTMLButtonElement & Bag} */ (document.createElement("button")); n.type = "button"; n.className = "iep"; n.setAttribute("role", "slider"); p.el.appendChild(n);
  const at = (/** @type {PointerEvent} */ ev) => xOf(p, ev.clientX - p.cv.getBoundingClientRect().left);
  const done = () => { if (intSnap(p) !== n._snap) pushIh(p, n._snap); uiSave(); };
  // an anchor: a manual one follows the pointer up and down; one on the trace or at zero dragged 16 px off its line becomes manual
  const lift = (/** @type {PointerEvent} */ ev) => {
    const it = n._it; if (!it.snap) return false;
    const py = ev.clientY - p.cv.getBoundingClientRect().top, y = raw(p, n._s, py);
    if (it.snap[n._e] !== "man" && Math.abs(py - (parseFloat(n.style.top) - p.cv.offsetTop)) < 16) return false;
    setMode(p, n, "man", y); return true;
  };
  n.addEventListener("pointerdown", ev => {
    if (ev.button > 0) return; ev.preventDefault(); ev.stopPropagation(); try { n.setPointerCapture(ev.pointerId); } catch (_) { /* events still arrive */ }
    setActive(p); n._snap = intSnap(p); n.classList.add("on"); if (p.isel !== n._it.id) { p.isel = n._it.id; draw(p); }
  });
  n.addEventListener("pointermove", ev => { if (n.classList.contains("on")) move(p, n, at(ev), 0, lift(ev)); });
  const end = async (/** @type {PointerEvent} */ ev) => { if (!n.classList.contains("on")) return; n.classList.remove("on"); if (ev.type === "pointerup") await move(p, n, at(ev), 0, lift(ev)); done(); };
  n.addEventListener("pointerup", end); n.addEventListener("pointercancel", end);
  n.addEventListener("contextmenu", ev => { if (!n._it.snap) return; ev.preventDefault(); menu(p, n); });
  n.addEventListener("keydown", async ev => {
    const it = n._it;
    if (it.snap && (ev.key === "ContextMenu" || (ev.key === "F10" && ev.shiftKey))) { ev.preventDefault(); ev.stopPropagation(); menu(p, n); return; }
    if (it.snap && (ev.key === "s" || ev.key === "S") && !ev.ctrlKey && !ev.metaKey && !ev.altKey) {
      ev.preventDefault(); ev.stopPropagation(); setActive(p); n._snap = intSnap(p);
      setMode(p, n, SNAPS[(SNAPS.indexOf(it.snap[n._e]) + 1) % 3], null); draw(p); done(); return;
    }
    const v = it.snap && (ev.key === "ArrowUp" ? 1 : ev.key === "ArrowDown" ? -1 : 0);
    const d = v ? 0 : ev.key === "ArrowLeft" || ev.key === "ArrowDown" ? -1 : ev.key === "ArrowRight" || ev.key === "ArrowUp" ? 1 : 0;
    if ((!d && !v) || ev.ctrlKey || ev.metaKey || ev.altKey) return;
    ev.preventDefault(); ev.stopPropagation(); setActive(p); n._snap = intSnap(p);
    if (v) { const s = n._s, y = baseY(s, it, shown(s, it, n._e)) + v * (ev.shiftKey ? 0.05 : 0.01) * (p._a.ymax - p._a.ymin) * s.sc; setMode(p, n, "man", y); draw(p); }   // ↑ ↓: the level of a manual anchor, 1 % of the axis (Shift 5 %)
    else await move(p, n, null, d * (ev.shiftKey ? 5 : 1));
    done();
  });
  return n;
}
// the small menu of an anchor: the three modes as radio items; arrows, Enter, Esc; a click outside closes it
function menu(/** @type {Bag} */ p, /** @type {Bag} */ n) {
  document.querySelectorAll(".iep-menu").forEach(m => m.remove());
  const m = document.createElement("div"), it = n._it; m.className = "iep-menu"; m.setAttribute("role", "menu"); m.setAttribute("aria-label", I18N.t("int.snap.menu"));
  m.style.left = parseFloat(n.style.left) + 14 + "px"; m.style.top = parseFloat(n.style.top) + 14 + "px";
  const close = () => { m.remove(); document.removeEventListener("pointerdown", out, true); n.focus(); };
  const out = (/** @type {Event} */ ev) => { if (!m.contains(/** @type {Node} */ (ev.target))) close(); };
  for (const q of SNAPS) {
    const b = document.createElement("button"); b.type = "button"; b.setAttribute("role", "menuitemradio"); b.dataset.m = q; b.textContent = I18N.t(`int.snap.${q}`);
    b.setAttribute("aria-checked", String(it.snap[n._e] === q)); b.tabIndex = -1;
    b.addEventListener("click", () => { n._snap = intSnap(p); setMode(p, n, q, null); draw(p); if (intSnap(p) !== n._snap) pushIh(p, n._snap); uiSave(); close(); });
    m.appendChild(b);
  }
  m.addEventListener("keydown", ev => {
    const k = /** @type {HTMLElement[]} */ ([...m.children]), i = k.indexOf(/** @type {HTMLElement} */ (document.activeElement));
    if (ev.key === "Escape" || ev.key === "Tab") { ev.preventDefault(); ev.stopPropagation(); close(); }
    else if (ev.key === "ArrowDown" || ev.key === "ArrowUp") { ev.preventDefault(); ev.stopPropagation(); k[(i + (ev.key === "ArrowDown" ? 1 : 2)) % 3].focus(); }
  });
  p.el.appendChild(m); document.addEventListener("pointerdown", out, true);
  /** @type {HTMLElement} */ (m.querySelector("[aria-checked=true]") || m.firstChild).focus();
}
// the live text of the panel: the end that moved and the area it gives, once the area is known
function say(/** @type {Bag} */ p) {
  const w = p._iepSay, s = w && p._a && p._a.sr && p._a.sr.find((/** @type {Bag} */ q) => q.key === w.it.key); if (!w || !s || !Number.isFinite(w.it.area)) return;
  const own = w.it.bm && w.it.bm !== "edges" && typeof INTBASE !== "undefined" ? INTBASE.area(p, s, w.it) : null; if (own && own.wait) return;          // the engine's area of the new window is not there yet
  let r = p.el.querySelector(".iep-live");
  if (!r) { r = document.createElement("div"); r.className = "iep-live sr-only"; r.setAttribute("role", "status"); r.setAttribute("aria-live", "polite"); p.el.appendChild(r); }
  const v = I18N.t(w.e === lo(w.it) ? "int.end.liveA" : "int.end.liveB", { rt: w.it[w.e].toFixed(3), area: I18N.sciE(own ? own.area : w.it.area) });
  if (r.textContent !== v) r.textContent = v;
  p._iepSay = null;
}
function sync(/** @type {Bag} */ p) {
  const a = p._a, old = /** @type {Bag[]} */ (p._ep || []); p._ep = [];
  const list = a && a.sr && !a.cas && a.X && a.Y && a.U && !p._exp && p.type !== "spec" && p.type !== "map" ? p.ints : [];
  const pts = (p._bp || []).filter((/** @type {HTMLElement} */ b) => !b.hidden).map((/** @type {HTMLElement} */ b) => [parseFloat(b.style.left), parseFloat(b.style.top)]);
  // the two ends of a window narrower than a handle move apart just enough not to overlap (each stays over its point); where handles meet (the same window
  // on two traces, two windows that touch) the first stands for both; the one in hand stays. pts: the centres shown (a point of a custom line too)
  for (const it of list) {
    const s = a.sr.find((/** @type {Bag} */ q) => q.key === it.key); if (!s) continue;
    const at = (/** @type {string} */ e) => { const t = shown(s, it, e); return [t, a.X(t), a.Y(a.U(s, baseY(s, it, t)))]; };
    const L = at(lo(it)), H = at(lo(it) === "a" ? "b" : "a"), d = Math.abs(L[2] - H[2]) < 24 ? Math.max(0, (24 - (H[1] - L[1])) / 2) : 0;
    for (const e of ["a", "b"]) {
      const b = old.shift() || button(p), first = e === lo(it), [t, px, py] = first ? L : H; b._it = it; b._e = e; b._s = s; b.dataset.e = first ? "0" : "1"; p._ep.push(b);
      const x = p.cv.offsetLeft + px + (first ? -d : d), y = p.cv.offsetTop + py;
      const out = px < M.l - 2 || px > a.W - M.r + 2 || py < M.t - 2 || py > a.H - M.b + 2;
      const held = b.classList.contains("on") || document.activeElement === b;
      b.hidden = out || (!held && pts.some(q => Math.abs(q[0] - x) < 24 && Math.abs(q[1] - y) < 24));
      if (!b.hidden) pts.push([x, y]);
      b.style.left = x + "px"; b.style.top = y + "px";
      const v = label(it, e, t, baseY(s, it, t)); b.setAttribute("aria-label", v); b.title = v;
      b.classList.toggle("sel", it.id === p.isel); b.classList.toggle("anc", !!it.snap); if (it.snap) b.dataset.m = it.snap[e]; else delete b.dataset.m;
      b.setAttribute("aria-valuenow", t.toFixed(3)); b.setAttribute("aria-valuetext", it.snap ? v : I18N.t("cursor.handle.value", { rt: t.toFixed(3) }));
      if (a.full) { b.setAttribute("aria-valuemin", a.full[0].toFixed(3)); b.setAttribute("aria-valuemax", a.full[1].toFixed(3)); }
    }
  }
  old.forEach(n => n.remove()); say(p);
  const W = /** @type {any} */ (window);                          // the drop lines and the proposal of «Split at the valleys» (integra-valli.js)
  if (W.INTVALLI) W.INTVALLI.sync(p); else if (p._split || p.ints.some((/** @type {Bag} */ i) => i.splits)) carica("m:integra-valli.js").then(() => draw(p));
}
{                                                                  // the look of the handles: a ring on the baseline (colours from the CSS variables)
  const st = document.createElement("style");
  st.textContent = ".iep{position:absolute;z-index:3;width:24px;height:24px;margin:-12px 0 0 -12px;padding:0;border:0;border-radius:50%;background:transparent;touch-action:none;cursor:ew-resize;transition:background-color .12s}" +
    ".iep::after{content:\"\";position:absolute;left:7px;top:7px;width:10px;height:10px;box-sizing:border-box;border-radius:50%;border:2px solid var(--ink);background:var(--panel)}" +
    ".iep:focus-visible{outline:2px solid var(--accent);outline-offset:1px}.iep.on{background:color-mix(in srgb,var(--accent) 25%,transparent)}" +
    ".iep.anc{cursor:move}.iep[data-m=man]::after{border-style:dashed}.iep.sel::after{left:5px;top:5px;width:14px;height:14px;border-width:3px}" +
    ".iep-menu{position:absolute;z-index:6;display:flex;flex-direction:column;padding:2px;background:var(--panel);border:1px solid var(--line);border-radius:4px}" +
    ".iep-menu button{min-height:24px;text-align:left;border:0;background:transparent;color:var(--ink);padding:2px 8px}.iep-menu button[aria-checked=true]{font-weight:bold}" +
    ".iep-menu button[aria-checked=true]::before{content:\"✓ \"}.iep-menu button:focus-visible{outline:2px solid var(--accent)}" +
    "@media (prefers-reduced-motion:reduce){.iep{transition:none}}@media (forced-colors:active){.iep::after{border-color:CanvasText;background:Canvas}.iep:focus-visible{outline-color:Highlight}.iep.on{background:Highlight}}";
  document.head.appendChild(st);
}
window.INTEND = { sync, baseY, shown };
