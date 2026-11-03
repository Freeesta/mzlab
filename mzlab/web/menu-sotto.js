// @ts-check
"use strict";
// Submenu of the right-click menu (menu() of explore.js): an entry with `sub` (its rows, asked again after a Ctrl/Shift click) opens beside it on hover, click or the right
// arrow; a row has a colour swatch and a check mark, arrows move between the rows and Left or Esc go back to the entry. Used for the ions of a SIM file (sim-ui.js).
window.MENUSUB = {
  /** @param {HTMLElement} d the entry @param {{ sub: () => any[] }} it @param {HTMLElement} m the menu */
  wire(d, it, m) {
    d.tabIndex = -1; d.classList.add("hsub"); d.setAttribute("role", "menuitem"); d.setAttribute("aria-haspopup", "menu"); d.setAttribute("aria-expanded", "false");
    const car = document.createElement("span"); car.className = "ck"; car.setAttribute("aria-hidden", "true"); car.textContent = "\u203A"; d.appendChild(car);
    const sm = document.createElement("div"); sm.className = "ctxsub"; sm.setAttribute("role", "menu"); sm.hidden = true; d.appendChild(sm);
    const rowsOf = () => [...sm.children];
    const fill = () => {
      sm.innerHTML = "";
      it.sub().forEach((/** @type {Bag} */ r) => {
        const e = document.createElement("div"); e.tabIndex = -1; e.setAttribute("role", "menuitemcheckbox"); e.setAttribute("aria-checked", String(!!r.check)); e.setAttribute("aria-label", r.aria || r.nome);
        if (r.color) { const sw = document.createElement("span"); sw.className = "sw"; sw.setAttribute("aria-hidden", "true"); sw.style.background = r.color; e.appendChild(sw); }
        e.appendChild(document.createTextNode(r.nome));
        if (r.check) { const ck = document.createElement("span"); ck.className = "ck"; ck.setAttribute("aria-hidden", "true"); ck.textContent = "\u2713"; e.appendChild(ck); }
        e.onclick = (/** @type {MouseEvent} */ ev) => { ev.stopPropagation(); const keep = r.keep && r.keep(ev); r.esegui(ev); if (keep) { fill(); const i = rowsOf().findIndex(x => x.getAttribute("aria-label") === (r.aria || r.nome)); if (i >= 0) /** @type {HTMLElement} */ (rowsOf()[i]).focus(); } else m.hidden = true; };
        sm.appendChild(e);
      });
    };
    const open = (/** @type {boolean} */ v) => {
      if (v === !sm.hidden) return;
      if (v) { fill(); sm.hidden = false; sm.style.left = ""; sm.style.right = ""; if (sm.getBoundingClientRect().right > innerWidth - 4) { sm.style.left = "auto"; sm.style.right = "100%"; } }
      else sm.hidden = true;
      d.setAttribute("aria-expanded", String(v));
    };
    d.onclick = (/** @type {MouseEvent} */ ev) => { ev.stopPropagation(); if (/** @type {HTMLElement} */ (ev.target).closest(".ctxsub")) return; open(!!sm.hidden); if (!sm.hidden && ev.detail === 0) { const f = /** @type {HTMLElement | undefined} */ (rowsOf()[0]); if (f) f.focus(); } };
    d.onmouseenter = () => open(true); d.onmouseleave = () => { if (!sm.contains(document.activeElement)) open(false); };
    d.addEventListener("keydown", (/** @type {KeyboardEvent} */ e) => {
      if (e.target === d && e.key === "ArrowRight") { e.preventDefault(); e.stopPropagation(); open(true); const f = /** @type {HTMLElement | undefined} */ (rowsOf()[0]); if (f) f.focus(); }
      else if (e.target !== d) {                                  // inside the submenu: arrows move between its rows, Left or Esc go back to the entry
        const rs = /** @type {HTMLElement[]} */ (rowsOf()), i = rs.indexOf(/** @type {HTMLElement} */ (document.activeElement));
        if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); e.stopPropagation(); if (rs.length) rs[(i + (e.key === "ArrowDown" ? 1 : -1) + rs.length) % rs.length].focus(); }
        else if (e.key === "ArrowLeft" || e.key === "Escape") { e.preventDefault(); e.stopPropagation(); open(false); d.focus(); }
        else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); if (i >= 0) rs[i].dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: e.ctrlKey, metaKey: e.metaKey, shiftKey: e.shiftKey })); }
      }
    });
  },
};
