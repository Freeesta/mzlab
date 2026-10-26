// @ts-strict
// «Import my data» (settings, group «Data and privacy»): reads a mzlab-dati-<date>.zip made by esporta-dati.js (the ZIP is read by Rust, CRC checked, unsafe names refused),
// shows what would be overwritten, and only after the user confirms writes it back (localStorage, IndexedDB, saved libraries) and reloads the page. Nothing leaves the computer.
import { lab } from "./qqq-db.js";
import { SECRET, RUNTIME, DEV, OURS, IDB_STORES, work } from "./esporta-dati.js";

/** @type {any} */
const W = window;
/** @param {string} k @param {Record<string, any>} [p] */
const T = (k, p) => W.I18N.t(k, p);
const dec = new TextDecoder();

/** @typedef {{ ls: [string, string][], changed: string[], idb: { store: string, rows: { key: any, value: any }[] }[], libs: { path: string, bytes: Uint8Array }[] }} Plan */

/** The entries of the packed buffer of the worker (u16 name length, u32 data length, name, data). @param {Uint8Array} packed */
function unpack(packed) {
  const dv = new DataView(packed.buffer, packed.byteOffset, packed.byteLength), m = new Map();
  for (let p = 0; p < packed.length;) {
    const nl = dv.getUint16(p, true), dl = dv.getUint32(p + 2, true); p += 6;
    const name = dec.decode(packed.subarray(p, p + nl)); p += nl;
    m.set(name, packed.subarray(p, p + dl)); p += dl;
  }
  return m;
}
/** JSON with the binary values of the export ({ $base64 }) restored. @param {Uint8Array} b */
const parse = b => JSON.parse(dec.decode(b), (_k, x) => x && typeof x === "object" && typeof x.$base64 === "string" ? Uint8Array.from(atob(x.$base64), c => c.charCodeAt(0)) : x);
const allowed = (/** @type {string} */ k) => OURS.test(k) && !SECRET.test(k) && !RUNTIME.test(k) && !DEV.test(k);

/** What the file would restore; null when it is not a file of the export. @param {Map<string, Uint8Array>} e @returns {Plan | null} */
export function plan(e) {
  const man = e.get("manifesto.json");
  try { if (!man || JSON.parse(dec.decode(man)).format !== "mzlab-dati/1") return null; } catch (_) { return null; }
  /** @type {Plan} */ const out = { ls: [], changed: [], idb: [], libs: [] };
  const ls = e.get("localStorage.json");
  if (ls) for (const [k, v] of Object.entries(JSON.parse(dec.decode(ls)))) {
    if (typeof v !== "string" || !allowed(k)) continue;
    out.ls.push([k, v]);
    let now = null; try { now = localStorage.getItem(k); } catch (_) { /* blocked: nothing to compare */ }
    if (now !== null && now !== v) out.changed.push(k);
  }
  for (const store of IDB_STORES) { const b = e.get(`indexeddb/${store}.json`); if (b) out.idb.push({ store, rows: parse(b) }); }
  for (const [n, bytes] of e) if (n.startsWith("opfs/librerie/") && n.length > 14) out.libs.push({ path: n.slice(5), bytes });
  return out;
}

/** Writes the plan. Returns the library files that could not be written (in use elsewhere). @param {Plan} pl */
export async function apply(pl) {
  for (const [k, v] of pl.ls) { try { localStorage.setItem(k, v); } catch (_) { /* blocked */ } }
  if (pl.idb.length) {
    const db = await lab()();
    for (const { store, rows } of pl.idb) {
      if (!db.objectStoreNames.contains(store)) continue;
      await new Promise((res, rej) => {
        const tx = db.transaction(store, "readwrite"), os = tx.objectStore(store);
        for (const r of rows) { if (os.keyPath !== null) os.put(r.value); else os.put(r.value, r.key); }
        tx.oncomplete = () => res(null); tx.onerror = tx.onabort = () => rej(tx.error);
      });
    }
  }
  if (!pl.libs.length) return [];
  const files = pl.libs.map(f => ({ path: f.path, bytes: f.bytes }));
  return /** @type {string[]} */ ((await work({ op: "opfs", files })).failed);
}

/** The lines of the preview. @param {Plan} pl */
function preview(pl) {
  const esc = W.IMPCOMP.esc, rows = [];
  rows.push(`<li>${esc(T("import.ls", { n: pl.ls.length, m: pl.changed.length }))}</li>`);
  if (pl.changed.length) rows.push(`<li><details><summary>${esc(T("import.keys"))}</summary><ul>${pl.changed.map(k => `<li><code>${esc(k)}</code></li>`).join("")}</ul></details></li>`);
  for (const d of pl.idb) rows.push(`<li>${esc(T("import.idb", { store: d.store, n: d.rows.length }))}</li>`);
  if (pl.libs.length) rows.push(`<li>${esc(T("import.lib", { n: pl.libs.length }))}</li>`);
  return `<p><strong>${esc(T("import.preview"))}</strong></p><ul>${rows.join("")}</ul>`
    + `<div class="row"><button type="button" id="uip-import-go">${esc(T("import.go"))}</button><button type="button" id="uip-import-no">${esc(T("import.cancel"))}</button></div>`;
}

/** The chosen file: read it, show the preview and wait for the confirmation. @param {HTMLElement} box @param {File} file */
export async function begin(box, file) {
  const st = /** @type {HTMLElement} */ (box.querySelector("#uip-import-st")), pv = /** @type {HTMLElement} */ (box.querySelector("#uip-import-box"));
  pv.hidden = true; pv.textContent = ""; st.textContent = T("import.busy");
  let pl = null;
  try {
    const buf = await file.arrayBuffer();
    pl = plan(unpack(new Uint8Array((await work({ op: "unzip", zip: buf }, [buf])).packed)));
  } catch (_) { pl = null; }
  if (!pl) { st.textContent = T("import.bad"); return; }
  if (!pl.ls.length && !pl.idb.length && !pl.libs.length) { st.textContent = T("import.nothing"); return; }
  st.textContent = ""; pv.innerHTML = preview(pl); pv.hidden = false;
  const go = /** @type {HTMLButtonElement} */ (pv.querySelector("#uip-import-go")), no = /** @type {HTMLButtonElement} */ (pv.querySelector("#uip-import-no"));
  go.focus();
  no.addEventListener("click", () => { pv.hidden = true; pv.textContent = ""; st.textContent = ""; });
  go.addEventListener("click", async () => {
    go.disabled = no.disabled = true; st.textContent = T("import.busy");
    try {
      const failed = await apply(/** @type {Plan} */ (pl));
      st.textContent = failed.length ? T("import.partial", { n: failed.length }) : T("import.done");
      pv.hidden = true; setTimeout(() => location.reload(), 1500);
    } catch (_) { st.textContent = T("import.err"); go.disabled = no.disabled = false; }
  });
}
