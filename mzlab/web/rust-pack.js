// @ts-strict
// `file pack`, `file unpack` and `file check-pack` in a worker (docs/agenti/file-pack.md): the .wasm `pack` of wasm-feat/manifest.json (crates/mzlab-pack-wasm) is fetched on
// the first use and reads and writes OPFS files through sync access handles, so nothing is held in memory but one 1 MiB block. Parts and manifest are the command line's, byte for byte.
// A sync handle can only be opened with `await`, and the module calls back synchronously: every file and part is opened before the call, so one call serves up to MAX_OPEN of them
// (a larger campaign uses the app or the command line, or a larger part size). A failure is thrown as the JSON string `{error_key, params}` of the module.

import { load } from "./rust-msngrafo.js";

export const MAX_OPEN = 1024;

/** @typedef {{path: string, handle: any}} Input */

/** @param {string} detail */
const refuse = detail => JSON.stringify({ error_key: "err.run.input", params: { detail } });

/** @param {any} ah @param {number} off @param {number} len */
function readAt(ah, off, len) {
  const buf = new Uint8Array(len), n = ah.read(buf, { at: off });
  return n === len ? buf : buf.subarray(0, n);
}

/** A short write is a full disk. @param {any} ah @param {Uint8Array} bytes @param {number} at */
function writeAll(ah, bytes, at) {
  for (let done = 0; done < bytes.length;) {
    const n = ah.write(bytes.subarray(done), { at: at + done });
    if (!n) throw new Error("storage full");
    done += n;
  }
}

/** @param {any[]} handles */
function closeAll(handles) { for (const h of handles) try { h?.close(); } catch (_) { /* already closed */ } }

/** The folder of a relative path under `root` (created on the way) and the file name. @param {any} root @param {string} path */
async function folderOf(root, path) {
  const parts = path.split("/"), name = /** @type {string} */ (parts.pop());
  let d = root;
  for (const p of parts) d = await d.getDirectoryHandle(p, { create: true });
  return { d, name };
}

/** The parts of a manifest as sync read handles (null = not there). @param {any} dir @param {string[]} names */
async function openParts(dir, names) {
  if (names.length > MAX_OPEN) throw refuse(`${names.length} parts: use a part size that gives at most ${MAX_OPEN}, or the app`);
  return Promise.all(names.map(async n => {
    try { return await (await dir.getFileHandle(n)).createSyncAccessHandle(); } catch (_) { return null; }
  }));
}

/** @param {(any | null)[]} handles */
const partReaders = handles => ({
  size: (/** @type {number} */ i) => handles[i] ? handles[i].getSize() : -1,
  read: (/** @type {number} */ i, /** @type {number} */ off, /** @type {number} */ len) => readAt(handles[i], off, len),
});

/**
 * Packs `files` (relative path + FileSystemFileHandle) into parts in `dir` (a FileSystemDirectoryHandle) and writes `NAME.pack.json` there; returns `{manifest, doc}`, the
 * manifest text and the `pack-run/1` document. `dryRun` reads nothing and writes nothing. Existing parts of that name are replaced: the caller asks first.
 * @param {{files: Input[], dir: any, name?: string, partSize?: number, dryRun?: boolean}} m
 */
export async function packFiles({ files, dir, name = "pack", partSize = 25e6, dryRun = false }) {
  const mod = await load("pack");
  if (files.length > MAX_OPEN) throw refuse(`${files.length} files: at most ${MAX_OPEN} in one call, use the app`);
  const sizes = await Promise.all(files.map(async f => (await f.handle.getFile()).size));
  const list = JSON.stringify(files.map((f, i) => [f.path, sizes[i]]));
  const plan = mod.pack_plan(list, name, partSize);
  if (dryRun) return { manifest: "", doc: plan };
  const names = /** @type {{name: string}[]} */ (JSON.parse(plan).parts).map(p => p.name);
  if (names.length > MAX_OPEN) throw refuse(`${names.length} parts: use a larger part size, or the app`);
  /** @type {any[]} */ const ins = [], outs = [];
  try {
    for (const f of files) ins.push(await f.handle.createSyncAccessHandle());
    for (const n of names) { const h = await (await dir.getFileHandle(n, { create: true })).createSyncAccessHandle(); h.truncate(0); outs.push(h); }
    const at = names.map(() => 0);
    const manifest = mod.pack(list, name, partSize,
      (/** @type {number} */ i, /** @type {number} */ off, /** @type {number} */ len) => readAt(ins[i], off, len),
      (/** @type {number} */ part, /** @type {Uint8Array} */ bytes) => { writeAll(outs[part], bytes, at[part]); at[part] += bytes.length; });
    for (const h of outs) h.flush();
    closeAll(outs);
    const mh = await (await dir.getFileHandle(`${name}.pack.json`, { create: true })).createSyncAccessHandle();
    try { mh.truncate(0); writeAll(mh, new TextEncoder().encode(manifest), 0); mh.flush(); } finally { mh.close(); }
    return { manifest, doc: mod.pack_doc(manifest) };
  } catch (e) {
    closeAll(outs);
    for (const n of names) await dir.removeEntry(n).catch(() => {});   // a failed run leaves no parts, as the command line
    throw e;
  } finally { closeAll(ins); }
}

