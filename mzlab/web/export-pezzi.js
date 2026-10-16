// @ts-check
// Exports written piece by piece (docs/agenti/dati-esporta.md). ES module; only EXPORT_PEZZI goes on window.
// A source is a string, a Uint8Array, a Blob or an (async) iterable of those: the pieces go to export-worker.js, which writes them into OPFS (mzlab/export/),
// and the download is the File read from there, so the page never holds the whole text. Same bytes as `new Blob(pieces)` (strings are UTF-8).
// A short export, or one without OPFS or Worker, is saved from memory with saveBlob.
/** @typedef {string | Uint8Array | Blob} Piece */
const FLUSH = 1 << 20;                 // characters/bytes gathered before a message to the worker
export const MIN_STREAM = 8 << 20;     // a plain string or Blob below this size is saved the old way
const KEEP_MS = 10 * 60e3;             // the file is removed this long after the download starts
const OLD_MS = 3600e3;                 // at start, what an interrupted export left (older than an hour) is removed
/** @type {Worker | null} */
let worker = null;
let seq = 0;
/** @type {Map<string, { ok: (m: any) => void, ko: (e: Error) => void }>} */
const waiting = new Map();
export const available = () => !!(typeof Worker !== "undefined" && navigator.storage && navigator.storage.getDirectory);
function start() {
  if (worker) return worker;
  worker = new Worker("static/export-worker.js", { type: "module" });
  worker.onmessage = ev => {
    const m = ev.data, w = waiting.get(m.id); if (!w) return;
    waiting.delete(m.id);
    if (m.op === "fail") w.ko(Object.assign(new Error(m.message), { name: m.name || "Error" })); else w.ok(m);
  };
  worker.onerror = () => { for (const w of waiting.values()) w.ko(new Error("export worker")); waiting.clear(); worker = null; };
  return worker;
}
/** @param {any} msg @returns {Promise<any>} */
const ask = msg => new Promise((ok, ko) => { waiting.set(msg.id, { ok, ko }); start().postMessage(msg); });
const isIter = (/** @type {any} */ s) => s != null && typeof s !== "string" && !(s instanceof Blob) && !(s instanceof Uint8Array) && (Symbol.iterator in Object(s) || Symbol.asyncIterator in Object(s));

/** @param {string | Uint8Array | Blob | Iterable<Piece> | AsyncIterable<Piece>} source @returns {AsyncGenerator<Piece>} */
async function* pieces(source) {
  if (isIter(source)) yield* /** @type {any} */ (source); else yield /** @type {Piece} */ (source);
}
/** Writes the source into OPFS and returns the File and a `drop()` that removes it. @param {Parameters<typeof pieces>[0]} source */
export async function write(source) {
  const id = "x" + Date.now().toString(36) + "-" + (++seq) + "-" + Math.random().toString(36).slice(2, 8);
  await ask({ op: "open", id });
  let held = "", n = 0;
  const send = async (/** @type {Piece} */ data) => { await ask({ op: "put", id, data }); n++; };
  try {
    for await (const p of pieces(source)) {
      if (typeof p === "string") {
        held += p;
        if (held.length >= FLUSH) {                 // a pair of UTF-16 halves is never cut between two messages
          const cut = /[\ud800-\udbff]$/.test(held) ? held.length - 1 : held.length;
          await send(held.slice(0, cut)); held = held.slice(cut);
        }
      } else {
        if (held) { await send(held); held = ""; }
        await send(p);
      }
    }
    if (held) await send(held);
    await ask({ op: "end", id });
  } catch (e) { await ask({ op: "abort", id }).catch(() => {}); throw e; }
  const root = await navigator.storage.getDirectory();
  const file = await (await (await root.getDirectoryHandle("mzlab")).getDirectoryHandle("export")).getFileHandle(id).then(h => h.getFile());
  return { file, pieces: n, drop: () => ask({ op: "drop", id }).catch(() => {}) };
}
/** Pieces collected in memory (the way back without OPFS) @param {Parameters<typeof pieces>[0]} source @returns {Promise<Piece[]>} */
export async function parts(source) {
  const out = [];
  for await (const p of pieces(source)) out.push(p);
  return out;
}
/** Saves the source as a download named `name`; true when it went through OPFS, false when it was small (or OPFS is missing) and was saved from memory. @param {string} name @param {Parameters<typeof pieces>[0]} source @param {string} type */
export async function save(name, source, type) {
  const save1 = /** @type {any} */ (window).saveBlob, it = pieces(source), got = /** @type {Piece[]} */ ([]);
  let size = 0, done = false;
  while (size < MIN_STREAM) {                          // a short export never needs OPFS
    const r = await it.next();
    if (r.done) { done = true; break; }
    got.push(r.value); size += typeof r.value === "string" ? r.value.length : r.value instanceof Blob ? r.value.size : r.value.byteLength;
  }
  if (done || !available()) { for (;;) { const r = await it.next(); if (r.done) break; got.push(r.value); } save1(new Blob(/** @type {BlobPart[]} */ (got), { type }), name); return false; }
  const r = await write((async function* () { yield* got; yield* it; })());
  save1(r.file, name, KEEP_MS);                        // the File read from OPFS: the browser streams it from disk
  setTimeout(r.drop, KEEP_MS);
  return true;
}
/** removes what an interrupted export left, once at start */
export function clean() { if (available()) { start().postMessage({ op: "clean", olderMs: OLD_MS }); } }
const api = { available, write, parts, save, clean, MIN_STREAM, isIter };
/** @type {any} */ (window).EXPORT_PEZZI = api;
setTimeout(clean, 3000);
