// @ts-strict
// ‹ › in the header of the MS2 panel of a DDA trio: the previous / next MS2 of the same precursor (within the tolerance of the profile), «i/N» between them.
// Shift+click walks all the MS2 in time, as ← → do. Loaded by dda-marcatori.js on the first drawing of an MS2 of a trio; `I` is the inside of DDA.
// An MS2 is shown as acquired: the precursor m/z groups the scans, no compound name.
const T = (/** @type {string} */ k, /** @type {any} */ o = {}) => I18N.t(k, o);

/** The MS2 of the file whose precursor is the one shown, in the order of the acquisition, and where the shown one is among them.
 * @param {Bag} I @param {Bag} p @returns {{ D: Bag, same: number[], at: number } | null} */
function same(I, p) {
  const D = p.k != null ? I.ready(p.k) : null, i = D && p.sid != null ? D.idx.get(p.sid) : null;
  if (!D || i == null || D.prec[i] == null || p.avg) return null;
  const m = D.prec[i], t = I.tolOf(E.files[p.k], m), out = [];
  for (let j = 0; j < D.sid.length; j++) if (D.prec[j] != null && Math.abs(D.prec[j] - m) <= t) out.push(j);
  return { D, same: out, at: out.indexOf(i) };
}
/** The group ‹ i/N ›, made once, before «XIC» in the header (the group that a tight header never moves into «⋯»). @param {Bag} I @param {Bag} p @returns {HTMLElement} */
function group(I, p) {
  const old = /** @type {HTMLElement | null} */ (p.el.querySelector(".ddanav")); if (old) return old;
  const g = document.createElement("span"); g.className = "ddanav"; g.setAttribute("role", "group"); g.setAttribute("aria-label", T("dda.nav.group"));
  const btn = (/** @type {number} */ d) => {
    const b = document.createElement("button"); b.className = "bt ddanb"; b.dataset.a = d < 0 ? "ddaprev" : "ddanext"; b.textContent = d < 0 ? "‹" : "›";
    for (const t of ["keydown", "keyup"]) b.addEventListener(t, e => { if (/** @type {KeyboardEvent} */ (e).key === " ") e.stopPropagation(); });   // Space presses it: the walk over the scans must not take it
    b.onclick = e => go(I, p, d, e.shiftKey);
    return b;
  };
  const n = document.createElement("span"); n.className = "ddann"; n.setAttribute("aria-hidden", "true");    // the count is in the names of the buttons
  g.append(btn(-1), n, btn(1));
  const xb = p.el.querySelector('[data-a="xicb"]');
  if (xb) xb.before(g); else p.el.querySelector(".hd")?.append(g);
  return g;
}
/** The state of ‹ i/N › for what the panel shows (nothing, an average or one MS2). @param {Bag} I @param {Bag} p */
export function sync(I, p) {
  if (!p.el) return;
  const s = same(I, p), g = group(I, p), n = /** @type {HTMLElement} */ (g.querySelector(".ddann"));
  const i = s ? s.at + 1 : 0, N = s ? s.same.length : 0, txt = s ? `${i}/${N}` : "–";
  if (n.textContent !== txt) n.textContent = txt;
  for (const b of /** @type {NodeListOf<HTMLButtonElement>} */ (g.querySelectorAll("button"))) {
    const d = b.dataset.a === "ddaprev" ? -1 : 1, lab = T(d < 0 ? "dda.nav.prev" : "dda.nav.next", { i: i || "–", n: N || "–" });
    if (b.title !== lab) { b.title = lab; b.setAttribute("aria-label", lab); }
    b.setAttribute("aria-disabled", String(!s || (d < 0 ? i <= 1 : i >= N)));          // stays focusable: Shift+click still walks in time
  }
}
/** One step: the next MS2 of this precursor, or with Shift the next MS2 in time (DDA.step). @param {Bag} I @param {Bag} p @param {number} d @param {boolean} shift */
function go(I, p, d, shift) {
  if (shift || p.sid == null) { window.DDA.step(p, d); return; }
  const s = same(I, p); if (!s) return;
  const j = s.same[s.at + d];
  if (j == null) { nearMsg(T(s.same.length > 1 ? (d > 0 ? "dda.nav.last" : "dda.nav.first") : "dda.nav.only")); return; }
  I.selectMs2(p, s.D.sid[j], { zoom: false });
}
window.DDANAV = { sync };
