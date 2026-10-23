// @ts-check
// Opening a saved fragmentation graph (a g.json, msngrafo/1) without its raw data: a dropped or picked .json is recognised by its head, a graph without `vista`
// is completed by the .wasm msngrafo (op `completa`; no calculation here) and shown read-only in a window with the same body as the «Fragmentation graph» tab.
// Used by shell.js (upload0, the data area) and infusione-grafo.js (the tab). docs/agenti/infusione-grafo.md.
import { G } from "./infusione-grafo-stato.js";
import { mostraErrore } from "./infusione-grafo-avanza.js";

const HEAD = /"formato"\s*:\s*"msngrafo\/(\d+)/;
const KNOWN = 1;
/** The graph window opens as the head of the file says, but a JSON this large is not a graph the page can draw. */
const MAX_BYTES = 200e6;

/** The version of msngrafo/N a file says in its first bytes, or 0 when it is not a graph. @param {File} f */
export async function version(f) {
  if (!/\.json$/i.test(f.name) || f.size > MAX_BYTES) return 0;
  const m = HEAD.exec(await f.slice(0, 4096).text());
  return m ? +m[1] : 0;
}

/** @param {string} key @param {Record<string, any>} [params] */
const fail = (key, params = {}) => I18N.err({ error_key: key, params });

/**
 * The graph of a file as an object, with `vista` (a graph without it goes through the .wasm); a keyed error when it is broken, newer than this page or cannot be completed.
 * @param {File} f @param {number} v the version `version(f)` found
 */
async function read(f, v) {
  if (v > KNOWN) throw fail("error.newerFormat", { kind: "msngrafo", version: v, file: f.name });
  let text = await f.text(), j;
  try { j = JSON.parse(text); } catch (e) { throw fail("error.msngrafo.badFile", { file: f.name, raw: String(/** @type {Error} */ (e).message) }); }
  if (!j || j.formato !== "msngrafo/1" || !Array.isArray(j.nodi) || !Array.isArray(j.archi)) throw fail("error.msngrafo.badFile", { file: f.name, raw: "not a msngrafo/1 graph" });
  if (!j.vista) {
    try { text = await (await import("./rust-msngrafo.js")).msnCompleta(text); } catch (e) {
      let o = null; try { o = JSON.parse(String(e)); } catch (_) { /* a plain error of the page */ }
      throw o && o.error_key ? fail(o.error_key, o.params) : fail("error.msngrafo.badFile", { file: f.name, raw: String(e && /** @type {Error} */ (e).message || e) });
    }
    j = JSON.parse(text);
  }
  return j;
}

/** @type {HTMLDialogElement | null} */ let win = null;

/** Shows the graph `j` of the file `name` in a window; the state of the tab is put back when it closes. @param {any} j @param {string} name */
async function show(j, name) {
  await carica("m:infusione-grafo.js");
  if (win) win.remove();
  const keep = { ...G }, opener = /** @type {HTMLElement | null} */ (document.activeElement);
  Object.assign(G, { ro: name, j, sel: null, error: null, errorObj: null, rf: false, key: "" });
  const d = document.createElement("dialog"); d.id = "ig-ro"; d.className = "ig-ro"; d.setAttribute("aria-labelledby", "ig-ro-h");
  d.innerHTML = h`<div class="td-hd"><h3 id="ig-ro-h">${I18N.t("msngrafo.file.title", { name })}</h3><button type="button" class="bt" data-x="1" aria-label="${I18N.t("msngrafo.file.close")}" title="${I18N.t("msngrafo.file.close")}">&times;</button></div><div id="ig-ro-body"></div>`;
  document.body.appendChild(d);
  win = d;
  const close = () => { d.close(); d.remove(); if (win === d) win = null; Object.assign(G, keep); if (opener && document.contains(opener)) opener.focus(); };
  d.querySelector("[data-x]")?.addEventListener("click", close);
  d.addEventListener("cancel", e => { e.preventDefault(); close(); });
  d.addEventListener("click", e => { if (e.target instanceof Element && e.target.closest('[data-a="draw"]')) setTimeout(close); });   // Draw opens another view: the window gets out of the way
  dropZone(d);
  d.showModal();
  await INFG.renderRo(/** @type {HTMLElement} */ (d.querySelector("#ig-ro-body")), name);
  /** @type {HTMLElement | null} */ (d.querySelector("[data-x]"))?.focus();
}

