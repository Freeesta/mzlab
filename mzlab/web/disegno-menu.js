// @ts-check
// Rows of the Draw context menu (#ctx) as real menu items: role, roving tabindex, arrows / Enter / Space / Esc, the pattern of explore.js menu().
// Shared by the canvas menu and the label menu of draw.js.
export function ctxRow(/** @type {HTMLElement} */ m, /** @type {string} */ text, /** @type {() => void} */ fn, /** @type {string | undefined} */ title = undefined) {
  const e = document.createElement("div");
  e.textContent = text; e.setAttribute("role", "menuitem"); e.tabIndex = -1; e.onclick = fn;
  if (title) e.title = title;
  e.addEventListener("keydown", (/** @type {KeyboardEvent} */ k) => {
    const its = [...m.querySelectorAll("div[role=menuitem]")], i = its.indexOf(e);
    if (k.key === "ArrowDown" || k.key === "ArrowUp") { k.preventDefault(); k.stopPropagation(); /** @type {HTMLElement} */ (its[(i + (k.key === "ArrowDown" ? 1 : -1) + its.length) % its.length]).focus(); }
    else if (k.key === "Enter" || k.key === " ") { k.preventDefault(); k.stopPropagation(); e.click(); }
    else if (k.key === "Escape") { m.hidden = true; const r = /** @type {any} */ (m)._ret; if (r && r.focus) r.focus(); }
  });
  m.appendChild(e);
}
export function ctxShow(/** @type {any} */ m, /** @type {number} */ x, /** @type {number} */ y, /** @type {number} */ w, /** @type {boolean} */ kbd) {
  m.setAttribute("role", "menu"); m._ret = kbd ? document.activeElement : null; m._lp = false;
  m.hidden = false;
  m.style.left = Math.min(x, innerWidth - w) + "px"; m.style.top = Math.max(4, Math.min(y, innerHeight - m.offsetHeight - 8)) + "px";
  if (kbd) { const f = m.querySelector("div[role=menuitem]"); if (f) f.focus(); }
}
