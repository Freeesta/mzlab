// Loading in the worker of the engine (rust-carica.js, rust-nativo.js), with a fake OPFS archive and synthetic members; argv[2] = folder with the copied modules. Exit code 1 on the first failure.
import assert from "assert";
import { pathToFileURL } from "url";
// a file:// URL: on Windows an absolute path (C:\...) is not a valid ESM specifier
const dir = pathToFileURL(process.argv[2]).href;
let quota = 0, usage = 0;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { storage: { estimate: async () => ({ quota, usage }) } } });
globalThis.self = globalThis;
globalThis.FileReaderSync = class {};
const { createPlainPut, createOpenGate } = await import(dir + "/rust-carica.js");
const { createNativo, PREFIX, MAGIC } = await import(dir + "/rust-nativo.js");
const ARCHIVE = await import(dir + "/archivio.js");
const MB = 1 << 20;

// fake archive of `files/`: name -> { size, bytes? }
const store = new Map();
const mkARCH = () => ({
  get: async (_k, n) => store.get(n) || null,
  list: async () => [...store.keys()].map(name => ({ name })),
  put: async (_k, n, data) => { const f = new File([data], n); store.set(n, f); return f; },
  remove: async (_k, n) => { store.delete(n); },
  release: n => { released.push(n); }, putChunks: undefined,
});
const released = [];
const fakeMember = (name, size, mtime) => ({ name, blob: { size, lastModified: mtime, slice: () => { throw new Error("no copy expected"); } } });
const fakeFile = (size) => ({ size });
const stubFile = (members) => new File([JSON.stringify({ mzlab: MAGIC, ds: "k", c: 0, id: 0, centroid: false, members, size: 0, dim: null })], "s.mzML");

// ---- D3: the room is asked for the members still to write, under the key of the sample
{
  const ARCH = mkARCH(), nat = { key: "k", c: 0, id: 1, members: [fakeMember("a.wiff", 40 * MB, 5), fakeMember("a.wiff.scan", 60 * MB, 6)] };
  for (const x of nat.members) store.set(PREFIX + "k~" + x.name, fakeFile(x.blob.size));       // both members are in OPFS already (the 2nd sample of a .wiff)
  quota = 100 * MB; usage = 0;                                           // 100 MB x 1.8 would not fit; nothing is to be written
  const n = createNativo({ ARCH, readerFor: null, fileOf: async n => store.get(n) || null, jerr: k => { throw k; }, native_samples: null, native_to_mzml: null, native_srm: null, native_external: null, impronte: undefined });
  const st = await n.put({ name: "s2.mzML", nat }, () => {});
  assert.ok(st, "nothing to write: no false quota");
  // a missing member is asked for with the 1.8 factor, and refused with the figures of the bytes still to write
  store.delete(PREFIX + "k~a.wiff.scan");
  let err = null;
  try { await n.put({ name: "s3.mzML", nat }, () => {}); } catch (e) { err = JSON.parse(e); }
  assert.strictEqual(err && err.error_key, "error.quota");
  assert.strictEqual(err.params.serve_mb, Math.ceil(60 * 1.8));          // 60 MB missing, not 100
  ARCHIVE.release("s3.mzML");
}

// ---- V6a: a member already in OPFS gets its fingerprints from the stub of another sample
{
  const ARCH = mkARCH(); store.clear();
  const prints = { sha256: "aa", sha1: "bb", probe: "cc" };
  const members = [{ name: "b.wiff", store: PREFIX + "k~b.wiff", size: 10, mtime: 7, ...prints }, { name: "b.wiff.scan", store: PREFIX + "k~b.wiff.scan", size: 20, mtime: 8, ...prints }];
  store.set("s1.mzML", stubFile(members));
  for (const m of members) store.set(m.store, fakeFile(m.size));
  const n = createNativo({ ARCH, readerFor: null, fileOf: async n => store.get(n) || null, jerr: k => { throw k; }, native_samples: null, native_to_mzml: null, native_srm: null, native_external: null, impronte: undefined });
  const nat = { key: "k", c: 0, id: 2, members: [fakeMember("b.wiff", 10, 7), fakeMember("b.wiff.scan", 20, 8)] };
  await n.put({ name: "s2.mzML", nat }, () => {});
  const st = JSON.parse(await store.get("s2.mzML").text());
  assert.deepStrictEqual(st.members.map(m => [m.sha256, m.sha1, m.probe]), [["aa", "bb", "cc"], ["aa", "bb", "cc"]]);
  // another lastModified: the file is not the same one, no fingerprints are borrowed
  const nat2 = { key: "k", c: 0, id: 3, members: [fakeMember("b.wiff", 10, 99), fakeMember("b.wiff.scan", 20, 8)] };
  const n2 = createNativo({ ARCH, readerFor: null, fileOf: async n => store.get(n) || null, jerr: k => { throw k; }, native_samples: null, native_to_mzml: null, native_srm: null, native_external: null, impronte: undefined });
  await n2.put({ name: "s3.mzML", nat: nat2 }, () => {});
  const st3 = JSON.parse(await store.get("s3.mzML").text());
  assert.strictEqual(st3.members[0].sha256, undefined);
}

