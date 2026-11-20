// @ts-strict
// @ts-noany
// The window «Molecules» (the «Manage» link of the search box loads this module on the first click and opens it): search the
// database, add a molecule, edit one (a catalogue molecule is edited as a copy of the user, the catalogue stays), delete the ones of the user, import a CSV or JSON
// file (one error per wrong row) and export CSV or JSON. Everything that is computed is computed by the engine (molecules-archive.js); this file is the interface.
// Keyboard: Tab stays inside the window, Esc goes back from the form or closes, and the focus returns to what opened it. window.MOLDB also has the search box (field).
import * as A from "./molecules-archive.js";
import { field } from "./molecules-field.js";
import { fieldsHtml, formTexts, fill, read } from "./molecules-module.js";

/** @typedef {import("./molecules-archive.js").RowError} RowError */

/** @type {any} */
const W = window;
const t = (/** @type {string} */ k, /** @type {any} */ p = undefined) => (W.I18N ? W.I18N.t(k, p) : k);
const STYLE_ID = "mol-style";
const MAX_ERRORS = 200;
const CSS = `#moldlg{position:fixed;inset:0;z-index:6200;display:flex;align-items:center;justify-content:center;padding:16px;background:color-mix(in srgb,var(--ink) 45%,transparent)}
#moldlg[hidden]{display:none}
#moldlg .mol-box{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:10px;width:min(820px,100%);max-height:calc(100vh - 32px);overflow:auto;padding:16px 18px;display:flex;flex-direction:column;gap:12px}
#moldlg h2{margin:0;font-size:1.0625rem}#moldlg h3{margin:0;font-size:1rem}
#moldlg .mol-hd{display:flex;align-items:center;justify-content:space-between;gap:8px}
#moldlg .mol-bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center}
#moldlg .mol-bar #mol-q{flex:1 1 220px;display:flex;flex-direction:column;gap:4px}#moldlg #mol-q label{font-size:.875rem;font-weight:600}
#moldlg #mol-form{display:flex;flex-direction:column;gap:10px}
#moldlg .mol-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:0 14px}#moldlg .mol-wide{grid-column:1/-1}
#moldlg button{min-height:32px;padding:0 12px;border-radius:6px;border:1px solid var(--line);background:var(--panel);color:var(--ink);font:inherit;font-weight:600;cursor:pointer}
#moldlg button.mol-go{background:var(--accent);border-color:var(--accent);color:var(--on-accent)}
#moldlg table{border-collapse:collapse;width:100%;font-size:.875rem}
#moldlg caption{text-align:left;font-weight:600;padding:4px 0}
#moldlg th,#moldlg td{text-align:left;padding:6px 8px;border-bottom:1px solid var(--line);vertical-align:middle}
#moldlg td.mol-n{text-align:right;font-variant-numeric:tabular-nums}
#moldlg td.mol-act{white-space:nowrap;text-align:right}#moldlg td.mol-act button{margin-left:6px;min-height:28px}
#moldlg .mol-fld{display:flex;flex-direction:column;gap:3px;margin-bottom:8px}#moldlg .mol-fld label{font-size:.875rem;font-weight:600}
#moldlg input,#moldlg textarea{font:inherit;color:var(--ink);background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:6px 8px;box-sizing:border-box;width:100%}
#moldlg .mol-hint,#moldlg .mol-note{margin:0;font-size:.8125rem;color:var(--muted)}
#moldlg .mol-st{min-height:1.2em;font-size:.875rem}
#moldlg .avv{margin:0}#moldlg .avv ul{margin:6px 0 0;padding-left:20px}
#moldlg .mol-row-act{display:flex;gap:8px;flex-wrap:wrap}
#moldlg :is(button,a,textarea,input):focus-visible{outline:2px solid var(--accent);outline-offset:2px}
#moldlg [hidden]{display:none}
@media (pointer:coarse){#moldlg button,#moldlg td.mol-act button,#moldlg input{min-height:40px}}`;

/** @type {HTMLElement | null} */ let dlg = null;
/** @type {Element | null} */ let back = null;
/** @type {string | null} */ let editing = null;
let token = 0;

