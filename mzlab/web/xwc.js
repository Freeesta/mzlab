// @ts-strict
// XWC: the trace of the DAD at a band of wavelengths, extracted like an XIC. A split button on the TWC/XWC panel and on the UV spectrum: the main part opens
// the band the engine proposes (api/uvpropose: λmax of the strongest UV peak, or of the spectrum in view), Shift adds it to the XWC panel as one more trace
// (`p.xw`, undoable), the arrow opens a popover with λ and bandwidth. Loaded on first use (eventi.js wlCtl, uv-spettro.js, grafici.js); hidden without a matrix.
const T = (/** @type {string} */ k, /** @type {any} */ o = undefined) => I18N.t(k, o);
const g = /** @type {any} */ (window);
const num = (/** @type {string} */ v) => Number(String(v).trim().replace(",", "."));

/** The proposal of file k (window rt0..rt1 or none): {nm, bw, rt, n}, or null when the file has no matrix (the answer is kept per file and window). @param {number} k @param {number[] | null} [w] */
export function propose(k, w) {
  const f = g.E.files[k];
  if (!f) return Promise.resolve(null);
  const q = w ? `&rt0=${w[0]}&rt1=${w[1]}` : "", m = (f._xwc ||= {});
  return (m[q] ||= fetch(`api/uvpropose?k=${k}${q}`).then(r => r.json()).then(j => j && !j.error && isFinite(j.nm) ? j : null, () => null));
}

/** The files of a panel that can have a matrix (the files of its tab with a DAD, the one of a UV spectrum first). @param {any} p @returns {any[]} */
const dadFiles = p => {
  const fs = (typeof shownFor === "function" ? shownFor(p) : g.E.files).filter((/** @type {any} */ f) => f && f.pda && !f.gone);
  const own = p.uv ? g.E.files[p.k] : null;
  return own ? [own, ...fs.filter((/** @type {any} */ f) => f !== own)] : fs;
};
/** The first file of the panel with a matrix, or null. @param {any} p */
async function fileOf(p) {
  for (const f of dadFiles(p)) if (await propose(f.k)) return f;
  return null;
}

/** The XWC panel a Shift adds to: this one if it is an XWC, else the newest XWC of its tab. @param {any} from */
const target = from => from && from.type === "chrom" && from.kind === "pda" && from.mz0 != null ? from
  : [...g.E.panels].reverse().find((/** @type {any} */ q) => q.type === "chrom" && q.kind === "pda" && q.mz0 != null && q.tab === (from?.tab || g.E.tab)) || null;
/** The panel a new XWC goes under: the chromatogram of a UV spectrum. @param {any} from */
const anchor = from => from && from.type === "spec" ? g.E.panels.find((/** @type {any} */ q) => q.id === (from.link || from.src)) || from : from;

/** Opens the band nm0..nm1 of file k, or with `add` puts it in the XWC panel as one more trace. @param {any} from @param {number} nm0 @param {number} nm1 @param {number} k @param {boolean} add */
export async function band(from, nm0, nm1, k, add) {
  const p = add ? target(from) : null;
  if (!p) return (g.UVSPEC ||= await import("./uv-spettro.js")).openWl(anchor(from), nm0, nm1, k);
  if ((p.mz0 === nm0 && p.mz1 === nm1) || (p.xw || []).some((/** @type {number[]} */ b) => b[0] === nm0 && b[1] === nm1)) return p;
  COMANDI.fai("panel.source", p, () => { p.xw = [...(p.xw || []), [nm0, nm1]]; });
  draw(p); uiSave();
  return p;
}
const ends = (/** @type {number} */ nm, /** @type {number} */ bw) => [+(nm - bw / 2).toFixed(1), +(nm + bw / 2).toFixed(1)];

/** The main part of the button: the band the engine proposes for the panel (a UV spectrum: the one of its time window). @param {any} p @param {boolean} add */
export async function press(p, add) {
  const f = await fileOf(p);
  if (!f) return null;
  const w = p.uv && p.r0 != null ? [p.r0, p.r1 ?? p.r0] : p.type === "chrom" && p.sel ? p.sel : null;
  const j = (w && await propose(f.k, w)) || await propose(f.k);
  const [a, b] = ends(j.nm, j.bw);
  return band(p, a, b, f.k, add);
}

/** The series of the extra bands of an XWC panel (`p.xw`), one per file and band, after the panel's own. @param {any} p @param {any[]} fl */
export async function series(p, fl) {
  const out = await Promise.all((p.xw || []).flatMap((/** @type {number[]} */ b, /** @type {number} */ i) => fl.map(f => getChrom(f.k, "pda", p.flv ?? f.lv, b[0], b[1], null, p.filt, p.ptol).then((/** @type {any} */ d) => ({
    x: d.rt, y: d.y, color: PAL[(i + 1) % PAL.length], dash: fdash(f), name: `${f.label} · ${b[0]}–${b[1]} nm`, k: f.k, key: `c|${f.k}|pda|${b[0]}-${b[1]}`, ion: `${b[0]}–${b[1]} nm`, time: f.time
  })))));
  return out.filter(s => s.x.length);
}

