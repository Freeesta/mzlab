// @ts-strict
// Area label of an integration ("A = 1.234E+05"): a click on it, or «Copia valore» in the context menu, puts the area on the clipboard at the precision of
// the CSV (1.23456E+05, so it pastes into Excel). A small «Copiato!» (aria-live, a DOM element: never in a PNG/PDF) sits by the pointer and fades after 2 s;
// a refused clipboard says so in the same place. `put` serves the other copyable values too (the cells of the «Manual integrations» table).
// The label boxes (p._a.lbls, with `copy` and `int`) come from grafici.js; a press on one is taken here, before the chart starts a drag or moves the cursor.
/** @typedef {Record<string, any>} Bag */
const hit = (/** @type {Bag} */ p, /** @type {number} */ px, /** @type {number} */ py) => (p._a?.lbls || []).find((/** @type {Bag} */ b) => b.copy != null && px >= b.x && px <= b.x + b.w && py >= b.y && py <= b.y + b.h);
// the label for the menu and the keyboard: under the pointer, else of the integration at the time x, else the selected peak
const target = (/** @type {Bag} */ p, /** @type {number} */ x, /** @type {number} */ px, /** @type {number} */ py) => {
  const l = hit(p, px, py); if (l) return l;
  const it = p.ints.find((/** @type {Bag} */ i) => x >= iLo(i) && x <= iHi(i)) || p.ints.find((/** @type {Bag} */ i) => i.id === p.isel);
  return it ? (p._a?.lbls || []).find((/** @type {Bag} */ b) => b.int === it && b.copy != null) : undefined;
};
/** @type {number | undefined} */ let tm;
// the notice at a point of the window (client pixels): «Copiato!» or the honest failure, 2 s
function notice(/** @type {number} */ x, /** @type {number} */ y, /** @type {boolean} */ ok) {
  let n = /** @type {HTMLElement | null} */ (document.querySelector("body > .areacopied"));
  if (!n) { n = document.createElement("div"); n.className = "areacopied"; n.setAttribute("role", "status"); n.setAttribute("aria-live", "polite"); document.body.appendChild(n); }
  const el = n; el.textContent = ""; el.classList.remove("show"); el.classList.toggle("bad", !ok);
  el.style.left = Math.max(40, Math.min(innerWidth - 40, x)) + "px"; el.style.top = Math.min(innerHeight - 24, y + 14) + "px";
  setTimeout(() => { el.textContent = I18N.t(ok ? "int.copy.done" : "bn.noClip"); el.classList.add("show"); }, 20);      // text set after the clear: a screen reader announces every copy
  clearTimeout(tm); tm = window.setTimeout(() => el.classList.remove("show"), 2000);
}
// puts a text on the clipboard (called in the click or key handler: Safari asks for the gesture) and says how it went at (x, y)
async function put(/** @type {string} */ text, /** @type {number} */ x, /** @type {number} */ y) {
  let ok = true; try { await navigator.clipboard.writeText(text); } catch (_) { ok = false; }
  notice(x, y, ok); return ok;
}
// a label: by the pointer when there is one, else under the label
function copy(/** @type {Bag} */ p, /** @type {Bag} */ l, /** @type {number[] | null} */ at = null) {
  const r = p.cv.getBoundingClientRect();
  return put(l.copy, at ? at[0] : r.left + l.x + l.w / 2, at ? at[1] : r.top + l.y + l.h);
}
/** @type {{ p: Bag, l: Bag } | null} */ let down = null;
const where = (/** @type {MouseEvent} */ e, /** @type {Bag} */ p) => { const r = p.cv.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
const panelOf = (/** @type {EventTarget | null} */ t) => (t instanceof HTMLCanvasElement ? E.panels.find((/** @type {Bag} */ q) => q.cv === t) : undefined);
document.addEventListener("mousedown", e => {
  down = null; const p = e.button === 0 && !e.shiftKey ? panelOf(e.target) : undefined; if (!p) return;
  const [px, py] = where(e, p), l = hit(p, px, py); if (!l || p.ints.some((/** @type {Bag} */ i) => Math.abs(p._a.X(i.a) - px) < 6 || Math.abs(p._a.X(i.b) - px) < 6)) return;   // a border under the label still drags (eventi.js edgeAt)
  down = { p, l }; e.stopPropagation(); front(p.el); setActive(p);                  // no drag, no cursor move, no selection change; the panel still comes forward
}, true);
document.addEventListener("mouseup", e => {
  const d = down; down = null; if (!d || e.target !== d.p.cv) return;
  const [px, py] = where(e, d.p); const l = hit(d.p, px, py); if (l) copy(d.p, l, [e.clientX, e.clientY]);
}, true);
if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "int.copyArea", gruppo: "tools", ordine: 0, viste: ["chrom", "xic", "mrm"],
  visibile: (/** @type {Bag} */ c) => !!target(c.p, c.x, c.px, c.py), nome: () => I18N.t("int.copy.menu"),
  esegui: (/** @type {Bag} */ c) => { const l = target(c.p, c.x, c.px, c.py); if (l) copy(c.p, l); } });
// the baseline of the integration under the pointer or selected (key B): its dialog in integra-base.js, loaded on demand
if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "int.baseline", gruppo: "tools", ordine: 0, viste: ["chrom", "xic", "mrm"], scorciatoia: { tasto: "b", dove: "chart", et: "scorc.base" },
  visibile: (/** @type {Bag} */ c) => !!target(c.p, c.x, c.px, c.py), nome: () => I18N.t("int.base.menu"),
  esegui: (/** @type {Bag} */ c) => { const l = target(c.p, c.x, c.px, c.py); if (l) carica("integra-base.js").then(() => INTBASE.open(c.p, l.int)); } });
window.AREACOPY = { target, copy, put };
