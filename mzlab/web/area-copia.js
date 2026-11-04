// @ts-strict
// Area label of an integration ("A = 1.234E+05"): a click on it, or «Copia valore» in the context menu, puts the number the label shows on the clipboard
// (as displayed, so it pastes into Excel). A small «Copiato!» (aria-live, a DOM element: never in a PNG/PDF) sits under the label and fades after 1.5 s.
// The label boxes (p._a.lbls, with `copy` and `int`) come from grafici.js; a press on one is taken here, before the chart starts a drag or moves the cursor.
/** @typedef {Record<string, any>} Bag */
const hit = (/** @type {Bag} */ p, /** @type {number} */ px, /** @type {number} */ py) => (p._a?.lbls || []).find((/** @type {Bag} */ b) => b.copy != null && px >= b.x && px <= b.x + b.w && py >= b.y && py <= b.y + b.h);
// the label for the menu and the keyboard: under the pointer, else of the integration at the time x, else the selected peak
const target = (/** @type {Bag} */ p, /** @type {number} */ x, /** @type {number} */ px, /** @type {number} */ py) => {
  const l = hit(p, px, py); if (l) return l;
  const it = p.ints.find((/** @type {Bag} */ i) => x >= iLo(i) && x <= iHi(i)) || p.ints.find((/** @type {Bag} */ i) => i.id === p.isel);
  return it ? (p._a?.lbls || []).find((/** @type {Bag} */ b) => b.int === it && b.copy != null) : undefined;
};
async function copy(/** @type {Bag} */ p, /** @type {Bag} */ l) {
  try { await navigator.clipboard.writeText(l.copy); } catch (_) { if (window.toast) toast(I18N.t("bn.noClip")); return; }
  let n = /** @type {HTMLElement | null} */ (p.el.querySelector(".areacopied"));
  if (!n) { n = document.createElement("div"); n.className = "areacopied"; n.setAttribute("role", "status"); n.setAttribute("aria-live", "polite"); p.el.appendChild(n); }
  const el = n; el.textContent = ""; el.classList.remove("show");
  el.style.left = p.cv.offsetLeft + l.x + l.w / 2 + "px"; el.style.top = p.cv.offsetTop + l.y + l.h + 2 + "px";
  setTimeout(() => { el.textContent = I18N.t("int.copy.done"); el.classList.add("show"); }, 20);      // text set after the clear: a screen reader announces every copy
  clearTimeout(p._acT); p._acT = setTimeout(() => el.classList.remove("show"), 1500);
}
/** @type {{ p: Bag, l: Bag } | null} */ let down = null;
const where = (/** @type {MouseEvent} */ e, /** @type {Bag} */ p) => { const r = p.cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
const panelOf = (/** @type {EventTarget | null} */ t) => (t instanceof HTMLCanvasElement ? E.panels.find((/** @type {Bag} */ q) => q.cv === t) : undefined);
document.addEventListener("mousedown", e => {
  down = null; const p = e.button === 0 && !e.shiftKey ? panelOf(e.target) : undefined; if (!p) return;
  const [px, py] = where(e, p), l = hit(p, px, py); if (!l) return;
  down = { p, l }; e.stopPropagation(); front(p.el); setActive(p);                  // no drag, no cursor move, no selection change; the panel still comes forward
}, true);
document.addEventListener("mouseup", e => {
  const d = down; down = null; if (!d || e.target !== d.p.cv) return;
  const [px, py] = where(e, d.p); const l = hit(d.p, px, py); if (l) copy(d.p, l);
}, true);
if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "int.copyArea", gruppo: "tools", ordine: 0, viste: ["chrom", "xic", "mrm"],
  visibile: (/** @type {Bag} */ c) => !!target(c.p, c.x, c.px, c.py), nome: () => I18N.t("int.copy.menu"),
  esegui: (/** @type {Bag} */ c) => { const l = target(c.p, c.x, c.px, c.py); if (l) copy(c.p, l); } });
window.AREACOPY = { target, copy };
