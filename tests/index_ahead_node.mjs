// The index ahead of the «Choose samples» dialog (mzlab/web/rust-index-ahead.js inside the real createPrefetch, copy gate and dataset lock of the engine worker), with a fake OPFS (files/ and idx/)
// and a fake index pool: the index of a ticked sample is built right after its copy, under a name that does not depend on the selection; «Open selected» MOVES it to the final name (the pool is not
// asked again: counted); an opening that comes while it is still being built waits for it; a stub that is not the sample the index was built for adopts nothing; untick and X cancel the build and remove
// the file; at most 2 builds run at once. Fails on any difference, never skips.
import assert from "assert";
import { pathToFileURL } from "url";
const root = pathToFileURL(process.argv[2]).href;
globalThis.self = globalThis;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { locks: { request: (_n, _o, fn) => Promise.resolve().then(fn) } } });
const { createPrefetch } = await import(root + "/rust-prefetch.js");
const { createIndexAhead, sigOf } = await import(root + "/rust-index-ahead.js");
const { createCopyGate } = await import(root + "/rust-carica.js");
const { held, copyMember, storeOf, partOf, PREFIX, MAGIC } = await import(root + "/rust-nativo.js");

function world() {
  const files = new Map([["nat~k~a.wiff", { size: 10 }]]), idx = new Map(), gates = new Map(), writing = new Map(), taken = new Map();
  let live = 0, peak = 0, builds = 0, cancels = 0; const names = [];
  const dirs = { files, idx };
  const ARCH = {
    get: async (k, n) => dirs[k].get(n) || null,
    remove: async (k, n) => { dirs[k].delete(n); },
    list: async k => [...dirs[k].keys()].map(name => ({ name })),
    promote: async (k, from, to) => { const f = dirs[k].get(from); assert.ok(f, "promote of a missing file " + from); dirs[k].delete(to); dirs[k].set(to, f); dirs[k].delete(from); return f; },      // a move: the same object, no copy
  };
  const gate = n => { if (!gates.has(n)) { let go; const p = new Promise(r => { go = r; }); gates.set(n, { p, go }); } return gates.get(n); };
  const sparse = async (_k, store, size, _r, chunk) => { files.set(store, { size }); await chunk(0, 1); await chunk(1, size); };      // the copy ends at once
  // the pool: a build writes `part`, ends when its gate opens (a test holds it), and answers { ok, stats }; cancel ends a running one as the real pool does
  const pending = new Map();
  const pool = {
    build: async j => {
      builds++; live++; peak = Math.max(peak, live); names.push(j.name);
      const g = gate(j.name); idx.set(j.part, { size: 1000, bytes: "index-of-" + j.name, job: j.nat.st.id });
      try {
        const r = await Promise.race([g.p.then(() => "go"), new Promise(res => pending.set(j.name, () => res("cut")))]);
        if (r === "cut") { idx.delete(j.part); return { ok: false, reason: "cancelled" }; }
        j.onProgress({ frac: 1 }); return { ok: true, stats: { missing: 0, cause: "" } };
      } finally { live--; }
    },
    cancel: name => { cancels++; const c = pending.get(name); if (c) c(); },
  };
  const ahead = createIndexAhead({ ARCH, pool: () => pool, conf: () => ({ cap: 0, budget: undefined, isolated: false }), taken, lateJob: async job => job, storeOf, prefix: PREFIX, magic: MAGIC });
  const deps = { ARCH, writing, taken, sparse };
  const pf = createPrefetch({ ARCH, jerr: k => { throw new Error(k); }, held, copy: copyMember, deps, storeOf, partOf, workOf: (nat, x) => { const r = partOf(nat, x); return r ? r.reduce((a, b) => a + b[1] - b[0], 0) : x.blob.size; },
    missing: async () => 0, ensureRoom: async () => {}, release: () => {}, stubOf: async () => null, ahead, now: () => 0 });
  const copies = createCopyGate(), cancelled = () => new Error("cancelled"), posted = [];
  const blob = size => ({ size, lastModified: 7, slice: (o, e) => ({ arrayBuffer: async () => new ArrayBuffer(e - o) }) });
  const members = [{ name: "a.wiff", blob: blob(10) }, { name: "a.wiff.scan", blob: blob(100) }];
  const m = (sid, c = sid - 1) => ({ op: "natprefetch", id: sid, key: "k", sid, c, tag: String(sid), ranges: { "a.wiff.scan": [[0, 50]] }, members });
  const run = sid => pf.op(m(sid), copies, cancelled, o => posted.push(o)).then(() => "done", e => String(e.message));
  // the stub the opening writes for sample `sid` under a final name that depends on the whole selection
  const stub = (sid, c = sid - 1) => ({ mzlab: MAGIC, ds: "k", c, id: sid, centroid: false, size: 110, dim: null,
    members: members.map(x => ({ name: x.name, store: storeOf({ key: "k", tag: String(sid), ranges: { "a.wiff.scan": [[0, 50]] } }, x), size: x.blob.size, mtime: 7 })) });
  return { files, idx, gate, pool, ahead, pf, copies, cancelled, run, m, stub, peak: () => peak, builds: () => builds, cancels: () => cancels, names, posted };
}
const tick = () => new Promise(r => setTimeout(r, 5));

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
  const w = world(); const rs = [1, 2].map(w.run); await tick();
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
  const w = world(); const rs = [1, 2, 3, 4].map(w.run); await tick(); w.gate("nat~k~1~pre").go(); await tick();
  await w.pf.op({ op: "natunprefetch", key: "k", sids: null, end: true }, w.copies, w.cancelled, () => {});
  await Promise.all(rs);
  assert.strictEqual(w.idx.size, 0, [...w.idx.keys()]); assert.ok(![...w.files.keys()].some(n => n.endsWith(".scan")), [...w.files.keys()]);
  assert.strictEqual(w.ahead.tags("k").length, 0); assert.strictEqual(w.pf.holds.size, 0);
}
{ // with four samples ticked at most two copy-and-index jobs run together (the copy gate holds a slot through the index); the others wait and build in turn
  const w = world(); const rs = [1, 2, 3, 4].map(w.run); await tick();
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
console.log("ok");
