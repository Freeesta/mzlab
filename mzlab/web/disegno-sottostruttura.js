// @ts-check
// «Substructure search» of Draw: the right-click rows send the selection (or the whole drawing) as a query to the Rust search over a structure list
// (`substructure` of the molkit .wasm, `subdb/1`; the app asks `feat/grafo`, kind `sub`): the catalogue of the program or a file the user picks, read
// in memory. The hits are candidates to check, never identifications; «Show» draws one, selects its matched atoms and colours the moiety
// (`disegno-evidenza.js`), «Back» restores the drawing.
import { carica } from "./disegno-verifica.js";
import { molfileOf } from "./disegno-molfile.js";

const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => I18N.t(k, p);
const SHOWN = 200;                                   // rows listed at once; the data table keeps them all
/** @type {{query: string, atoms: number, what: string} | null} */ let query = null;
/** @type {any} */ let last = null;                  // the last subdb/1 answer
/** @type {string | null} */ let saved = null;       // the drawing (KET) before the first «Show»
/** @type {{name: string, text: string} | null} */ let file = null;
let run = 0;

/** Ketcher of the frame, once Draw has started it */
async function ketcher() {
  await /** @type {any} */ (window).TPDraw.when();
  return /** @type {any} */ (document.querySelector("#kframe")).contentWindow.ketcher;
}

/** `subdb/1` text of a request: the native engine in the app, the molkit .wasm on the site */
async function ask(/** @type {string} */ request) {
  if (/** @type {any} */ (window).MZLAB_DESKTOP) {
    const base = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
    const r = await fetch(base + "rpc", { method: "POST", body: JSON.stringify({ op: "feat/grafo", kind: "sub", request }) });
    const x = await r.text();
    if (r.status !== 200) throw x;
    return x;
  }
  return (await carica()).substructure(request);
}

/** the message of an engine error (`{error_key, params}` text) or of anything else */
function message(/** @type {any} */ e) {
  try { const j = JSON.parse(String(e)); if (j.error_key) return t(j.error_key, j.params || {}); } catch (_) { /* not JSON */ }
  return e && e.message ? e.message : String(e);
}

const $ = (/** @type {string} */ s) => /** @type {HTMLElement} */ (document.querySelector(s));

/** the card, made on first use after the check card (or the properties card) */
function card() {
  const have = document.querySelector("#sub-card");
  if (have) return /** @type {HTMLElement} */ (have);
  const c = document.createElement("div");
  c.className = "card"; c.id = "sub-card"; c.dataset.rqOpen = "1";   // made by a menu row: it opens
  c.innerHTML = '<h2 data-i18n="draw.sub.heading"></h2><p class="rq-d" data-i18n="draw.sub.intro"></p>'
    + '<div id="sub-query" class="sm"></div>'
    + '<fieldset class="sub-src"><legend class="sm" data-i18n="draw.sub.source"></legend>'
    + '<label><input type="radio" name="sub-src" value="cat" checked> <span data-i18n="draw.sub.source.cat"></span></label> '
    + '<label><input type="radio" name="sub-src" value="file"> <span data-i18n="draw.sub.source.file"></span></label> '
    + '<input type="file" id="sub-file" accept=".sdf,.sd,.mol,.smi,.smiles,.txt,.csv,.tsv" data-i18n-aria="draw.sub.file.aria"> <span id="sub-fname" class="sm muted"></span></fieldset>'
    + '<div style="margin:3px 0"><label><input type="checkbox" autocomplete="off" id="sub-stereo"> <span data-i18n="draw.sub.stereo"></span></label></div>'
    + '<div class="bar"><button type="button" id="sub-run" data-i18n="draw.sub.run" data-i18n-title="draw.sub.run.title"></button>'
    + ' <button type="button" id="sub-table" class="sm" data-i18n="draw.sub.table" data-i18n-title="draw.sub.table.title" hidden></button>'
    + ' <button type="button" id="sub-back" class="sm" data-i18n="draw.sub.back" data-i18n-title="draw.sub.back.title" hidden></button></div>'
    + '<div id="sub-status" class="sm" role="status"></div><ul id="sub-list" class="sm" aria-labelledby="sub-status"></ul>';
  c.querySelectorAll("[data-i18n]").forEach(e => { e.textContent = t(/** @type {string} */ (e.getAttribute("data-i18n"))); });
  c.querySelectorAll("[data-i18n-title]").forEach(e => { e.setAttribute("title", t(/** @type {string} */ (e.getAttribute("data-i18n-title")))); });
  c.querySelectorAll("[data-i18n-aria]").forEach(e => { e.setAttribute("aria-label", t(/** @type {string} */ (e.getAttribute("data-i18n-aria")))); });
  const css = document.createElement("style");
  css.textContent = "#sub-list{list-style:none;padding:0;margin:4px 0 0}#sub-list li{margin:4px 0;padding-left:6px;border-left:3px solid var(--lv1)}"
    + "#sub-card button,#sub-card input[type=file]{min-height:24px;min-width:24px}#sub-card fieldset{border:0;padding:0;margin:4px 0}"
    + "#sub-card label{display:inline-flex;align-items:center;min-height:24px;margin-right:8px}#sub-card input[type=file]{max-width:100%;box-sizing:border-box}.sub-show{margin-left:6px}";
  c.appendChild(css);
  const after = document.querySelector("#chk-card") || document.querySelector("#anal-card");
  if (!after || !after.parentNode) throw new Error("Draw cards");
  after.parentNode.insertBefore(c, after.nextSibling);
  /** @type {HTMLInputElement} */ (c.querySelector("#sub-file")).onchange = async (e) => {
    const f = /** @type {HTMLInputElement} */ (e.target).files?.[0];
    if (!f) return;
    file = { name: f.name, text: await f.text() };
    $("#sub-fname").textContent = f.name;
    /** @type {HTMLInputElement} */ (c.querySelector('input[value="file"]')).checked = true;
  };
  $("#sub-run").onclick = () => { searchNow(); };
  $("#sub-table").onclick = dataTable;
  $("#sub-back").onclick = () => { back(); };
  return c;
}

