// @ts-check
// Archive of the browser version: the files the student opened and their .mzidx indexes live in OPFS (the origin private file system),
// outside the memory of the page. The Rust engine's worker (rust-worker.js) and the index builders use this module, so they are the only places that hold
// OPFS handles; each is opened for one write and closed at once. Reading goes through File objects (getFile).
// Folders: files/ (the mzML), idx/ ("<name of the mzML>.mzidx"). No OPFS (an old Safari, a private window that refuses it): available() is false and the
// page says that the browser is too old (browser.js).
import { infoDi, nuovo } from "./manifesto.js";
const ROOT = "mzlab";
const STEP = 8 << 20;        // bytes written at a time (a Blob is read slice by slice, never whole)
let rootP = null;
// a function told of every change of the folders ("put" | "del", kind, name, size): the manifest of the files (migra.js) follows through it
let onChange = null;
export const watch = f => { onChange = f; };
const told = (ev, kind, name, size) => {
  if (onChange) { try { Promise.resolve(onChange(ev, kind, name, size)).catch(() => {}); } catch (_) { /* optional */ } }
  try { channel().postMessage({ t: "file", ev, kind, name, size }); } catch (_) { /* optional */ }
};

// the other tabs hear of every change (BroadcastChannel "mzlab": { t: "file", ev, kind, name, size }); `listen(f)` is called for the ones that come from elsewhere
let bc = null;
const channel = () => bc || (bc = new BroadcastChannel("mzlab"));
export const listen = f => { try { channel().addEventListener("message", e => f(e.data)); } catch (_) { /* no BroadcastChannel */ } };

// one tab at a time writes a file (Web Locks "mzlab-file:<kind>/<name>"): a lock held for more than WAIT_MS is not waited for silently, it is an error with a key
const WAIT_MS = 2000;
const fail = (key, params) => JSON.stringify({ error_key: key, params });          // thrown as a string: the worker answers it as an error of the engine (a key, not a fault)
export async function locked(kind, name, fn) {
  const L = self.navigator && navigator.locks;
  if (!L) return fn();
  const key = "mzlab-file:" + kind + "/" + name, ctl = new AbortController(), timer = setTimeout(() => ctl.abort(), WAIT_MS);
  try {
    return await L.request(key, { mode: "exclusive", signal: ctl.signal }, async () => { clearTimeout(timer); return fn(); });
  } catch (e) {
    if (e && e.name === "AbortError") throw fail("error.fileInUse", { file: name });
    throw e;
  } finally { clearTimeout(timer); }
}

// storage: persist() once (the first file); before a big write the room is checked (sources + their index take about 1.8 times the size)
let asked = false;
export async function persistOnce() {
  if (asked) return;
  asked = true;
  try { if (navigator.storage && navigator.storage.persist && !(await navigator.storage.persisted())) await navigator.storage.persist(); } catch (_) { /* optional */ }
}
export const BIG = 50 << 20, ROOM = 1.8;
const held = new Map();                                     // room promised to the files being loaded (file + index), not yet written: several files loaded together do not count the same free space twice
export const release = key => { held.delete(key); };
export async function ensureRoom(bytes, key = "") {
  if (bytes < BIG) return;
  const e = await estimate();
  if (!e.quota) return;                                      // unknown: not refused
  let promised = 0; for (const [k, v] of held) if (k !== key) promised += v;
  const need = bytes * ROOM, free = e.quota - e.usage - promised;
  if (need > free) throw fail("error.quota", { serve_mb: Math.ceil(need / 1048576), libero_mb: Math.max(0, Math.floor(free / 1048576)) });
  if (key) held.set(key, need);
}

export const available = () => {
  try { return !!(self.navigator && navigator.storage && /** @type {any} */ (navigator.storage).getDirectory && typeof FileSystemFileHandle !== "undefined" && /** @type {any} */ (FileSystemFileHandle.prototype).createSyncAccessHandle); }
  catch (_) { return false; }
};

