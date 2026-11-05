// Loading in the worker of the engine (rust-carica.js, rust-nativo.js), with a fake OPFS archive and synthetic members; argv[2] = folder with the copied modules. Exit code 1 on the first failure.
import assert from "assert";
import { pathToFileURL } from "url";
// a file:// URL: on Windows an absolute path (C:\...) is not a valid ESM specifier
const dir = pathToFileURL(process.argv[2]).href;
let quota = 0, usage = 0;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { storage: { estimate: async () => ({ quota, usage }) } } });
globalThis.self = globalThis;
globalThis.FileReaderSync = class {};
const { createPlainPut, createOpenGate, gzRoom, gzClean, GZ_SLICE } = await import(dir + "/rust-carica.js");
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
// ---- the streaming gunzip of a .mzML.gz / .mzXML.gz: a fake Gunzip that passes its input on, a fake archive that collects the stream
{
  const calls = [], disk = new Map();
  const ARCH = {
    release: n => calls.push("release:" + n),
    remove: async (k, n) => { calls.push("remove:" + k + "/" + n); disk.delete(k + "/" + n); },
    putStream: async (k, n, next) => {
      const parts = [];
      for (let b; (b = await next()) !== null;) { assert.ok(b.length <= GZ_SLICE, "one push is one slice at most"); parts.push(b); }
      if (ARCH.full) throw { name: "QuotaExceededError" };
      const f = new File(parts, n); disk.set(k + "/" + n, f); return f;
    },
    promote: async (k, from, to) => { calls.push("promote:" + from + ">" + to); const f = new File([disk.get(k + "/" + from)], to); disk.delete(k + "/" + from); disk.set(k + "/" + to, f); return f; },
  };
  let freed = 0, cancelAt = -1, checks = 0;
  const fake = ({ whole = true, out = true } = {}) => cap => {
    assert.strictEqual(cap, 4 * 1024 * 1024 * 1024);
    let n = 0;
    return { push: b => { n += b.length; return out ? b.slice() : new Uint8Array(0); }, finish: () => new Uint8Array(0), whole: () => whole, written: () => out ? n : 0, free: () => { freed++; } };
  };
  const mk = g => createPlainPut({ ARCH, impronte: { start: () => null }, prints: new Map(), cancelled: () => "CANCELLED", isCancelled: () => ++checks === cancelAt, post: () => {}, free: async () => {}, gunzip: g });
  const data = new Uint8Array(3 * MB + 12345).map((_, i) => (i * 31) & 255), blob = new Blob([data]);
  // success: the inflated bytes are the stream, written as <name>.gz.part and renamed to the mzML; the room stays (lr_index releases it)
  const f = await mk(fake()).putGz({ name: "s.mzML", blob, id: 1 });
  assert.strictEqual(f.name, "s.mzML");
  assert.deepStrictEqual(new Uint8Array(await f.arrayBuffer()), data);
  assert.deepStrictEqual(calls, ["promote:s.mzML.gz.part>s.mzML"]);
  assert.deepStrictEqual([...disk.keys()], ["files/s.mzML"]);
  assert.strictEqual(freed, 1);
  // cancel at the second piece of the .gz: the part file goes, the room is released, the Gunzip freed
  calls.length = 0; checks = 0; cancelAt = 2;
  let err = null;
  try { await mk(fake()).putGz({ name: "c.mzML", blob, id: 2 }); } catch (e) { err = e; }
  assert.strictEqual(err, "CANCELLED");
  assert.deepStrictEqual(calls, ["remove:files/c.mzML.gz.part", "release:c.mzML"]);
  assert.strictEqual(freed, 2);
  cancelAt = -1;
  // OPFS full: the error reaches the page (its quota message), nothing is left
  calls.length = 0; ARCH.full = true; err = null;
  try { await mk(fake()).putGz({ name: "q.mzML", blob, id: 3 }); } catch (e) { err = e; }
  assert.strictEqual(err.name, "QuotaExceededError");
  assert.deepStrictEqual(calls, ["remove:files/q.mzML.gz.part", "release:q.mzML"]);
  ARCH.full = false;
  // a stream that ends early with nothing inflated is a broken gzip; without a Gunzip the stream is refused
  err = null;
  try { await mk(fake({ whole: false, out: false })).putGz({ name: "t.mzML", blob, id: 4 }); } catch (e) { err = JSON.parse(e); }
  assert.deepStrictEqual(err, { error_key: "err.file.zlib", params: { detail: "truncated" } });
  err = null;
  try { await mk(undefined).putGz({ name: "u.mzML", blob, id: 5 }); } catch (e) { err = JSON.parse(e); }
  assert.strictEqual(err.error_key, "err.file.gzStream");
  // a .mzXML.gz into conv/: the room is released when done
  calls.length = 0;
  const x = await mk(fake()).gunzipTo({ name: "x.mzXML.gz", blob, id: 6, dest: "gz~sid~ab.mzXML" });
  assert.strictEqual(x.size, data.length);
  assert.ok(disk.has("conv/gz~sid~ab.mzXML"));
  assert.deepStrictEqual(calls, ["release:x.mzXML.gz"]);
  // the room asked: ISIZE of the trailer, or 5 times the file when the trailer cannot be the size (over 4 GiB, not gzip)
  const trailer = n => { const b = new Uint8Array(40); new DataView(b.buffer).setUint32(36, n, true); return new Blob([b]); };
  assert.strictEqual(await gzRoom(trailer(123456)), 123456);
  assert.strictEqual(await gzRoom(trailer(7)), 200);
  assert.strictEqual(await gzRoom(new Blob([new Uint8Array(10)])), 10);
}
// ---- what a closed tab left: free part files go, and the mzXML of pages whose lock is free (a live page holds its own)
{
  const gone = [], held = new Set(["live"]);
  navigator.locks = { request: async (key, opt, fn) => fn(held.has(key.split(":")[1]) ? null : { name: key }) };
  const ARCH = {
    list: async k => (k === "files" ? ["a.mzML", "b.mzML.gz.part"] : ["gz~dead~x.mzXML", "gz~live~y.mzXML", "other.mzML"]).map(name => ({ name })),
    removeIfFree: async (k, n) => gone.push(k + "/" + n), remove: async (k, n) => gone.push(k + "/" + n),
  };
  await gzClean(ARCH);
  assert.deepStrictEqual(gone, ["files/b.mzML.gz.part", "conv/gz~dead~x.mzXML"]);
  delete navigator.locks;
}
// ---- F6: at boot the members no stub points to (a copy a closed tab never finished) go, unless another tab holds the lock of their dataset
{
  const { LOCK } = await import(dir + "/rust-nativo.js");
  // fake Web Locks: shared / exclusive, ifAvailable; `held` = what the "other tab" holds
  const held = new Map(), asked = [];
  navigator.locks = { async request(name, opts, fn) {
    const mode = opts.mode || "exclusive", h = held.get(name);
    asked.push([name, mode]);
    if (h && !(mode === "shared" && h.mode === "shared")) { if (opts.ifAvailable) return fn(null); throw new Error("the fake lock would wait"); }
    held.set(name, { mode, n: (h ? h.n : 0) + 1 });
    try { return await fn({ name, mode }); } finally { const x = held.get(name); if (--x.n === 0) held.delete(name); }
  } };
  const ARCH = mkARCH(); store.clear();
  const mk = () => createNativo({ ARCH, readerFor: null, fileOf: async n => store.get(n) || null, jerr: k => { throw k; }, native_samples: null, native_to_mzml: null, native_srm: null, native_external: null, impronte: undefined });
  const kept = [{ name: "a.wiff", store: PREFIX + "a.wiff-k1~a.wiff", size: 10 }, { name: "a.wiff.scan", store: PREFIX + "a.wiff-k1~a.wiff.scan", size: 20 }];
  store.set("a.mzML", stubFile(kept)); for (const m of kept) store.set(m.store, fakeFile(m.size));
  // 0 orphans: nothing removed, nothing to say
  assert.deepStrictEqual(await mk().orphans(), []);
  assert.deepStrictEqual([...store.keys()].sort(), ["a.mzML", ...kept.map(m => m.store)].sort());
  // a pair copied by a tab closed before its stub: it goes, named by its .wiff; the members a stub uses stay
  store.set(PREFIX + "b.wiff-k2~d~b.wiff", fakeFile(5)); store.set(PREFIX + "b.wiff-k2~d~b.wiff.scan.part", fakeFile(3));
  assert.deepStrictEqual(await mk().orphans(), ["b.wiff"]);
  assert.deepStrictEqual([...store.keys()].sort(), ["a.mzML", ...kept.map(m => m.store)].sort());
  // two tabs: the other one is copying c (holds the shared lock of its dataset) while this one boots: nothing of c is removed
  store.set(PREFIX + "c.wiff-k3~c.wiff", fakeFile(5));
  let free; const other = navigator.locks.request(LOCK + "c.wiff-k3", { mode: "shared" }, () => new Promise(r => { free = r; }));
  assert.deepStrictEqual(await mk().orphans(), []);
  assert.ok(store.has(PREFIX + "c.wiff-k3~c.wiff"), "a member another tab is copying stays");
  // that tab writes its stub and lets go: the member is used, the next boot keeps it
  store.set("c.mzML", stubFile([{ name: "c.wiff", store: PREFIX + "c.wiff-k3~c.wiff", size: 5 }])); free(); await other;
  assert.deepStrictEqual(await mk().orphans(), []);
  assert.ok(store.has(PREFIX + "c.wiff-k3~c.wiff"));
  // a stub written between the first look and the lock keeps its members (checked again under the lock)
  store.set(PREFIX + "e.wiff-k4~e.wiff", fakeFile(5));
  const real = navigator.locks.request;
  navigator.locks.request = (name, opts, fn) => { store.set("e.mzML", stubFile([{ name: "e.wiff", store: PREFIX + "e.wiff-k4~e.wiff", size: 5 }])); navigator.locks.request = real; return real.call(navigator.locks, name, opts, fn); };
  assert.deepStrictEqual(await mk().orphans(), []);
  assert.ok(store.has(PREFIX + "e.wiff-k4~e.wiff"));
  // put holds the shared lock of its dataset while it copies and writes the stub
  asked.length = 0;
  const nat = { key: "c.wiff-k3", c: 0, id: 2, members: [fakeMember("c.wiff", 5, 0)] };
  await mk().put({ name: "c2.mzML", nat }, () => {});
  assert.deepStrictEqual(asked, [[LOCK + "c.wiff-k3", "shared"]]);
  // without Web Locks another tab's copy cannot be told apart: nothing is removed
  delete navigator.locks;
  store.set(PREFIX + "f.wiff-k5~f.wiff", fakeFile(5));
  assert.deepStrictEqual(await mk().orphans(), []);
  assert.ok(store.has(PREFIX + "f.wiff-k5~f.wiff"));
}
console.log("ok");
