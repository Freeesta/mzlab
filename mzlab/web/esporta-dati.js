// @ts-strict
// «Export all my data» (settings, group «Data and privacy»): gathers what the user made in THIS browser (settings and views in localStorage, notebook, lists
// and molecules in IndexedDB, saved libraries in OPFS), adds manifesto.json (what is in, what is left out and why) and downloads one mzlab-dati-<date>.zip.
// The ZIP is written by Rust (esporta-dati-worker.js); nothing leaves the computer. Format and inventory: docs/agenti/esporta-dati.md.

import { lab } from "./qqq-db.js";

/** @type {any} */
const W = window;
const FORMAT = "mzlab-dati/1";
/** @param {string} k @param {Record<string, any>} [p] */
const T = (k, p) => W.I18N.t(k, p);
const enc = new TextEncoder();

// localStorage keys the export never takes, with the reason written in the manifest
export const SECRET = /token|key|secret|passw/i;
export const RUNTIME = /^(mzlab|qqq)\.(run|avvii|migrato|memProbe|memBudget(\.log)?|memLevel|kprefetch|sessione)$/;
export const DEV = /^(mzlab|qqq)\.(admin|dev\..*)$/;
export const OURS = /^(mzlab|qqq)[.-]/;
// IndexedDB stores worth keeping; the others are rebuilt (manifest of OPFS) or are file copies
export const IDB_STORES = ["notebook", "kv", "liste_utente", "molecole"];

/** @typedef {{ name: string, bytes: Uint8Array, what: string }} Entry */
/** @typedef {{ item: string, reason: string }} Left */

/** JSON that survives binary values (typed arrays become base64) and drops what cannot be written (blobs, functions). @param {unknown} v */
function jsonSafe(v) {
  return JSON.stringify(v, (_k, x) => {
    if (x instanceof ArrayBuffer) x = new Uint8Array(x);
    if (ArrayBuffer.isView(x)) {
      const u = new Uint8Array(x.buffer, x.byteOffset, x.byteLength); let s = "";
      for (let i = 0; i < u.length; i += 0x8000) s += String.fromCharCode(...u.subarray(i, i + 0x8000));
      return { $base64: btoa(s) };
    }
    if (typeof Blob !== "undefined" && x instanceof Blob) return { $omitted: "blob" };
    return x;
  }, 1);
}
const text = (/** @type {string} */ s) => enc.encode(s);

/** localStorage: every mzlab/qqq key except the secrets, the run state and the developer flags (a legacy qqq.* key only when no mzlab.* one replaces it). */
function localKeys() {
  /** @type {Record<string, string>} */ const kept = {};
  /** @type {Left[]} */ const left = [];
  /** @type {string[]} */ const names = [];
  try { for (let i = 0; i < localStorage.length; i++) { const k = localStorage.key(i); if (k && OURS.test(k)) names.push(k); } } catch (_) { return { kept, left: [{ item: "localStorage", reason: "blocked" }] }; }
  const have = new Set(names);
  for (const k of names.sort()) {
    if (SECRET.test(k)) { left.push({ item: k, reason: "secret" }); continue; }
    if (RUNTIME.test(k)) { left.push({ item: k, reason: "run state" }); continue; }
    if (DEV.test(k)) { left.push({ item: k, reason: "developer flag" }); continue; }
    if (k.startsWith("qqq") && have.has("mzlab" + k.slice(3))) { left.push({ item: k, reason: "legacy copy" }); continue; }
    try { const v = localStorage.getItem(k); if (v !== null) kept[k] = v; } catch (_) { left.push({ item: k, reason: "unreadable" }); }
  }
  return { kept, left };
}

/** @param {IDBDatabase} db @param {string} store */
const allOf = (db, store) => new Promise(res => {
  const tx = db.transaction(store, "readonly"), os = tx.objectStore(store), out = /** @type {{ key: any, value: any }[]} */ ([]);
  const cur = os.openCursor();
  cur.onsuccess = () => { const c = cur.result; if (c) { out.push({ key: c.key, value: c.value }); c.continue(); } else res(out); };
  cur.onerror = () => res(null);
});

/** The records of the stores of mzlab_lab, one JSON per store. @param {Entry[]} files @param {Left[]} left */
async function indexed(files, left) {
  if (!W.indexedDB) return;
  try {
    const db = await lab()();                          // the one connection of the app (qqq-db.js): the only file that opens the database
    for (const s of IDB_STORES) {
      if (!db.objectStoreNames.contains(s)) continue;
      const rows = await allOf(db, s);
      if (rows === null) { left.push({ item: `indexeddb/${s}`, reason: "unreadable" }); continue; }
      if (rows.length) files.push({ name: `indexeddb/${s}.json`, bytes: text(jsonSafe(rows)), what: `IndexedDB mzlab_lab/${s}` });
    }
    for (const s of ["files", "manifest"]) if (db.objectStoreNames.contains(s)) left.push({ item: `indexeddb/${s}`, reason: s === "files" ? "copies of the loaded files" : "rebuilt from the files" });
  } catch (_) { left.push({ item: "indexeddb", reason: "unreadable" }); }
}

