// @ts-check
// The window «Molecules» (the «Manage» link of the search box loads this module on the first click and opens it): search the
// database, add a molecule, edit one (a catalogue molecule is edited as a copy of the user, the catalogue stays), delete the ones of the user, import a CSV or JSON
// file (one error per wrong row) and export CSV or JSON. Everything that is computed is computed by the engine (molecole-archivio.js); this file is the interface.
// Keyboard: Tab stays inside the window, Esc goes back from the form or closes, and the focus returns to what opened it. window.MOLDB also has the search box (campo).
import * as A from "./molecole-archivio.js";
import { campo } from "./molecole-campo.js";
import { campiHtml, testiModulo, riempi, leggi } from "./molecole-modulo.js";

/** @typedef {import("./molecole-archivio.js").ErroreRiga} ErroreRiga */

/** @type {any} */
const W = window;
const t = (/** @type {string} */ k, /** @type {any} */ p) => (W.I18N ? W.I18N.t(k, p) : k);
const STYLE_ID = "mol-style";
const MAX_ERRORI = 200;
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
/** @type {string | null} */ let inModifica = null;
let token = 0;

const q = (/** @type {string} */ s) => /** @type {any} */ ((/** @type {HTMLElement} */ (dlg)).querySelector(s));
const stato = (/** @type {string} */ msg) => { q("#mol-st").textContent = msg; };
/** The text of a row error: the key comes from the engine (`error.molecules.*`), the detail is the value that was wrong. */
const testoErrore = (/** @type {ErroreRiga} */ e) => `${t(`error.molecules.${e.chiave.slice("error.molecules.".length)}`)}${e.testo ? ` (${e.testo})` : ""}`;

function mostraErrori(/** @type {ErroreRiga[]} */ errs, /** @type {string} */ titolo) {
  const box = q("#mol-errs");
  box.replaceChildren();
  if (!errs.length) { box.hidden = true; return; }
  const p = document.createElement("p"), ul = document.createElement("ul");
  p.textContent = titolo;
  ul.replaceChildren(...errs.slice(0, MAX_ERRORI).map(e => { const li = document.createElement("li"); li.textContent = (e.riga ? t("mol.err.row", { riga: e.riga }) + " " : "") + testoErrore(e); return li; }));
  box.append(p, ul);
  if (errs.length > MAX_ERRORI) { const m = document.createElement("p"); m.textContent = t("mol.err.more", { n: errs.length - MAX_ERRORI }); box.append(m); }
  box.hidden = false;
}

