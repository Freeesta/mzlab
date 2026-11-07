// @ts-strict
// The «N MS²» button of the Full Scan of a DDA trio: how many MS2 started from the scan(s) shown and their list, one row per precursor
// (/api/dda_window, Rust). Click, Enter or Space opens the list; ↑ ↓ Home End move, Enter shows the most intense MS2 of the precursor, Esc closes.
// Also «▲ MS²», the switch of the triangles of the MS2 on the chromatogram of the trio (`tri`). Loaded by dda-marcatori.js on the first drawing of a DDA Full Scan; `I` is the inside of DDA. An MS2 is shown as acquired: no compound name.
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);

/** The button, made once, in its own group before the one of «XIC» in the header. @param {Bag} I @param {Bag} p @returns {HTMLButtonElement} */
function button(I, p) {
  const old = /** @type {HTMLButtonElement | null} */ (p.el.querySelector('[data-a="ddaw"]'));
  if (old) return old;
  const b = document.createElement("button");
  b.className = "bt ddaw"; b.dataset.a = "ddaw"; b.setAttribute("aria-haspopup", "listbox"); b.setAttribute("aria-expanded", "false");
  for (const t of ["keydown", "keyup"]) b.addEventListener(t, e => { if (/** @type {KeyboardEvent} */ (e).key === " ") e.stopPropagation(); });   // Space presses it: the walk over the scans must not take it
  b.onclick = () => { if (b.getAttribute("aria-disabled") !== "true") open(I, p, b); };
  const g = document.createElement("span"); g.className = "tbg"; g.append(b);       // a group of its own: a tight header moves it into «⋯» (fitHd), «XIC» stays
  const xg = p.el.querySelector('[data-a="xicb"]')?.closest(".tbg");
  if (xg) xg.before(g); else p.el.querySelector(".hd")?.append(g);
  fitHdSoon(p);
  return b;
}
/** The text of the button for the answer of the window (null = not here yet); `why` = the reason of an empty MS2 panel.
 * @param {Bag} I @param {Bag} p @param {Bag | null} w @param {string} why */
