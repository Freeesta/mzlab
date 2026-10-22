// @ts-check
// Export of the cleaned spectra of the «Fragmentation graph» (CSV, MGF, MSP; one node or the whole tree; clean or all) and «Add to library»: one msnlib/1 entry
// appended to the library file of the user, kept in the browser (OPFS) and downloadable. The text of every export and the library are made by the engine.
import { G, params, query } from "./infusione-grafo-stato.js";

const LIB_FILE = "msnlib.json";
const MIME = { csv: "text/csv", mgf: "text/plain", msp: "text/plain" };

/** The text of an export from the engine. @returns {Promise<string>} */
export async function exportText(/** @type {number} */ k, /** @type {"csv" | "mgf" | "msp"} */ fmt, /** @type {boolean} */ clean, /** @type {string} */ node) {
  const r = await fetch(`api/msn_grafo/export?formato=${fmt}&pulito=${clean ? 1 : 0}&nodo=${encodeURIComponent(node)}&${query(k)}`);
  const t = await r.text();
  if (t.startsWith("{") && /^\{\s*"error"/.test(t)) throw I18N.err(JSON.parse(t));
  return t;
}

const baseName = (/** @type {any} */ f) => String((f && f.name) || "infusion").replace(/\.[^.]+$/, "");

/** Downloads (or copies) one export. */
export async function runExport(/** @type {any} */ file, /** @type {"csv" | "mgf" | "msp"} */ fmt, /** @type {boolean} */ clean, /** @type {string} */ node, /** @type {boolean} */ copy) {
  try {
    const text = await exportText(file.k, fmt, clean, node);
    if (copy) { await navigator.clipboard.writeText(text); nearMsg(I18N.t("msngrafo.copied")); return; }
    await dl(`${baseName(file)}_${node === "tutto" ? "tree" : node.replace(/\W+/g, "_")}${clean ? "" : "_all"}.${fmt}`, text, MIME[fmt]);
  } catch (e) { nearMsg(/** @type {any} */ (e).message || String(e)); }
}

/** @returns {Promise<string>} the library file of the user ("" when there is none) */
export async function readLibrary() {
  try {
    const root = await navigator.storage.getDirectory(), fh = await root.getFileHandle(LIB_FILE);
    return await (await fh.getFile()).text();
  } catch (_) { return ""; }
}

async function writeLibrary(/** @type {string} */ text) {
  const root = await navigator.storage.getDirectory(), fh = await root.getFileHandle(LIB_FILE, { create: true });
  const w = await fh.createWritable(); await w.write(text); await w.close();
}

/** Appends the entry of the molecule, adduct and polarity to the library; says so when the same one is already there (the entry is not added twice). */
export async function addToLibrary(/** @type {any} */ file) {
  const mol = G.mol;
  try {
    const body = { libreria: await readLibrary(), molecola_id: mol ? mol.id : "", ...params(file.k) };
    const r = await fetch("api/msnlib/aggiungi", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
    const j = await r.json();
    if (j.error) throw I18N.err(j);
    if (j.doppione) { nearMsg(I18N.t("msngrafo.lib.dup")); return; }
    await writeLibrary(j.libreria);
    nearMsg(I18N.t("msngrafo.lib.added"));
  } catch (e) { nearMsg(/** @type {any} */ (e).message || String(e)); }
}

/** Downloads the library file of the user. */
export async function downloadLibrary() {
  const t = await readLibrary();
  if (!t) return nearMsg(I18N.t("msngrafo.lib.empty"));
  await dl("msnlib.json", t, "application/json", false);
}

/** The export row: scope (node or tree), clean or all, format, then Download | Copy; and the library buttons. */
export function exportRow(/** @type {HTMLElement} */ host, /** @type {any} */ file, /** @type {string | null} */ node) {
  const opt = (/** @type {string} */ v, /** @type {string} */ t, /** @type {boolean} */ sel = false) => raw(h`<option value="${v}"${raw(sel ? " selected" : "")}>${t}</option>`);
  host.innerHTML = h`<div class="row ig-export"><label>${I18N.t("msngrafo.scope")} <select id="ig-scope">${opt("tutto", I18N.t("msngrafo.scope.tree"))}${node ? opt(node, I18N.t("msngrafo.scope.node"), true) : ""}</select></label>` +
    h`<label>${I18N.t("msngrafo.spectra")} <select id="ig-clean">${opt("1", I18N.t("msngrafo.clean"))}${opt("0", I18N.t("msngrafo.all"))}</select></label>` +
    h`<label>${I18N.t("msngrafo.format")} <select id="ig-fmt">${opt("csv", "CSV")}${opt("mgf", "MGF")}${opt("msp", "MSP")}</select></label>` +
    h`<button type="button" class="bt" data-a="dl" title="${I18N.t("msngrafo.export.tip")}">${I18N.t("msngrafo.export")}</button><button type="button" class="bt" data-a="copy" title="${I18N.t("msngrafo.copy.tip")}">${I18N.t("msngrafo.copy")}</button>` +
    h`<button type="button" class="bt" id="ig-lib" title="${I18N.t("msngrafo.lib.tip")}">${I18N.t("msngrafo.lib")}</button><button type="button" class="bt" id="ig-libdl">${I18N.t("msngrafo.lib.dl")}</button></div>`;
  const val = (/** @type {string} */ id) => /** @type {HTMLSelectElement} */ (host.querySelector(id)).value;
  host.querySelectorAll("button[data-a]").forEach(b => b.addEventListener("click", () => {
    runExport(file, /** @type {any} */ (val("#ig-fmt")), val("#ig-clean") === "1", val("#ig-scope"), /** @type {HTMLElement} */ (b).dataset.a === "copy");
  }));
  host.querySelector("#ig-lib")?.addEventListener("click", () => addToLibrary(file));
  host.querySelector("#ig-libdl")?.addEventListener("click", () => downloadLibrary());
}