/** The saved libraries (OPFS `librerie/`), the only OPFS folder that holds something the user chose to keep. @param {Entry[]} files @param {Left[]} left */
async function libraries(files, left) {
  try {
    if (!(navigator.storage && /** @type {any} */ (navigator.storage).getDirectory)) return;
    const root = /** @type {any} */ (await navigator.storage.getDirectory());
    for await (const [n] of root.entries()) if (n !== "librerie") left.push({ item: `opfs/${n}`, reason: "loaded files and indexes: rebuilt from the original files" });
    /** @type {any} */ let dir; try { dir = await root.getDirectoryHandle("librerie"); } catch (_) { return; }
    /** @param {any} d @param {string} prefix */
    const walk = async (d, prefix) => {
      for await (const [n, h] of d.entries()) {
        if (h.kind === "directory") { await walk(h, `${prefix}${n}/`); continue; }
        if (/\.tmp$/.test(n)) continue;
        files.push({ name: `opfs/librerie/${prefix}${n}`, bytes: new Uint8Array(await (await h.getFile()).arrayBuffer()), what: "saved library" });
      }
    };
    await walk(dir, "");
  } catch (_) { left.push({ item: "opfs/librerie", reason: "unreadable" }); }
}

/** Everything to export: the entries (without the manifest) and what is left out. */
export async function collect() {
  /** @type {Entry[]} */ const files = [];
  const { kept, left } = localKeys();
  if (Object.keys(kept).length) files.push({ name: "localStorage.json", bytes: text(JSON.stringify(kept, null, 1)), what: "settings and views (localStorage)" });
  await indexed(files, left);
  await libraries(files, left);
  left.push({ item: "sessionStorage", reason: "state of this run only" }, { item: "cache of the app and service worker", reason: "offline copy of the program" });
  return { files, left };
}

/** manifesto.json as bytes. @param {Entry[]} files @param {Left[]} left @param {Date} when */
export function manifest(files, left, when) {
  return text(JSON.stringify({
    format: FORMAT, app: W.APP_NAME || "", version: W.MZLAB_VERSION || null, created: when.toISOString(),
    files: files.map(f => ({ name: f.name, bytes: f.bytes.length, what: f.what })), excluded: left,
  }, null, 1));
}

/** The packed buffer of esporta-dati-worker.js: per entry u16 name length, u32 data length (little endian), name, data. @param {{ name: string, bytes: Uint8Array }[]} entries */
export function pack(entries) {
  const names = entries.map(e => enc.encode(e.name));
  const total = entries.reduce((s, e, i) => s + 6 + names[i].length + e.bytes.length, 0), out = new Uint8Array(total), dv = new DataView(out.buffer);
  let p = 0;
  entries.forEach((e, i) => { dv.setUint16(p, names[i].length, true); dv.setUint32(p + 2, e.bytes.length, true); p += 6; out.set(names[i], p); p += names[i].length; out.set(e.bytes, p); p += e.bytes.length; });
  return out;
}

/** One request to the worker of the ZIP (esporta-dati-worker.js); the buffers in `transfer` are handed over. @param {Record<string, any>} msg @param {ArrayBuffer[]} [transfer] @returns {Promise<any>} */
export const work = (msg, transfer = []) => new Promise((res, rej) => {
  const w = new Worker(new URL("esporta-dati-worker.js", import.meta.url), { type: "module" });
  w.onmessage = ev => { w.terminate(); ev.data.error ? rej(new Error(ev.data.error)) : res(ev.data); };
  w.onerror = ev => { w.terminate(); rej(new Error(ev.message || "worker")); };
  w.postMessage({ id: 1, ...msg }, transfer);
});

/** @param {Uint8Array} packed @returns {Promise<Uint8Array>} */
const zipIt = async packed => {
  const buf = /** @type {ArrayBuffer} */ (packed.buffer.slice(packed.byteOffset, packed.byteOffset + packed.byteLength));
  return new Uint8Array((await work({ op: "zip", packed: buf }, [buf])).zip);
};

/** The export, end to end: collect, pack, download. Returns the archive name and the number of entries. */
export async function run() {
  const when = new Date(), { files, left } = await collect();
  const all = [{ name: "manifesto.json", bytes: manifest(files, left, when) }, ...files];
  const zip = await zipIt(pack(all));
  const name = `mzlab-dati-${when.toISOString().slice(0, 10)}.zip`;
  const url = URL.createObjectURL(new Blob([/** @type {BlobPart} */ (zip)], { type: "application/zip" }));
  const a = document.createElement("a"); a.href = url; a.download = name; document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
  return { name, n: all.length };
}

/** The click of the button: runs the export and says what happened in the status line of the group (a polite live region). @param {HTMLElement} box */
export async function start(box) {
  const btn = /** @type {HTMLButtonElement | null} */ (box.querySelector("#uip-export")), st = box.querySelector("#uip-export-st");
  if (!btn || !st || btn.disabled) return;
  btn.disabled = true; st.textContent = T("export.busy");
  try { const r = await run(); st.textContent = T("export.done", { name: r.name, n: r.n }); }
  catch (_) { st.textContent = T("export.err"); }
  btn.disabled = false;
}
