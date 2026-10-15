// @ts-check
"use strict";
// Roles and times in the loading table (`campioni/2`, the Rust of mzlab-core). This file packs and shows: the proposals from the file names (`proponi`), the warnings
// (`valida`) and the two boxes Blank · Infusion come from the engine through `api/campioni/<op>`; nothing is decided here. The time is optional: nothing here holds back «Load data».
// An entry of ST.files keeps `type` (the base type the other routes know: sample | blank | standard) and, for this table, `role` (campioni/2 `tipo`), `prop` (the proposal was asked), `ed` (the student chose the role: a late proposal does not overwrite it). The time `f.time` is in minutes, `f.gT` is
// true while it is only proposed from the name. A sample at time 0 is the t0 (no role of its own); the time of a dark is negative (minutes before the time zero).
// Model part: the roles of an entry, the proposals. The warnings are in ruoli-avvisi.js, the cells and the strip in ruoli-celle.js, the lists of the boxes in ruoli-menu.js (each adds its functions to RUOLI).
const RUOLI = /** @type {any} */ ((() => {
  /** @type {(k: string, p?: any) => string} */
  const t = (k, p) => I18N.t(k, p);
  const OPTS = ["sample", "blank", "dark", "infusion", "std", "qc"];
  const BASE = /** @type {Record<string, string>} */ ({ sample: "sample", blank: "blank", dark: "sample", infusion: "sample", std: "standard", qc: "sample" });
  const TIMED = new Set(["sample", "dark"]);        // roles with a time to write
  const BOXES = ["blank", "infusion"];
  const BOX_ROLE = /** @type {Record<string, string>} */ ({ blank: "blank", infusion: "infusion" });
  const eh = (/** @type {any} */ s) => EH(s == null ? "" : s);
  const post = (/** @type {string} */ op, /** @type {any} */ body) => fetch("api/campioni/" + op, { method: "POST", body: JSON.stringify(body) }).then(r => r.json());

  /** the value of the role select of an entry: sample, blank, dark, infusion, std, qc (the old `t0` is a sample) */
  const val = (/** @type {Voce} */ f) => f.role === "calibration" ? "std" : f.role === "t0" ? "sample" : f.role || (f.type === "standard" ? "std" : f.type || "sample");
  const roleOf = val;
  const timed = (/** @type {Voce} */ f) => TIMED.has(roleOf(f));
  const label = (/** @type {string} */ v) => t(`load.role.${v}`);
  const cellLabel = (/** @type {Voce} */ f, /** @type {string} */ col) => t("load.cell.of", { f: f.name, col });

  /** the role `v` for an entry (the student's choice or a proposal); the time follows the role */
  function setRole(/** @type {Voce} */ f, /** @type {string} */ v) {
    const was = roleOf(f), r = v === "standard" ? "std" : v === "t0" ? "sample" : v;   // the option of the select says «standard» (the value the rest of the program knows); `t0` = a sample at 0
    f.role = r; f.type = BASE[r] || "sample";
    if (v === "t0") { f.time = 0; f.gT = false; }
    else if (!TIMED.has(r)) { f.time = null; f.gT = false; }
    else if (r === "dark") f.time = darkTime(f.time ?? null);
    else if (was === "dark" && f.time != null && f.time < 0) { f.time = null; f.gT = false; }
  }

  let busy = 0;   // proposals under way (the tests wait for 0)
  /** proposals from the names for the entries not asked yet; true when something changed */
  async function propose(/** @type {() => Voce[]} */ get) {
    const todo = get().filter(f => !f.prop);   // the list is rebuilt while the answer travels: entries are looked up again by name
    if (!todo.length) return false;
    todo.forEach(f => { f.prop = 1; });
    let doc;
    busy++;
    try { doc = await post("proponi", { files: todo.map(f => ({ name: f.name, sha256: "", kind: f.kind || "" })) }); } catch (_) { return false; } finally { busy--; }
    let changed = false;
    for (const c of (doc && doc.campioni) || []) {
      const f = get().find(x => x.name === c.file);
      if (!f || f.ed) continue;
      const r = c.tipo === "calibration" ? "std" : c.tipo === "t0" ? "sample" : c.tipo;
      setRole(f, r);
      if (TIMED.has(r) && c.tempo_min != null) { f.time = r === "dark" ? darkTime(c.tempo_min) : c.tempo_min; f.gT = true; }
      changed = true;
    }
    return changed;
  }
  /** the roles, subtypes and confirmed times of the table go to the files just opened (CAMPIONI.apply is the only writer) */
  function commit(/** @type {Voce[]} */ F) {
    CAMPIONI.apply(F.filter(f => f.use).map(f => { const r = roleOf(f), e = /** @type {any} */ ({ file: f.name, role: r }); if (TIMED.has(r)) { e.time = f.time ?? null; e.tconf = !f.gT; } return e; }), true);
  }
  return { commit, t, eh, post, OPTS, BASE, TIMED, BOXES, BOX_ROLE, val, roleOf, timed, label, cellLabel, setRole, propose, busy: () => busy };
})());
