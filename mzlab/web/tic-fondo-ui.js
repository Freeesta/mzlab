// @ts-strict
// One button on a TIC/BPC chromatogram: the trace without the chemical background, ion by ion (api/tic_background, engine ticbg.rs; docs/agenti/tic-background.md).
// Click = automatic removal, click again = the raw trace; right click, long press or ↓ opens the options (mode, action, blank, RT window, m/z windows, kept ions,
// persistence) and the table of the background ions. Loaded by grafici.js on the first drawing of a TIC/BPC; state p.ticbg = { on, mode, action, blank, rt0, rt1, ex, keep, persist }.
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);
const IC = '<svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M1 13h3l2-9 2 9h7"/><path d="M1 10.5h14" stroke-dasharray="2 2"/></svg>';
const IC_TAB = '<svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="1.5" y="2.5" width="13" height="11" rx="1"/><path d="M1.5 6.5h13M1.5 10h13M6 2.5v11"/></svg>';
const LONG_MS = 550;

/** a whole TIC or BPC of the level: the only traces the route serves (no m/z range, precursor or filter) @param {Bag} p */
export const applies = p => p.type === "chrom" && (p.kind === "tic" || p.kind === "bpc") && p.mz0 == null && p.mz1 == null && p.prec == null && !p.filt;
/** @param {Bag} p */
export const on = p => applies(p) && !!(p.ticbg && p.ticbg.on);

/** the request of one file: only what differs from the engine defaults is written, so the URL matches `mzlab-cli trace background --set` @param {Bag} p @param {Bag} f */
export function url(p, f) {
  const o = p.ticbg || {}, q = new URLSearchParams({ k: String(f.k), level: String(p.flv ?? f.lv ?? 1), kind: p.kind });
  if (o.mode && o.mode !== "auto") q.set("mode", o.mode);
  if (o.action === "exclude") q.set("action", "exclude");
  if (o.mode === "blank" && o.blank != null && o.blank !== "") q.set("blank", String(o.blank));
  if (o.mode === "quiet") { if (o.rt0 !== "" && o.rt0 != null) q.set("rt0", String(o.rt0)); if (o.rt1 !== "" && o.rt1 != null) q.set("rt1", String(o.rt1)); }
  for (const k of ["ex", "keep", "persist"]) if (o[k] !== "" && o[k] != null) q.set(k, String(o[k]).replace(/\s+/g, ""));
  return "api/tic_background?" + q.toString();
}

/** the traces of the panel: the cleaned `y` of each shown file; a refused request shows the raw trace and says why (nothing is hidden) @param {Bag} p @param {Bag[]} fl */
export async function series(p, fl) {
  p._tb = {};
  return Promise.all(fl.map(async f => {
    const u = url(p, f);
    let d;
    try { d = await memo(u, () => J(u)); p._tb[f.k] = d; }
    catch (e) { info(EH(/** @type {Error} */ (e).message || String(e))); d = await getChrom(f.k, p.kind, p.flv ?? f.lv); }
    return { x: d.rt, y: d.y, color: f.color, dash: fdash(/** @type {any} */ (f)), name: f.label, k: f.k, key: `c|${f.k}|${p.kind}`, ion: kindUp(p), time: f.time };
  }));
}

/** the button is the registry action `panel.ticbg` (azioni-barra.js); this wires its pointer and keys once and redraws its state when what the route serves or the switch changed @param {Bag} p */
export function mount(p) {
  const b = /** @type {HTMLButtonElement | null} */ (p.el && p.el.querySelector('[data-a="ticbg"]'));
  if (!b) return;
  if (!b.dataset.i) { b.dataset.i = "1"; b.innerHTML = IC; wire(p, b); }   // the icon comes with this lazy module: the button is hidden until now
  const sig = `${applies(p)}${on(p)}`, tip = T(on(p) ? "ctl.ticbg.off" : "ctl.ticbg.on");
  if (p._tbSig !== sig) { p._tbSig = sig; AZIONI.refresh({ view: "chrom", p }); }
  if (b.title !== tip) { b.title = tip; b.setAttribute("aria-label", tip); }
}

