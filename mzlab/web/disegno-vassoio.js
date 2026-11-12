// @ts-check
// «Structure tray» card of Draw: drawn structures kept with a name for the session, reordered or removed, saved as a suspect list (CSV with
// name, smiles, formula, mass, charge, adducts) and read back. Formula, mass, the CSV bytes and the reading are the Rust engine's (`tray` of
// the grafo .wasm, `tray/1`; the app asks `feat/grafo`, the CLI is `mzlab-cli mol-suspects`). A suspect list holds candidates to look for.
import { carica as grafo } from "./disegno-frammenti.js";
import { exportName } from "./disegno-nomi.js";

/** @typedef {{ name: string, smiles: string, formula: string, charge: number, mass: number }} Voce */

const W = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {any} */ p = undefined) => W.I18N.t(k, p);
const KEY = "mzlab.draw.tray";
const fmtF = (/** @type {string} */ f) => (W.fmtFormula ? W.fmtFormula(f) : h`${f}`).replace(/\[(\d+)([A-Z][a-z]?)\]/g, "<sup>$1</sup>$2");
const $ = (/** @type {string} */ id) => /** @type {HTMLElement} */ (document.getElementById(id));

let entries = /** @type {Voce[]} */ ([]), chosen = /** @type {string[] | null} */ (null), adducts = /** @type {any[]} */ ([]), ready = /** @type {Promise<void> | null} */ (null);

const ask = async (/** @type {any} */ req) => (await grafo()).tray(JSON.stringify(req));
const say = (/** @type {string} */ msg) => { $("tray-msg").textContent = msg; };
/** an engine error is the text `{error_key, params}` (or a key); shown in the user's language */
function fail(/** @type {unknown} */ e) {
  let k = String(e && /** @type {any} */ (e).message || e), p = {};
  try { const j = JSON.parse(k); k = j.error_key || j.key || k; p = j.params || {}; } catch (_) { /* a plain key */ }
  say(/^error\./.test(k) ? t(k, p) : t("draw.tray.unavailable"));
}

function save() { try { sessionStorage.setItem(KEY, JSON.stringify({ entries, chosen })); } catch (_) { /* private window: the tray lives until the page closes */ } }
function restore() {
  try { const v = JSON.parse(sessionStorage.getItem(KEY) || "null"); if (v && Array.isArray(v.entries)) { entries = v.entries; chosen = Array.isArray(v.chosen) ? v.chosen : null; } }
  catch (_) { /* nothing kept */ }
}

/** the adducts a list may name, from the engine; the common ones are offered, plus any a loaded list names */
async function init() {
  restore();
  const a = JSON.parse(await ask({ op: "adducts" })).adducts;
  adducts = a;
  if (!chosen) chosen = a.filter((/** @type {any} */ x) => x.on).map((/** @type {any} */ x) => x.name);
  $("tray-body").addEventListener("click", onClick);
  $("tray-body").addEventListener("change", onChange);
}

function adductBox() {
  const shown = adducts.filter(a => a.common || (chosen || []).includes(a.name));
  const box = (/** @type {any} */ a) => h`<label class="tray-ad"><input type="checkbox" data-ad="${a.name}"${raw((chosen || []).includes(a.name) ? " checked" : "")}> ${a.name}</label>`;
  return h`<fieldset class="tray-ads"><legend>${t("draw.tray.adducts")}</legend>${shown.map(a => raw(box(a)))}</fieldset>`;
}

function item(/** @type {Voce} */ e, /** @type {number} */ i) {
  const n = entries.length, name = e.name || t("draw.tray.defaultName", { n: i + 1 });
  const btn = (/** @type {string} */ act, /** @type {string} */ label, /** @type {string} */ aria, /** @type {boolean} */ off, /** @type {string} */ why = "") =>
    h`<button type="button" class="sm" data-act="${act}" data-i="${i}" aria-label="${aria}"${raw(off ? " disabled" : "")}${raw(why ? h` title="${why}"` : "")}>${label}</button>`;
  const tools = btn("up", "↑", t("draw.tray.up.aria", { name }), i === 0) + btn("down", "↓", t("draw.tray.down.aria", { name }), i === n - 1)
    + btn("draw", t("draw.tray.draw"), t("draw.tray.draw.aria", { name }), !e.smiles, e.smiles ? t("draw.tray.draw.title") : t("draw.tray.draw.noSmiles"))
    + btn("del", "✕", t("draw.tray.del.aria", { name }), false);
  const q = e.charge ? ` (${e.charge > 0 ? "+" : "−"}${Math.abs(e.charge) > 1 ? Math.abs(e.charge) : ""})` : "";
  return h`<li><input type="text" class="tray-name" data-i="${i}" value="${name}" aria-label="${t("draw.tray.name.aria", { n: i + 1 })}">`
    + h`<div class="tray-row"><span class="muted">${raw(fmtF(e.formula))}${q} · ${e.mass.toFixed(4)}</span><span class="bar">${raw(tools)}</span></div></li>`;
}

