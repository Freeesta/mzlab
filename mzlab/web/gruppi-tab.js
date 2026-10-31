// @ts-check
// Groups in the loading table (renderFiles in shell.js): the header rows (name, count, fold), the grip and the «Move to group» button of every row, the «Group by» menu and the drag of a row
// onto a header. Lazy (carica("gruppi-tab.js")): the labels are the `ugrp` of the ST.files rows, written by GRUPPI (gruppi.js) together with the side list; the drag itself is gruppi-dnd.js.
window.GRUTAB = (() => {
  const folded = new Set(), t = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
  /** The labels in display order: the order of `E.ugOrder` first, then the rest as they arrive; the ungrouped rows ("") come last. @param {string[]} seen */
  const order = seen => {
    const o = E.ugOrder || [], at = (/** @type {string} */ g) => { const i = o.indexOf(g); return i < 0 ? 1e9 : i; };
    return [...seen].sort((a, b) => at(a) - at(b));
  };
  return {
    /** Before renderFiles builds its rows: true = a drag holds the table (the repaint waits); dropped files take their label, a running «Group by» key reaches the new rows. */
    pre() {
      if (window.GDND && GDND.busy()) { GDND.defer(); return true; }
      window.GRUPPI?.arrive();
      if (E.ugBy && E.ugBy.by !== "custom") window.GRUPPI?.follow();
      return false;
    },
    /** The rows of the table body: `ents` = [{ f, i, h }] in table order (`h` = the row's HTML); with no label anywhere the rows are returned as they are. @param {{ f: any, i: number, h: string }[]} ents @param {number} ncol */
    rows(ents, ncol) {
      if (!ents.some(x => x.f.ugrp)) return ents.map(x => x.h);
      const by = /** @type {Map<string, typeof ents>} */ (new Map());
      ents.forEach(x => { const g = x.f.ugrp || ""; (by.get(g) || by.set(g, []).get(g))?.push(x); });
      const named = order([...by.keys()].filter(Boolean)), keys = by.has("") ? [...named, ""] : named, out = /** @type {string[]} */ ([]);
      const grip = `<span class="tg-grip" draggable="true" title="${EH(t("groups.tab.grip"))}" aria-hidden="true">⠿</span>`;
      keys.forEach(g => {
        const rs = by.get(g) || [], shut = folded.has(g), name = g || t("groups.move.none");
        out.push(`<tr class="tg-head" data-ug="${EH(g)}"><th scope="colgroup" colspan="${ncol}"><button type="button" class="tg-fold" data-fold="${EH(g)}" aria-expanded="${!shut}" aria-label="${EH(t(shut ? "groups.tab.open" : "groups.tab.fold", { g: name }))}">${shut ? "▸" : "▾"}</button> <b>${EH(name)}</b> <span class="muted sm">${t("groups.tab.count", { n: rs.length })}</span></th></tr>`);
        rs.forEach(x => out.push(x.h.replace("<tr ", `<tr data-nm="${EH(x.f.name)}"${shut ? " hidden" : ""} `).replace("<td>", "<td>" + grip)
          .replace("<button data-rm=", `<button type="button" class="sec sm" data-mv="${EH(x.f.name)}" aria-label="${EH(t("groups.move.menu"))}" title="${EH(t("groups.move.menu"))}">⇄</button> <button data-rm=`)));
      });
      return out;
    },
    /** The handlers of the table, once: fold, «Move to group», the grip's drag; the table takes the room of the grip while it has groups. @param {HTMLElement} el */
    wire(el) {
      el.querySelector(".ldtab")?.classList.toggle("tg-on", !!el.querySelector(".tg-head"));
      // files dragged in from the desktop onto a header need gruppi-dnd.js: it is there as soon as the table has headers
      if (el.querySelector(".tg-head")) carica("gruppi-dnd.js");
      if (el.dataset.tg) return; el.dataset.tg = "1";
      el.addEventListener("click", e => {
        const b = /** @type {HTMLElement | null} */ (/** @type {Element} */ (e.target).closest("[data-fold],[data-mv]"));
        if (!b) return;
        if (b.dataset.fold != null) { const g = b.dataset.fold; if (folded.has(g)) folded.delete(g); else folded.add(g); renderFiles(); /** @type {HTMLElement | null} */ (el.querySelector(`[data-fold="${CSS.escape(g)}"]`))?.focus(); return; }
        // the menu opens after the click has reached the document (its handler would close it at once)
        const name = /** @type {string} */ (b.dataset.mv); setTimeout(() => carica("gruppi-dnd.js").then(() => GDND.moveMenuT(e, name)), 0);
      });
      el.addEventListener("pointerover", e => { if (/** @type {Element} */ (e.target).closest(".tg-grip")) carica("gruppi-dnd.js"); });
      el.addEventListener("dragstart", e => {
        const g = /** @type {Element} */ (e.target).closest?.(".tg-grip"), tr = /** @type {HTMLElement | null} */ (g?.closest("tr[data-nm]"));
        if (!g || !tr || !e.dataTransfer) return;
        e.dataTransfer.setData("text/plain", /** @type {string} */ (tr.dataset.nm)); e.dataTransfer.effectAllowed = "move"; e.dataTransfer.setDragImage(tr, 8, 8);
        tr.classList.add("fl-dragging"); const n = /** @type {string} */ (tr.dataset.nm); carica("gruppi-dnd.js").then(() => GDND.startT(n));
      });
    },
    /** «Group by ▾» of the table: the menu of the side list, anchored to the button above the table. */
    async by() { await carica("gruppi.js"); await carica("gruppi-per.js"); GRUPPIPER.open("tb-by"); },
  };
})();
