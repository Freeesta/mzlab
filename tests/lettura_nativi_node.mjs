// Reads of the members of a maker's dataset in the worker of the engine (rust-nativo.js, lettura-sync.js) with a fake OPFS of read-only handles and a FileReaderSync that counts; argv[2] = folder with the copied modules.
import assert from "assert";
import { createHash } from "crypto";
const dir = process.argv[2];
const bytes = new Map();                       // store name -> Uint8Array in the fake OPFS `files/`
let fileReads = 0, handleReads = 0, handleOpens = 0, modes = [];
globalThis.self = globalThis;
globalThis.FileReaderSync = class { readAsArrayBuffer(b) { fileReads++; return b.__bytes.buffer.slice(b.__bytes.byteOffset, b.__bytes.byteOffset + b.__bytes.byteLength); } };
const mkBlob = (u, name) => { const f = new File([u], name); f.__bytes = u; f.slice = (a, b) => { const x = new Blob([u.slice(a, b)]); x.__bytes = u.slice(a, b); return x; }; return f; };
const filesDir = { getFileHandle: async n => { if (!bytes.has(n)) throw new Error("NotFound"); return { createSyncAccessHandle: async o => { handleOpens++; modes.push(o && o.mode); const u = bytes.get(n); let closed = false; return { getSize: () => u.length, read: (buf, { at }) => { if (closed) throw new Error("closed"); handleReads++; const k = Math.min(buf.length, u.length - at); buf.set(u.subarray(at, at + k)); return k; }, close: () => { closed = true; } }; } }; } };
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { storage: { getDirectory: async () => ({ getDirectoryHandle: async n => { if (n === "mzlab") return { getDirectoryHandle: async k => { if (k === "files") return filesDir; throw new Error("no " + k); } }; throw new Error("no " + n); } }) } } });
const { createNativo, PREFIX, MAGIC } = await import(dir + "/rust-nativo.js");
const { openSource } = await import(dir + "/lettura-sync.js");

// a synthetic 2-member dataset in OPFS: the members as Blobs (the Files of the archive) and as handle-readable bytes
const mem = (name, n, seed) => { const u = new Uint8Array(n); for (let i = 0; i < n; i++) u[i] = (i * 31 + seed) & 255; const store = PREFIX + "k~" + name; bytes.set(store, u); return { name, store, size: n, blob: mkBlob(u, store) }; };
const M = [mem("a.wiff", 5000, 1), mem("a.wiff.scan", 12000, 7)];
const stub = new File([JSON.stringify({ mzlab: MAGIC, ds: "k", c: 0, id: 0, centroid: false, members: M.map(({ name, store, size }) => ({ name, store, size })), size: 17000, dim: null })], "s.mzML");
const ARCH = { get: async (_k, n) => bytes.has(n) ? M.find(m => m.store === n).blob : null, list: async () => [], put: async () => null, remove: async () => {}, release: () => {} };
// the engine reads every member in blocks of 1000 bytes and the "mzML" is what it read (the sink gets one piece per block)
const run = (names, sizes, rd, _c, _cen, ...rest) => { const sink = rest.find(x => typeof x === "function"); let n = 0; sizes.forEach((sz, i) => { for (let o = 0; o < sz; o += 1000) { const b = rd(i, o, Math.min(1000, sz - o)); sink(Uint8Array.from(b)); n++; } }); return n; };
const nat = createNativo({ ARCH, readerFor: null, fileOf: async () => stub, jerr: k => { throw k; }, native_samples: null, native_to_mzml: (names, sizes, rd, c, cen, np, sink) => run(names, sizes, rd, c, cen, sink), native_srm: (names, sizes, rd, c, cen, sink) => run(names, sizes, rd, c, cen, sink), native_external: null, impronte: undefined });
const want = createHash("sha256").update(Buffer.concat(M.map(m => Buffer.from(bytes.get(m.store))))).digest("hex");
const sha = parts => createHash("sha256").update(Buffer.concat(parts.map(p => Buffer.from(p)))).digest("hex");

// ---- C04: «Download .mzML» of a native sample reads through read-only handles (no FileReaderSync), closed afterwards, the same bytes
{
  fileReads = handleReads = handleOpens = 0; modes = [];
  const sink = await nat.toMzml({ name: "s.mzML", dest: "d.mzML", numpress: false }, () => {});
  assert.strictEqual(sha(sink.parts), want, "same bytes as reading the members as they are");
  assert.strictEqual(fileReads, 0, "no FileReaderSync read");
  assert.strictEqual(handleReads, 5 + 12);
  assert.strictEqual(handleOpens, 2); assert.deepStrictEqual(modes, ["read-only", "read-only"]);
}
// ---- C04: the SRM traces the same way
{
  fileReads = handleReads = 0;
  let got = []; const n2 = createNativo({ ARCH, readerFor: null, fileOf: async () => stub, jerr: k => { throw k; }, native_samples: null, native_to_mzml: null, native_srm: (names, sizes, rd, c, cen, sink) => { let t = 0; sizes.forEach((sz, i) => { for (let o = 0; o < sz; o += 1000) t += rd(i, o, Math.min(1000, sz - o)).length; }); sink(new Uint8Array([t & 255])); }, native_external: null, impronte: undefined });
  const out = await n2.srm("s.mzML");
  assert.deepStrictEqual([...out], [17000 & 255]);
  assert.strictEqual(fileReads, 0); assert.strictEqual(handleReads, 17);
}
// ---- a reader kept after its call (a session of the engine) goes on through the Blob: a closed handle never throws at the engine
{
  fileReads = handleReads = 0;
  const f = await openSource("files", M[0].store, M[0].blob);
  assert.strictEqual(f.hand, true);
  assert.deepStrictEqual([...f.read(10, 4)], [...bytes.get(M[0].store).subarray(10, 14)]); assert.strictEqual(handleReads, 1);
  f.close(); f.close();
  assert.deepStrictEqual([...f.read(10, 4)], [...bytes.get(M[0].store).subarray(10, 14)]); assert.strictEqual(fileReads, 1);
}
console.log("ok");
