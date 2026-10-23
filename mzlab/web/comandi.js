// @ts-strict
// @ts-check
// Registry of the actions of the Dati view that change state (one row per action) and the way to make them undoable: an action runs through
// COMANDI.fai / COMANDI.inizia, which takes a small snapshot before and after and records ONE step in the stack of storia.js (STORIA).
// The table is the only source of docs/azioni.md (python3 tools/doc_comandi.py) and of the guards of tests/test_comandi.py. Classic script, loaded before storia.js.
//   id        = module.action          nome = i18n key of the label shown in «Undone: …»      modulo = file that owns the action
//   dove      = every file that changes the fields (the test refuses an assignment to a registered field in another file)
//   campi     = the state fields it changes     parametri = what the user chooses       tasto = shortcut      unisci = how drags / bursts become one step
//   annulla   = "istantanea" (zoom, integrations, fields: snapshots of storia.js) | "comando" (handler registered with COMANDI.gestore)
//             | "tolto" (a removal with its own undo/redo closures, STORIA.tolto) | "layout" (affianca.js) | "da-portare" (not undoable yet: the ratchet of the test) | "nessuna" (+ motivo)

// TAB-BEGIN
const COMANDI_TAB = [
  { id: "view.zoom", nome: "storia.zoom", modulo: "eventi.js", dove: ["eventi.js", "mappa.js", "pannelli.js", "banco.js", "dda.js", "spettro.js", "azioni-tasti.js", "scroll.js", "tabs.js", "grafici.js"], campi: ["zoom", "zoomY", "lock"],
    parametri: "m/z or RT interval, intensity interval", tasto: "wheel, Backspace, double click", unisci: "a burst of wheel steps and a drag are one step", annulla: "istantanea" },
  { id: "peaks.integration", nome: "storia.int", modulo: "picchi.js", dove: ["picchi.js", "eventi.js", "xic-spettro.js"], campi: ["ints", "isel"],
    parametri: "limits of the integrated peak", tasto: "I, Del", unisci: "an edge drag is one step", annulla: "istantanea" },
  { id: "panel.fields", nome: "storia.campi", modulo: "grafici.js", dove: ["grafici.js", "pannelli.js", "fondo-spettro.js", "sottrazione.js", "mappa.js", "barra.js"], campi: ["bk", "bg", "fondo", "smooth", "corr", "ref", "bgr"],
    parametri: "blank, background, smoothing, correction, reference", tasto: "", unisci: "", annulla: "istantanea" },
  { id: "spec.ruler", nome: "cmd.spec.ruler", modulo: "spettro.js", dove: ["spettro.js", "pannelli.js", "eventi.js", "banco.js"], campi: ["meas", "rul"],
    parametri: "the reference peak and the second peak of each measure", tasto: "R, Esc", unisci: "", annulla: "comando" },
  { id: "files.remove", nome: "storia.pannello", modulo: "lista-file.js", dove: ["lista-file.js"], campi: [],
    parametri: "the file", tasto: "", unisci: "", annulla: "tolto" },
  { id: "panel.close", nome: "storia.pannello", modulo: "pannelli.js", dove: ["pannelli.js"], campi: [],
    parametri: "the panel with its zoom, integrations and settings", tasto: "", unisci: "", annulla: "tolto" },
  { id: "map.marks.clear", nome: "storia.marcature", modulo: "mappa-tabella.js", dove: ["mappa-tabella.js"], campi: [],
    parametri: "the marks of the map", tasto: "", unisci: "", annulla: "tolto" },
  { id: "layout.arrange", nome: "aff.undone", modulo: "affianca.js", dove: ["affianca.js"], campi: [],
    parametri: "position and size of the panels", tasto: "Alt+arrows", unisci: "", annulla: "layout" },

  // ---- not undoable yet (annulla "da-portare": the count can only go down, tests/test_comandi.py) or not an action to undo ("nessuna" + motivo)
  { id: "view.pan", nome: "storia.zoom", modulo: "eventi.js", dove: ["eventi.js"], campi: ["zoom", "zoomY"], parametri: "drag with Shift, trackpad", tasto: "Shift+drag", unisci: "a drag is one step", annulla: "da-portare" },
  { id: "panel.source", nome: "storia.campi", modulo: "pannelli.js", dove: ["pannelli.js", "banco.js"], campi: ["k", "level", "prec", "all", "kind", "mode", "mz0", "mz1", "tr", "mapKind", "norm", "scale", "gamma", "log", "rel", "tl", "filt"],
    parametri: "file, MS level, precursor, chromatogram kind, display mode, m/z limits, scale", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "spec.annotation", nome: "storia.campi", modulo: "spettro.js", dove: ["spettro.js", "eventi.js"], campi: ["anns"], parametri: "add, edit, delete a note", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "spec.display", nome: "storia.campi", modulo: "visualizza.js", dove: ["visualizza.js", "spettro.js"], campi: ["thr", "nlab", "dec", "sticks", "dots", "mk", "noCont", "gdelay", "pdaDelay", "pdaAlign", "overlay"],
    parametri: "threshold, labels, decimals, sticks, dots, markers, contaminants, gradient overlay", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "spec.isotopes", nome: "storia.campi", modulo: "spettro.js", dove: ["spettro.js"], campi: ["iso"], parametri: "isotope simulation and resolution", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "spec.background", nome: "storia.fondo", modulo: "fondo-spettro.js", dove: ["fondo-spettro.js", "pannelli.js", "grafici.js"], campi: ["bw0", "bw1", "bwi"], parametri: "background window of the spectrum", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "peaks.noise", nome: "storia.int", modulo: "eventi.js", dove: ["eventi.js"], campi: ["noise"], parametri: "noise stretch of an integration", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "xic.traces", nome: "storia.campi", modulo: "pannelli.js", dove: ["pannelli.js", "xic-spettro.js", "grafici.js", "visualizza.js"], campi: ["traces"], parametri: "add, remove, tolerance and unit of the XIC traces, split", tasto: "X", unisci: "", annulla: "da-portare" },
  { id: "series.visibility", nome: "storia.campi", modulo: "grafici.js", dove: ["grafici.js", "picchi.js"], campi: ["hid", "showHid"], parametri: "hide or show a series from the legend", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "panel.link", nome: "storia.campi", modulo: "eventi.js", dove: ["eventi.js", "spettro.js"], campi: ["link", "src"], parametri: "freeze or relink the live spectrum", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "panel.rename", nome: "storia.pannello", modulo: "pannelli.js", dove: ["pannelli.js"], campi: ["title"], parametri: "title of the panel", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "panel.open", nome: "storia.pannello", modulo: "pannelli.js", dove: ["pannelli.js", "eventi.js", "mappa.js"], campi: [], parametri: "create a panel (spectrum, XIC, extracted window)", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "map.marks", nome: "storia.marcature", modulo: "mappa.js", dove: ["mappa.js", "mappa-tabella.js"], campi: [], parametri: "add, move, delete, note, reference of a mark", tasto: "", unisci: "a drag is one step", annulla: "da-portare" },
  { id: "files.view", nome: "storia.campi", modulo: "lista-file.js", dove: ["lista-file.js", "lista-file-menu.js", "explore.js"], campi: [], parametri: "visibility, name, colour, group, order of a file", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "samples.meta", nome: "storia.campi", modulo: "campioni-tabella.js", dove: ["campioni-tabella.js"], campi: [], parametri: "type, replicate, concentration, unit, time; TSV import", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "peaks.settings", nome: "storia.campi", modulo: "picchi.js", dove: ["picchi.js"], campi: [], parametri: "peak-finding parameters, options of the integration table", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "app.settings", nome: "storia.campi", modulo: "settings.js", dove: ["settings.js", "mappa.js"], campi: [], parametri: "theme, language, font, contrast, palette, map preferences", tasto: "", unisci: "", annulla: "da-portare" },
  { id: "flow.plan", nome: "storia.campi", modulo: "flow-shell.js", dove: ["flow-shell.js", "flow-form.js"], campi: [], parametri: "steps and parameters of the plan", tasto: "Ctrl+Z, Ctrl+Y", unisci: "one entry per change", annulla: "flow" },
  { id: "draw.editor", nome: "storia.campi", modulo: "draw.js", dove: ["draw.js"], campi: [], parametri: "the structure in the editor (Ketcher)", tasto: "Ctrl+Z, Ctrl+Y", unisci: "", annulla: "draw" },
  { id: "panel.selection", nome: "storia.campi", modulo: "eventi.js", dove: ["eventi.js"], campi: ["sel", "isel", "cur"], parametri: "selected interval, selected integration, cursor", tasto: "", unisci: "", annulla: "nessuna",
    motivo: "transient selection: it moves nothing the student made" },
  { id: "view.navigate", nome: "storia.campi", modulo: "pannelli.js", dove: ["pannelli.js", "banco.js"], campi: ["si", "r0", "r1"], parametri: "walk through the scans", tasto: "arrows", unisci: "", annulla: "nessuna",
    motivo: "navigation: it chooses what to look at, the data are the same" },
  { id: "view.fullscreen", nome: "storia.campi", modulo: "pannelli.js", dove: ["pannelli.js"], campi: [], parametri: "full screen of a panel", tasto: "F", unisci: "", annulla: "nessuna",
    motivo: "transient view of the page; Esc leaves it" },
];
// TAB-END

