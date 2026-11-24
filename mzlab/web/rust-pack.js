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

/** The folder of a relative path under `root` (created on the way, each one made is pushed to `made`) and the file name. @param {any} root @param {string} path @param {any[][]} made */
async function folderOf(root, path, made) {
  const parts = path.split("/"), name = /** @type {string} */ (parts.pop());
  let d = root;
  for (const p of parts) {
    try { d = await d.getDirectoryHandle(p); } catch (_) {
      // a file where a folder is wanted is the command line's `err.run.output`, not a raw DOMException
      const n = await d.getDirectoryHandle(p, { create: true }).catch(/** @param {any} e */ e => { throw JSON.stringify({ error_key: "err.run.output", params: { detail: `${path}: ${e?.name ?? e}` } }); });
      made.push([d, p]); d = n;
    }
  }
  return { d, name };
}

/** Moves the file `tmp` of `d` to `name` (an older file of that name is replaced); a sync handle on either must be closed. @param {any} d @param {string} tmp @param {string} name */
async function place(d, tmp, name) {
  const fh = await d.getFileHandle(tmp);
  await d.removeEntry(name).catch(() => {});
  if (typeof fh.move === "function") { await fh.move(d, name); return; }
  const bytes = new Uint8Array(await (await fh.getFile()).arrayBuffer());
  const h = await (await d.getFileHandle(name, { create: true })).createSyncAccessHandle();
  try { h.truncate(0); writeAll(h, bytes, 0); h.flush(); } finally { h.close(); }
  await d.removeEntry(tmp);
}

/** The names `NAME.partNNNN` and `NAME.pack.json` already in `dir`, sorted. @param {any} dir @param {string} name */
async function outputsOf(dir, name) {
  const prefix = `${name}.part`, v = [];
  for await (const n of dir.keys()) if (n === `${name}.pack.json` || (n.startsWith(prefix) && /^\d+$/.test(n.slice(prefix.length)))) v.push(n);
  return v.sort();
}

/** Whether a relative path exists under `root`; looks folders up without creating any. @param {any} root @param {string} path */
async function existsUnder(root, path) {
  const parts = path.split("/"), name = /** @type {string} */ (parts.pop());
  let d = root;
  try { for (const p of parts) d = await d.getDirectoryHandle(p); } catch (_) { return false; }
  return d.getFileHandle(name).then(() => true, () => d.getDirectoryHandle(name).then(() => true, () => false));
}