/**
 * `verify-pack/1` of a manifest text over the parts in `dir`; `resend` of the answer names the parts to send again. Nothing is written.
 * @param {{manifest: string, dir: any, seen?: (part: number, status: string) => void}} m
 */
export async function verifyPack({ manifest, dir, seen }) {
  const mod = await load("pack");
  const names = /** @type {{name: string}[]} */ (JSON.parse(manifest).parts).map(p => p.name);
  const hs = await openParts(dir, names);
  try { const r = partReaders(hs); return mod.verify_pack(manifest, r.size, r.read, seen ?? null); } finally { closeAll(hs); }
}

/**
 * `unpack-run/1`: rebuilds the files of a manifest text under `out` from the parts in `dir`. Without `force` an existing file is `err.pack.exists`; `dryRun` writes nothing.
 * A file is written under a temporary name and moved into place only when its SHA-256 matched.
 * @param {{manifest: string, dir: any, out: any, force?: boolean, dryRun?: boolean, seen?: (part: number, status: string) => void}} m
 */
export async function unpackPack({ manifest, dir, out, force = false, dryRun = false, seen }) {
  const mod = await load("pack");
  const doc = JSON.parse(manifest);
  const names = /** @type {{name: string}[]} */ (doc.parts).map(p => p.name), paths = /** @type {{path: string}[]} */ (doc.files).map(f => f.path);
  if (paths.length > MAX_OPEN) throw refuse(`${paths.length} files: at most ${MAX_OPEN} in one call, use the app`);
  /** @type {Set<string>} */ const taken = new Set();
  if (!force) for (const p of paths) {
    const { d, name } = await folderOf(out, p).catch(() => ({ d: null, name: "" }));
    if (d && await d.getFileHandle(name).then(() => true, () => false)) taken.add(p);
  }
  const hs = await openParts(dir, names);
  /** @type {Map<string, {fh: any, ah: any, d: any, name: string, tmp: string, keep?: boolean}>} */ const ready = new Map();
  try {
    if (!dryRun) for (const p of paths) {
      const { d, name } = await folderOf(out, p), tmp = `.${name}.mzpack-tmp`;
      const fh = await d.getFileHandle(tmp, { create: true }), ah = await fh.createSyncAccessHandle();
      ah.truncate(0);
      ready.set(p, { fh, ah, d, name, tmp });
    }
    /** @type {any} */ let cur = null;
    let at = 0;
    const files = {
      exists: (/** @type {string} */ p) => taken.has(p),
      begin: (/** @type {string} */ p) => { cur = ready.get(p) ?? null; at = 0; },
      write: (/** @type {Uint8Array} */ b) => { if (cur) { writeAll(cur.ah, b, at); at += b.length; } },
      end: (/** @type {boolean} */ keep) => { if (cur) { cur.ah.flush(); cur.ah.close(); cur.keep = keep; } cur = null; },
    };
    const r = partReaders(hs);
    const answer = mod.unpack(manifest, r.size, r.read, files, force, dryRun, seen ?? null);
    // the temporary files are moved into place (or dropped) once the module has decided about each of them
    for (const v of ready.values()) {
      if (v.keep) {
        if (force) await v.d.removeEntry(v.name).catch(() => {});
        if (typeof v.fh.move === "function") await v.fh.move(v.d, v.name);
        else { const f = await v.fh.getFile(); const h = await (await v.d.getFileHandle(v.name, { create: true })).createSyncAccessHandle(); try { h.truncate(0); writeAll(h, new Uint8Array(await f.arrayBuffer()), 0); h.flush(); } finally { h.close(); } await v.d.removeEntry(v.tmp); }
      } else await v.d.removeEntry(v.tmp).catch(() => {});
    }
    ready.clear();
    return answer;
  } finally {
    closeAll(hs);
    for (const v of ready.values()) { try { v.ah.close(); } catch (_) { /* closed */ } await v.d.removeEntry(v.tmp).catch(() => {}); }
  }
}
