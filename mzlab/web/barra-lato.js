// @ts-check
// Drop targets of the side-panel drag (barra.js, bindSideDrag): two edge strips as wide as the panel really is, a small ghost that follows the
// pointer, and the preview of the workspace sliding to the free side. Loaded by barra.js on the first press of the panel header.

/** @param {string} cls @param {string} text */
const el = (cls, text) => {
  const d = document.createElement("div");
  d.className = cls; d.textContent = text; d.setAttribute("aria-hidden", "true");
  document.body.appendChild(d);
  return d;
};
/** @param {Element} z @param {number} x @param {number} y */
const inside = (z, x, y) => { const b = z.getBoundingClientRect(); return x >= b.left && x <= b.right && y >= b.top && y <= b.bottom; };

/**
 * Updates strips, ghost and preview for the pointer at (x, y); returns the side under it, or null (a release there cancels).
 * @param {{ x: number, y: number, width: number, sheet: boolean, side: string, title: string, texts: { l: string, r: string, no: string } }} o
 * @returns {"left" | "right" | null}
 */
function show(o) {
  const l = document.querySelector(".sb-dz.l") || el("sb-dz l", o.texts.l);
  const r = document.querySelector(".sb-dz.r") || el("sb-dz r", o.texts.r);
  const g = document.querySelector(".sb-ghost") || el("sb-ghost", "");
  const w = Math.round(Math.min(o.sheet ? innerWidth / 2 : o.width, innerWidth / 2));
  /** @type {HTMLElement} */ (l).style.width = /** @type {HTMLElement} */ (r).style.width = w + "px";
  document.documentElement.style.setProperty("--sb-dz-w", w + "px");
  const side = inside(l, o.x, o.y) ? "left" : inside(r, o.x, o.y) ? "right" : null;
  l.classList.toggle("hot", side === "left");
  r.classList.toggle("hot", side === "right");
  g.classList.toggle("no", !side);
  g.textContent = side ? o.title : o.texts.no;
  /** @type {HTMLElement} */ (g).style.transform = `translate(${Math.round(o.x + 14)}px,${Math.round(o.y + 14)}px)`;
  document.body.dataset.sbShift = !o.sheet && side && side !== o.side ? (side === "right" ? "l" : "r") : "";
  return side;
}

function clear() {
  delete document.body.dataset.sbShift;
  document.documentElement.style.removeProperty("--sb-dz-w");
  document.querySelectorAll(".sb-dz,.sb-ghost").forEach(z => z.remove());
}

window.BARRA_LATO = { show, clear };
