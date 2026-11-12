// @ts-strict
// «Split at the valleys» of an integration on a SIM/MRM panel: the engine proposes the drop lines (`api/manvalley`, mzlab_core::valley, the bytes of
// `mzlab-cli peak-split`) at the depth shown in the bar over the window; the user drags them (mouse, finger, pen; ← → one point, Shift five, Delete
// removes one), then Split keeps them on the record (`it.splits`, undoable, saved) or Esc / Cancel drops them. A split record shows its drop lines
// from the baseline to the trace (`paint`, on the canvas); the table gives one row per part (integrazioni-manuali.js). Loaded by area-copia.js and integra-estremi.js.
/** @typedef {Record<string, any>} Bag */
const G = /** @type {any} */ (window);
const T = (/** @type {string} */ k, /** @type {Record<string, any>} */ v = {}) => I18N.t(k, v);
const DKEY = () => NOMI.KEY("int.split.depth");
/** The depth the user last set, in % (10 the first time). */
const depthOf = () => { try { const v = Number(localStorage.getItem(DKEY())); return Number.isFinite(v) && v >= 0 && v <= 100 && localStorage.getItem(DKEY()) != null ? v : 10; } catch (_) { return 10; } };
const lo = (/** @type {Bag} */ it) => Math.min(it.a, it.b), hi = (/** @type {Bag} */ it) => Math.max(it.a, it.b);
const traceOf = (/** @type {Bag} */ p, /** @type {Bag} */ it) => p._a && p._a.sr ? p._a.sr.find((/** @type {Bag} */ s) => s.key === it.key) : null;
/** The points of the trace strictly inside the window: [first, last] index. @param {Bag} s @param {Bag} it */
const inner = (s, it) => [nearIdx(s.x, lo(it)) + 1, nearIdx(s.x, hi(it)) - 1];
/** The baseline the parts share at time t (manint::drop_line): `edges` is the line between the 3-point means of the two ends. @param {Bag} s @param {Bag} it @param {number} t */
function baseAtT(s, it, t) {
  if (it.bm === "edges" && !it.base) {
    const i0 = nearIdx(s.x, lo(it)), i1 = nearIdx(s.x, hi(it)), y0 = edge3(s.y, i0), y1 = edge3(s.y, i1);
    return s.x[i1] === s.x[i0] ? y0 : y0 + (y1 - y0) * (t - s.x[i0]) / (s.x[i1] - s.x[i0]);
  }
  return G.INTEND ? G.INTEND.baseY(s, it, t) : 0;
}

/** Opens the proposal on the record `it` of `p`: its own drop lines, or the engine's at the depth last used. @param {Bag} p @param {Bag} it */
async function start(p, it) {
  if (!G.MANINT) await carica("m:integrazioni-manuali.js");
  p._split = { it, ts: it.splits ? [...it.splits] : [], depth: depthOf(), tok: 0, none: false, focus: true };
  p.isel = it.id; setActive(p);
  if (!it.splits) await ask(p); else draw(p);
}
/** The engine's drop lines at the depth of the bar; the last answer wins. @param {Bag} p */
async function ask(p) {
  const sp = p._split; if (!sp) return;
  const tok = ++sp.tok, v = await G.MANINT.valleys(sp.it, sp.depth / 100);
  if (p._split !== sp || tok !== sp.tok) return;
  sp.ts = Array.isArray(v) ? v : []; sp.none = Array.isArray(v) && !v.length; draw(p);
}
/** Split keeps the drop lines on the record (none: the record is whole again); undoable. @param {Bag} p */
function confirm(p) {
  const sp = p._split; if (!sp) return;
  const ts = [...new Set(sp.ts)].filter(t => t > lo(sp.it) && t < hi(sp.it)).sort((x, y) => x - y), was = JSON.stringify(sp.it.splits || []);
  p._split = null;
  if (JSON.stringify(ts) !== was) { pushIh(p); if (ts.length) sp.it.splits = ts; else delete sp.it.splits; uiSave(); }
  draw(p); focusPanel(p);
}
/** @param {Bag} p */
function cancel(p) { if (!p._split) return; p._split = null; draw(p); focusPanel(p); }
const focusPanel = (/** @type {Bag} */ p) => { try { p.cv.focus({ preventScroll: true }); } catch (_) { /* gone */ } };