function render() {
  const body = $("tray-body");
  if (!entries.length) { body.innerHTML = ""; return; }
  const list = entries.map((e, i) => raw(item(e, i)));
  body.innerHTML = h`${raw(adductBox())}<ol class="tray-l" aria-label="${t("draw.tray.caption", { n: entries.length })}">${list}</ol>`
    + h`<div class="bar"><button type="button" class="sm" data-act="save" title="${t("draw.tray.save.title")}">${t("draw.tray.save")}</button></div>`;
}

async function add() {
  const all = /** @type {any[]} */ (await W.TPDraw.structures(true)).filter(p => !(p.atoms.length === 1 && p.atoms[0].q));
  if (!all.length) { say(t("draw.tray.empty")); return; }
  const base = entries.length;
  const req = { op: "entries", structures: all.map((p, k) => ({ name: t("draw.tray.defaultName", { n: base + k + 1 }), smiles: p.smiles, atoms: p.atoms })) };
  const got = JSON.parse(await ask(req)).entries;
  const ok = got.filter((/** @type {any} */ e) => !e.error);
  entries.push(...ok);
  const bad = got.find((/** @type {any} */ e) => e.error);
  say(bad ? t(bad.error, { label: bad.label }) : t("draw.tray.added", { n: ok.length }));
}

async function onClick(/** @type {Event} */ ev) {
  const b = /** @type {HTMLElement} */ (ev.target).closest("button[data-act]");
  if (!(b instanceof HTMLButtonElement) || b.disabled) return;
  const i = +(b.dataset.i || 0), act = b.dataset.act;
  try {
    if (act === "save") {
      const csv = await ask({ op: "csv", entries, adducts: chosen });
      W.saveBlob(new Blob([csv], { type: "text/csv" }), exportName("csv"));
      say(t("draw.tray.saved", { n: entries.length })); return;
    }
    if (act === "draw") { await W.TPDraw.load(entries[i].smiles); return; }
    if (act === "del") { const [gone] = entries.splice(i, 1); say(t("draw.tray.removed", { name: gone.name })); }
    if (act === "up" || act === "down") { const j = act === "up" ? i - 1 : i + 1; [entries[i], entries[j]] = [entries[j], entries[i]]; }
    save(); render();
    // the focus stays on the moved row's button (or the next row after a removal)
    const k = act === "up" ? i - 1 : act === "down" ? i + 1 : Math.min(i, entries.length - 1);
    const next = /** @type {HTMLElement | null} */ ($("tray-body").querySelector(`button[data-act="${act === "del" ? "del" : act}"][data-i="${k}"]:not([disabled])`) || $("tray-body").querySelector(`.tray-name[data-i="${k}"]`));
    next?.focus();
  } catch (e) { fail(e); }
}

function onChange(/** @type {Event} */ ev) {
  const el = /** @type {HTMLInputElement} */ (ev.target);
  if (el.classList.contains("tray-name")) entries[+(el.dataset.i || 0)].name = el.value.trim();
  else if (el.dataset.ad) chosen = adducts.map(a => a.name).filter(n => n === el.dataset.ad ? el.checked : (chosen || []).includes(n));
  save();
}

async function onFile(/** @type {Event | undefined} */ ev) {
  const inp = /** @type {HTMLInputElement} */ (ev && ev.target), f = inp && inp.files && inp.files[0];
  if (!f) return;
  inp.value = "";
  try {
    const back = JSON.parse(await ask({ op: "read", text: await f.text() }));
    entries.push(...back.entries);
    if (back.adducts.length) chosen = back.adducts;
    save(); render();
    say(t(back.skipped ? "draw.tray.loadedSkipped" : "draw.tray.loaded", { n: back.entries.length, skipped: back.skipped, file: f.name }));
  } catch (e) { fail(e); }
}

/** the card's actions: "add" the drawing (or the selection), "file" the list chosen in #tray-file (its change event), "" only shows what the
 * session kept; draw.js loads the module then (the file chooser opens in draw.js, on the click itself) */
export async function carica(/** @type {string} */ act = "", /** @type {Event | undefined} */ ev = undefined) {
  say("");
  try {
    ready = ready || init();
    await ready;
    if (act === "add") { await add(); save(); }
    if (act === "file") await onFile(ev);
    render();
  } catch (e) { ready = act ? ready : null; fail(e); }
}