const COMANDI = (() => {
  /** @type {Record<string, { leggi: (p: any) => string, applica: (p: any, snap: string) => void }>} */
  const gestori = {};
  /** commands running now: a command inside a command is part of the outer step @type {{ id: string, p: any }[]} */
  const dentro = [];
  /** @param {string} id */
  const riga = id => COMANDI_TAB.find(r => r.id === id);
  /** @param {string} id */
  function voce(id) {
    const r = riga(id);
    if (!r) throw new Error("COMANDI: action not in the registry: " + id);
    return r;
  }
  /** The owner module says how to read the state of its action and how to put a snapshot back (the «undo» of the command). @param {string} id @param {{ leggi: (p: any) => string, applica: (p: any, snap: string) => void }} g */
  function gestore(id, g) {
    if (voce(id).annulla !== "comando") throw new Error("COMANDI: " + id + " is not a command");
    gestori[id] = g;
  }
  /** @param {string} id */
  function gest(id) { voce(id); const g = gestori[id]; if (!g) throw new Error("COMANDI: no handler for " + id); return g; }
  /** Runs `cambia` and records it as one undoable step when the state of the action changed. @param {string} id @param {any} p @param {() => any} cambia @param {{ unisci?: string }} [o] */
  function fai(id, p, cambia, o) {
    const g = gest(id);
    if (dentro.some(d => d.id === id && d.p === p)) return cambia();
    const prima = g.leggi(p), d = { id, p };
    dentro.push(d);
    let r;
    try { r = cambia(); } finally { dentro.splice(dentro.indexOf(d), 1); }
    if (g.leggi(p) !== prima && STORIA) STORIA.comando(id, p, prima, o && o.unisci);
    return r;
  }
  /** The change comes later (a drag): `fine()` records the step if the state is not the one of now. @param {string} id @param {any} p @param {{ unisci?: string }} [o] */
  function inizia(id, p, o) {
    const g = gest(id), prima = g.leggi(p);
    return { fine() { if (g.leggi(p) !== prima && STORIA) STORIA.comando(id, p, prima, o && o.unisci); } };
  }
  /** Ids of the registered commands without a handler (the e2e wants it empty). */
  const senzaGestore = () => COMANDI_TAB.filter(r => r.annulla === "comando" && !gestori[r.id]).map(r => r.id);
  return { tab: COMANDI_TAB, riga, gestore, gest, fai, inizia, senzaGestore };
})();