const root = () => rootP || (rootP = navigator.storage.getDirectory().then(d => d.getDirectoryHandle(ROOT, { create: true })).catch(e => { rootP = null; throw e; }));
const dir = async kind => (await root()).getDirectoryHandle(kind, { create: true });

// Writes all of `data` at `at` into a sync handle. A full disk (or the quota of the origin) does not always throw: write() can return fewer bytes than asked, and a file silently
// cut short is unreadable later ("this file cannot be read"). A short write or a QuotaExceededError is the error.quota of the page, with the room needed and left.
export function writeFull(ah, data, at) {                  // bytes really written (less than data.length: the disk or the quota is full)
  let done = 0;
  try { while (done < data.length) { const n = ah.write(data.subarray(done), { at: at + done }); if (!(n > 0)) break; done += n; } }
  catch (e) { if (!(e && e.name === "QuotaExceededError")) throw e; }
  return done;
}
// the error.quota of the page (as a string, like the other errors of the engine): `needed` bytes in all, the room left asked to the browser
export async function quotaError(needed) {
  const e = await estimate(), free = e && e.quota ? Math.max(0, e.quota - e.usage) : 0;
  return fail("error.quota", { serve_mb: Math.ceil(needed / 1048576), libero_mb: Math.floor(free / 1048576) });
}
export async function writeAll(ah, data, at) { if (writeFull(ah, data, at) < data.length) throw await quotaError(at + data.length); }

// write `size` bytes produced by chunk(offset, length) -> Uint8Array into kind/name (replacing it); the partial file is removed if something fails
function write(kind, name, size, chunk, after) { return locked(kind, name, () => writeNow(kind, name, size, chunk, after)); }
async function writeNow(kind, name, size, chunk, after) {
  await dropLease(kind, name);
  const d = await dir(kind);
  const fh = await d.getFileHandle(name, { create: true });
  const ah = await fh.createSyncAccessHandle();
  try {
    ah.truncate(0);
    for (let off = 0; off < size; off += STEP) { const b = await chunk(off, Math.min(STEP, size - off)); await writeAll(ah, b, off); if (after) await after(b, off); }
    ah.flush();
    if (ah.getSize() < size) throw await quotaError(size);        // the browser took the writes but the file is shorter (quota reached)
  } catch (e) {
    try { ah.close(); } catch (_) { /* already closed */ }
    try { await d.removeEntry(name); } catch (_) { /* nothing to remove */ }
    throw e;
  }
  ah.close();
  const f = await fh.getFile();
  told("put", kind, name, f.size);
  return f;
}

// `size` bytes produced by chunk(offset, length) -> Promise<Uint8Array> -> File in OPFS (the desktop copies the index built by the native side)
// `after(piece, offset)`, when given, is awaited once the piece is written (the page takes the fingerprints of the file there: impronta.js); the piece is its to keep
export const putChunks = (kind, name, size, chunk, after) => write(kind, name, size, chunk, after);

// a Blob, an ArrayBuffer or a Uint8Array -> File in OPFS
export async function put(kind, name, data) {
  if (data instanceof Blob) return write(kind, name, data.size, async (o, n) => new Uint8Array(await data.slice(o, o + n).arrayBuffer()));
  const u8 = data instanceof Uint8Array ? data : new Uint8Array(data);
  return write(kind, name, u8.length, async (o, n) => u8.subarray(o, o + n));
}

// a file written under a temporary name (by a worker of the pool) takes its final name; a half-written file never has it
export async function promote(kind, from, to) {
  await dropLease(kind, from); await dropLease(kind, to);
  const d = await dir(kind), fh = await d.getFileHandle(from);
  if (kind === "idx" || kind === "feat") {                    // a file of a newer program (another tab, a later version of the page) is never replaced by an older one
    const old = await d.getFileHandle(to).then(h => h.getFile(), () => null), info = old && await infoDi(kind, old).catch(() => null);
    if (nuovo(kind, info)) { try { await d.removeEntry(from); } catch (_) { /* optional */ } throw fail("error.newerFormat", { file: to, version: info.versione_formato }); }
  }
  try { await d.removeEntry(to); } catch (_) { /* nothing to replace */ }
  if (typeof fh.move === "function") { await fh.move(to); const f = await (await d.getFileHandle(to)).getFile(); told("put", kind, to, f.size); return f; }
  const f = await put(kind, to, await fh.getFile());          // no move() in this browser: a copy
  try { await d.removeEntry(from); } catch (_) { /* optional */ }
  return f;
}

