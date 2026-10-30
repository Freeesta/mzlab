// @ts-strict
"use strict";
// Roles and times in the loading table (`campioni/2`, the Rust of mzlab-core). The proposals from the file names (`proponi`) come from the engine through `api/campioni/<op>`;
// nothing is decided here. The time is optional and written in the table: the files open as soon as they are read, and the table writes through to them (OPENNOW).
// An entry of ST.files keeps `type` (the base type the other routes know: sample | blank | standard) and, for this table, `role` (campioni/2 `tipo`), `prop` (the proposal was asked), `ed` (the student chose the role: a late proposal does not overwrite it), `gR` / `gT` (role / time from a proposal, untouched: yellow `guess` cells). The time `f.time` is in minutes. A sample at time 0 is the t0 (no role of its own); the time of a dark is negative (minutes before the time zero).
// The cells are in ruoli-celle.js (it adds its functions to RUOLI).
const RUOLI = /** @type {any} */ ((() => {
  /** @type {(k: string, p?: any) => string} */
  const t = (k, p) => I18N.t(k, p);
  const OPTS = ["sample", "blank", "dark", "infusion", "std", "qc"];
  const BASE = /** @type {Record<string, string>} */ ({ sample: "sample", blank: "blank", dark: "sample", infusion: "sample", std: "standard", qc: "sample" });
  const TIMED = new Set(["sample", "dark"]);        // roles with a time to write
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
    // the option of the select says «standard» (the value the rest of the program knows); `t0` = a sample at 0
    const was = roleOf(f), r = v === "standard" ? "std" : v === "t0" ? "sample" : v;
    f.role = r; f.type = BASE[r] || "sample";
    if (v === "t0") f.time = 0;
    else if (!TIMED.has(r)) f.time = null;
    else if (r === "dark") f.time = darkTime(f.time ?? null);
    else if (was === "dark" && f.time != null && f.time < 0) f.time = null;
  }

  let busy = 0;   // proposals under way (the tests wait for 0)
  /** proposals from the names for the entries not asked yet; true when something changed */
  async function propose(/** @type {() => Voce[]} */ get) {
    // the list is rebuilt while the answer travels: entries are looked up again by name
    const todo = get().filter(f => !f.prop);
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
      setRole(f, r); f.gR = true;
      if (TIMED.has(r) && c.tempo_min != null) { f.time = r === "dark" ? darkTime(c.tempo_min) : c.tempo_min; f.gT = true; }
      changed = true;
    }
    return changed;
  }
  /** the roles and times of the table go to the files just opened (CAMPIONI.apply is the only writer) */
  function commit(/** @type {Voce[]} */ F) {
    CAMPIONI.apply(F.filter(f => f.use).map(f => { const r = roleOf(f), e = /** @type {any} */ ({ file: f.name, role: r }); if (TIMED.has(r)) { e.time = f.time ?? null; e.tconf = true; } return e; }), true);
  }
  /** Repaints and proposals for a table fed by many merges. `soon()` asks one repaint per animation frame (a hidden tab has no frame: `flush()` paints it); `flush(force)` paints now and asks
   *  the proposals once for every entry merged since (only when something was merged, unless `force`). `raf`/`caf` default to FRAME.next/FRAME.cancel. The table of a loading repaints per frame and asks the proposals once when the loading flushes; a merge outside a loading flushes at once. */
  function coalesce(/** @type {() => void} */ paint, /** @type {() => unknown} */ propose, /** @type {any} */ raf, /** @type {any} */ caf) {
    const ask = raf || ((/** @type {() => void} */ f) => FRAME.next(f)), drop = caf || ((/** @type {any} */ h) => FRAME.cancel(h));
    let h = /** @type {any} */ (null), dirty = false;
    return {
      soon() { dirty = true; if (h == null) h = ask(() => { h = null; paint(); }); },
      flush(/** @type {boolean} */ force = false) {
        if (h != null) { drop(h); h = null; }
        if (!dirty && !force) return undefined;
        dirty = false; paint();
        return propose();
      },
    };
  }
  return { commit, coalesce, t, eh, post, OPTS, BASE, TIMED, val, roleOf, timed, label, cellLabel, setRole, propose, busy: () => busy };
})());