// ---- D2: forget drops the stub cache of a name; forgetAll drops them all
{
  const ARCH = mkARCH(); store.clear();
  const read = []; const stub = n => stubFile([{ name: "x", store: "nat~x", size: 1 }]);
  store.set("t.mzML", stub());
  const n = createNativo({ ARCH, readerFor: null, fileOf: async n => { read.push(n); return store.get(n) || null; }, jerr: k => { throw k; }, native_samples: null, native_to_mzml: null, native_srm: null, native_external: null, impronte: undefined });
  assert.ok(await n.stubOf("t.mzML"));
  store.set("t.mzML", new File(["<mzML/>".repeat(10)], "t.mzML"));      // a plain mzML takes the name
  assert.ok(await n.stubOf("t.mzML"), "cached: the stale stub (what lrPut must forget)");
  n.forgetAll();
  assert.strictEqual(await n.stubOf("t.mzML"), null);
}

// ---- D4: cancel between pieces, room released on any failure
{
  const calls = [];
  let cancelNow = false;
  const ARCH = {
    release: n => calls.push("release:" + n),
    putChunks: async (_k, n, size, chunk) => { await chunk(0, 4); cancelNow = true; await chunk(4, 4); return { size }; },
  };
  const blob = new Blob(["12345678"]);
  const mk = () => createPlainPut({ ARCH, impronte: { start: () => null }, prints: new Map(), cancelled: () => "CANCELLED", isCancelled: () => cancelNow, post: () => {}, free: async () => { calls.push("free"); } });
  let err = null;
  try { await mk().putPlain({ name: "p.mzML", blob, id: 1 }); } catch (e) { err = e; }
  assert.strictEqual(err, "CANCELLED");
  assert.deepStrictEqual(calls, ["release:p.mzML"]);
  // a second failure after the retry releases too
  calls.length = 0; cancelNow = false;
  const ARCH2 = { release: n => calls.push("release:" + n), putChunks: async () => { throw { name: "QuotaExceededError" }; } };
  err = null;
  try { await createPlainPut({ ARCH: ARCH2, impronte: { start: () => null }, prints: new Map(), cancelled: () => "C", isCancelled: () => false, post: () => {}, free: async () => { calls.push("free"); } }).putPlain({ name: "q.mzML", blob, id: 2 }); } catch (e) { err = e; }
  assert.ok(err);
  assert.deepStrictEqual(calls, ["free", "release:q.mzML"]);
  // success keeps the room (lr_index releases it)
  calls.length = 0;
  const ARCH3 = { release: n => calls.push("release:" + n), putChunks: async (_k, n, size) => ({ size }) };
  const f = await createPlainPut({ ARCH: ARCH3, impronte: { start: () => null }, prints: new Map(), cancelled: () => "C", isCancelled: () => false, post: () => {}, free: async () => {} }).putPlain({ name: "r.mzML", blob, id: 3 });
  assert.strictEqual(f.size, 8);
  assert.deepStrictEqual(calls, []);
}

// ---- V3c: the gate
{
  const g = createOpenGate(), ix = { size: 100, lastModified: 5 };
  assert.strictEqual(g.same(1, ix), false);
  g.mark(1, ix);
  assert.strictEqual(g.same(1, { size: 100, lastModified: 5 }), true);
  assert.strictEqual(g.same(1, { size: 100, lastModified: 6 }), false);
  assert.strictEqual(g.same(1, { size: 101, lastModified: 5 }), false);
  assert.strictEqual(g.same(2, ix), false);
  g.drop(1); assert.strictEqual(g.same(1, ix), false);
  g.mark(1, ix); g.clear(); assert.strictEqual(g.same(1, ix), false);
}
console.log("ok");