/** the click of the action: toggles, unless it ends a long press that opened the options @param {Bag} p */
export function press(p) {
  if (p._tbLong) { p._tbLong = false; return; }
  p.ticbg = { ...(p.ticbg || {}), on: !on(p) }; close(p); draw(p);
}

/** a long press, right click, ↓, Shift+F10 or the menu key open the options (the click itself is `press`) @param {Bag} p @param {HTMLButtonElement} b */
export function wire(p, b) {
  if (b.dataset.w) return; b.dataset.w = "1";   // once per button: the header one and the twin the bar redraws
  let timer = 0;
  const stop = () => { clearTimeout(timer); timer = 0; };
  b.addEventListener("pointerdown", e => { if (e.button !== 0) return; p._tbLong = false; stop(); timer = window.setTimeout(() => { p._tbLong = true; open(p, b); }, LONG_MS); });
  for (const t of ["pointerup", "pointerleave", "pointercancel"]) b.addEventListener(t, stop);
  b.addEventListener("contextmenu", e => { e.preventDefault(); e.stopPropagation(); open(p, b); });
  b.addEventListener("keydown", e => { if (e.key === "ArrowDown") { e.preventDefault(); e.stopPropagation(); open(p, b); } });
}

/** the header button and its twin in the shared bar (only while the bar shows this panel) @param {Bag} p @returns {HTMLElement[]} */
const twins = p => /** @type {HTMLElement[]} */ ([p.el.querySelector('[data-a="ticbg"]'), window.PBAR?.on() && E.active === p ? document.querySelector('#pbar [data-a="ticbg"]') : null].filter(Boolean));
/** @param {Bag} p @param {boolean} [back] focus returns to the button that opened the options (Esc) */
function close(p, back = false) {
  const pop = p.el && p.el.querySelector(".tb-pop"); if (!pop) return;
  if (p._tbOff) p._tbOff();
  pop.remove();
  const l = twins(p); for (const x of l) x.setAttribute("aria-expanded", "false");
  if (back) (l.find(a => a.offsetParent) || l[0])?.focus({ preventScroll: true });   // the bar twin when the header one is hidden
}