export function sync(I, p, w, why) {
  const b = button(I, p), n = w ? w.n : 0, one = w && w.ms1 === 1, txt = T("dda.win.btn", { n });
  const tip = !w ? T("dda.win.wait") : !n ? why : one ? T("dda.win.one") : T("dda.win.avg", { n: w.ms1 });
  if (b.textContent !== txt) { b.textContent = txt; fitHdSoon(p); }
  if (b.title !== tip) { b.title = tip; b.setAttribute("aria-label", txt + ". " + tip); }
  b.setAttribute("aria-disabled", String(!n));
}
/** @param {Bag} p */
function close(p) {
  const pop = p.el.querySelector(".dw-pop"); if (!pop) return;
  if (p._dwOff) p._dwOff();
  pop.remove();
  const b = p.el.querySelector('[data-a="ddaw"]'); if (b) b.setAttribute("aria-expanded", "false");
}
/** The list under the button. @param {Bag} I @param {Bag} p @param {HTMLButtonElement} b */
function open(I, p, b) {
  const w = p._dw; if (!w || !w.n) return;
  if (p.el.querySelector(".dw-pop")) { close(p); return; }
  const s2 = I.target(p), dec = HR.prof(E.files[s2 ? s2.k : p.k], 2).dec;
  const pop = document.createElement("div");
  pop.className = "sp-pop dw-pop"; pop.setAttribute("role", "listbox"); pop.tabIndex = 0; pop.setAttribute("aria-label", T("dda.win.list", { n: w.n }));
  /** @type {HTMLElement[]} */
  const opts = w.groups.map((/** @type {Bag} */ q, /** @type {number} */ i) => {
    const o = document.createElement("div"), a = Math.min(...q.rt), z = Math.max(...q.rt);
    o.id = `dw${p.id}-${i}`; o.setAttribute("role", "option"); o.setAttribute("aria-selected", "false");
    o.textContent = T(q.sid.length > 1 ? "dda.win.rowN" : "dda.win.row1", { i: i + 1, prec: q.prec.toFixed(dec), n: q.sid.length, rt: a.toFixed(2), rt1: z.toFixed(2) });
    o.onclick = () => choose(I, p, q);
    return o;
  });
  let at = 0;
  const move = (/** @type {number} */ i) => { opts[at].setAttribute("aria-selected", "false"); at = Math.max(0, Math.min(opts.length - 1, i)); opts[at].setAttribute("aria-selected", "true"); pop.setAttribute("aria-activedescendant", opts[at].id); opts[at].scrollIntoView({ block: "nearest" }); };
  pop.append(...opts);
  pop.addEventListener("keydown", e => {
    const k = e.key, step = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: opts.length - 1 }[k];
    if (step != null) move(step);
    else if (k === "Enter" || k === " ") choose(I, p, w.groups[at]);
    else if (k === "Escape") { close(p); b.focus(); }
    else if (k === "Tab") { close(p); return; }
    else return;
    e.preventDefault(); e.stopPropagation();
  });
  const br = b.getBoundingClientRect(), pr = p.el.getBoundingClientRect();
  pop.style.top = Math.round(br.bottom - pr.top + 4) + "px"; pop.style.left = Math.max(8, Math.round(br.left - pr.left)) + "px";
  p.el.append(pop); b.setAttribute("aria-expanded", "true"); move(0); pop.focus();
  const ac = new AbortController(); p._dwOff = () => ac.abort();
  p._ac.signal.addEventListener("abort", () => ac.abort(), { once: true, signal: ac.signal });   // the panel closing ends the listener too (the floor is Safari 16.4)
  document.addEventListener("mousedown", e => { const t = /** @type {HTMLElement} */ (e.target); if (!t.closest(".dw-pop") && !t.closest('[data-a="ddaw"]')) close(p); }, { capture: true, signal: ac.signal });
}
/** The most intense MS2 of a precursor in the MS2 panel; the Full Scan stays as it is. @param {Bag} I @param {Bag} p @param {Bag} q */
function choose(I, p, q) {
  close(p);
  const s2 = I.target(p); if (!s2) return;
  const b = q.pint.reduce((/** @type {number} */ m, /** @type {number | null} */ v, /** @type {number} */ i) => ((v ?? -1) > (q.pint[m] ?? -1) ? i : m), 0);
  I.selectMs2(s2, q.sid[b], { parent: false, zoom: false });
  const bt = /** @type {HTMLElement | null} */ (p.el.querySelector('[data-a="ddaw"]')); if (bt) bt.focus();
}
/** «▲ MS²» first among the buttons of the chromatogram (or followed XIC) of the trio: the triangles of the MS2 scans, otherwise only in its «Intervallo» box.
 * @param {Bag | null} c */
export function tri(c) {
  if (!c || !c.el) return;
  let b = /** @type {HTMLButtonElement | null} */ (c.el.querySelector('[data-a="ms2tri"]'));
  if (!b) {
    const at = c.el.querySelector(".hd .tbs"); if (!at) return;
    b = document.createElement("button"); b.type = "button"; b.className = "bt tgl hdkeep"; b.dataset.a = "ms2tri"; b.textContent = T("dda.tri.btn");
    b.title = T("ctl.ms2tri.title"); b.setAttribute("aria-label", T("ctl.ms2tri"));
    b.onclick = () => { c.ms2tri = !c.ms2tri; ctl(c); draw(c); uiSave(); tri(c); };
    const g = document.createElement("span"); g.className = "tbg"; g.append(b); at.prepend(g); fitHdSoon(c);          // a group of its own that a tight header keeps in sight (fitHd)
  }
  const on = String(!!c.ms2tri); if (b.getAttribute("aria-pressed") !== on) { b.setAttribute("aria-pressed", on); b.classList.toggle("on", !!c.ms2tri); }
}
window.DDAELENCO = { sync, tri };
