// @ts-strict
// @ts-check
// Registry of the actions of the Dati view that change state (one row per action) and the way to make them undoable: an action runs through
// COMMANDS.run / COMMANDS.begin, which takes a small snapshot before and after and records ONE step in the stack of history.js (HISTORY).
// The table is the only source of docs/actions.md (python3 tools/doc_commands.py) and of the guards of tests/test_comandi.py. Classic script, loaded before history.js.
//   id        = module.action          name = i18n key of the label shown in «Undone: …»      module = file that owns the action
//   where     = every file that changes the fields (the test refuses an assignment to a registered field in another file)
//   fields    = the state fields it changes     params = what the user chooses       key = shortcut      merge = how drags / bursts become one step
//   p of a command = the panel it acts on, or COMMANDS.GLOBAL for the actions of no panel
//   undo      = "snapshot" (zoom, integrations, fields: snapshots of history.js) | "command" (handler registered with COMMANDS.handler)
//             | "removed" (a removal with its own undo/redo closures, HISTORY.removed) | "layout" (arrange.js) | "to-port" (not undoable yet: the ratchet of the test) | "none" (+ reason)

// TAB-BEGIN
const COMMANDS_TAB = [
  { id: "view.zoom", name: "history.zoom", module: "eventi.js", where: ["eventi.js", "mappa.js", "pannelli.js", "banco.js", "dda.js", "spettro.js", "azioni-tasti.js", "scroll.js", "tabs.js", "grafici.js", "xic-bordi.js", "zoom-reset.js"], fields: ["zoom", "zoomY", "lock"],
    params: "m/z or RT interval, intensity interval", key: "wheel, Backspace, double click, Shift+drag", merge: "a burst of wheel steps, a drag and a pan are one step", undo: "snapshot" },
  { id: "peaks.integration", name: "history.int", module: "picchi.js", where: ["picchi.js", "eventi.js", "xic-spettro.js", "pannelli.js", "panels-undo.js", "integra-estremi.js", "integra-valli.js", "integra-tutti.js"], fields: ["ints", "isel"],
    params: "limits of the integrated peak", key: "I, Del", merge: "an edge drag is one step", undo: "snapshot" },
  { id: "panel.fields", name: "history.fields", module: "grafici.js", where: ["grafici.js", "pannelli.js", "fondo-spettro.js", "fondo-piatto.js", "sottrazione.js", "mappa.js", "barra.js"], fields: ["bk", "bg", "fondo", "smooth", "corr", "ref", "bgr"],
    params: "blank, background, smoothing, correction, reference", key: "", merge: "", undo: "snapshot" },
  { id: "spec.ruler", name: "cmd.spec.ruler", module: "spettro.js", where: ["spettro.js", "pannelli.js", "eventi.js", "banco.js"], fields: ["meas", "rul"],
    params: "the reference peak and the second peak of each measure", key: "R, Esc", merge: "", undo: "command" },
  { id: "files.remove", name: "history.panel", module: "lista-file.js", where: ["lista-file.js"], fields: [],
    params: "the file", key: "", merge: "", undo: "removed" },
  { id: "panel.close", name: "history.panel", module: "pannelli.js", where: ["pannelli.js"], fields: [],
    params: "the panel with its zoom, integrations and settings", key: "", merge: "", undo: "removed" },
  { id: "map.marks.clear", name: "history.marks", module: "mappa-tabella.js", where: ["mappa-tabella.js"], fields: [],
    params: "the marks of the map", key: "", merge: "", undo: "removed" },
  { id: "layout.arrange", name: "arrange.undone", module: "arrange.js", where: ["arrange.js"], fields: [],
    params: "position and size of the panels", key: "Alt+arrows", merge: "", undo: "layout" },

  // ---- panel.source (a command) and the actions that are not undoable by design ("none" + motivo, enforced by tests/test_comandi.py); "to-port" stays at zero
  { id: "panel.source", name: "cmd.source", module: "pannelli.js", where: ["pannelli.js", "banco.js", "lista-file.js", "grafici.js", "picchi.js", "infusione.js", "eventi.js", "sim-ui.js", "canali.js", "xic-bordi.js", "xwc.js"], fields: ["k", "kSet", "level", "prec", "all", "kind", "mode", "mz0", "mz1", "xw", "tr", "mapKind", "norm", "scale", "gamma", "log"],
    params: "file, MS level, precursor, chromatogram kind, display mode, m/z limits, scale", key: "", merge: "", undo: "command" },
  { id: "spec.annotation", name: "cmd.annotation", module: "eventi.js", where: ["spettro.js", "eventi.js"], fields: ["anns"], params: "add, edit, delete a note", key: "", merge: "", undo: "command" },
  { id: "spec.display", name: "cmd.display", module: "visualizza.js", where: ["visualizza.js", "spettro.js", "pannelli.js", "barra.js", "gradiente.js", "etichette-picchi.js", "dad-mrm.js", "peak-info.js"], fields: ["thr", "nlab", "dec", "sticks", "dots", "mk", "noCont", "rel", "gdelay", "pdaDelay", "pdaAlign", "overlay", "etpk", "lblOn", "lblOff"],
    params: "threshold, labels, decimals, sticks, dots, markers, contaminants, gradient overlay", key: "", merge: "", undo: "command" },
  { id: "spec.isotopes", name: "history.fields", module: "spettro.js", where: ["spettro.js", "banco.js", "picchi.js"], fields: ["iso"], params: "isotope simulation and resolution", key: "", merge: "", undo: "command" },
  { id: "spec.background", name: "history.background", module: "fondo-spettro.js", where: ["fondo-spettro.js", "pannelli.js", "grafici.js", "sottrazione.js"], fields: ["bg", "bw0", "bw1", "bwi"], params: "background window of the spectrum", key: "", merge: "", undo: "command" },
  { id: "peaks.noise", name: "history.int", module: "eventi.js", where: ["eventi.js", "picchi.js"], fields: ["noise"], params: "noise stretch of an integration", key: "", merge: "", undo: "snapshot" },
  { id: "xic.traces", name: "history.fields", module: "panels-undo.js", where: ["pannelli.js", "panels-undo.js", "xic-spettro.js", "xic-bordi.js", "grafici.js", "visualizza.js"], fields: ["traces"], params: "add, remove, label, window and unit of the XIC traces (the split into panels is not undone)", key: "X", merge: "", undo: "command" },
  { id: "series.visibility", name: "history.fields", module: "grafici.js", where: ["grafici.js", "picchi.js"], fields: ["hid", "showHid"], params: "hide or show a series from the legend", key: "", merge: "", undo: "command" },
  { id: "panel.link", name: "history.fields", module: "eventi.js", where: ["eventi.js", "spettro.js", "banco.js", "pannelli.js", "dda.js"], fields: ["link", "src"], params: "freeze or relink the live spectrum", key: "", merge: "", undo: "command" },
  { id: "panel.rename", name: "history.panel", module: "panels-undo.js", where: ["pannelli.js", "panels-undo.js", "banco.js"], fields: ["title"], params: "title of the panel", key: "", merge: "", undo: "command" },
  { id: "panel.open", name: "history.panel", module: "pannelli.js", where: ["pannelli.js", "eventi.js", "banco.js", "uv-spettro.js"], fields: [], params: "create a panel (spectrum, XIC, extracted window, UV spectrum, wavelength trace)", key: "", merge: "", undo: "removed" },
  { id: "map.marks", name: "history.marks", module: "mappa.js", where: ["mappa.js", "mappa-tabella.js"], fields: [], params: "add, move, delete, note, reference of a mark", key: "", merge: "a drag is one step", undo: "command" },
  { id: "files.view", name: "history.fields", module: "lista-file.js", where: ["lista-file.js", "lista-file-menu.js", "explore.js"], fields: [], params: "visibility, name, colour, group, order of a file", key: "", merge: "", undo: "command" },
  { id: "samples.meta", name: "history.fields", module: "samples-table.js", where: ["samples-table.js", "qaqc.js"], fields: [], params: "type, replicate, concentration, unit, time; TSV import", key: "", merge: "", undo: "command" },
  { id: "peaks.settings", name: "history.fields", module: "picchi.js", where: ["picchi.js", "integrazioni-manuali.js"], fields: [], params: "peak-finding parameters, options of the integration table", key: "", merge: "", undo: "command" },
  { id: "app.settings", name: "history.fields", module: "settings.js", where: ["settings.js"], fields: [], params: "theme, palette, font, contrast, motion, side by side, tolerance and decimals of the high resolution (the language reloads the page and is not undone)", key: "", merge: "", undo: "command" },
  { id: "flow.plan", name: "history.fields", module: "flow-shell.js", where: ["flow-shell.js", "flow-form.js"], fields: [], params: "steps and parameters of the plan", key: "Ctrl+Z, Ctrl+Y", merge: "one entry per change", undo: "flow" },
  { id: "draw.editor", name: "history.fields", module: "draw.js", where: ["draw.js"], fields: [], params: "the structure in the editor (Ketcher)", key: "Ctrl+Z, Ctrl+Y", merge: "", undo: "draw" },
  { id: "panel.selection", name: "history.fields", module: "eventi.js", where: ["eventi.js"], fields: ["sel", "isel", "cur"], params: "selected interval, selected integration, cursor", key: "", merge: "", undo: "none",
    reason: "transient selection: it moves nothing the student made" },
  { id: "view.navigate", name: "history.fields", module: "pannelli.js", where: ["pannelli.js", "banco.js"], fields: ["si", "r0", "r1"], params: "walk through the scans", key: "arrows", merge: "", undo: "none",
    reason: "navigation: it chooses what to look at, the data are the same" },
  { id: "view.fullscreen", name: "history.fields", module: "pannelli.js", where: ["pannelli.js"], fields: [], params: "full screen of a panel", key: "F", merge: "", undo: "none",
    reason: "transient view of the page; Esc leaves it" },
];
// TAB-END

