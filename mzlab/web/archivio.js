// Archive of the browser version: the files the student opened and their .mzidx indexes live in OPFS (the origin private file system),
// outside the memory of the page. Only browser-worker.js (the worker of the engine) uses this module, so it is the only place that holds
// OPFS handles; each is opened for one write and closed at once. Reading goes through File objects (getFile), which Pyodide mounts with WORKERFS.
// Folders: files/ (the mzML), idx/ ("<name of the mzML>.mzidx"). No OPFS (an old Safari, a private window that refuses it): available() is false and the
// program keeps the files in the memory of the engine and in IndexedDB, as before.
const ROOT = "mzlab";
const STEP = 8 << 20;        // bytes written at a time (a Blob is read slice by slice, never whole)
let rootP = null;

export const available = () => {
  try { return !!(self.navigator && navigator.storage && navigator.storage.getDirectory && typeof FileSystemFileHandle !== "undefined" && FileSystemFileHandle.prototype.createSyncAccessHandle); }
  catch (_) { return false; }
};

const root = () => rootP || (rootP = navigator.storage.getDirectory().then(d => d.getDirectoryHandle(ROOT, { create: true })).catch(e => { rootP = null; throw e; }));
const dir = async kind => (await root()).getDirectoryHandle(kind, { create: true });

// write `size` bytes produced by chunk(offset, length) -> Uint8Array into kind/name (replacing it); the partial file is removed if something fails
async function write(kind, name, size, chunk) {
  const d = await dir(kind);
  const fh = await d.getFileHandle(name, { create: true });
  const ah = await fh.createSyncAccessHandle();
  try {
    ah.truncate(0);
    for (let off = 0; off < size; off += STEP) ah.write(await chunk(off, Math.min(STEP, size - off)), { at: off });
    ah.flush();
  } catch (e) {
    try { ah.close(); } catch (_) { /* already closed */ }
    try { await d.removeEntry(name); } catch (_) { /* nothing to remove */ }
    throw e;
  }
  ah.close();
  return fh.getFile();
}

// a Blob, an ArrayBuffer or a Uint8Array -> File in OPFS
export async function put(kind, name, data) {
  if (data instanceof Blob) return write(kind, name, data.size, async (o, n) => new Uint8Array(await data.slice(o, o + n).arrayBuffer()));
  const u8 = data instanceof Uint8Array ? data : new Uint8Array(data);
  return write(kind, name, u8.length, async (o, n) => u8.subarray(o, o + n));
}

// a file of the Emscripten file system of Pyodide (FS) -> File in OPFS, read in pieces
export async function putFromFS(kind, name, FS, path) {
  const size = FS.stat(path).size, st = FS.open(path, "r"), buf = new Uint8Array(Math.min(STEP, size) || 1);
  try { return await write(kind, name, size, async (o, n) => { const got = FS.read(st, buf, 0, n, o); return buf.subarray(0, got); }); }
  finally { FS.close(st); }
}

// a file written under a temporary name (by a worker of the pool) takes its final name; a half-written file never has it
export async function promote(kind, from, to) {
  const d = await dir(kind), fh = await d.getFileHandle(from);
  try { await d.removeEntry(to); } catch (_) { /* nothing to replace */ }
  if (typeof fh.move === "function") { await fh.move(to); return (await d.getFileHandle(to)).getFile(); }
  const f = await put(kind, to, await fh.getFile());          // no move() in this browser: a copy
  try { await d.removeEntry(from); } catch (_) { /* optional */ }
  return f;
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
  try { await (await dir(kind)).removeEntry(name); return true; } catch (_) { return false; }
}

export async function clear(kind) {
  for (const e of await list(kind)) await remove(kind, e.name);
}

export async function estimate() {
  try { const e = await navigator.storage.estimate(); return { usage: e.usage || 0, quota: e.quota || 0 }; } catch (_) { return { usage: 0, quota: 0 }; }
}