/** One marker: a slider over the plot, its line from top to bottom. @param {Bag} p */
function marker(p) {
  const n = /** @type {HTMLButtonElement & Bag} */ (document.createElement("button")); n.type = "button"; n.className = "ivl-mk"; n.setAttribute("role", "slider");
  const set = (/** @type {number} */ t) => { const sp = p._split; if (!sp) return; sp.ts[n._i] = t; sync(p); };
  const at = (/** @type {PointerEvent} */ ev) => { const s = p._split && traceOf(p, p._split.it); if (!s) return null; const [i0, i1] = inner(s, p._split.it), i = Math.min(i1, Math.max(i0, nearIdx(s.x, xOf(p, ev.clientX - p.cv.getBoundingClientRect().left)))); return s.x[i]; };
  n.addEventListener("pointerdown", ev => { if (ev.button > 0) return; ev.preventDefault(); ev.stopPropagation(); try { n.setPointerCapture(ev.pointerId); } catch (_) { /* events still arrive */ } n.classList.add("on"); n.focus(); });
  n.addEventListener("pointermove", ev => { if (!n.classList.contains("on")) return; const t = at(ev); if (t != null) set(t); });
  const end = () => n.classList.remove("on");
  n.addEventListener("pointerup", end); n.addEventListener("pointercancel", end);
  n.addEventListener("keydown", ev => {
    const sp = p._split; if (!sp || ev.ctrlKey || ev.metaKey || ev.altKey) return;
    const k = ev.key, s = traceOf(p, sp.it);
    if (k === "Escape") { ev.preventDefault(); ev.stopPropagation(); cancel(p); return; }
    if (k === "Enter") { ev.preventDefault(); ev.stopPropagation(); confirm(p); return; }
    if (k === "Delete" || k === "Backspace") { ev.preventDefault(); ev.stopPropagation(); sp.ts.splice(n._i, 1); sp.focus = Math.max(0, n._i - 1); sync(p); return; }
    const d = k === "ArrowLeft" || k === "ArrowDown" ? -1 : k === "ArrowRight" || k === "ArrowUp" ? 1 : 0; if (!d || !s) return;
    ev.preventDefault(); ev.stopPropagation();
    const [i0, i1] = inner(s, sp.it), i = Math.min(i1, Math.max(i0, nearIdx(s.x, sp.ts[n._i]) + d * (ev.shiftKey ? 5 : 1))); set(s.x[i]);
  });
  return n;
}
/** The bar over the window: depth (%), Split, Cancel and the engine's answer when it found no valley. @param {Bag} p */
function bar(p) {
  const b = document.createElement("div"); b.className = "ivl-bar"; b.setAttribute("role", "group"); b.setAttribute("aria-label", T("int.split.menu"));
  b.innerHTML = h`<label>${T("int.split.depth")} <input type="number" min="0" max="100" step="1" inputmode="numeric" data-depth title="${T("int.split.depth.tip")}" aria-describedby="ivl-d"></label><span id="ivl-d" class="sr-only">${T("int.split.depth.tip")}</span><span class="ivl-none" role="status" aria-live="polite"></span><button type="button" data-ok>${T("int.split.ok")}</button><button type="button" data-no>${T("common.cancel")}</button>`;
  const inp = /** @type {HTMLInputElement} */ (b.querySelector("[data-depth]"));
  inp.addEventListener("change", () => {
    const sp = p._split, v = Number(inp.value); if (!sp) return;
    if (!Number.isFinite(v) || v < 0 || v > 100) { inp.value = String(sp.depth); return; }
    sp.depth = v; try { localStorage.setItem(DKEY(), String(v)); } catch (_) { /* this session only */ } ask(p);
  });
  b.addEventListener("keydown", ev => {
    ev.stopPropagation();
    if (ev.key === "Escape") { ev.preventDefault(); cancel(p); }
    else if (ev.key === "Enter" && ev.target === inp) { ev.preventDefault(); inp.dispatchEvent(new Event("change")); }
  });
  b.querySelector("[data-ok]")?.addEventListener("click", () => confirm(p));
  b.querySelector("[data-no]")?.addEventListener("click", () => cancel(p));
  return b;
}

/** The drop lines of a split record on the canvas (so a PNG of the chart has them): from the shared baseline to the trace, in the trace's colour;
 * hidden while the record's proposal is open (its markers stand for them). grafici.js calls it for each record of a SIM/MRM panel.
 * @param {Bag} p @param {CanvasRenderingContext2D} g @param {(t: number) => number} X @param {(v: number) => number} Y @param {(s: Bag, v: number) => number} U @param {Bag} s @param {Bag} it */
