// @ts-check
// The samples of a big SCIEX .wiff, chosen before anything is copied or read (lazy: carica("wiff-scelta.js") from wiffToMzml in shell.js). The list of the samples comes from the
// engine (nativeSamples reads only the index of the .wiff) or from the readers module; the choice is a set of positions in that list, and only those samples become files of the loading.
window.WIFFPICK = (() => {
  const t = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
  /**
   * The dialog: one checkbox per sample (none ticked), «Open selected», «Open all», «Cancel».
   * @param {string} file name of the .wiff @param {{ name?: string, id: number, scans?: number, size?: number, why?: string }[]} rows
   * @returns {Promise<Set<number> | null>} the chosen positions, `null` for all, an empty set for cancel
   */
  function choose(file, rows) {
    return new Promise(done => {
      const d = document.createElement("dialog"), id = "wpick-t" + Date.now();
      d.className = "wpick-dlg"; d.setAttribute("aria-labelledby", id);
      const h = document.createElement("h2"); h.id = id; h.textContent = t("load.pick.title");
      const sub = document.createElement("p"); sub.className = "muted sm"; sub.textContent = t("load.pick.file", { name: file, n: rows.length });
      const list = document.createElement("div"); list.className = "wpick-list"; list.setAttribute("role", "group"); list.setAttribute("aria-label", t("load.pick.title"));
      const boxes = rows.map((r, i) => {
        const l = document.createElement("label"), c = document.createElement("input"), s = document.createElement("span");
        c.type = "checkbox"; c.value = String(i);
        s.textContent = t(r.scans ? "load.pick.row" : "load.pick.row0", { name: r.name || "#" + r.id, scans: r.scans ?? 0, mb: r.size ? (r.size / 1048576).toFixed(1) : "—" }) + (r.why === "empty" ? " · " + t("warn.wiff.emptySample") : r.why === "inferred" ? " · " + t("warn.wiff.inferredSample") : "");
        l.append(c, s); list.append(l); return c;
      });
      const status = document.createElement("p"); status.className = "muted sm"; status.setAttribute("role", "status");
      const bar = document.createElement("div"); bar.className = "row";
      const mk = (/** @type {string} */ k, /** @type {string} */ cls) => { const b = document.createElement("button"); b.type = "button"; b.textContent = t(k); if (cls) b.className = cls; return b; };
      const no = mk("load.pick.cancel", "sec"), all = mk("load.pick.all", "sec"), ok = mk("load.pick.open", "");
      bar.append(no, all, ok);
      const chosen = () => new Set(boxes.filter(c => c.checked).map(c => +c.value));
      const refresh = () => { const n = chosen().size; status.textContent = t("load.pick.count", { n, total: rows.length }); ok.disabled = !n; };
      let result = /** @type {Set<number> | null} */ (new Set());
      list.addEventListener("change", refresh);
      no.onclick = () => d.close();
      all.onclick = () => { result = null; d.close(); };
      ok.onclick = () => { result = chosen(); d.close(); };
      d.onclose = () => { d.remove(); done(result); };
      d.append(h, sub, list, status, bar); document.body.append(d); refresh(); d.showModal(); boxes[0]?.focus();
    });
  }
  return { choose };
})();
