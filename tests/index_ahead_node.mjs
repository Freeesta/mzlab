// The index ahead of the «Choose samples» dialog (mzlab/web/rust-index-ahead.js inside the real createPrefetch, copy gate and dataset lock of the engine worker), with a fake OPFS (files/ and idx/)
// and a fake index pool: the index of a ticked sample is built right after its copy, under a name that does not depend on the selection; «Open selected» MOVES it to the final name (the pool is not
// asked again: counted); an opening that comes while it is still being built waits for it; a stub that is not the sample the index was built for adopts nothing; untick and X cancel the build and remove
// the file; at most 2 builds run at once; a failure mid-copy or mid-index (storage full) leaves nothing and the opening goes the normal way; two dialogs (two datasets) at once do not touch each other;
// an opening during the COPY phase takes the sample by itself and nothing is built behind it; a dialog reopened after X works again. Fails on any difference, never skips.
import assert from "assert";
import { pathToFileURL } from "url";
const root = pathToFileURL(process.argv[2]).href;
globalThis.self = globalThis;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { locks: { request: (_n, _o, fn) => Promise.resolve().then(fn) } } });
const { createPrefetch } = await import(root + "/rust-prefetch.js");
const { createIndexAhead, sigOf } = await import(root + "/rust-index-ahead.js");
const { createCopyGate } = await import(root + "/rust-carica.js");
const { held, copyMember, storeOf, partOf, PREFIX, MAGIC } = await import(root + "/rust-nativo.js");

