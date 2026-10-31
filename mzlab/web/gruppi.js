// @ts-check
// The writer of the user groups of the side list («Group by», gruppi-per.js) and the cache of the engine's answer about the file names (`api/campioni/variabili`).
// Lazy (carica("gruppi.js")). The page never splits a name: the engine answers, this file maps the answer to `ugrp` (one undoable files.view step).
window.GRUPPI = (() => {
  const MAX = 40;
  const names = () => [...new Set(flSide().map(/** @param {any} f */ f => fileBase(f.file)))];
  /** @type {{ key: string, ans: any } | null} */ let cache = null;
  let followed = "", timer = 0;
  /** The `nomivar/1` answer for the files of the list and an optional manual rule; null when the engine refuses (more than 5 000 names...). @param {any} [manuale] */
  async function vars(manuale) {
    const ns = names(), key = JSON.stringify([ns, manuale ?? null]);
    if (cache?.key === key) return cache.ans;
    const body = JSON.stringify({ files: ns.map(name => ({ name })), manuale: manuale || undefined });
    const ans = await fetch("api/campioni/variabili", { method: "POST", body }).then(r => r.json()).catch(() => null);
    cache = { key, ans: ans && !ans.error ? ans : null };
    return cache.ans;
  }
  /** The value of every file of a variable as its label ("" = no value), by index of `ans.files`. @param {any} ans @param {any} v @param {string} other */
  const column = (ans, v, other) => {
    const m = /** @type {string[]} */ (new Array(ans.files.length).fill(""));
    if (!v) return m;
    v.valori.forEach((/** @type {any} */ x) => x.file.forEach((/** @type {number} */ i) => { m[i] = x.etichetta; }));
    if (other) (v.senza || []).forEach((/** @type {number} */ i) => { m[i] = other; });
    return m;
  };
  const clip = (/** @type {string} */ s) => s.trim().slice(0, MAX).trim();
  /** Labels per file name and the labels in display order, from an answer and the key `spec` = {by, then?} (`by` = a variable id or "manuale"); "—" files get no label, the files that match no term of «Contains» get «Other». @param {any} ans @param {any} spec @param {string} [other] */
  function labelsOf(ans, spec, other = "") {
    const man = spec.by === "manuale", pick = (/** @type {string} */ id) => id === "manuale" ? ans.manuale : ans.variabili.find((/** @type {any} */ v) => v.id === id);
    const a = column(ans, pick(spec.by), man && ans.manuale && ans.manuale.id === "has" ? other : ""), b = spec.then ? column(ans, pick(spec.then), "") : null;
    const out = /** @type {Map<string, string>} */ (new Map()), order = /** @type {string[]} */ ([]);
    ans.files.forEach((/** @type {any} */ f, /** @type {number} */ i) => { if (a[i]) out.set(f.name, clip(b && b[i] ? a[i] + " · " + b[i] : a[i])); });
    // the groups in the order of the first key, then the second key's: the order of the values in the answer
    const va = pick(spec.by)?.valori.map((/** @type {any} */ x) => x.etichetta) || [], vb = b ? pick(spec.then).valori.map((/** @type {any} */ x) => x.etichetta) : [""];
    if (other && va.length && !va.includes(other)) va.push(other);
    va.forEach((/** @type {string} */ x) => vb.forEach((/** @type {string} */ y) => { const l = clip(y ? x + " · " + y : x); if (l && [...out.values()].includes(l) && !order.includes(l)) order.push(l); }));
    return { out, order };
  }
  /** Writes the labels (`map`: physical file name → label) and the key; `undo` makes it one files.view step. @param {Map<string, string>} map @param {any} by @param {string[]} order @param {boolean} undo */
  function write(map, by, order, undo) {
    const go = () => { E.files.forEach(/** @param {any} f */ f => { f.ugrp = map.get(fileBase(f.file)) || undefined; }); E.ugBy = by; E.ugOrder = order.length ? order : undefined; };
    if (undo) fileViewDo(go); else go();
    renderFileList(); uiSave();
  }
  const note = (/** @type {string} */ key) => {
    const n = document.getElementById("fl-note"); if (!n) return;
    n.textContent = I18N.t(key) + " · "; n.hidden = false;
    const b = document.createElement("button"); b.type = "button"; b.className = "sec sm"; b.textContent = I18N.t("groups.undo");
    b.onclick = () => { if (STORIA) STORIA.annulla(); n.hidden = true; };
    n.append(b); setTimeout(() => { n.hidden = true; }, 9000);
  };
  const hasGroups = () => E.files.some(/** @param {any} f */ f => f.ugrp);
  return {
    vars, labelsOf, names,
    /** Puts the files `bases` (physical file names) into group `label` ("" = out of every group): a manual edit, one undo step. @param {string[]} bases @param {string} label */
    set(bases, label) {
      const map = new Map(flSide().map(/** @param {any} f */ f => [fileBase(f.file), f.ugrp || ""]));
      bases.forEach(b => map.set(b, clip(label)));
      write(map, { by: "custom" }, E.ugOrder || [], true);
    },
    /** «Group by»: every file takes the label of its value of the key `spec` (a manual group set is replaced, undo brings it back); null = None. @param {any} spec */
    async by(spec) {
      const replaced = hasGroups();
      if (!spec) { write(new Map(), undefined, [], true); return true; }
      const ans = await vars(spec.manuale);
      if (!ans) return false;
      const { out, order } = labelsOf(ans, spec, I18N.t("groups.other"));
      followed = names().join("\n");
      write(out, spec, order, true);
      if (replaced) note("groups.replaced");
      return true;
    },
    /** Files that arrive later take the key while it is not «custom»: debounced 300 ms, no undo step of its own (a new file only). */
    follow() {
      const ub = E.ugBy; if (!ub || ub.by === "custom") return;
      const key = names().join("\n"); if (key === followed) return;
      clearTimeout(timer);
      timer = window.setTimeout(async () => {
        const ans = await vars(ub.manuale); if (!ans || E.ugBy !== ub) return;
        followed = key;
        const { out, order } = labelsOf(ans, ub, I18N.t("groups.other"));
        if (E.files.every(/** @param {any} f */ f => (f.ugrp || "") === (out.get(fileBase(f.file)) || "")) && (E.ugOrder || []).join("\n") === order.join("\n")) return;
        write(out, ub, order, false);
      }, 300);
    },
    /** The user groups of the list in the order of `E.ugOrder` (the other groups keep their places). @param {{ g: string, fs: any[] }[]} groups */
    order(groups) {
      const o = E.ugOrder; if (!o) return;
      const at = (/** @type {{ g: string }} */ G) => { const i = o.indexOf(G.g.slice(2)); return i < 0 ? 1e9 : i; };
      const us = groups.filter(G => G.g.startsWith("u:")).sort((a, b) => at(a) - at(b));
      let i = 0; groups.forEach((G, j) => { if (G.g.startsWith("u:")) groups[j] = us[i++]; });
    },
  };
})();