/**
 * Opens the graph file `f`; throws the keyed error when it is not readable.
 * @param {File} f @param {number} [v] the version already found by `version(f)`
 */
export async function open(f, v) {
  const ver = v || await version(f);
  if (!ver) throw fail("error.msngrafo.badFile", { file: f.name, raw: "not a msngrafo/1 graph" });
  await show(await read(f, ver), f.name);
}

/**
 * Takes the graph files out of the files of a loading and opens the last one; the other files go on to the usual loading. An error of a graph file goes to the report of the loading.
 * @param {File[]} files @param {{ errs: Record<string, string[]>, diag?: Record<string, any> }} rep @returns {Promise<File[]>}
 */
export async function take(files, rep) {
  const rest = [], graphs = /** @type {[File, number][]} */ ([]);
  for (const f of files) { const v = await version(f); if (v) graphs.push([f, v]); else rest.push(f); }
  for (const [f, v] of graphs) {
    try { await show(await read(f, v), f.name); } catch (e) {
      const err = /** @type {any} */ (e);
      (rep.errs[err.message] = rep.errs[err.message] || []).push(f.name);
      if (!rep.diag) rep.diag = {};
      if (!rep.diag[err.message]) rep.diag[err.message] = DIAG.da(err, { name: f.name, size: f.size, format: "json" }, "");
    }
  }
  return rest;
}

/** Drag and drop of graph files on `host` (the tab or the window): a keyed error is written in `#ig-msg` of the host. @param {HTMLElement} host */
export function dropZone(host) {
  host.addEventListener("dragover", e => { if (e.dataTransfer && [...e.dataTransfer.types].includes("Files")) e.preventDefault(); });
  host.addEventListener("drop", e => {
    const list = [...(e.dataTransfer?.files || [])];
    if (!list.length) return;
    e.preventDefault(); e.stopPropagation();
    openFiles(host, list);
  });
}

/** Opens the first graph among `list`; an error is shown in `#ig-open-msg` of `host` with «Details» and «Copy diagnosis». @param {HTMLElement} host @param {File[]} list */
async function openFiles(host, list) {
  const msg = /** @type {HTMLElement | null} */ (host.querySelector("#ig-open-msg"));
  if (msg) msg.textContent = "";
  for (const f of list) {
    const v = await version(f);
    try { if (!v) throw fail("error.msngrafo.badFile", { file: f.name, raw: "not a msngrafo/1 graph" }); await show(await read(f, v), f.name); return; } catch (e) {
      const err = /** @type {any} */ (e);
      if (msg) mostraErrore(msg, err.message, err, { name: f.name, size: f.size }, "load"); else nearMsg(err.message);
      return;
    }
  }
}

/** The button «Open a graph file…» of the tab (a file picker: the keyboard way to what a drop does) and the line for its errors. @param {HTMLElement} host */
export function openRow(host) {
  host.innerHTML = h`<div class="row"><button type="button" class="bt" id="ig-open" title="${I18N.t("msngrafo.file.open.tip")}">${I18N.t("msngrafo.file.open")}</button><input type="file" id="ig-open-in" accept=".json,application/json" hidden></div><div id="ig-open-msg" aria-live="polite"></div>`;
  const input = /** @type {HTMLInputElement} */ (host.querySelector("#ig-open-in"));
  host.querySelector("#ig-open")?.addEventListener("click", () => input.click());
  input.addEventListener("change", () => { openFiles(host, [...(input.files || [])]); input.value = ""; });
}

window.INFGAPRI = { take, open, openRow, dropZone, version };
