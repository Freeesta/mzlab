// @ts-strict
// @ts-noany
// Contaminant chooser of the Quick draw box (Disegno): lists the built-in contaminants (Keller 2008) whose structure is KNOWN and puts the SMILES of the chosen one in the SMILES field.
// A structure is known only when the molecules database has a record with the same formula AND the same name (or a synonym); nothing is guessed from the formula alone.
import { cerca, prendi } from "./molecole-archivio.js";

/** @typedef {{ id: string, name: string, formula: string, smiles: string }} Known */
/** @type {Promise<{ known: Known[], total: number }> | null} */
let loaded = null;

/** Letters and digits only, lower case: "N-Methyl 2-pyrrolidone" and "n methyl2pyrrolidone" are the same name. */
const norm = (/** @type {string} */ s) => s.toLowerCase().normalize("NFKD").replace(/[^a-z0-9]/g, "");

/** The names a contaminant answers to: the whole name, the name without its closing parenthesis, and each "/" part of both (e.g. "diphenyl ketone (benzophenone)"). */
export function nameVariants(/** @type {string} */ name) {
  const out = new Set([name]);
  const m = /^(.*?)\s*\(([^()]*)\)\s*$/.exec(name);
  if (m) { out.add(m[1]); m[2].split("/").forEach(p => out.add(p)); m[1].split(" / ").forEach(p => out.add(p)); }
  name.split(" / ").forEach(p => out.add(p));
  return [...out].map(norm).filter(Boolean);
}

/** The contaminants of the built-in list, one per entry (not per ion), with the structure found for each; entries that stand for a series or for isomers have none. */
async function load() {
  const r = await fetch("api/contaminants");
  if (!r.ok) throw new Error("api/contaminants: " + r.status);
  const items = /** @type {any[]} */ ((await r.json()).lists?.flatMap((/** @type {any} */ l) => l.items || []) || []);
  /** @type {Map<string, any>} */
  const entries = new Map();
  for (const it of items) if (!entries.has(it.id)) entries.set(it.id, it);
  /** @type {Known[]} */
  const known = [];
  for (const it of entries.values()) {
    const en = String(it.name_en || it.name || "");
    if (it.series || /isomer/i.test(en) || /isomer/i.test(String(it.name || "")) || !it.formula || /x\d/.test(it.formula)) continue;
    const want = new Set(nameVariants(en));
    let found = null;
    for (const hit of await cerca(it.formula, 25, 0)) {
      if (hit.formula !== it.formula || !hit.smiles) continue;
      const rec = await prendi(hit.id);
      const names = [hit.nome, ...(rec?.sinonimi || [])].map(norm);
      if (names.some(n => want.has(n))) { found = hit; break; }
    }
    if (found && found.smiles) known.push({ id: it.id, name: en, formula: it.formula, smiles: found.smiles });
  }
  known.sort((a, b) => a.name.localeCompare(b.name));
  return { known, total: entries.size };
}

/** The contaminants with a known structure and how many entries the list has; an empty answer when the list or the database cannot be read. */
export function contaminants() {
  loaded = loaded || load().catch(() => { loaded = null; return { known: [], total: 0 }; });   // a failure is not remembered: the next call tries again
  return loaded;
}

/** Fills `select` with the contaminants that have a structure; choosing one calls `onPick(smiles)` and stays shown (the caller puts the prompt back after drawing).
 * @param {HTMLSelectElement} select
 * @param {(key: string) => string} t translator
 * @param {(smiles: string) => void} onPick */
export async function mountContaminants(select, t, onPick) {
  const { known } = await contaminants();
  select.textContent = "";
  const first = document.createElement("option");
  first.value = ""; first.textContent = known.length ? t("draw.contam.prompt") : t("draw.contam.none");
  select.append(first);
  for (const k of known) {
    const o = document.createElement("option");
    o.value = k.smiles; o.textContent = k.name + " · " + k.formula;
    select.append(o);
  }
  select.disabled = !known.length;
  select.onchange = () => { if (select.value) onPick(select.value); };
}

let started = false;
/** Once, when the Disegno tab is first shown: Enter in the SMILES field draws it (as the button does) and the chooser is filled a moment later, off the first paint.
 * Choosing a contaminant only puts its SMILES in the field.
 * @param {(key: string) => string} t translator */
export function initContaminants(t) {
  if (started) return;
  started = true;
  const smi = /** @type {HTMLInputElement} */ (document.getElementById("ex-smi")), sel = /** @type {HTMLSelectElement} */ (document.getElementById("ex-cont"));
  smi.addEventListener("keydown", e => { if (e.key === "Enter") document.getElementById("ex-load")?.click(); });
  setTimeout(() => mountContaminants(sel, t, s => { smi.value = s; }).catch(() => { started = false; }), 300);
}
