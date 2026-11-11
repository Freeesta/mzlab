// @ts-check
// Rows of the Draw context menu (#ctx) as real menu items: role, roving tabindex, arrows / Enter / Space / Esc, the pattern of explore.js menu().
// Shared by the canvas menu and the label menu of draw.js; the canvas rows come from an ordered registry (menuAdd) so modules add rows without editing draw.js.
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
  // with a finger the menu opens beside it (above it when there is no room below): the finger never rests on a row
  const off = document.documentElement.classList.contains("touch") ? 16 : 0, h = m.offsetHeight;
  let top = y + off; if (off && top + h > innerHeight - 8) top = y - off - h;
  m.style.left = Math.min(x + off, innerWidth - w) + "px"; m.style.top = Math.max(4, Math.min(top, innerHeight - h - 8)) + "px";
  if (kbd) { const f = m.querySelector("div[role=menuitem]"); if (f) f.focus(); }
}
// Copies to the clipboard and tells the truth: «Copied» only when the browser accepted it, otherwise «Copy failed» (never an unhandled rejection).
export async function copyText(/** @type {string} */ text, /** @type {string} */ okKey = "draw.copy.done") {
  let ok = false;
  try { await navigator.clipboard.writeText(text); ok = true; } catch (_) { /* clipboard refused or missing */ }
  if (window.toast) toast(I18N.t(ok ? okKey : "draw.copy.fail"));
  return ok;
}

// Registry of the canvas menu. An entry: {id, when: "selection" | "whole" | "any", order, label: i18n key or (ctx) => text,
// title?: key or (ctx) => text, show?: (ctx) => boolean, run: (ctx) => void} or, for rows computed at open time, {id, when, order, rows: async (ctx) => [{text, title?, run}]}.
// ctx: {K, ev, sel, atoms (Set of atom ids), bonds (selected bond ids), whole (no selection), smiles, x, y, kbd}. Same id = replaced.
/** @type {any[]} */
const REG = [];
export function menuAdd(/** @type {any} */ e) {
  if (!e || !e.id || (!e.run && !e.rows)) throw new Error("menu entry needs id and run or rows");
  const i = REG.findIndex(r => r.id === e.id); const x = { when: "any", order: 100, ...e };
  if (i >= 0) REG[i] = x; else REG.push(x);
  REG.sort((a, b) => a.order - b.order);
}
export const menuIds = () => REG.map(r => r.id);
const txt = (/** @type {any} */ v, /** @type {any} */ c) => typeof v === "function" ? v(c) : v ? I18N.t(v) : undefined;
// Fills #ctx: a header saying what the menu acts on (aria-hidden, the same words are the menu's aria-label), then the rows that apply.
// A row that fails is left out: one module's error never takes the whole menu with it.
export async function menuFill(/** @type {any} */ m, /** @type {any} */ c) {
  const head = c.whole ? I18N.t("draw.ctx.head.whole") : I18N.t("draw.ctx.head.sel", { n: c.atoms.size });
  m.innerHTML = ""; m.setAttribute("aria-label", head);
  const h = document.createElement("div"); h.className = "dim ctx-head"; h.setAttribute("aria-hidden", "true"); h.textContent = head; m.appendChild(h);
  for (const r of REG) {
    if (r.when === "selection" ? c.whole : r.when === "whole" ? !c.whole : false) continue;
    try {
      if (r.rows) { for (const it of await r.rows(c)) { ctxRow(m, it.text, () => { m.hidden = true; if (it.run) it.run(); }, it.title); /** @type {HTMLElement} */ (m.lastElementChild).dataset.row = r.id; } continue; }
      if (r.show && !r.show(c)) continue;
      ctxRow(m, txt(r.label, c), () => { m.hidden = true; r.run(c); }, txt(r.title, c));
      /** @type {HTMLElement} */ (m.lastElementChild).dataset.row = r.id;
    } catch (_) { /* this row is not offered */ }
  }
}
