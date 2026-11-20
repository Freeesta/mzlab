// @ts-check
// The in-place rename of a group header of the loading table (groups-tab.js): the name becomes a text field, Enter or leaving it saves (GROUPS.renameTo, one undo step), Esc cancels.
// Lazy (loadScript("groups-name.js")): double click, F2 or Enter on the name asks for it.
window.GRUNOME = (() => {
  const t = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
/** The name of a group header becomes a text field: Enter or leaving it saves, Esc cancels, empty = the default name; a name that exists stays open with a short message.@param {HTMLElement} th */
  async function edit(th) {
      const b = /** @type {HTMLElement | null} */ (th.querySelector(".tg-name")); if (!b || th.querySelector(".tg-edit")) return;
      await loadScript("groups.js");
      const g = b.dataset.gn || "", shown = b.textContent || "", inp = document.createElement("input"), msg = document.createElement("span");
      inp.type = "text"; inp.className = "tg-edit"; inp.value = g ? shown : (E.ugNone || ""); inp.maxLength = 40; inp.size = 18; inp.autocomplete = "off";
      inp.setAttribute("aria-label", t("groups.tab.rename", { g: shown })); inp.placeholder = g ? "" : t("groups.move.none");
      msg.className = "tg-err sm"; msg.setAttribute("role", "alert");
      let done = false;
      const close = () => { done = true; inp.replaceWith(b); msg.remove(); b.focus(); };
      const save = (/** @type {boolean} */ stay) => {
        if (done) return;
        if (GROUPS.renameTo(g, inp.value) === "clash") {
          if (stay) { inp.setAttribute("aria-invalid", "true"); msg.textContent = t("groups.tab.dup", { g: inp.value.trim() }); return; }
          close(); return;
        }
        const want = inp.value.trim(); close();   // the table is drawn again with the new name; the focus follows it
        if (want) setTimeout(() => /** @type {HTMLElement | null} */ (document.querySelector(`.tg-name[data-gn="${CSS.escape(want.slice(0, 40))}"]`))?.focus(), 80);
      };
      inp.addEventListener("keydown", e => { if (e.key === "Enter") { e.preventDefault(); save(true); } else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); } e.stopPropagation(); });
      inp.addEventListener("input", () => { inp.removeAttribute("aria-invalid"); msg.textContent = ""; });
      inp.addEventListener("blur", () => save(false));
      b.replaceWith(inp); inp.after(msg); inp.focus(); inp.select();
  }
  return { edit };
})();
