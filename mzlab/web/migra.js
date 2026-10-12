// Migrations of the archive (docs/agenti/memoria.md): numbered steps, each idempotent and repeatable (an interrupted step is done again from the start),
// run once at start inside an exclusive Web Lock so that two tabs never migrate together, before the engine and the pools open any file.
// `kv.schema` (IndexedDB "qqq_lab") says which step the data are at; SCHEMA (qqq-db.js) is where they must be.
import { SCHEMA } from "./qqq-db.js";

const KINDS = { files: "sorgente", idx: "indice", feat: "pipeline" };
export const FOLDERS = Object.keys(KINDS);

/** the manifest row of a file of OPFS (what is not known yet is null) */
export const rowOf = (kind, name, size, time, info) => ({ path: kind + "/" + name, tipo: KINDS[kind] || kind, bytes: size, creato: time || Date.now(), usato: time || Date.now(),
  formato: (/\.([a-z]+)$/i.exec(name) || [])[1] || null, versione_formato: null, crate: null, sorgente_sha256: null, ...(info || {}) });

// io: { getSchema(), setSchema(n), list(kind) -> [{ name, size, lastModified }], putManifest(row) }
export const STEPS = {
  // v3: the manifest store exists (qqq-db.js upgrade) and holds a row for every file already in OPFS
  async 3(io) { for (const kind of FOLDERS) for (const e of await io.list(kind)) await io.putManifest(rowOf(kind, e.name, e.size, e.lastModified)); },
};

/** brings the data from the schema they are at to `target`, one step at a time; returns the schema reached */
export async function runSteps(io, steps = STEPS, target = SCHEMA) {
  let n = (await io.getSchema()) || 2;                  // data without `kv.schema` are the ones of IndexedDB v2
  while (n < target) {
    if (!steps[n + 1]) throw new Error("no migration step " + (n + 1));
    await steps[n + 1](io);
    await io.setSchema(++n);
  }
  return n;
}

/** the same, inside the exclusive lock "mzlab-migra" (no Web Locks: straight, the steps are idempotent anyway) */
export function migra(io, steps = STEPS, target = SCHEMA) {
  const L = typeof navigator !== "undefined" && navigator.locks;
  return L ? L.request("mzlab-migra", { mode: "exclusive" }, () => runSteps(io, steps, target)) : runSteps(io, steps, target);
}