const q = (/** @type {string} */ s) => /** @type {any} */ ((/** @type {HTMLElement} */ (dlg)).querySelector(s));
const status = (/** @type {string} */ msg) => { q("#mol-st").textContent = msg; };
/** The text of a row error: the key comes from the engine (`error.molecules.*`), the detail is the value that was wrong. */
const errorText = (/** @type {RowError} */ e) => `${t(`error.molecules.${e.key.slice("error.molecules.".length)}`)}${e.text ? ` (${e.text})` : ""}`;

function showErrors(/** @type {RowError[]} */ errs, /** @type {string} */ title) {
  const box = q("#mol-errs");
  box.replaceChildren();
  if (!errs.length) { box.hidden = true; return; }
  const p = document.createElement("p"), ul = document.createElement("ul");
  p.textContent = title;
  ul.replaceChildren(...errs.slice(0, MAX_ERRORS).map(e => { const li = document.createElement("li"); li.textContent = (e.line ? t("mol.err.row", { row: e.line }) + " " : "") + errorText(e); return li; }));
  box.append(p, ul);
  if (errs.length > MAX_ERRORS) { const m = document.createElement("p"); m.textContent = t("mol.err.more", { n: errs.length - MAX_ERRORS }); box.append(m); }
  box.hidden = false;
}

function build() {
  if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
  const d = document.createElement("div");
  d.id = "moldlg"; d.hidden = true;
  d.innerHTML = `<div class="mol-box" role="dialog" aria-modal="true" aria-labelledby="mol-h">
<div class="mol-hd"><h2 id="mol-h"></h2><button type="button" id="mol-x"></button></div>
<div id="mol-list">
<div class="mol-bar"><div id="mol-q"><label for="mol-qi"></label><input id="mol-qi" type="search" autocomplete="off" spellcheck="false"></div><button type="button" id="mol-add" class="mol-go"></button><button type="button" id="mol-imp"></button><button type="button" id="mol-csv"></button><button type="button" id="mol-json"></button><input type="file" id="mol-file" accept=".csv,.json,text/csv,application/json" hidden></div>
<div id="mol-errs" class="avv bad" role="alert" hidden></div>
<div id="mol-conf" class="avv warn" role="alertdialog" aria-labelledby="mol-conf-t" hidden><p id="mol-conf-t"></p><div class="mol-row-act"><button type="button" id="mol-conf-ok"></button><button type="button" id="mol-conf-no"></button></div></div>
<table><caption id="mol-cap"></caption><thead><tr><th scope="col" id="mol-c1"></th><th scope="col" id="mol-c2"></th><th scope="col" id="mol-c3"></th><th scope="col" id="mol-c4"></th><th scope="col" id="mol-c5"><span class="sr-only"></span></th></tr></thead><tbody id="mol-body"></tbody></table>
</div>
<form id="mol-form" novalidate hidden><h3 id="mol-fh"></h3><p class="mol-note" id="mol-fn" hidden></p>${fieldsHtml(t)}
<div id="mol-ferr" class="avv bad" role="alert" hidden></div>
<div class="mol-row-act"><button type="submit" id="mol-save" class="mol-go"></button><button type="button" id="mol-cancel"></button></div></form>
<div class="mol-st" id="mol-st" role="status" aria-live="polite"></div></div>`;
  document.body.appendChild(d);
  dlg = d;
  d.addEventListener("mousedown", ev => { if (ev.target === d) close(); });
  d.addEventListener("keydown", ev => {
    if (ev.key === "Escape") { ev.preventDefault(); ev.stopPropagation(); if (!q("#mol-form").hidden) backToList(); else if (!q("#mol-conf").hidden) closeConfirm(); else close(); return; }
    if (ev.key !== "Tab") return;
    const f = /** @type {HTMLElement[]} */ ([...d.querySelectorAll("textarea,input:not([hidden]),button,a[href]")]).filter(e => !e.hasAttribute("disabled") && !e.closest("[hidden]") && e.getAttribute("aria-hidden") !== "true");
    const first = f[0], last = f[f.length - 1], cur = document.activeElement;
    if (ev.shiftKey && cur === first) { ev.preventDefault(); last.focus(); } else if (!ev.shiftKey && cur === last) { ev.preventDefault(); first.focus(); }
  });
  q("#mol-q input").addEventListener("input", () => { const run = ++token; setTimeout(() => { if (run === token) list(); }, 200); });
  q("#mol-x").onclick = close;
  q("#mol-add").onclick = () => openForm(null);
  q("#mol-imp").onclick = () => q("#mol-file").click();
  q("#mol-file").onchange = importPicked;
  q("#mol-csv").onclick = () => download("csv");
  q("#mol-json").onclick = () => download("json");
  q("#mol-cancel").onclick = backToList;
  q("#mol-conf-no").onclick = closeConfirm;
  q("#mol-form").onsubmit = (/** @type {Event} */ ev) => { ev.preventDefault(); saveForm(); };
  return d;
}