function world({ two = false } = {}) {
  const files = new Map([["nat~k~a.wiff", { size: 10 }], ["nat~k2~a.wiff", { size: 10 }]]), idx = new Map(), gates = new Map(), writing = new Map(), taken = new Map();
  let live = 0, peak = 0, builds = 0, cancels = 0; const names = [], fail = { copy: null, pool: null, promote: null }, hold = { copy: null };      // fail.*: a message to throw (storage full); hold.copy: a promise the copy waits for
  const dirs = { files, idx };
  const ARCH = {
    get: async (k, n) => dirs[k].get(n) || null,
    remove: async (k, n) => { dirs[k].delete(n); },
    list: async k => [...dirs[k].keys()].map(name => ({ name })),
    promote: async (k, from, to) => { if (fail.promote && k === "idx" && (fail.promote === "any" || from.endsWith(fail.promote))) throw new Error("QuotaExceededError"); const f = dirs[k].get(from); assert.ok(f, "promote of a missing file " + from); dirs[k].delete(to); dirs[k].set(to, f); dirs[k].delete(from); return f; },      // a move: the same object, no copy
  };
  const gate = n => { if (!gates.has(n)) { let go; const p = new Promise(r => { go = r; }); gates.set(n, { p, go }); } return gates.get(n); };
  const sparse = async (_k, store, size, _r, chunk) => {
    if (hold.copy) await hold.copy; if (fail.copy && store.endsWith(fail.copy)) throw new Error("QuotaExceededError");      // the real copy writes a .part and removes it on an error: nothing under the store name
    files.set(store, { size });      // the copy ends at once; one that fails removes its partial file (putSparse of archivio.js)
    try { await chunk(0, 1); await chunk(1, size); } catch (e) { files.delete(store); throw e; }
  };
  // the pool: a build writes `part`, ends when its gate opens (a test holds it), and answers { ok, stats }; cancel ends a running one as the real pool does
  const pending = new Map();
  const pool = {
    build: async j => {
      builds++; live++; peak = Math.max(peak, live); names.push(j.name);
      if (fail.pool) { live--; throw new Error("QuotaExceededError"); }
      const g = gate(j.name); idx.set(j.part, { size: 1000, bytes: "index-of-" + j.name, job: j.nat.st.id });
      try {
        const r = await Promise.race([g.p.then(() => "go"), new Promise(res => pending.set(j.name, () => res("cut")))]);
        if (r === "cut") { idx.delete(j.part); return { ok: false, reason: "cancelled" }; }
        j.onProgress({ frac: 1 }); return { ok: true, stats: { missing: 0, cause: "" } };
      } finally { live--; pending.delete(j.name); }
    },
    cancel: name => { cancels++; const c = pending.get(name); if (c) c(); },
  };
  const ahead = createIndexAhead({ ARCH, pool: () => pool, conf: () => ({ cap: 0, budget: undefined, isolated: false }), taken, lateJob: async job => job, storeOf, prefix: PREFIX, magic: MAGIC });
  const deps = { ARCH, writing, taken, sparse };
  const pf = createPrefetch({ ARCH, jerr: k => { throw new Error(k); }, held, copy: copyMember, deps, storeOf, partOf, workOf: (nat, x) => { const r = partOf(nat, x); return r ? r.reduce((a, b) => a + b[1] - b[0], 0) : x.blob.size; },
    missing: async () => 0, ensureRoom: async () => {}, release: () => {}, stubOf: async () => null, ahead, now: () => 0 });
  const copies = createCopyGate(), cancelled = () => new Error("cancelled"), posted = [];
  const blob = size => ({ size, lastModified: 7, slice: (o, e) => ({ arrayBuffer: async () => new ArrayBuffer(e - o) }) });
  const members = [{ name: "a.wiff", blob: blob(10) }, { name: "a.wiff.scan", blob: blob(100) }, ...(two ? [{ name: "b.wiff.scan", blob: blob(100) }] : [])];
  const ranges = { "a.wiff.scan": [[0, 50]], ...(two ? { "b.wiff.scan": [[0, 50]] } : {}) };
  const m = (sid, c = sid - 1, key = "k") => ({ op: "natprefetch", id: sid, key, sid, c, tag: String(sid), ranges, members });
  const run = (sid, key = "k") => pf.op(m(sid, sid - 1, key), copies, cancelled, o => posted.push(o)).then(() => "done", e => String(e.message));
  const unrun = (key, sids, end = false) => pf.op({ op: "natunprefetch", key, sids, end }, copies, cancelled, () => {});
  // the stub the opening writes for sample `sid` under a final name that depends on the whole selection
  const stub = (sid, c = sid - 1, key = "k") => ({ mzlab: MAGIC, ds: key, c, id: sid, centroid: false, size: 110, dim: null,
    members: members.map(x => ({ name: x.name, store: storeOf({ key, tag: String(sid), ranges }, x), size: x.blob.size, mtime: 7 })) });
  return { files, idx, gate, pool, ahead, pf, copies, cancelled, run, unrun, m, stub, fail, hold, peak: () => peak, builds: () => builds, cancels: () => cancels, live: () => live, pending: () => pending.size, names, posted };
}
const tick = () => new Promise(r => setTimeout(r, 5));
const scans = w => [...w.files.keys()].filter(n => /\.scan$/.test(n));      // the ranged copies (the whole member `a.wiff` is there from the start)
const nothing = (w, key = "k") => {      // everything of dataset `key` is freed: files, index files, rows, holds, copy slots, pool jobs
  assert.deepStrictEqual(scans(w).filter(n => n.startsWith("nat~" + key + "~")), [], "no copy left");
  assert.deepStrictEqual([...w.idx.keys()].filter(n => n.startsWith("nat~" + key + "~")), [], "no index file left");
  assert.strictEqual(w.ahead.tags(key).length, 0, "no row left"); assert.ok(![...w.pf.holds].some(([st, k]) => k === key && /\.scan$/.test(st)), "no hold on a ranged copy left");      // a hold on the whole member stays until the sweep, as before
};