/** the text of the chosen structure list: the catalogue of the program, or the file picked */
async function listText() {
  const src = /** @type {HTMLInputElement} */ (document.querySelector('input[name="sub-src"]:checked')).value;
  if (src === "file") {
    if (!file) throw new Error(t("draw.sub.file.none"));
    return { text: file.text, label: file.name };
  }
  const r = await fetch("static/molecole/catalogo.csv");
  if (!r.ok) throw new Error("catalogo.csv: " + r.status);
  return { text: await r.text(), label: t("draw.sub.source.cat") };
}

/** runs the search with the query taken from the menu and fills the list */
export async function searchNow() {
  const status = $("#sub-status"), list = $("#sub-list"), me = ++run;
  if (!query) return null;
  status.textContent = t("draw.sub.busy");
  list.textContent = ""; $("#sub-table").hidden = true; last = null;
  try {
    const db = await listText();
    const stereo = /** @type {HTMLInputElement} */ ($("#sub-stereo")).checked;
    const r = JSON.parse(await ask(JSON.stringify({ query: query.query, db: db.text, stereo, max: 100 })));
    if (me !== run) return null;
    last = { ...r, label: db.label };
    r.hits.slice(0, SHOWN).forEach((/** @type {any} */ h) => list.appendChild(row(h)));
    const shown = r.hits.length > SHOWN ? " " + t("draw.sub.more", { n: SHOWN }) : "";
    const errs = r.errors.length ? " " + t("draw.sub.unread", { n: r.errors.length }) : "";
    status.textContent = (r.hits.length ? t("draw.sub.found", { n: r.hits.length, total: r.total, list: db.label }) : t("draw.sub.none", { total: r.total, list: db.label })) + shown + errs;
    $("#sub-table").hidden = !r.hits.length && !r.errors.length;
    return r;
  } catch (e) {
    if (me === run) status.textContent = t("draw.sub.fail", { message: message(e) });
    return null;
  }
}

/** the name of an entry, or its place in the list when it has none */
const nameOf = (/** @type {any} */ h) => h.name || t("draw.sub.entry", { n: h.i + 1 });

/** one candidate: its name and number of matches, and «Show» that draws it with the matched atoms selected */
function row(/** @type {any} */ h) {
  const li = document.createElement("li");
  li.dataset.i = String(h.i);
  const b = document.createElement("b"); b.textContent = nameOf(h);
  const show = document.createElement("button");
  show.type = "button"; show.className = "sm sub-show"; show.textContent = t("draw.sub.show");
  show.setAttribute("aria-label", t("draw.sub.show.aria", { name: nameOf(h) })); show.title = t("draw.sub.show.title");
  show.onclick = () => { showHit(h).catch(e => { $("#sub-status").textContent = t("draw.sub.fail", { message: message(e) }); }); };
  li.append(b, " " + t("draw.sub.matches", { n: h.n }) + (h.id ? " · " + h.id : ""), show);
  return li;
}