const COMMANDS = (() => {
  /** @type {Record<string, { read: (p: any) => string, apply: (p: any, snap: string) => void }>} */
  const handlers = {};
  /** commands running now: a command inside a command is part of the outer step @type {{ id: string, p: any }[]} */
  const running = [];
  /** @param {string} id */
  const row = id => COMMANDS_TAB.find(r => r.id === id);
  /** @param {string} id */
  function entry(id) {
    const r = row(id);
    if (!r) throw new Error("COMMANDS: action not in the registry: " + id);
    return r;
  }
  /** The owner module says how to read the state of its action and how to put a snapshot back (the «undo» of the command). @param {string} id @param {{ read: (p: any) => string, apply: (p: any, snap: string) => void }} g */
  function handler(id, g) {
    if (entry(id).undo !== "command") throw new Error("COMMANDS: " + id + " is not a command");
    handlers[id] = g;
  }
  /** @param {string} id */
  function handlerOf(id) { entry(id); const g = handlers[id]; if (!g) throw new Error("COMMANDS: no handler for " + id); return g; }
  /** Puts a snapshot back (the undo itself): a command run by the handler is part of this step and records nothing. @param {string} id @param {any} p @param {string} snap */
  function restore(id, p, snap) {
    const g = handlerOf(id), d = { id, p };
    running.push(d);
    try { g.apply(p, snap); } finally { running.splice(running.indexOf(d), 1); }
  }
  /** Runs `change` and records it as one undoable step when the state of the action changed. @param {string} id @param {any} p @param {() => any} change @param {{ merge?: string }} [o] */
  function run(id, p, change, o) {
    const g = handlerOf(id);
    if (running.some(d => d.id === id && d.p === p)) return change();
    const before = g.read(p), d = { id, p };
    running.push(d);
    let r;
    try { r = change(); } finally { running.splice(running.indexOf(d), 1); }
    if (g.read(p) !== before && HISTORY) HISTORY.command(id, p, before, o && o.merge);
    return r;
  }
  /** The change comes later (a drag): `end()` records the step if the state is not the one of now. @param {string} id @param {any} p @param {{ merge?: string }} [o] */
  function begin(id, p, o) {
    const g = handlerOf(id), before = g.read(p);
    return { end() { if (g.read(p) !== before && HISTORY) HISTORY.command(id, p, before, o && o.merge); } };
  }
  /** The `p` of the actions that belong to no panel (tables, settings, the file list): always visible to the undo of the Dati view. */
  const GLOBAL = { global: true };
  /** Ids of the registered commands without a handler (the e2e wants it empty). */
  const withoutHandler = () => COMMANDS_TAB.filter(r => r.undo === "command" && !handlers[r.id]).map(r => r.id);
  return { tab: COMMANDS_TAB, GLOBAL, row, handler, handlerOf, run, begin, restore, withoutHandler };
})();
