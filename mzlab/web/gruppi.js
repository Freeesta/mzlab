// @ts-check
// The writer of the user groups of the side list («Group by», gruppi-per.js) and the cache of the engine's answer about the file names (`api/samples/variables`).
// Lazy (carica("gruppi.js")). The page never splits a name: the engine answers, this file maps the answer to `ugrp` (one undoable files.view step).
// Pure label maths of GRUPPI (the answer of the engine → the label of every file); same lazy file.
window.GRUPPI_ETI = (() => {
  const MAX = 40;
  /** The value of every file of a variable as its label ("" = no value), by index of `ans.files`. @param {any} ans @param {any} v @param {string} other */
  const column = (ans, v, other) => {
    const m = /** @type {string[]} */ (new Array(ans.files.length).fill(""));
    if (!v) return m;
    v.values.forEach((/** @type {any} */ x) => x.files.forEach((/** @type {number} */ i) => { m[i] = x.label; }));
    if (other) (v.without || []).forEach((/** @type {number} */ i) => { m[i] = other; });
    return m;
  };
  const clip = (/** @type {string} */ s) => s.trim().slice(0, MAX).trim();
  /** Labels per file name and the labels in display order, from an answer and the key `spec` = {by, then?} (`by` = a variable id or "manual"); "—" files get no label, the files that match no term of «Contains» get «Other». @param {any} ans @param {any} spec @param {string} [other] */
  function labelsOf(ans, spec, other = "") {
    const man = spec.by === "manual", pick = (/** @type {string} */ id) => id === "manual" ? ans.manual : ans.variables.find((/** @type {any} */ v) => v.id === id);
    const a = column(ans, pick(spec.by), man && ans.manual && ans.manual.id === "has" ? other : ""), b = spec.then ? column(ans, pick(spec.then), "") : null;
    const out = /** @type {Map<string, string>} */ (new Map()), order = /** @type {string[]} */ ([]);
    ans.files.forEach((/** @type {any} */ f, /** @type {number} */ i) => { if (a[i]) out.set(f.name, clip(b && b[i] ? a[i] + " · " + b[i] : a[i])); });
    // the groups in the order of the first key, then the second key's: the order of the values in the answer
    const va = pick(spec.by)?.values.map((/** @type {any} */ x) => x.label) || [], vb = b ? pick(spec.then).values.map((/** @type {any} */ x) => x.label) : [""];
    if (other && va.length && !va.includes(other)) va.push(other);
    va.forEach((/** @type {string} */ x) => vb.forEach((/** @type {string} */ y) => { const l = clip(y ? x + " · " + y : x); if (l && [...out.values()].includes(l) && !order.includes(l)) order.push(l); }));
    return { out, order };
  }
  return { clip, labelsOf };
})();
window.GRUPPI = (() => {
  /** The label of an undo step (comandi.js shows it in «Undone: …») per group action. */
  const UNDO = /** @type {Record<string, string>} */ ({ by: "groups.undo.by", then: "groups.undo.then", token: "groups.undo.token", contains: "groups.undo.contains", none: "groups.undo.none", move: "groups.undo.move",
    join: "groups.undo.join", new: "groups.undo.new", rename: "groups.undo.rename", ungroup: "groups.undo.ungroup" });
  // the files of the side list and the rows of the loading table (ST.files: a row keeps its label until the file opens, adopt() hands it over)
  const rowsT = () => /** @type {any[]} */ (typeof ST === "undefined" ? [] : ST.files);
  const names = () => [...new Set([...flSide().map(/** @param {any} f */ f => fileBase(f.file)), ...rowsT().map(f => f.name)])];
  /** Labels the files the user dropped on a group header will take when their rows arrive (name → label). @type {Map<string, { g: string, at: number }>} */
  const pend = new Map();
  /** @type {{ key: string, ans: Promise<any> } | null} */ let cache = null;
  let followed = "", timer = 0;
  /** The `sample-vars/1` answer for the files of the list and an optional manual rule; null when the engine refuses (more than 5 000 names...). @param {any} [manual] */
  async function vars(manual) {
    const ns = names(), key = JSON.stringify([ns, manual ?? null]);
    if (cache?.key === key) return cache.ans;
    const body = JSON.stringify({ files: ns.map(name => ({ name })), manual: manual || undefined });
    // the promise is cached, so renders that arrive while the engine answers share one request
    const p = fetch("api/samples/variables", { method: "POST", body }).then(r => r.json()).then(a => a && !a.error ? a : null).catch(() => null);
    cache = { key, ans: p };
    return p;
  }
  /** True when «Group by» can give a group of 2+ files: the list already has one, or a variable of the names has a value shared by two files. */
  async function useful() {
    const n = new Map(); E.files.forEach(/** @param {any} f */ f => { if (!f.gone && f.ugrp) n.set(f.ugrp, (n.get(f.ugrp) || 0) + 1); });
    if ([...n.values()].some(c => c > 1)) return true;
    const ans = await vars();
    return !!ans && ans.variables.some((/** @type {any} */ v) => v.values.some((/** @type {any} */ x) => x.files.length > 1));
  }
  const { clip, labelsOf } = GRUPPI_ETI;
  /** Writes the labels (`map`: physical file name → label) and the key; `undo` makes it one files.view step. @param {Map<string, string>} map @param {any} by @param {string[]} order @param {boolean} undo */
  function write(map, by, order, undo, action = "move") {
    const go = () => { E.files.forEach(/** @param {any} f */ f => { f.ugrp = map.get(fileBase(f.file)) || undefined; }); rowsT().forEach(f => { f.ugrp = map.get(f.name) || undefined; }); E.ugBy = by; E.ugOrder = order.length ? order : undefined; };
    if (undo) step(go, action); else go();
    renderFileList(); if (typeof carica === "function") carica("gruppi-tab.js").then(() => renderFiles()); uiSave();
  }
  /** Runs `go` as one files.view step and names the step after the group action (`groups.undo.<action>`) in the undo notice. @param {() => void} go @param {string} action */
  function step(go, action) {
    const top = () => STORIA?.pila.v[STORIA.pila.v.length - 1], was = top(); fileViewDo(go);
    const l = top(); if (l && l !== was && l.cmd === "files.view") l.w = UNDO[action];
  }
  /** Spelling-blind key of a group name: «Full_scan», «full scan» and «FULL SCAN» are the same name. @param {string} x */
  const nk = x => String(x).toLowerCase().replace(/[\s_]+/g, "");
  /** True when `label` would make a second group of the same name: the name of an experiment-type group, or another spelling of an existing user group (the exact same label just joins it). @param {string} label @param {string} [except] a label that is being renamed */
  function clash(label, except = "") {
    const k = nk(label); if (!k) return false;
    if (E.ugNone && label !== E.ugNone && nk(E.ugNone) === k) return true;
    if ([...Object.values(KIND), ...Object.values(GRP_HR).flatMap(/** @param {any} f */ f => [f([{ lv: 2 }]), f([{ lv: 3 }])])].some(x => nk(x) === k)) return true;
    return [...flSide().map(/** @param {any} x */ x => x.ugrp), ...rowsT().map(x => x.ugrp)].some(l => l && l !== label && l !== except && nk(l) === k);
  }
  /** The labels of a «Group by» result with every clash made unique by « (2)». @param {{ out: Map<string, string>, order: string[] }} r */
  function fit(r) {
    const m = new Map(); const f = /** @param {string} l */ l => { if (!clash(l)) return l; if (!m.has(l)) m.set(l, l + " (2)"); return m.get(l); };
    r.out.forEach((l, b) => r.out.set(b, f(l))); r.order = r.order.map(f); return r;
  }
  /** The name of a new group: the common tokens of the names (engine) when no group has it yet, else «Group N». @param {string[]} files */
  async function newName(files) {
    const used = new Set([...flSide().map(/** @param {any} x */ x => x.ugrp), ...rowsT().map(x => x.ugrp)]);
    const ans = await fetch("api/samples/group_name", { method: "POST", body: JSON.stringify({ names: files }) }).then(r => r.json()).catch(() => null);
    const name = ans && !ans.error && ans.name ? clip(String(ans.name)) : "";
    if (name && !used.has(name) && !clash(name)) return name;
    let n = 1; while (used.has(I18N.t("files.ugrp.name", { n })) || clash(I18N.t("files.ugrp.name", { n }))) n++;
    return I18N.t("files.ugrp.name", { n });
  }
  /** The label of the file `base` in the side list or the table ("" = none). @param {string} base */
  const labelOf = base => flSide().find(/** @param {any} f */ f => fileBase(f.file) === base)?.ugrp || rowsT().find(f => f.name === base)?.ugrp || "";
  const note = (/** @type {string} */ key) => {
    const n = document.getElementById("fl-note"); if (!n) return;
    n.textContent = I18N.t(key) + " · "; n.hidden = false;
    const b = document.createElement("button"); b.type = "button"; b.className = "sec sm"; b.textContent = I18N.t("groups.undo");
    b.onclick = () => { if (STORIA) STORIA.annulla(); n.hidden = true; };
    n.append(b); setTimeout(() => { n.hidden = true; }, 9000);
  };
  /** Every file name of the list and the table with the label `fn(its label)` gives. @param {(l: string) => string} fn */
  const remap = fn => new Map([...rowsT().map(f => /** @type {[string, string]} */ ([f.name, fn(f.ugrp || "")])), ...flSide().map(/** @param {any} f */ f => /** @type {[string, string]} */ ([fileBase(f.file), fn(f.ugrp || "")]))]);
  function hasGroups() { return E.files.some(/** @param {any} f */ f => f.ugrp) || rowsT().some(f => f.ugrp); }
  /** Puts the files `bases` (physical file names) into group `label` ("" = out of every group): a manual edit, one undo step. @param {string[]} bases @param {string} label */
  function set(bases, label, action = "move") {
    if (clash(label)) { directNote("groups.clash", { g: label }); return false; }
    const map = remap(l => l);
    bases.forEach(b => map.set(b, clip(label)));
    write(map, { by: "custom" }, E.ugOrder || [], true, action); return true;
  }
  /** A file dropped ONTO another one: both (and the dragged files' companions) join the group of the target, or make a new one; returns the group. @param {string[]} bases @param {string} target */
  async function onto(bases, target) {
    const g = labelOf(target);
    if (g) { set(bases, g, "join"); return g; }
    const v = await newName([...new Set([...bases, target])]);
    set([...new Set([...bases, target])], v, "new"); rename(v); return v;
  }
  /** Renames group `old` in the side list and the table (one undo step); asks for the name. @param {string} old */
  async function rename(old) {
    const v = clip((await ask(I18N.t("files.group.prompt"), old)) || "");
    if (!v || v === old) return;
    if (renameTo(old, v) === "clash") directNote("groups.clash", { g: v });
  }
  /** The first «Group N» that no group has. */
  function freeName() {
    const used = new Set([...flSide().map(/** @param {any} x */ x => x.ugrp), ...rowsT().map(x => x.ugrp)]);
    let n = 1; while (used.has(I18N.t("files.ugrp.name", { n })) || clash(I18N.t("files.ugrp.name", { n }))) n++;
    return I18N.t("files.ugrp.name", { n });
  }
  /** Renames group `old` to `raw` (the inline edit of the table header; one undo step): "" = done or nothing to do, "clash" = refused (the name exists). An empty name goes back to the default one; `old` "" is the ungrouped bucket, whose label is only a name (`E.ugNone`), no group is made. @param {string} old @param {string} raw */
  function renameTo(old, raw) {
    let v = clip(raw);
    if (!old) {
      if (v === I18N.t("groups.move.none")) v = "";
      if (v === (E.ugNone || "")) return "";
      if (v && clash(v)) return "clash";
      step(() => { E.ugNone = v || undefined; }, "rename");
      renderFileList(); if (typeof carica === "function") carica("gruppi-tab.js").then(() => renderFiles()); uiSave(); return "";
    }
    if (!v) v = freeName();
    if (v === old) return "";
    if (clash(v, old)) return "clash";
    E._grpOpen = E._grpOpen || {}; if (E._grpOpen["u:" + old] !== undefined) E._grpOpen["u:" + v] = E._grpOpen["u:" + old];
    const map = remap(l => l === old ? v : l);
    write(map, { by: "custom" }, (E.ugOrder || []).map(x => x === old ? v : x), true, "rename");
    return "";
  }
  /** Frees every file of group `old` (side list and table); one undo step. @param {string} old */
  function ungroup(old) {
    const map = remap(l => l === old ? "" : l);
    write(map, { by: "custom" }, (E.ugOrder || []).filter(x => x !== old), true, "ungroup");
  }
  /** Names the undo step that a start-up file (lista-file.js) has just recorded. @param {string} action */
  function lab(action) {
    const l = STORIA?.pila.v[STORIA.pila.v.length - 1];
    if (l && l.cmd === "files.view" && Date.now() - l.t < 2000) l.w = UNDO[action];
  }
  const api = {
    vars, useful, labelsOf, names, has: hasGroups,
    set,
    newName, labelOf,
    onto, rename, renameTo, ungroup, lab,
    /** The shown name of the ungrouped rows: the user's label, else «Ungrouped». */
    noneName: () => E.ugNone || I18N.t("groups.move.none"),
    /** «Group by»: every file takes the label of its value of the key `spec` (a manual group set is replaced, undo brings it back); null = None. @param {any} spec */
    async by(spec) {
      const replaced = hasGroups();
      if (!spec || spec.by === "flat") { write(new Map(), spec || undefined, [], true, "none"); return true; }
      const ans = await vars(spec.manual);
      if (!ans) return false;
      const { out, order } = fit(labelsOf(ans, spec, I18N.t("groups.other")));
      followed = names().join("\n");
      write(out, spec, order, true, spec.manual?.has ? "contains" : spec.manual ? "token" : spec.then ? "then" : "by");
      if (replaced) note("groups.replaced");
      return true;
    },
    /** Files that arrive later take the key while it is not «custom»: debounced 300 ms, no undo step of its own (a new file only). */
    follow() {
      const ub = E.ugBy; if (!ub || ub.by === "custom" || ub.by === "flat") return;
      const key = names().join("\n"); if (key === followed) return;
      clearTimeout(timer);
      timer = window.setTimeout(async () => {
        const ans = await vars(ub.manual); if (!ans || E.ugBy !== ub) return;
        followed = key;
        const { out, order } = fit(labelsOf(ans, ub, I18N.t("groups.other")));
        if (E.files.every(/** @param {any} f */ f => (f.ugrp || "") === (out.get(fileBase(f.file)) || "")) && rowsT().every(f => (f.ugrp || "") === (out.get(f.name) || "")) && (E.ugOrder || []).join("\n") === order.join("\n")) return;
        write(out, ub, order, false);
      }, 300);
    },
    /** A resumed session: the table rows take the user group the restored files carry, so the table and the side list show the same groups. @param {{ file: string, ugrp?: string, gone?: boolean }[]} saved */
    rowsFrom(saved) {
      let n = 0;
      saved.forEach(sf => { if (!sf.ugrp || sf.gone) return; const r = rowsT().find(x => x.name === sf.file.split("#")[0]); if (r && !r.ugrp) { r.ugrp = sf.ugrp; n++; } });
      if (n && typeof renderFiles === "function") renderFiles();
    },
    /** The labels of the table rows for the undo snapshot (no argument), or put back from one (`null` = none): the table repaints. @param {[string, string][] | null} [snap] */
    stg(snap) {
      if (snap === undefined) return rowsT().filter(f => f.ugrp).map(f => [f.name, f.ugrp]);
      const m = new Map(snap || []); rowsT().forEach(f => { f.ugrp = m.get(f.name) || undefined; });
      if (typeof renderFiles === "function") renderFiles();
    },
    /** A file that opens takes the label of its table row (renderFileList calls it first). */
    adopt() {
      const m = new Map(rowsT().filter(f => f.ugrp).map(f => [f.name, f.ugrp]));
      if (m.size) E.files.forEach(/** @param {any} f */ f => { if (!f.ugrp && m.has(fileBase(f.file))) f.ugrp = m.get(fileBase(f.file)); });
    },
    /** Files dropped from the desktop on a group header: their rows take the label as they arrive (no undo step of their own). @param {string[]} files @param {string} label */
    expect(files, label) { files.forEach(n => pend.set(n, { g: clip(label), at: Date.now() })); },
    /** The table rows of dropped files take the label they were promised; renderFiles calls it first. */
    arrive() {
      if (!pend.size) return;
      const now = Date.now(); let n = 0;
      rowsT().forEach(f => { const p = pend.get(f.name); if (p) { f.ugrp = p.g; pend.delete(f.name); n++; } });
      pend.forEach((p, k) => { if (now - p.at > 60000) pend.delete(k); });
      if (n) E.ugBy = { by: "custom" };
    },
    /** The user groups of the list in the order of `E.ugOrder` (the other groups keep their places). @param {{ g: string, fs: any[] }[]} groups */
    order(groups) {
      const o = E.ugOrder; if (!o) return;
      const at = (/** @type {{ g: string }} */ G) => { const i = o.indexOf(G.g.slice(2)); return i < 0 ? 1e9 : i; };
      const us = groups.filter(G => G.g.startsWith("u:")).sort((a, b) => at(a) - at(b));
      let i = 0; groups.forEach((G, j) => { if (G.g.startsWith("u:")) groups[j] = us[i++]; });
    },
  };
  return api;
})();
