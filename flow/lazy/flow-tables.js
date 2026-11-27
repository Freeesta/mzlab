// @ts-check
// Flow (administrator mode): the column «Feature tables» on the right of the scheme (docs/studies/MZFLOW_SCREENS.md §A). One button per .mzfeat the plan made: the step and the polarity,
// rows · files, the state and the time. The facts come from Rust (`tables/1`: the page's wasm `table_entry` for the tables of OPFS, the native route `flow/live/tables` for the app, the same
// bytes as `mzlab-cli flow tables`); this file only draws. A button opens the table in the Feature table view; «Open .mzfeat…» opens one from disk.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const OPFS = "mzlab/flow/live/";
  // the texts are plain English like those of every module of the Flow (`flow/` is outside the catalog `lang/`, as tools/controlla_i18n.py lists it)
  const T = {
    title: "Feature tables", empty: "No feature table yet. Run the plan to make one, or open a .mzfeat file.", fail: "The tables could not be listed",
    pick: "Choose a .mzfeat file", open: "Open .mzfeat…", pos: "ESI+", neg: "ESI−", ok: "done", cached: "up to date", fromDisk: "from disk",
    n: (/** @type {number} */ k, /** @type {string} */ w) => `${k} ${w}${k === 1 ? "" : "s"}`,
  };
  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];

  /** @typedef {{name: string, title: string, rows: number, files: number, polarity: number, status: string, ms: number, load: () => Promise<File>}} Row */

  /** a row from a `tables/1` entry; the title is the step's name when the entry knows its step @param {any} e @param {any} S @param {() => Promise<File>} load @returns {Row} */
  const rowOf = (e, S, load) => {
    const inst = e.instance && S.plan.steps.find((/** @type {any} */ i) => i.id === e.instance);
    return { name: e.name, title: inst ? F.nameOf(base(inst))[0] : String(e.name).replace(/\.mzfeat$/i, ""), rows: e.rows, files: e.files, polarity: e.polarity || 0, status: e.status || "", ms: e.ms || 0, load };
  };

  /** the tables of the last run of the app: the native route lists the run folder @param {any} S @returns {Promise<{rows: Row[], skipped: string[]}>} */
  const fromApp = async S => {
    const doc = JSON.parse(await F.desktopPost("flow/live/tables", {}));
    return { rows: doc.tables.map((/** @type {any} */ e) => rowOf(e, S, () => F.desktopFile(e.name))), skipped: doc.skipped.map((/** @type {any} */ s) => s.name) };
  };

  /** the tables this browser keeps in OPFS for the steps the scheduler finished: the page tells the wasm which step made each one @param {any} S @returns {Promise<{rows: Row[], skipped: string[]}>} */
  const fromBrowser = async S => {
    const R = F.runners.web, x = F.live && F.live.state && F.live.state(), none = { rows: [], skipped: [] };
    if (!R || !R.available() || !x) return none;
    if (!R.kit.poolIfAny()) return none;
    // a step is done or reused only once the pool exists, so there is nothing to list (and no pool to start) before
    const pool = R.kit.poolIfAny(), rows = /** @type {Row[]} */ ([]), skipped = /** @type {string[]} */ ([]);
    for (const inst of S.plan.steps) {
      const key = x.tableKey(inst), node = F.liveView.node(inst.id), path = OPFS + key + ".mzfeat", name = inst.id + ".mzfeat";
      if (!key || !node || (node.state !== "done" && node.state !== "reused") || !(await pool.fs.exists(path))) continue;
      const made = JSON.stringify({ instance: inst.id, step: inst.step, status: node.state === "reused" ? "cached" : "ok", ms: node.ms || 0, warnings: 0 });
      try { rows.push(rowOf(JSON.parse(await pool.tableEntry({ path, name, made })), S, async () => new File([await pool.fs.read(path)], name))); } catch (_) { skipped.push(name); }
    }
    return { rows, skipped };
  };

  /** the column; `refresh()` lists again (after a run, or when the scheduler says a step changed) @param {any} S @returns {{root: HTMLElement, refresh: () => Promise<void>}} */
  F.tablesPanel = S => {
    const { el, btn } = F;
    const list = el("ul", { class: "flx-tabs-list" }), note = el("p", { class: "fl-note", role: "status" });
    const pick = el("input", { type: "file", hidden: true, accept: ".mzfeat", "aria-label": T.pick });
    const root = el("section", { class: "flx-tabs", "aria-label": T.title }, el("h3", { text: T.title }), list, note, pick,
      btn("import", T.pick, () => pick.click(), T.open));
    /** @type {File[]} */ const disk = [];
    /** opens a table in the Feature table view @param {string} name @param {() => Promise<File>} load */
    const open = async (name, load) => {
      try { F.openResult({ type: "table", label: name, file: await load() }); } catch (e) { note.textContent = F.message ? F.message(/** @type {any} */ (e)) : String(e); }
    };
    const fact = (/** @type {Row} */ r) => `${T.n(r.rows, "row")} · ${T.n(r.files, "file")}`;
    let sig = "";
    const draw = (/** @type {Row[]} */ rows, /** @type {string[]} */ skipped) => {
      // the same list again is not drawn again: a button that has the focus keeps it
      const now = JSON.stringify([rows.map(r => [r.name, r.title, r.rows, r.files, r.polarity, r.status, r.ms]), skipped, disk.map(f => f.name)]);
      if (now === sig) return;
      sig = now; list.replaceChildren();
      for (const r of rows) {
        const state = [r.status ? (r.status === "cached" ? T.cached : T.ok) : "", r.ms ? `${(r.ms / 1000).toFixed(1)} s` : ""].filter(Boolean).join(" · ");
        const pol = r.polarity ? " " + (r.polarity > 0 ? T.pos : T.neg) : "";
        list.append(el("li", {}, el("button", { type: "button", class: "flx-tab", "aria-label": `Open ${r.name}: ${fact(r)}`, onclick: () => open(r.name, r.load) },
          el("strong", { text: r.title + pol }), el("small", { text: fact(r) }), state ? el("small", { text: state }) : null)));
      }
      for (const f of disk) {
        list.append(el("li", {}, el("button", { type: "button", class: "flx-tab", "aria-label": `Open ${f.name}: ${T.fromDisk}`, onclick: () => open(f.name, async () => f) },
          el("strong", { text: f.name }), el("small", { text: `${T.fromDisk} · ${(f.size / 1048576).toFixed(1)} MB` }))));
      }
      note.textContent = skipped.length ? skipped.map(n => `${n} could not be read as a feature table`).join(" ") : list.children.length ? "" : T.empty;
    };
    let asked = 0;
    const refresh = async () => {
      const n = ++asked;
      try {
        const got = await (F.runners.desktop && F.runners.desktop.available() ? fromApp(S) : fromBrowser(S));
        if (n === asked) draw(got.rows, got.skipped);
      } catch (_) { if (n === asked) { draw([], []); note.textContent = T.fail; } }
    };
    pick.addEventListener("change", () => { const f = pick.files && pick.files[0]; if (!f) return; disk.push(f); pick.value = ""; refresh().then(() => open(f.name, async () => f)); });
    // the scheduler reports every change of a card; a short wait lets a burst of events make one listing
    let due = 0;
    const soon = () => { if (!root.isConnected) { document.removeEventListener("flow:live-view", soon); document.removeEventListener("flow:changed", soon); return; } clearTimeout(due); due = window.setTimeout(refresh, 300); };
    document.addEventListener("flow:live-view", soon); document.addEventListener("flow:changed", soon);
    draw([], []);
    refresh();
    return { root, refresh };
  };
})();