function texts() {
  q("#mol-h").textContent = t("mol.title");
  q("#mol-x").textContent = t("mol.close");
  q("#mol-add").textContent = t("mol.add");
  q("#mol-imp").textContent = t("mol.import");
  q("#mol-csv").textContent = t("mol.export.csv");
  q("#mol-json").textContent = t("mol.export.json");
  q("#mol-save").textContent = t("mol.save");
  q("#mol-cancel").textContent = t("mol.cancel");
  q("#mol-conf-ok").textContent = t("mol.delete.yes");
  q("#mol-conf-no").textContent = t("mol.cancel");
  [["#mol-c1", "mol.col.name"], ["#mol-c2", "mol.col.formula"], ["#mol-c3", "mol.col.mass"], ["#mol-c4", "mol.col.level"]].forEach(([s, k]) => { q(s).textContent = t(k); });
  q("#mol-c5 span").textContent = t("mol.col.actions");
  formTexts(q("#mol-form"), t);
  q("#mol-q label").textContent = t("mol.search.label");
  q("#mol-q input").placeholder = t("mol.search.placeholder");
}

function row(/** @type {{ id: string, name: string, formula: string, mass?: number }} */ m) {
  const tr = document.createElement("tr"), lvl = A.level(m.id);
  const cell = (/** @type {string} */ txt, /** @type {string} */ cls = "") => { const td = document.createElement("td"); td.textContent = txt; if (cls) td.className = cls; return td; };
  const name = document.createElement("th");
  name.scope = "row"; name.textContent = m.name;
  const act = document.createElement("td");
  act.className = "mol-act";
  const button = (/** @type {string} */ text, /** @type {string} */ aria, /** @type {() => void} */ fn) => { const b = document.createElement("button"); b.type = "button"; b.textContent = text; b.setAttribute("aria-label", aria); b.onclick = fn; act.appendChild(b); };
  button(t("mol.edit"), t("mol.edit.aria", { name: m.name }), () => openForm(m.id));
  if (lvl !== "catalog") button(t("mol.delete"), t("mol.delete.aria", { name: m.name }), () => askDelete(m.id, m.name));
  tr.append(name, cell(m.formula), cell(m.mass ? m.mass.toFixed(4) : "", "mol-n"), cell(t(`mol.level.${lvl}`)), act);
  return tr;
}

/** The table: the molecules of the user, or the hits of the search when something is typed (the search is the one of the engine). */
async function list() {
  const run = ++token, tx = q("#mol-q input") ? q("#mol-q input").value.trim() : "";
  /** @type {{ id: string, name: string, formula: string, mass?: number }[]} */
  let lines;
  try { lines = tx.length >= 2 ? await A.search(tx, 50) : A.myMolecules().sort((a, b) => a.name.localeCompare(b.name)); } catch (_) { status(t("mol.err.load")); return; }
  if (run !== token) return;
  q("#mol-cap").textContent = tx.length >= 2 ? t("mol.cap.results", { n: lines.length }) : t("mol.cap.mine", { n: lines.length });
  q("#mol-body").replaceChildren(...lines.map(row));
  q("#mol-body").dataset.q = tx;
  status(lines.length ? "" : t(tx.length >= 2 ? "mol.search.none" : "mol.empty.mine"));
}

function view(/** @type {"list" | "form"} */ v) {
  q("#mol-list").hidden = v !== "list";
  q("#mol-form").hidden = v !== "form";
}

