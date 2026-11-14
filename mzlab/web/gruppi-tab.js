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
  /** A row's HTML with its name, grip and «Move to group» button. @param {{ f: any }} x @param {string} h @param {boolean} shut */
  const deco = (x, h, shut) => h.replace("<tr ", `<tr data-nm="${EH(x.f.name)}"${shut ? " hidden" : ""} `).replace("<td>", `<td><span class="tg-grip" draggable="true" title="${EH(t("groups.tab.grip"))}" aria-hidden="true">⠿</span>`)
    .replace("<button data-rm=", `<button type="button" class="sec sm" data-mv="${EH(x.f.name)}" aria-label="${EH(t("groups.move.menu"))}" title="${EH(t("groups.move.menu"))}">⇄</button> <button data-rm=`);
  const api = {
    /** Before renderFiles builds its rows: true = a drag holds the table (the repaint waits); dropped files take their label, a running «Group by» key reaches the new rows. */
    pre() {
      if (window.GDND && GDND.busy()) { GDND.defer(); return true; }
      window.GRUPPI?.arrive();
      if (E.ugBy && E.ugBy.by !== "custom" && E.ugBy.by !== "flat") window.GRUPPI?.follow();
      return false;
    },
    /** The rows of the table body: `ents` = [{ f, i, h }] in table order (`h` = the row's HTML): every row of a table of two or more gets its grip and «Move» button, the headers come with the first label. @param {{ f: any, i: number, h: string }[]} ents @param {number} ncol */
    rows(ents, ncol) {
      const lab = (/** @type {{ f: any, h: string }} */ x) => x.h.replace('data-k="use"', `data-k="use" aria-label="${EH(x.f.name)}"`);
      if (ents.length < 2) return ents.map(lab);
      if (!ents.some(x => x.f.ugrp)) return ents.map(x => deco(x, lab(x), false));
      const by = /** @type {Map<string, typeof ents>} */ (new Map());
      ents.forEach(x => { const g = x.f.ugrp || ""; (by.get(g) || by.set(g, []).get(g))?.push(x); });
      const named = order([...by.keys()].filter(Boolean)), keys = by.has("") ? [...named, ""] : named, out = /** @type {string[]} */ ([]);
      keys.forEach(g => {
        const rs = by.get(g) || [], shut = folded.has(g), name = g || (window.GRUPPI ? GRUPPI.noneName() : t("groups.move.none"));
        out.push(`<tr class="tg-head" data-ug="${EH(g)}"><th scope="colgroup" colspan="${ncol}"><button type="button" class="tg-fold" data-fold="${EH(g)}" aria-expanded="${!shut}" aria-label="${EH(t(shut ? "groups.tab.open" : "groups.tab.fold", { g: name }))}">${shut ? "▸" : "▾"}</button> <b class="tg-name" data-gn="${EH(g)}" tabindex="0" role="button" aria-label="${EH(t("groups.tab.rename", { g: name }))}" title="${EH(t("groups.tab.rename.tip"))}">${EH(name)}</b> <span class="muted sm">${t("groups.tab.count", { n: rs.length })}</span></th></tr>`);
        rs.forEach(x => out.push(deco(x, lab(x), shut)));
      });
      return out;
    },
    /** The handlers of the table, once: fold, «Move to group», the grip's drag; the table takes the room of the grip while it has groups. @param {HTMLElement} el */
    wire(el) {
      el.querySelector(".ldtab")?.classList.toggle("tg-on", !!el.querySelector("tr[data-nm]"));
      // «Group by ▾» sits at the end of the sample heading
      const head = el.querySelector(".samphead");
      if (head && !head.querySelector("#tb-by")) {
        const b = document.createElement("button"); b.type = "button"; b.className = "sec sm"; b.id = "tb-by"; b.setAttribute("aria-haspopup", "menu"); b.setAttribute("aria-expanded", "false"); b.textContent = t("groups.by.btn") + " \u25be";
        b.onclick = () => api.by(); head.append(b);
      }
      // files dragged in from the desktop onto a header (and the grips) need gruppi-dnd.js: it is there as soon as the table has headers
      if (el.querySelector(".tg-head")) carica("gruppi-dnd.js");
      if (el.dataset.tg) return; el.dataset.tg = "1";
      el.addEventListener("click", e => {
        const b = /** @type {HTMLElement | null} */ (/** @type {Element} */ (e.target).closest("[data-fold],[data-mv]"));
        if (!b) return;
        // the second click of a double click (the rename) must not fold the group again
        if (b.dataset.fold != null) { if (e.detail > 1) return; const g = b.dataset.fold; if (folded.has(g)) folded.delete(g); else folded.add(g); renderFiles(); /** @type {HTMLElement | null} */ (el.querySelector(`[data-fold="${CSS.escape(g)}"]`))?.focus(); return; }
        // the menu opens after the click has reached the document (its handler would close it at once)
        const name = /** @type {string} */ (b.dataset.mv); setTimeout(() => carica("gruppi-dnd.js").then(() => GDND.moveMenuT(e, name)), 0);
      });
      // rename in place: double click on the header (not the fold button), F2 or Enter on the focused name
      el.addEventListener("dblclick", e => {
        const th = /** @type {Element} */ (e.target).closest(".tg-head th");
        if (th && !/** @type {Element} */ (e.target).closest("button.tg-fold")) { e.preventDefault(); api.edit(/** @type {HTMLElement} */ (th)); }
      });
      el.addEventListener("keydown", e => {
        const n = /** @type {Element} */ (e.target).closest?.(".tg-name");
        if (n && (e.key === "F2" || e.key === "Enter")) { e.preventDefault(); api.edit(/** @type {HTMLElement} */ (n.closest("th"))); }
      });
      el.addEventListener("pointerover", e => { if (/** @type {Element} */ (e.target).closest(".tg-grip")) carica("gruppi-dnd.js"); });
      el.addEventListener("dragstart", e => {
        const g = /** @type {Element} */ (e.target).closest?.(".tg-grip"), tr = /** @type {HTMLElement | null} */ (g?.closest("tr[data-nm]"));
        if (!g || !tr || !e.dataTransfer) return;
        e.dataTransfer.setData("text/plain", /** @type {string} */ (tr.dataset.nm)); e.dataTransfer.effectAllowed = "move"; e.dataTransfer.setDragImage(tr, 8, 8);
        tr.classList.add("fl-dragging"); const n = /** @type {string} */ (tr.dataset.nm); carica("gruppi-dnd.js").then(() => GDND.startT(n));
      });
    },
    /** The name of a group header becomes a text field: Enter or leaving it saves, Esc cancels, empty = the default name; a name that exists stays open with a short message. @param {HTMLElement} th */
    async edit(th) {
      const b = /** @type {HTMLElement | null} */ (th.querySelector(".tg-name")); if (!b || th.querySelector(".tg-edit")) return;
      await carica("gruppi.js");
      const g = b.dataset.gn || "", shown = b.textContent || "", inp = document.createElement("input"), msg = document.createElement("span");
      inp.type = "text"; inp.className = "tg-edit"; inp.value = g ? shown : (E.ugNone || ""); inp.maxLength = 40; inp.size = 18; inp.autocomplete = "off";
      inp.setAttribute("aria-label", t("groups.tab.rename", { g: shown })); inp.placeholder = g ? "" : t("groups.move.none");
      msg.className = "tg-err sm"; msg.setAttribute("role", "alert");
      let done = false;
      const close = () => { done = true; inp.replaceWith(b); msg.remove(); b.focus(); };
      const save = (/** @type {boolean} */ stay) => {
        if (done) return;
        if (GRUPPI.renameTo(g, inp.value) === "clash") {
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
    },
    /** «Group by ▾» of the table: the menu of the side list, anchored to the button above the table. */
    async by() { await carica("gruppi.js"); await carica("gruppi-per.js"); GRUPPIPER.open("tb-by"); },
  };
  return api;
})();