/** the options of the removal, with no text on screen: each control says what it is in its aria-label and title @param {Bag} p @param {HTMLButtonElement} b */
function open(p, b) {
  if (p.el.querySelector(".tb-pop")) { close(p, true); return; }
  const o = p.ticbg = { mode: "auto", action: "subtract", ...(p.ticbg || {}) };
  const opt = (/** @type {string} */ v, /** @type {string} */ k, /** @type {unknown} */ cur) => h`<option value="${v}"${raw(String(cur) === v ? " selected" : "")}>${T(k)}</option>`;
  const sel = (/** @type {string} */ k, /** @type {string} */ key, /** @type {string} */ opts) => h`<select data-t="${k}" aria-label="${T(key)}" title="${T(key)}">${raw(opts)}</select>`;
  const inp = (/** @type {string} */ k, /** @type {string} */ key, /** @type {string} */ ph, /** @type {string} */ extra = "") => h`<input data-t="${k}" aria-label="${T(key)}" title="${T(key)}" placeholder="${ph}" value="${o[k] ?? ""}" ${raw(extra)}>`;
  const others = E.files.filter((/** @type {Bag} */ f) => f && f.k !== p.k);
  const pop = document.createElement("div");
  pop.className = "sp-pop tb-pop"; pop.setAttribute("role", "dialog"); pop.setAttribute("aria-label", T("ctl.ticbg.opts"));
  pop.innerHTML = h`${raw(sel("mode", "ctl.ticbg.mode", opt("auto", "ctl.ticbg.mode.auto", o.mode) + opt("quiet", "ctl.ticbg.mode.quiet", o.mode) + opt("blank", "ctl.ticbg.mode.blank", o.mode)))}`
    + sel("action", "ctl.ticbg.action", opt("subtract", "ctl.ticbg.action.subtract", o.action) + opt("exclude", "ctl.ticbg.action.exclude", o.action))
    + sel("blank", "ctl.ticbg.blank", others.map((/** @type {Bag} */ f) => h`<option value="${f.k}"${raw(String(o.blank) === String(f.k) ? " selected" : "")}>${f.label || f.file}</option>`).join(""))
    + h`<span class="tb-rt">${raw(inp("rt0", "ctl.ticbg.rt0", "t0", 'type="number" step="0.1" min="0"') + inp("rt1", "ctl.ticbg.rt1", "t1", 'type="number" step="0.1" min="0"'))}</span>`
    + inp("ex", "ctl.ticbg.ex", "m/z:tol") + inp("keep", "ctl.ticbg.keep", "m/z") + inp("persist", "ctl.ticbg.persist", "0.8", 'type="number" step="0.05" min="0.05" max="1"')
    + h`<button type="button" class="bt" data-t="table" aria-label="${T("ctl.ticbg.table")}" title="${T("ctl.ticbg.table")}">${raw(IC_TAB)}</button>`;
  const Q = (/** @type {string} */ k) => /** @type {HTMLInputElement} */ (pop.querySelector(`[data-t="${k}"]`));
  const shape = () => {
    Q("blank").hidden = o.mode !== "blank"; /** @type {HTMLElement} */ (pop.querySelector(".tb-rt")).hidden = o.mode !== "quiet";
    if (o.mode === "blank" && (o.blank == null || o.blank === "") && others.length) o.blank = String(others[0].k);
  };
  pop.addEventListener("change", e => {
    const x = /** @type {HTMLInputElement} */ (e.target), k = x.dataset.t; if (!k) return;
    o[k] = x.value.trim(); o.on = true; shape();
    if (o.mode !== "quiet" || (o.rt0 && o.rt1)) draw(p);         // a quiet window is asked for once both ends are written
  });
  Q("table").onclick = () => table(p);
  pop.addEventListener("keydown", e => { if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(p, true); } });
  shape();
  const pr = p.el.getBoundingClientRect(), br = (p.el.contains(b) ? b : p.el.querySelector(".hd") || p.el).getBoundingClientRect();   // a button in the bar is far from the panel: the options go under the header
  pop.style.top = Math.round(br.bottom - pr.top + 4) + "px"; pop.style.left = Math.max(8, Math.round(Math.min(br.left - pr.left, pr.width - 250))) + "px";
  p.el.append(pop); for (const x of twins(p)) x.setAttribute("aria-expanded", "true"); Q("mode").focus();
  const ac = new AbortController(); p._tbOff = () => ac.abort();
  p._ac.signal.addEventListener("abort", () => ac.abort(), { once: true, signal: ac.signal });
  document.addEventListener("pointerdown", e => { const t = /** @type {HTMLElement} */ (e.target); if (!t.closest(".tb-pop") && !t.closest('[data-a="ticbg"]')) close(p); }, { capture: true, signal: ac.signal });
}

/** the background ions of every shown file and the canonical parameters of each answer (TDATI) @param {Bag} p */
export function table(p) {
  const tb = p._tb || {}, rows = [], notes = [], n = (/** @type {number} */ v, /** @type {number} */ d) => TDATI.num(v, d);
  for (const k of Object.keys(tb)) {
    const d = tb[k], f = E.files[+k], name = f ? f.label || f.file : k;
    for (const i of d.ions || []) rows.push([name, n(i.mz, 4), n(i.lo, 4), n(i.hi, 4), n(i.persistence, 3), i.cv == null ? "" : n(i.cv, 3), n(i.share, 4), n(i.confidence, 3)]);
    notes.push(`${name}: ${d.params}`);
  }
  return TDATI.table({ title: T("ctl.ticbg.table"), head: [T("ctl.ticbg.col.file"), "m/z", T("ctl.ticbg.col.lo"), T("ctl.ticbg.col.hi"), T("ctl.ticbg.col.persist"), "CV", T("ctl.ticbg.col.share"), T("ctl.ticbg.col.conf")], rows, note: notes.join("\n") });
}