/** draws a candidate (the drawing is kept for «Back») and selects the atoms of its matches and the bonds inside each match */
async function showHit(/** @type {any} */ h) {
  const K = await ketcher();
  if (saved === null) saved = await K.getKet();
  await K.setMolecule(h.structure);
  const st = K.editor.struct(), ids = [...st.atoms.keys()].sort((a, b) => a - b);
  const atoms = new Set(), bonds = /** @type {number[]} */ ([]);
  for (const m of h.maps) for (const a of m) if (ids[a] !== undefined) atoms.add(ids[a]);
  const inMatch = (/** @type {number} */ x, /** @type {number} */ y) => h.maps.some((/** @type {number[]} */ m) => m.some(a => ids[a] === x) && m.some(a => ids[a] === y));
  st.bonds.forEach((/** @type {any} */ b, /** @type {number} */ id) => { if (inMatch(b.begin, b.end)) bonds.push(id); });
  K.editor.selection({ atoms: [...atoms], bonds });
  if (query) await /** @type {any} */ (window).TPDraw.highlight([{ kind: "moiety", query: query.query, label: t("draw.sub.ev.label") }]);
  $("#sub-back").hidden = false;
  $("#sub-status").textContent = t("draw.sub.shown", { name: nameOf(h), n: atoms.size });
}

/** puts back the drawing that was on the canvas before the first «Show» */
async function back() {
  if (saved === null) return;
  const K = await ketcher();
  await K.setMolecule(saved);
  saved = null; $("#sub-back").hidden = true;
  $("#sub-status").textContent = t("draw.sub.restored");
}

/** every hit and unread entry as a real table (TDATI) */
function dataTable() {
  if (!last || !(/** @type {any} */ (window).TDATI)) return;
  const rows = last.hits.map((/** @type {any} */ h) => [String(h.i + 1), nameOf(h), h.id, String(h.n), h.maps.map((/** @type {number[]} */ m) => m.map(a => a + 1).join(" ")).join("; ")]);
  for (const e of last.errors) rows.push([String(e.i + 1), e.name || t("draw.sub.entry", { n: e.i + 1 }), "", "—", e.error]);
  /** @type {any} */ (window).TDATI.table({
    title: t("draw.sub.table.caption", { list: last.label }),
    head: [t("draw.sub.col.n"), t("draw.sub.col.name"), t("draw.sub.col.id"), t("draw.sub.col.matches"), t("draw.sub.col.atoms")],
    rows, note: t("draw.sub.note"),
  });
}

/** takes the query from the menu (the selection or the whole drawing as a molfile) and opens the card */
async function fromMenu(/** @type {any} */ c) {
  const st = c.K.editor.struct(), ids = c.whole ? [...st.atoms.keys()] : [...c.atoms];
  const box = card();
  if (!ids.length) { $("#sub-status").textContent = t("draw.sub.empty"); return; }
  const mf = /** @type {string} */ (molfileOf(st, ids, undefined, () => true));
  query = { query: mf, atoms: ids.length, what: c.whole ? "whole" : "sel" };
  saved = null; $("#sub-back").hidden = true;
  $("#sub-query").textContent = t(c.whole ? "draw.sub.query.whole" : "draw.sub.query.sel", { n: ids.length });
  $("#sub-list").textContent = ""; $("#sub-status").textContent = ""; $("#sub-table").hidden = true;
  document.dispatchEvent(new CustomEvent("mzlab-card-open", { detail: box }));
  box.scrollIntoView({ block: "nearest" });
  $("#sub-run").focus();
}

/** the two rows of the canvas menu: the selection, or the whole drawing when nothing is selected */
export function mountSub() {
  const menu = /** @type {any} */ (window).TPDraw && /** @type {any} */ (window).TPDraw.menu;
  if (!menu) return;
  menu.add({ id: "sub-find-sel", when: "selection", order: 75, label: "draw.sub.ctx.sel", title: "draw.sub.ctx.title", run: fromMenu });
  menu.add({ id: "sub-find-whole", when: "whole", order: 75, label: "draw.sub.ctx.whole", title: "draw.sub.ctx.title", run: fromMenu });
}