{ // built right after the copy under a selection-independent name; Open adopts it by a move: the pool is not asked again, the bytes are the same object
  const w = world(); const r = w.run(1); await tick();
  assert.deepStrictEqual([...w.idx.keys()], ["nat~k~1~pre.mzidx.part"], "the index is being built while the dialog is open");
  w.gate("nat~k~1~pre").go(); assert.strictEqual(await r, "done");
  assert.deepStrictEqual([...w.idx.keys()], ["nat~k~1~pre.mzidx"], "built before Open");
  const before = w.idx.get("nat~k~1~pre.mzidx");
  const got = await w.ahead.adopt("S (FILE _1).mzML", w.stub(1));
  assert.deepStrictEqual(got, { missing: 0, cause: "" });
  assert.deepStrictEqual([...w.idx.keys()], ["S (FILE _1).mzML.mzidx"], "moved to the final name, nothing left under the neutral one");
  assert.strictEqual(w.idx.get("S (FILE _1).mzML.mzidx"), before, "the very same bytes, not rebuilt or copied");
  assert.strictEqual(w.builds(), 1, "one build in all"); assert.deepStrictEqual(w.ahead.stats, { built: 1, adopted: 1, missed: 0, atOpen: 0 });
  assert.strictEqual(await w.ahead.adopt("again.mzML", w.stub(1)), null, "taken once: a second stub finds nothing and builds its own");
}
{ // the name of the stub does not matter, only the sample and its stores: the same index adopted under any final name
  assert.strictEqual(sigOf(world().stub(2)), sigOf(world().stub(2)));
  assert.notStrictEqual(sigOf(world().stub(2)), sigOf(world().stub(3)));
}
{ // Open comes while the build still runs: it waits for it and adopts, the pool is not asked a second time
  const w = world(); const r = w.run(1); await tick();
  let adopted = null; const open = w.ahead.adopt("A.mzML", w.stub(1)).then(x => { adopted = x; });
  await tick(); assert.strictEqual(adopted, null, "waits for the build");
  w.gate("nat~k~1~pre").go(); await open; await r;
  assert.ok(adopted); assert.strictEqual(w.builds(), 1); assert.ok(w.idx.has("A.mzML.mzidx"));
}
{ // a stub that is not the sample the index was built for (other position, other stores) adopts nothing and the stale index is removed
  const w = world(); const r = w.run(1); await tick(); w.gate("nat~k~1~pre").go(); await r;
  assert.strictEqual(await w.ahead.adopt("B.mzML", w.stub(1, 5)), null);
  assert.strictEqual(w.idx.size, 0, "the stale index is removed"); assert.strictEqual(w.ahead.stats.missed, 1);
}
{ // a sample whose build failed: the copy is kept, nothing is adopted, the opening builds as before
  const w = world(); const r = w.run(1); await tick(); w.pool.cancel("nat~k~1~pre");
  assert.strictEqual(await r, "done"); assert.strictEqual(w.idx.size, 0); assert.ok(w.files.has("nat~k~1~a.wiff.scan"), "the copy stays");
  assert.strictEqual(await w.ahead.adopt("C.mzML", w.stub(1)), null);
}
{ // untick a sample whose index is being built: the build is cancelled at once (not waited for), the file and the copy are gone, the other sample is untouched
  const w = world(); const rs = [1, 2].map(s => w.run(s)); await tick();
  assert.deepStrictEqual([...w.idx.keys()].sort(), ["nat~k~1~pre.mzidx.part", "nat~k~2~pre.mzidx.part"]);
  await w.pf.op({ op: "natunprefetch", key: "k", sids: [2], end: false }, w.copies, w.cancelled, () => {});
  w.gate("nat~k~1~pre").go(); const out = await Promise.all(rs);
  assert.strictEqual(out[0], "done"); assert.ok(w.cancels() >= 1);
  assert.deepStrictEqual([...w.idx.keys()], ["nat~k~1~pre.mzidx"]); assert.ok(!w.files.has("nat~k~2~a.wiff.scan"), "its copy goes too");
  assert.deepStrictEqual(w.ahead.tags("k"), ["k#1"]);
}
{ // untick a sample whose index is finished: the file goes
  const w = world(); const r = w.run(1); await tick(); w.gate("nat~k~1~pre").go(); await r;
  assert.ok(w.idx.has("nat~k~1~pre.mzidx"));
  await w.pf.op({ op: "natunprefetch", key: "k", sids: [1], end: false }, w.copies, w.cancelled, () => {});
  assert.strictEqual(w.idx.size, 0); assert.ok(!w.files.has("nat~k~1~a.wiff.scan")); assert.strictEqual(w.ahead.tags("k").length, 0);
}
{ // X with some finished, some building, some waiting for a slot: every index and every copy is gone, no entry stays
  const w = world(); const rs = [1, 2, 3, 4].map(s => w.run(s)); await tick(); w.gate("nat~k~1~pre").go(); await tick();
  await w.pf.op({ op: "natunprefetch", key: "k", sids: null, end: true }, w.copies, w.cancelled, () => {});
  await Promise.all(rs);
  assert.strictEqual(w.idx.size, 0, [...w.idx.keys()]); assert.ok(![...w.files.keys()].some(n => n.endsWith(".scan")), [...w.files.keys()]);
  assert.strictEqual(w.ahead.tags("k").length, 0); assert.strictEqual(w.pf.holds.size, 0);
}
{ // with four samples ticked at most two copy-and-index jobs run together (the copy gate holds a slot through the index); the others wait and build in turn
  const w = world(); const rs = [1, 2, 3, 4].map(s => w.run(s)); await tick();
  assert.strictEqual(w.copies.running, 2); assert.strictEqual(w.copies.queued, 2); assert.strictEqual(w.builds(), 2);
  for (const sid of [1, 2, 3, 4]) { w.gate(`nat~k~${sid}~pre`).go(); await tick(); }
  assert.deepStrictEqual(await Promise.all(rs), ["done", "done", "done", "done"]);
  assert.strictEqual(w.peak(), 2, "never more than two builds at once"); assert.strictEqual(w.builds(), 4);
  assert.deepStrictEqual([...w.idx.keys()].sort(), [1, 2, 3, 4].map(s => `nat~k~${s}~pre.mzidx`));
}
{ // a sample asked without its position (c): copied as before, no index is built
  const w = world(); const mm = w.m(1); delete mm.c;
  assert.strictEqual(await w.pf.op(mm, w.copies, w.cancelled, () => {}).then(() => "done"), "done"); assert.strictEqual(w.builds(), 0); assert.strictEqual(w.idx.size, 0);
}
{ // a failure in the middle of the copy (storage full): the copy of the sample is removed, no row, no index, nothing the opening would trip over; it builds by itself
  const w = world({ two: true }); w.fail.copy = "b.wiff.scan";      // the first ranged member is written, the second fails
  assert.match(await w.run(1), /QuotaExceededError/); nothing(w); assert.strictEqual(w.builds(), 0);
  assert.strictEqual(await w.ahead.adopt("D.mzML", w.stub(1)), null, "the normal path");
  w.fail.copy = null; const again = w.run(1); await tick(); w.gate("nat~k~1~pre").go(); assert.strictEqual(await again, "done", "ticked again: it works"); assert.strictEqual(w.builds(), 1);
}
{ // a failure in the middle of the index (storage full while building): the copy is kept (the opening needs it), no index file, no .part; the opening builds by itself
  const w = world(); w.fail.pool = true;
  assert.strictEqual(await w.run(1), "done"); assert.strictEqual(w.idx.size, 0, [...w.idx.keys()]); assert.ok(w.files.has("nat~k~1~a.wiff.scan"), "the copy stays");
  assert.strictEqual(w.live(), 0); assert.strictEqual(await w.ahead.adopt("E.mzML", w.stub(1)), null); assert.strictEqual(w.ahead.tags("k").length, 0);
  w.fail.pool = null; const next = w.run(2); await tick(); w.gate("nat~k~2~pre").go(); assert.strictEqual(await next, "done"); assert.ok(w.idx.has("nat~k~2~pre.mzidx"), "the next sample indexes again");
}
{ // the move of the finished index (.part -> .mzidx) fails: nothing is left under any name; the file of the second move (to the final name) fails: the opening gets null, builds by itself, nothing is left
  const w = world(); w.fail.promote = ".part"; const r = w.run(1); await tick(); w.gate("nat~k~1~pre").go(); assert.strictEqual(await r, "done");
  assert.strictEqual(w.idx.size, 0, [...w.idx.keys()]);
  const v = world(); const r2 = v.run(1); await tick(); v.gate("nat~k~1~pre").go(); await r2; assert.ok(v.idx.has("nat~k~1~pre.mzidx"));
  v.fail.promote = "any"; assert.strictEqual(await v.ahead.adopt("F.mzML", v.stub(1)), null, "the move failed: the opening goes the normal way");
  assert.strictEqual(v.idx.size, 0, "the neutral file is removed too"); assert.strictEqual(v.ahead.tags("k").length, 0);
}
{ // Open comes in the COPY phase (the copy still goes on): the opening takes the sample by itself, nothing is built behind it, no neutral index stays, the copy it needs is there
  const w = world(); let free; w.hold.copy = new Promise(r => { free = r; });
  const r = w.run(1), r2 = w.run(2); await tick(); assert.strictEqual(w.builds(), 0, "still copying");
  assert.strictEqual(await w.ahead.adopt("G.mzML", w.stub(1)), null, "not built yet: the normal path");
  w.hold.copy = null; free(); w.gate("nat~k~2~pre").go();
  assert.deepStrictEqual(await Promise.all([r, r2]), ["done", "done"]);
  assert.strictEqual(w.builds(), 1, "only sample 2 is built, sample 1 was taken by the opening"); assert.deepStrictEqual([...w.idx.keys()], ["nat~k~2~pre.mzidx"], "no orphan index of sample 1");
  assert.ok(w.files.has("nat~k~1~a.wiff.scan"), "the copy stays for the opening"); assert.deepStrictEqual(w.ahead.tags("k"), ["k#2"]);
  assert.deepStrictEqual(w.ahead.stats, { built: 1, adopted: 0, missed: 1, atOpen: 0 });
}
{ // untick in the COPY phase: the copy is stopped, the row goes at once, nothing is left, nothing waits
  const w = world(); let free; w.hold.copy = new Promise(r => { free = r; });
  const r = w.run(1); await tick(); const u = w.unrun("k", [1]); free(); w.hold.copy = null; await u;
  assert.match(await r, /cancel/i); nothing(w); assert.strictEqual(w.builds(), 0); assert.strictEqual(w.copies.running, 0);
}
{ // two dialogs at once (two datasets): at most 2 jobs run together over both; X of one leaves the other's copies, indexes and adoption untouched, and frees all of its own
  const w = world(); const rs = [w.run(1, "k"), w.run(2, "k"), w.run(1, "k2"), w.run(2, "k2")]; await tick();
  assert.strictEqual(w.copies.running, 2); assert.strictEqual(w.copies.queued, 2);
  for (const n of ["nat~k~1~pre", "nat~k~2~pre", "nat~k2~1~pre", "nat~k2~2~pre"]) { w.gate(n).go(); await tick(); }
  assert.deepStrictEqual(await Promise.all(rs), ["done", "done", "done", "done"]); assert.strictEqual(w.peak(), 2, "never more than two builds at once, over both dialogs");
  await w.unrun("k", null, true); nothing(w, "k");
  assert.deepStrictEqual([...w.idx.keys()].sort(), ["nat~k2~1~pre.mzidx", "nat~k2~2~pre.mzidx"], "the other dialog keeps its indexes");
  assert.deepStrictEqual(scans(w).sort(), ["nat~k2~1~a.wiff.scan", "nat~k2~2~a.wiff.scan"], "and its copies");
  assert.ok(await w.ahead.adopt("H.mzML", w.stub(1, 0, "k2")), "and it adopts"); assert.deepStrictEqual(w.ahead.tags("k2"), ["k2#2"]);
  await w.unrun("k2", null, true); nothing(w, "k2"); assert.strictEqual(w.idx.size, 1, "the adopted index is the stub's now: only it is left"); assert.ok(w.idx.has("H.mzML.mzidx"));
  assert.strictEqual(w.copies.running, 0); assert.strictEqual(w.copies.queued, 0); assert.strictEqual(w.pending(), 0); assert.strictEqual(w.live(), 0);
}
{ // X while jobs run and wait, then the dialog is opened again on the same dataset and the samples are ticked again: everything works from a clean state, memory and files back to zero after the second X too
  const w = world(); const rs = [1, 2, 3].map(s => w.run(s)); await tick(); w.gate("nat~k~1~pre").go(); await tick();
  await w.unrun("k", null, true); await Promise.all(rs); nothing(w); assert.strictEqual(w.pending(), 0); assert.strictEqual(w.live(), 0); assert.strictEqual(w.copies.running + w.copies.queued, 0);
  const before = w.builds(); const rs2 = [1, 2].map(s => w.run(s)); await tick();
  assert.strictEqual(w.ahead.tags("k").length, 2); for (const s of [1, 2]) { w.gate(`nat~k~${s}~pre`).go(); await tick(); }
  assert.deepStrictEqual(await Promise.all(rs2), ["done", "done"]); assert.strictEqual(w.builds(), before + 2, "built again, none reused from before X");
  assert.ok(await w.ahead.adopt("I.mzML", w.stub(1)), "and the opening adopts it");
  await w.unrun("k", null, true); nothing(w); assert.strictEqual(w.pending(), 0); assert.strictEqual(w.live(), 0); assert.strictEqual(w.idx.size, 1); assert.ok(w.idx.has("I.mzML.mzidx"));
}
console.log("ok");