function backToList() {
  view("list");
  editing = null;
  const f = q("#mol-q input");
  if (f) f.focus();
  list();
}

async function openForm(/** @type {string | null} */ id) {
  const f = q("#mol-form");
  editing = id;
  const base = id ? await A.getMolecule(id) : null;
  fill(f, base);
  q("#mol-fh").textContent = base ? t("mol.form.edit", { name: base.name }) : t("mol.form.add");
  const note = q("#mol-fn");
  note.hidden = !(base && base.origin.type === "catalog");
  note.textContent = t("mol.form.copyNote");
  q("#mol-ferr").hidden = true;
  view("form");
  q("#mol-f-name").focus();
}

/** Runs a writer of the archive; a failure of the store (not an error of a row) is one message. */
async function guarded(/** @type {() => Promise<void>} */ fn) {
  try { await fn(); } catch (_) { status(t("mol.err.save")); }
}

async function saveForm() {
  await guarded(saveFormNow);
}

async function saveFormNow() {
  const parsed = read(q("#mol-form"), editing ? await A.getMolecule(editing) : null);
  const errBox = q("#mol-ferr");
  const show = (/** @type {RowError[]} */ errs) => { errBox.replaceChildren(...errs.map(e => { const p = document.createElement("p"); p.textContent = errorText(e); return p; })); errBox.hidden = false; };
  if ("errors" in parsed) return show(parsed.errors);
  const r = await A.save(parsed.rec, editing || "");
  if ("errors" in r) return show(r.errors);
  if ("exists" in r) return show([{ line: 0, key: "error.molecules.dupId", text: r.exists.name }]);
  const name = parsed.rec.name || "";
  showErrors([], "");   // a save writes the whole archive again: what the old archive had wrong is gone
  backToList();
  status(t("mol.saved", { name: name }));
}

function closeConfirm() {
  q("#mol-conf").hidden = true;
  q("#mol-conf-ok").onclick = null;
  const f = q("#mol-q input");
  if (f) f.focus();
}

function askDelete(/** @type {string} */ id, /** @type {string} */ name) {
  q("#mol-conf-t").textContent = t("mol.delete.confirm", { name: name }) + " " + t("mol.delete.catalogBack");
  q("#mol-conf").hidden = false;
  q("#mol-conf-ok").onclick = () => guarded(async () => {
    const r = await A.remove(id);
    closeConfirm();
    if ("errors" in r) showErrors(/** @type {RowError[]} */ (r.errors), t("mol.err.save"));
    else { await list(); status(t("mol.deleted", { name: name })); }
  });
  q("#mol-conf-no").focus();
}

function importPicked() {
  return guarded(importPickedNow);
}

async function importPickedNow() {
  const input = /** @type {HTMLInputElement} */ (q("#mol-file")), file = input.files && input.files[0];
  input.value = "";
  if (!file) return;
  showErrors([], "");
  const r = await A.importFile(file.name, await file.text());
  if ("errors" in r) { showErrors(r.errors, t("mol.import.errors", { n: r.errors.length })); status(""); return; }
  await list();
  status(t("mol.import.done", { n: r.n, replaced: r.replaced, kept: r.kept }));
}

async function download(/** @type {"csv" | "json"} */ format) {
  const f = await A.exportAs(format);
  saveBlob(new Blob([f.text], { type: f.type }), f.name, 1000);
  status(t("mol.exported", { file: f.name }));
}

export async function open() {
  back = document.activeElement;
  dlg = dlg || build();
  texts();
  view("list");
  showErrors([], "");
  dlg.hidden = false;
  status(t("mol.loading"));
  try { await A.ready(); } catch (_) { status(t("mol.err.load")); return; }
  const broken = A.errors();
  if (broken.length) showErrors(broken, t("mol.err.archive"));
  await list();
  if (!broken.length && !q("#mol-st").textContent) status("");
  q("#mol-q input").focus();
}

export function close() {
  if (!dlg || dlg.hidden) return;
  dlg.hidden = true;
  if (back instanceof HTMLElement && back.isConnected) back.focus();
}

W.MOLDB = { open, close, field, search: A.search };
