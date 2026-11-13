// @ts-check
// Rows of the Draw context menu (#ctx) as real menu items: role, roving tabindex, arrows / Enter / Space / Esc, the pattern of explore.js menu().
// Shared by the canvas menu and the label menu of draw.js; the canvas rows come from an ordered registry (menuAdd) so modules add rows without editing draw.js,
// and a row can open a submenu (ctxSub).
export function ctxRow(/** @type {HTMLElement} */ m, /** @type {string} */ text, /** @type {() => void} */ fn, /** @type {string | undefined} */ title = undefined) {
  const e = document.createElement("div");
  e.textContent = text; e.setAttribute("role", "menuitem"); e.tabIndex = -1; e.onclick = fn;
  if (title) e.title = title;
  e.addEventListener("keydown", (/** @type {KeyboardEvent} */ k) => {
    const its = [...m.children].filter(x => x.getAttribute("role") === "menuitem"), i = its.indexOf(e), up = /** @type {any} */ (m)._up;
    if (k.key === "ArrowDown" || k.key === "ArrowUp") { k.preventDefault(); k.stopPropagation(); /** @type {HTMLElement} */ (its[(i + (k.key === "ArrowDown" ? 1 : -1) + its.length) % its.length]).focus(); }
    else if (k.key === "Enter" || k.key === " " || (k.key === "ArrowRight" && e.hasAttribute("aria-haspopup"))) { k.preventDefault(); k.stopPropagation(); e.click(); }
    else if (up && (k.key === "Escape" || k.key === "ArrowLeft")) { k.preventDefault(); k.stopPropagation(); up.close(true); }   // a submenu: back to its row
    else if (k.key === "Escape") { m.hidden = true; const r = /** @type {any} */ (m)._ret; if (r && r.focus) r.focus(); }
  });
  m.appendChild(e);
  return e;
}
// A row that opens a submenu (ARIA menu button inside the menu): hover, click or tap, Enter / Space / Right arrow open it on its first row; Left arrow or Esc
// come back to the row. The submenu sits beside the row (left of it when there is no room on the right) and its rows close the whole menu like any other.
export function ctxSub(/** @type {HTMLElement} */ m, /** @type {string} */ text, /** @type {{ text: string, title?: string, id?: string, run?: () => void }[]} */ items, /** @type {() => void} */ done, /** @type {string | undefined} */ title = undefined) {
  const sub = document.createElement("div"); sub.className = "ctx-sub"; sub.setAttribute("role", "menu"); sub.setAttribute("aria-label", text); sub.hidden = true;
  let t = 0;
  const open = (/** @type {boolean} */ focus) => {
    clearTimeout(t); m.querySelectorAll(":scope > .ctx-sub").forEach(o => { if (o !== sub) /** @type {any} */ (o)._up.close(false); });
    sub.hidden = false; row.setAttribute("aria-expanded", "true");
    const r = row.getBoundingClientRect(), w = sub.offsetWidth, right = r.right + w <= innerWidth - 4;
    sub.style.left = (right ? row.offsetLeft + row.offsetWidth - 2 : row.offsetLeft - w + 2) + "px";
    sub.style.top = Math.max(-m.getBoundingClientRect().top + 4, Math.min(row.offsetTop - 3, innerHeight - m.getBoundingClientRect().top - sub.offsetHeight - 8)) + "px";
    if (focus) { const f = sub.querySelector("[role=menuitem]"); if (f) /** @type {HTMLElement} */ (f).focus(); }
  };
  const row = ctxRow(m, text, () => { if (sub.hidden) open(true); else /** @type {any} */ (sub)._up.close(true); }, title);
  row.setAttribute("aria-haspopup", "menu"); row.setAttribute("aria-expanded", "false"); row.classList.add("ctx-has");
  row.addEventListener("click", ev => ev.stopPropagation());       // the page closes #ctx on any click that reaches the document (explore.js)
  /** @type {any} */ (sub)._up = { close: (/** @type {boolean} */ focus) => { clearTimeout(t); sub.hidden = true; row.setAttribute("aria-expanded", "false"); if (focus) row.focus(); } };
  const later = () => { clearTimeout(t); t = window.setTimeout(() => /** @type {any} */ (sub)._up.close(false), 300); };
  row.addEventListener("mouseenter", () => { clearTimeout(t); t = window.setTimeout(() => open(false), 120); });
  row.addEventListener("mouseleave", later); sub.addEventListener("mouseenter", () => clearTimeout(t)); sub.addEventListener("mouseleave", later);
  for (const it of items) { const e = ctxRow(sub, it.text, () => { done(); if (it.run) it.run(); }, it.title); if (it.id) e.dataset.row = it.id; }
  m.appendChild(sub);
  return row;
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
// title?: key or (ctx) => text, show?: (ctx) => boolean, run: (ctx) => void} or, for rows computed at open time, {id, when, order, rows: async (ctx) => [{text, title?, id?, run}]} (id: the row's data-row, else the entry's);
// a computed row with `sub: [{text, title?, id?, run}]` (and no run) opens a submenu of those rows.
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
// Fills #ctx with the rows that apply; the menu's name (aria-label) says what it acts on, the selection or the whole drawing.
// A row that fails is left out: one module's error never takes the whole menu with it.
/** fills the menu from the registry; false when a newer fill took over */
export async function menuFill(/** @type {any} */ m, /** @type {any} */ c) {
  m.innerHTML = ""; m.setAttribute("aria-label", c.whole ? I18N.t("draw.ctx.head.whole") : I18N.t("draw.ctx.head.sel", { n: c.atoms.size }));
  const gen = m._gen = (m._gen || 0) + 1;                                // a newer fill (a second right click) wins: the older one stops instead of adding its rows twice
  const done = () => { m.hidden = true; };
  for (const r of REG) {
    if (r.when === "selection" ? c.whole : r.when === "whole" ? !c.whole : false) continue;
    try {
      if (r.rows) {
        const rows = await r.rows(c); if (m._gen !== gen) return false;
        for (const it of rows) {
          if (it.sub) { if (it.sub.length) ctxSub(m, it.text, it.sub, done, it.title).dataset.row = it.id || r.id; continue; }
          ctxRow(m, it.text, () => { done(); if (it.run) it.run(); }, it.title).dataset.row = it.id || r.id;
        }
        continue;
      }
      if (r.show && !r.show(c)) continue;
      ctxRow(m, txt(r.label, c), () => { done(); r.run(c); }, txt(r.title, c)).dataset.row = r.id;
    } catch (_) { /* this row is not offered */ }
  }
  return true;
}
