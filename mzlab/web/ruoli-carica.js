// @ts-check
"use strict";
// Roles and times in the loading table (`campioni/2`, the Rust of mzlab-core). This file packs and shows: the proposals from the file names (`proponi`), the warnings
// (`valida`) and the three boxes Blank · t0 · Infusion come from the engine through `api/campioni/<op>`; nothing is decided here.
// An entry of ST.files keeps `type` (the base type the other routes know: sample | blank | standard) and, for this table, `role` (campioni/2 `tipo`), `sub`
// (`tipo_bianco`), `prop` (the proposal was asked), `ed` (the student chose the role: a late proposal does not overwrite it). The time `f.time` is in minutes, `f.gT` is
// true while it is only proposed from the name.
// Model part: the roles of an entry, the proposals. The warnings are in ruoli-avvisi.js, the cells and the strip in ruoli-celle.js, the lists of the boxes in ruoli-menu.js (each adds its functions to RUOLI).
const RUOLI = /** @type {any} */ ((() => {
  /** @type {(k: string, p?: any) => string} */
  const t = (k, p) => I18N.t(k, p);
  const OPTS = ["sample", "t0", "blank", "blank:solvent", "blank:procedure", "dark", "infusion", "std", "qc"];
  const BASE = /** @type {Record<string, string>} */ ({ sample: "sample", t0: "sample", blank: "blank", dark: "sample", infusion: "sample", std: "standard", qc: "sample" });
  const TIMED = new Set(["sample", "dark"]);        // roles with a time to write
  const BOXES = ["blank", "t0", "infusion"];
  const REQUIRED = new Set(["timeMissing", "timeUnconfirmed"]);
  const BOX_ROLE = /** @type {Record<string, string>} */ ({ blank: "blank", t0: "t0", infusion: "infusion" });
  const eh = (/** @type {any} */ s) => EH(s == null ? "" : s);
  const post = (/** @type {string} */ op, /** @type {any} */ body) => fetch("api/campioni/" + op, { method: "POST", body: JSON.stringify(body) }).then(r => r.json());

  /** the value of the role select of an entry: sample, t0, blank, blank:solvent, blank:procedure, dark, infusion, std, qc */
  const val = (/** @type {Voce} */ f) => { const r = f.role === "calibration" ? "std" : f.role || (f.type === "standard" ? "std" : f.type || "sample"); return r === "blank" && f.sub ? "blank:" + f.sub : r; };
  const roleOf = (/** @type {Voce} */ f) => val(f).split(":")[0];
  const timed = (/** @type {Voce} */ f) => TIMED.has(roleOf(f));
  const label = (/** @type {string} */ v) => t(`load.role.${v.replace(":", ".")}`);
  const cellLabel = (/** @type {Voce} */ f, /** @type {string} */ col) => t("load.cell.of", { f: f.name, col });

  /** the role `v` for an entry (the student's choice or a proposal); the time follows the role */
  function setRole(/** @type {Voce} */ f, /** @type {string} */ v) {
    const was = roleOf(f), [r, sub] = (v === "standard" ? "std" : v).split(":");   // the option of the select says «standard» (the value the rest of the program knows)
    f.role = r; f.sub = sub || null; f.type = BASE[r] || "sample";
    if (r === "t0") { f.time = 0; f.gT = false; }
    else if (!TIMED.has(r)) { f.time = null; f.gT = false; }
    else if (was === "t0") { f.time = null; f.gT = false; }
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
      let r = c.tipo === "calibration" ? "std" : c.tipo;
      if (r === "sample" && c.tempo_min === 0) r = "t0";
      setRole(f, r === "blank" && c.tipo_bianco ? "blank:" + c.tipo_bianco : r);
      if (TIMED.has(r) && c.tempo_min != null) { f.time = c.tempo_min; f.gT = true; }
      changed = true;
    }
    return changed;
  }
  return { t, eh, post, OPTS, BASE, TIMED, BOXES, REQUIRED, BOX_ROLE, val, roleOf, timed, label, cellLabel, setRole, propose, busy: () => busy };
})());