function paint(p, g, X, Y, U, s, it) {
  if (!it.splits || (p._split && p._split.it === it)) return;
  g.strokeStyle = s.color; g.lineWidth = 1.5; g.setLineDash([]); g.beginPath();
  for (const t0 of it.splits) {
    if (t0 <= lo(it) || t0 >= hi(it)) continue;
    const i = nearIdx(s.x, t0), x = X(s.x[i]); g.moveTo(x, Y(U(s, baseAtT(s, it, s.x[i])))); g.lineTo(x, Y(U(s, s.ys[i])));
  }
  g.stroke();
}

/** Places the markers and bar of the proposal; every draw of the panel calls it (integra-estremi.js). @param {Bag} p */
function sync(p) {
  const a = p._a, ok = a && a.sr && !a.cas && a.X && a.Y && a.U && !p._exp;
  if (p._split && !p.ints.includes(p._split.it)) p._split = null;
  const sp = ok ? p._split : null;
  /** @type {any[]} */ const mks = [...p.el.querySelectorAll(".ivl-mk")];
  const br = /** @type {HTMLElement | null} */ (p.el.querySelector(".ivl-bar"));
  const s = sp && traceOf(p, sp.it);
  if (!sp || !s) { mks.forEach(m => m.remove()); if (br) br.remove(); return; }
  const bx = br || p.el.appendChild(bar(p));
  const inp = /** @type {HTMLInputElement} */ (bx.querySelector("[data-depth]")); if (document.activeElement !== inp) inp.value = String(sp.depth);
  const none = /** @type {HTMLElement} */ (bx.querySelector(".ivl-none")), say = sp.none && !sp.ts.length ? T("int.split.none") : ""; if (none.textContent !== say) none.textContent = say;
  const cx = Math.max(M.l, Math.min(a.W - M.r, a.X((lo(sp.it) + hi(sp.it)) / 2)));
  bx.style.left = p.cv.offsetLeft + cx + "px"; bx.style.top = p.cv.offsetTop + M.t + 2 + "px";
  sp.ts.forEach((/** @type {number} */ t, /** @type {number} */ i) => {
    const m = mks.shift() || p.el.appendChild(marker(p)); m._i = i;
    const x = a.X(t), v = T("int.split.mark", { rt: t.toFixed(3) });
    m.hidden = x < M.l || x > a.W - M.r;
    m.style.left = p.cv.offsetLeft + x + "px"; m.style.top = p.cv.offsetTop + M.t + "px"; m.style.height = a.H - M.t - M.b + "px"; m.style.color = s.color;
    m.setAttribute("aria-label", v); m.title = v; m.setAttribute("aria-valuenow", t.toFixed(3)); m.setAttribute("aria-valuetext", T("cursor.handle.value", { rt: t.toFixed(3) }));
    m.setAttribute("aria-valuemin", lo(sp.it).toFixed(3)); m.setAttribute("aria-valuemax", hi(sp.it).toFixed(3));
  });
  mks.forEach(m => m.remove());
  if (sp.focus !== false) {
    const f = /** @type {HTMLElement[]} */ ([...p.el.querySelectorAll(".ivl-mk")])[sp.focus === true ? 0 : sp.focus] || inp; sp.focus = false;
    try { f.focus({ preventScroll: true }); } catch (_) { /* gone */ }
  }
}
{                                                                  // the look: colours from the CSS variables, the marker's line in the colour of its trace
  const st = document.createElement("style");
  st.textContent = ".ivl-mk{position:absolute;z-index:4;width:24px;margin-left:-12px;padding:0;border:0;background:transparent;cursor:ew-resize;touch-action:none}" +
    ".ivl-mk::after{content:\"\";position:absolute;left:11px;top:0;bottom:0;border-left:2px dashed currentColor}" +
    ".ivl-mk:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}.ivl-mk.on{background:color-mix(in srgb,var(--accent) 20%,transparent)}" +
    ".ivl-bar{position:absolute;z-index:6;transform:translateX(-50%);display:flex;align-items:center;gap:6px;padding:2px 6px;background:var(--panel);border:1px solid var(--line);border-radius:4px;font-size:12px;color:var(--ink);white-space:nowrap}" +
    ".ivl-bar input{width:4em;min-height:24px}.ivl-bar button{min-height:24px;min-width:24px}.ivl-none:empty{display:none}" +
    "@media (forced-colors:active){.ivl-mk::after{border-color:CanvasText}.ivl-mk:focus-visible{outline-color:Highlight}}";
  document.head.appendChild(st);
}
window.INTVALLI = { start, sync, paint, confirm, cancel };