/** The `field` of every entry of `key` in a manifest text; anything that is not that shape gives none, so the module's own check answers with its error key. @param {string} manifest @param {string} key @param {string} field */
function listed(manifest, key, field) {
  try {
    const v = JSON.parse(manifest)?.[key];
    return Array.isArray(v) ? v.map(x => String(x?.[field])) : [];
  } catch (_) { return []; }
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
 * manifest text and the `pack-run/1` document. `dryRun` reads nothing and writes nothing, and is refused for the names a real run refuses. Existing parts or manifest of that
 * name are `err.pack.exists` unless `force`; the new parts are written under temporary names and moved into place only when all of them are, so a run that fails before the moves leaves an older pack as it was.
 * @param {{files: Input[], dir: any, name?: string, partSize?: number, dryRun?: boolean, force?: boolean}} m
 */
export async function packFiles({ files, dir, name = "pack", partSize = 25e6, dryRun = false, force = false }) {
  const mod = await load("pack");
  if (files.length > MAX_OPEN) throw refuse(`${files.length} files: at most ${MAX_OPEN} in one call, use the app`);
  const sizes = await Promise.all(files.map(async f => (await f.handle.getFile()).size));
  const list = JSON.stringify(files.map((f, i) => [f.path, sizes[i]]));
  const plan = mod.pack_plan(list, name, partSize);   // the names and the part size are checked here, before anything is touched
  if (dryRun) return { manifest: "", doc: plan };
  const names = /** @type {{name: string}[]} */ (JSON.parse(plan).parts).map(p => p.name);
  if (names.length > MAX_OPEN) throw refuse(`${names.length} parts: use a larger part size, or the app`);
  const old = await outputsOf(dir, name);
  if (old.length && !force) throw JSON.stringify({ error_key: "err.pack.exists", params: { path: old[0], count: old.length } });
  const tmpOf = (/** @type {string} */ n) => `.${n}.mzpack-tmp`, manifestName = `${name}.pack.json`, manifestTmp = `.${manifestName}.tmp`;
  /** @type {any[]} */ const ins = [], outs = [];
  try {
    for (const f of files) ins.push(await f.handle.createSyncAccessHandle());
    for (const n of names) { const h = await (await dir.getFileHandle(tmpOf(n), { create: true })).createSyncAccessHandle(); h.truncate(0); outs.push(h); }
    const at = names.map(() => 0);
    const manifest = mod.pack(list, name, partSize,
      (/** @type {number} */ i, /** @type {number} */ off, /** @type {number} */ len) => readAt(ins[i], off, len),
      (/** @type {number} */ part, /** @type {Uint8Array} */ bytes) => { writeAll(outs[part], bytes, at[part]); at[part] += bytes.length; });
    for (const h of outs) h.flush();
    closeAll(outs);
    // the manifest is written first under its temporary name (a full disk shows before an older part is replaced), then the parts and the manifest are moved into place
    const mh = await (await dir.getFileHandle(manifestTmp, { create: true })).createSyncAccessHandle();
    try { mh.truncate(0); writeAll(mh, new TextEncoder().encode(manifest), 0); mh.flush(); } finally { mh.close(); }
    for (const n of names) await place(dir, tmpOf(n), n);
    await place(dir, manifestTmp, manifestName);
    for (const o of old) if (o !== manifestName && !names.includes(o)) await dir.removeEntry(o).catch(() => {});   // parts of an older, longer pack
    return { manifest, doc: mod.pack_doc(manifest) };
  } catch (e) {
    closeAll(outs);
    for (const n of [...names.map(tmpOf), manifestTmp]) await dir.removeEntry(n).catch(() => {});   // a failed run removes its own files only
    throw e;
  } finally { closeAll(ins); }
}

/**
 * `verify-pack/1` of a manifest text over the parts in `dir`; `resend` of the answer names the parts to send again. Nothing is written.
 * @param {{manifest: string, dir: any, seen?: (part: number, status: string) => void}} m
 */
export async function verifyPack({ manifest, dir, seen }) {
  const mod = await load("pack");
  const hs = await openParts(dir, listed(manifest, "parts", "name"));
  try { const r = partReaders(hs); return mod.verify_pack(manifest, r.size, r.read, seen ?? null); } finally { closeAll(hs); }
}

/**
 * `unpack-run/1`: rebuilds the files of a manifest text under `out` from the parts in `dir`. Without `force` an existing file is `err.pack.exists`; `dryRun` writes nothing.
 * A file is written under a temporary name and moved into place only when its SHA-256 matched.
 * @param {{manifest: string, dir: any, out: any, force?: boolean, dryRun?: boolean, seen?: (part: number, status: string) => void}} m
 */
export async function unpackPack({ manifest, dir, out, force = false, dryRun = false, seen }) {
  const mod = await load("pack");
  mod.pack_doc(manifest);   // the module reads the manifest first: a bad one is its error key, and no folder or file is created for it
  const names = listed(manifest, "parts", "name"), paths = listed(manifest, "files", "path");
  if (paths.length > MAX_OPEN) throw refuse(`${paths.length} files: at most ${MAX_OPEN} in one call, use the app`);
  /** @type {Set<string>} */ const taken = new Set(), began = new Set();
  if (!force) for (const p of paths) if (await existsUnder(out, p)) taken.add(p);
  const hs = await openParts(dir, names);
  /** @type {Map<string, {fh: any, ah: any, d: any, name: string, tmp: string, keep?: boolean}>} */ const ready = new Map();
  /** @type {Map<string, any[][]>} */ const madeBy = new Map();
  try {
    // nothing is created for a dry run or when the module is about to refuse because files exist: the command line writes nothing then either
    if (!dryRun && !taken.size) for (const p of paths) {
      /** @type {any[][]} */ const made = [];
      madeBy.set(p, made);
      const { d, name } = await folderOf(out, p, made), tmp = `.${name}.mzpack-tmp`;
      const fh = await d.getFileHandle(tmp, { create: true }), ah = await fh.createSyncAccessHandle();
      ah.truncate(0);
      ready.set(p, { fh, ah, d, name, tmp });
    }
    /** @type {any} */ let cur = null;
    let at = 0;
    const files = {
      exists: (/** @type {string} */ p) => taken.has(p),
      begin: (/** @type {string} */ p) => { cur = ready.get(p) ?? null; at = 0; began.add(p); },
      write: (/** @type {Uint8Array} */ b) => { if (cur) { writeAll(cur.ah, b, at); at += b.length; } },
      end: (/** @type {boolean} */ keep) => { if (cur) { cur.ah.flush(); cur.ah.close(); cur.keep = keep; } cur = null; },
    };
    const r = partReaders(hs);
    const answer = mod.unpack(manifest, r.size, r.read, files, force, dryRun, seen ?? null);
    // the temporary files are moved into place (or dropped) once the module has decided about each of them
    for (const v of ready.values()) {
      closeAll([v.ah]);   // a file the module never began (its parts are lost) still holds its handle, and an open file cannot be removed
      if (v.keep) await place(v.d, v.tmp, v.name);
      else await v.d.removeEntry(v.tmp).catch(() => {});
    }
    ready.clear();
    await dropEmptyFolders(madeBy, began);
    return answer;
  } finally {
    closeAll(hs);
    for (const v of ready.values()) { try { v.ah.close(); } catch (_) { /* closed */ } await v.d.removeEntry(v.tmp).catch(() => {}); }
    if (ready.size) await dropEmptyFolders(madeBy, began);
  }
}

/** The folders made for files the module never began (their parts were lost) go again, as the command line makes none for them; a folder with anything in it stays. @param {Map<string, any[][]>} madeBy @param {Set<string>} began */
async function dropEmptyFolders(madeBy, began) {
  for (const [p, made] of madeBy) if (!began.has(p)) for (const [parent, nm] of [...made].reverse()) await parent.removeEntry(nm).catch(() => {});
}