// ---- read leases: the worker of the engine reads an index in use through a SyncAccessHandle (direct, no copy through the browser process, which a File read has).
// The handle is exclusive, so it is closed after LEASE_IDLE ms without reads and whenever the worker has no request in flight (parkAll; another tab can write the file), before this worker writes, moves or removes the file,
// and reopened by wake() at the next request. Without a handle (taken, or not supported) leaseRead() answers -1 and the caller reads the File as before.
const LEASE_IDLE = 2000, leases = new Map();
const lkey = (kind, name) => kind + "/" + name;
function park(l) { clearTimeout(l.timer); l.timer = 0; if (l.ah) { try { l.ah.close(); } catch (_) { /* already closed */ } l.ah = null; } }
function arm(l) { l.timer = setTimeout(() => { const rest = l.last + LEASE_IDLE - performance.now(); if (rest > 0) { arm(l); } else park(l); }, LEASE_IDLE); }
async function openLease(l) {
  if (l.ah || l.open) return l.open;
  l.open = (async () => {
    try {
      const ah = await (await (await dir(l.kind)).getFileHandle(l.name)).createSyncAccessHandle();
      if (l.dead) { try { ah.close(); } catch (_) { /* already closed */ } return; }          // dropped while it was opening: never keep a handle nobody owns
      l.ah = ah; l.last = performance.now(); arm(l);
    } catch (_) { l.ah = null; }
    finally { l.open = null; }
  })();
  return l.open;
}
// the file is about to be written, moved or removed by this worker: the handle is closed first (an opening in flight is waited for)
export async function dropLease(kind, name) { const k = lkey(kind, name), l = leases.get(k); if (!l) return; leases.delete(k); l.dead = true; await l.open; park(l); }
export async function lease(kind, name) {
  if (!available()) return false;
  const k = lkey(kind, name);
  if (!leases.has(k)) leases.set(k, { kind, name, ah: null, timer: 0, last: 0, open: null, dead: false });
  const l = leases.get(k); await openLease(l); return !!l.ah;
}
export async function parkAll() { for (const l of leases.values()) { await l.open; park(l); } }
export async function wake() { await Promise.all([...leases.values()].map(l => (l.ah ? (l.last = performance.now(), 0) : openLease(l)))); }
/** bytes read into `buf` at `at` (the number read), or -1 when there is no handle */
export function leaseRead(kind, name, buf, at) {
  const l = leases.get(lkey(kind, name));
  if (!l || !l.ah) return -1;
  try { l.last = performance.now(); return l.ah.read(buf, { at }); } catch (_) { return -1; }
}

export async function get(kind, name) {
  try { return await (await (await dir(kind)).getFileHandle(name)).getFile(); } catch (_) { return null; }
}

export async function list(kind) {
  const out = [], d = await dir(kind);
  for await (const [name, h] of d.entries()) {
    if (h.kind !== "file") continue;
    try { const f = await h.getFile(); out.push({ name, size: f.size, lastModified: f.lastModified }); } catch (_) { /* vanished meanwhile */ }
  }
  return out;
}

export async function remove(kind, name) {
  await dropLease(kind, name);
  try { await (await dir(kind)).removeEntry(name); told("del", kind, name, 0); return true; } catch (_) { return false; }
}

export async function clear(kind) {
  for (const e of await list(kind)) await remove(kind, e.name);
}

export async function estimate() {
  try { const e = await navigator.storage.estimate(); return { usage: e.usage || 0, quota: e.quota || 0 }; } catch (_) { return { usage: 0, quota: 0 }; }
}