function costruisci() {
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
<form id="mol-form" novalidate hidden><h3 id="mol-fh"></h3><p class="mol-note" id="mol-fn" hidden></p>${campiHtml(t)}
<div id="mol-ferr" class="avv bad" role="alert" hidden></div>
<div class="mol-row-act"><button type="submit" id="mol-save" class="mol-go"></button><button type="button" id="mol-cancel"></button></div></form>
<div class="mol-st" id="mol-st" role="status" aria-live="polite"></div></div>`;
  document.body.appendChild(d);
  dlg = d;
  d.addEventListener("mousedown", ev => { if (ev.target === d) chiudi(); });
  d.addEventListener("keydown", ev => {
    if (ev.key === "Escape") { ev.preventDefault(); ev.stopPropagation(); if (!q("#mol-form").hidden) tornaElenco(); else if (!q("#mol-conf").hidden) chiudiConferma(); else chiudi(); return; }
    if (ev.key !== "Tab") return;
    const f = /** @type {HTMLElement[]} */ ([...d.querySelectorAll("textarea,input:not([hidden]),button,a[href]")]).filter(e => !e.hasAttribute("disabled") && !e.closest("[hidden]") && e.getAttribute("aria-hidden") !== "true");
    const first = f[0], last = f[f.length - 1], cur = document.activeElement;
    if (ev.shiftKey && cur === first) { ev.preventDefault(); last.focus(); } else if (!ev.shiftKey && cur === last) { ev.preventDefault(); first.focus(); }
  });
  q("#mol-q input").addEventListener("input", () => { const mio = ++token; setTimeout(() => { if (mio === token) elenco(); }, 200); });
  q("#mol-x").onclick = chiudi;
  q("#mol-add").onclick = () => apriModulo(null);
  q("#mol-imp").onclick = () => q("#mol-file").click();
  q("#mol-file").onchange = importaFile;
  q("#mol-csv").onclick = () => scarica("csv");
  q("#mol-json").onclick = () => scarica("json");
  q("#mol-cancel").onclick = tornaElenco;
  q("#mol-conf-no").onclick = chiudiConferma;
  q("#mol-form").onsubmit = ev => { ev.preventDefault(); salvaModulo(); };
  return d;
}

function testi() {
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
  testiModulo(q("#mol-form"), t);
  q("#mol-q label").textContent = t("mol.search.label");
  q("#mol-q input").placeholder = t("mol.search.placeholder");
}

function riga(/** @type {{ id: string, nome: string, formula: string, massa?: number }} */ m) {
  const tr = document.createElement("tr"), liv = A.livello(m.id);
  const cella = (/** @type {string} */ txt, /** @type {string} */ cls = "") => { const td = document.createElement("td"); td.textContent = txt; if (cls) td.className = cls; return td; };
  const nome = document.createElement("th");
  nome.scope = "row"; nome.textContent = m.nome;
  const act = document.createElement("td");
  act.className = "mol-act";
  const bottone = (/** @type {string} */ testo, /** @type {string} */ aria, /** @type {() => void} */ fn) => { const b = document.createElement("button"); b.type = "button"; b.textContent = testo; b.setAttribute("aria-label", aria); b.onclick = fn; act.appendChild(b); };
  bottone(t("mol.edit"), t("mol.edit.aria", { name: m.nome }), () => apriModulo(m.id));
  if (liv !== "catalogo") bottone(t("mol.delete"), t("mol.delete.aria", { name: m.nome }), () => chiediElimina(m.id, m.nome));
  tr.append(nome, cella(m.formula), cella(m.massa ? m.massa.toFixed(4) : "", "mol-n"), cella(t(`mol.level.${liv}`)), act);
  return tr;
}

/** The table: the molecules of the user, or the hits of the search when something is typed (the search is the one of the engine). */
async function elenco() {
  const mio = ++token, tx = q("#mol-q input") ? q("#mol-q input").value.trim() : "";
  /** @type {{ id: string, nome: string, formula: string, massa?: number }[]} */
  let righe;
  try { righe = tx.length >= 2 ? await A.cerca(tx, 50) : A.mieMolecole().sort((a, b) => a.nome.localeCompare(b.nome)); } catch (_) { stato(t("mol.err.load")); return; }
  if (mio !== token) return;
  q("#mol-cap").textContent = tx.length >= 2 ? t("mol.cap.results", { n: righe.length }) : t("mol.cap.mine", { n: righe.length });
  q("#mol-body").replaceChildren(...righe.map(riga));
  q("#mol-body").dataset.q = tx;
  stato(righe.length ? "" : t(tx.length >= 2 ? "mol.search.none" : "mol.empty.mine"));
}

function vista(/** @type {"elenco" | "modulo"} */ v) {
  q("#mol-list").hidden = v !== "elenco";
  q("#mol-form").hidden = v !== "modulo";
}

function tornaElenco() {
  vista("elenco");
  inModifica = null;
  const f = q("#mol-q input");
  if (f) f.focus();
  elenco();
}

async function apriModulo(/** @type {string | null} */ id) {
  const f = q("#mol-form");
  inModifica = id;
  const base = id ? await A.prendi(id) : null;
  riempi(f, base);
  q("#mol-fh").textContent = base ? t("mol.form.edit", { name: base.nome }) : t("mol.form.add");
  const nota = q("#mol-fn");
  nota.hidden = !(base && base.origine.tipo === "catalogo");
  nota.textContent = t("mol.form.copyNote");
  q("#mol-ferr").hidden = true;
  vista("modulo");
  q("#mol-f-nome").focus();
}

/** Runs a writer of the archive; a failure of the store (not an error of a row) is one message. */
async function protetto(/** @type {() => Promise<void>} */ fn) {
  try { await fn(); } catch (_) { stato(t("mol.err.save")); }
}

async function salvaModulo() {
  await protetto(salvaModuloOra);
}

async function salvaModuloOra() {
  const lettura = leggi(q("#mol-form"), inModifica ? await A.prendi(inModifica) : null);
  const errBox = q("#mol-ferr");
  const mostra = (/** @type {ErroreRiga[]} */ errs) => { errBox.replaceChildren(...errs.map(e => { const p = document.createElement("p"); p.textContent = testoErrore(e); return p; })); errBox.hidden = false; };
  if ("errori" in lettura) return mostra(lettura.errori);
  const r = await A.salva(lettura.rec, inModifica || "");
  if ("errori" in r) return mostra(r.errori);
  if ("esiste" in r) return mostra([{ riga: 0, chiave: "error.molecules.dupId", testo: r.esiste.nome }]);
  const nome = lettura.rec.nome || "";
  tornaElenco();
  stato(t("mol.saved", { name: nome }));
}

function chiudiConferma() {
  q("#mol-conf").hidden = true;
  q("#mol-conf-ok").onclick = null;
  const f = q("#mol-q input");
  if (f) f.focus();
}

function chiediElimina(/** @type {string} */ id, /** @type {string} */ nome) {
  q("#mol-conf-t").textContent = t("mol.delete.confirm", { name: nome }) + " " + t("mol.delete.catalogBack");
  q("#mol-conf").hidden = false;
  q("#mol-conf-ok").onclick = () => protetto(async () => {
    const r = await A.elimina(id);
    chiudiConferma();
    if ("errori" in r) mostraErrori(r.errori, t("mol.err.save"));
    else { await elenco(); stato(t("mol.deleted", { name: nome })); }
  });
  q("#mol-conf-no").focus();
}

function importaFile() {
  return protetto(importaFileOra);
}

async function importaFileOra() {
  const input = /** @type {HTMLInputElement} */ (q("#mol-file")), file = input.files && input.files[0];
  input.value = "";
  if (!file) return;
  mostraErrori([], "");
  const r = await A.importa(file.name, await file.text());
  if ("errori" in r) { mostraErrori(r.errori, t("mol.import.errors", { n: r.errori.length })); stato(""); return; }
  await elenco();
  stato(t("mol.import.done", { n: r.n, replaced: r.sostituite, kept: r.tenute }));
}

async function scarica(/** @type {"csv" | "json"} */ formato) {
  const f = await A.esporta(formato);
  const url = URL.createObjectURL(new Blob([f.testo], { type: f.tipo }));
  const a = document.createElement("a");
  a.href = url; a.download = f.nome;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  stato(t("mol.exported", { file: f.nome }));
}

export async function apri() {
  back = document.activeElement;
  dlg = dlg || costruisci();
  testi();
  vista("elenco");
  mostraErrori([], "");
  dlg.hidden = false;
  stato(t("mol.loading"));
  try { await A.pronto(); } catch (_) { stato(t("mol.err.load")); return; }
  const rotti = A.errori();
  if (rotti.length) mostraErrori(rotti, t("mol.err.archive"));
  await elenco();
  if (!rotti.length && !q("#mol-st").textContent) stato("");
  q("#mol-q input").focus();
}

export function chiudi() {
  if (!dlg || dlg.hidden) return;
  dlg.hidden = true;
  if (back instanceof HTMLElement && back.isConnected) back.focus();
}

W.MOLDB = { apri, chiudi, campo, cerca: A.cerca };