// ---------------------------------------------------------------- the popover: λ and bandwidth; Enter opens (Shift+Enter: one more trace), Esc closes
/** @param {any} p @param {HTMLElement} lb the arrow */
async function pop(p, lb) {
  if (p._xwPop) { close(p); return; }
  const f = await fileOf(p), j = f && await propose(f.k);
  if (!f || !j) return;
  const el = document.createElement("div"); el.className = "sp-pop xb-pop"; el.setAttribute("role", "dialog"); el.setAttribute("aria-label", T("xwc.pop.aria"));
  const field = (/** @type {string} */ k, /** @type {number} */ v, /** @type {string} */ txt) => { const l = document.createElement("label"), i = document.createElement("input"); i.dataset.f = k; i.setAttribute("inputmode", "decimal"); i.autocomplete = "off"; i.value = String(v); l.append(txt + " ", i); return l; };
  const btn = (/** @type {string} */ k, /** @type {string} */ txt) => { const b = document.createElement("button"); b.type = "button"; b.className = "bt"; b.dataset.f = k; b.textContent = txt; return b; };
  const msg = document.createElement("div"), row = document.createElement("div");
  msg.className = "xb-err"; msg.setAttribute("role", "alert"); row.className = "xb-row";
  row.append(btn("go", T("xicbtn.pop.go")), btn("no", T("xicbtn.pop.close")));
  el.append(field("nm", j.nm, T("xwc.pop.nm")), field("bw", j.bw, T("xwc.pop.bw")), msg, row);
  const pr = p.el.getBoundingClientRect(), br = lb.getBoundingClientRect();
  p.el.appendChild(el); p._xwPop = el; lb.setAttribute("aria-expanded", "true");
  el.style.left = Math.max(4, Math.min(br.left - pr.left, p.el.clientWidth - el.offsetWidth - 6)) + "px"; el.style.top = br.bottom - pr.top + 4 + "px";
  const [fn, fb] = /** @type {HTMLInputElement[]} */ ([...el.querySelectorAll("input")]);
  const apply = (/** @type {boolean} */ add) => {
    const nm = num(fn.value), bw = num(fb.value), okN = isFinite(nm) && nm > 0, okB = isFinite(bw) && bw >= 0;
    fn.setAttribute("aria-invalid", String(!okN)); fb.setAttribute("aria-invalid", String(!okB));
    msg.textContent = okN && okB ? "" : T("xicbtn.pop.bad");
    if (!okN || !okB) { (okN ? fb : fn).focus(); return; }
    close(p); const [a, b] = ends(nm, bw); band(p, a, b, f.k, add);
  };
  el.addEventListener("keydown", e => {
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(p); }
    else if (e.key === "Enter" && e.target instanceof HTMLInputElement) { e.preventDefault(); apply(e.shiftKey); }
  });
  /** @type {HTMLElement} */ (el.querySelector('[data-f="go"]')).onclick = e => apply(e.shiftKey);
  /** @type {HTMLElement} */ (el.querySelector('[data-f="no"]')).onclick = () => close(p);
  fn.focus(); fn.select();
}
/** @param {any} p */
function close(p) {
  if (!p._xwPop) return;
  p._xwPop.remove(); p._xwPop = null;
  const l = p.el.querySelector('[data-a="xwcl"]'); if (l) { l.setAttribute("aria-expanded", "false"); l.focus({ preventScroll: true }); }
}

/** The arrow of the split button, after the main part. @param {any} p @param {HTMLElement} b */
function arrow(p, b) {
  let l = /** @type {HTMLButtonElement | null} */ (p.el.querySelector('[data-a="xwcl"]'));
  if (l) return l;
  l = document.createElement("button"); l.className = "bt xbl"; l.dataset.a = "xwcl"; l.type = "button"; l.textContent = "▾";
  l.setAttribute("aria-haspopup", "dialog"); l.setAttribute("aria-expanded", "false"); l.title = T("xwc.pop.open"); l.setAttribute("aria-label", T("xwc.pop.open"));
  l.onclick = () => pop(p, /** @type {HTMLElement} */ (l));
  b.after(l);
  return l;
}
/** Shows the split button where a file of the panel has a matrix: a TWC-only file (a DAD trace, no matrix) gets none. @param {any} p @param {HTMLElement} b @param {HTMLElement} l @param {boolean} on */
async function show(p, b, l, on) {
  const has = on && !!(await fileOf(p));
  b.hidden = !has && on; l.hidden = !has;
  if (!has) close(p);
}

/** The XIC button of a chromatogram panel (wlCtl): on a DAD panel it is the XWC split button; on another kind the plain XIC button. @param {any} p */
export function dress(p) {
  const b = /** @type {any} */ (p.el.querySelector('[data-a="xic"]'));
  if (!b) return;
  const pda = p.kind === "pda", l = arrow(p, b);
  if (!b._xw) { const o = b.onclick; b._xw = true; b.onclick = (/** @type {MouseEvent} */ e) => p.kind === "pda" ? press(p, e.shiftKey) : o && o.call(b, e); }
  show(p, b, l, pda);
}

/** The XWC split button of a UV spectrum (uv-spettro.js draw), in its XIC group. @param {any} p */
export function dressSpec(p) {
  let b = /** @type {HTMLButtonElement | null} */ (p.el.querySelector('[data-a="xwc"]'));
  if (!b) {
    const grp = p.el.querySelector('[data-a="xicb"]')?.closest(".tbg");
    if (!grp) return;
    b = document.createElement("button"); b.className = "bt xbb"; b.dataset.a = "xwc"; b.type = "button"; b.textContent = "XWC"; b.title = T("panel.wl.title");
    b.onclick = e => press(p, e.shiftKey);
    grp.append(b);
  }
  show(p, b, arrow(p, b), true);
}
