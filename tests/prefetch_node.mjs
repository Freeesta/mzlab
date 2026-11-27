// The copy ahead of the «Choose samples» dialog (mzlab/web/rust-prefetch.js) with a fake OPFS, the engine's real copy gate, the real dataset lock wrapper (`held` of rust-nativo.js, over a fake Web Locks
// with shared and exclusive modes) and the real `copyMember`: at most 2 copies at once and two of one pair run side by side, unticking one sample (also halfway through its copy) stops only its copy and removes its
// store, X stops the running ones and drops the queued ones, a store a stub points to is never removed, and an opening that asks for a store being copied waits for that copy and writes nothing again.
import assert from "assert";
import { pathToFileURL } from "url";
const root = pathToFileURL(process.argv[2]).href;
globalThis.self = globalThis;
let shared = 0, peakShared = 0, exclusive = false;      // Web Locks as the browser has them: shared holders run together, an exclusive one runs alone
const waiting = [];
const grant = () => { while (waiting.length && !exclusive && (waiting[0].mode === "shared" || shared === 0)) { const w = waiting.shift(); w.go(); if (w.mode !== "shared") break; } };
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { locks: { request: (_n, { mode }, fn) => new Promise((res, rej) => {
  waiting.push({ mode, go: () => { if (mode === "shared") { shared++; peakShared = Math.max(peakShared, shared); } else exclusive = true; Promise.resolve().then(fn).then(res, rej).finally(() => { if (mode === "shared") shared--; else exclusive = false; grant(); }); } }); grant(); }) } } });
const { createPrefetch } = await import(root + "/rust-prefetch.js");
const { createCopyGate } = await import(root + "/rust-carica.js");
const { held, copyMember, storeOf, partOf } = await import(root + "/rust-nativo.js");

function world() {
  const files = new Map([["nat~k~a.wiff", { size: 10 }]]), gates = new Map(), stubs = new Map(), writing = new Map();
  let live = 0, peak = 0, sparseCalls = 0; const started = [];
  const ARCH = { get: async (_k, n) => files.get(n) || null, remove: async (_k, n) => { files.delete(n); }, list: async () => [...files.keys()].map(name => ({ name })) };
  const open = n => { if (!gates.has(n)) { let go; const p = new Promise(r => { go = r; }); gates.set(n, { p, go }); } return gates.get(n); };
  // a sparse copy creates its store at once (as putSparse does) and fills it when the gate opens, in chunks that report to `bump`
  const sparse = async (_kind, store, size, _ranges, chunk) => {
    sparseCalls++; live++; peak = Math.max(peak, live); started.push(store); files.set(store, { size });
    try { await chunk(0, 1); await open(store).p; await chunk(1, size); } catch (e) { files.delete(store); throw e; } finally { live--; }      // the partial file is removed if something fails, as putSparse does
  };
  const deps = { ARCH, writing, taken: new Map(), sparse };
  const pf = createPrefetch({ ARCH, jerr: k => { throw new Error(k); }, held, copy: copyMember, deps, storeOf, partOf, workOf: (nat, x) => { const r = partOf(nat, x); return r ? r.reduce((a, b) => a + b[1] - b[0], 0) : x.blob.size; },
    missing: async () => 0, ensureRoom: async () => {}, release: () => {}, stubOf: async n => stubs.get(n) || null, now: () => 0 });
  const copies = createCopyGate(), cancelled = () => new Error("cancelled"), posted = [];
  const blob = size => ({ size, slice: (o, e) => ({ arrayBuffer: async () => new ArrayBuffer(e - o) }) });
  const m = (sid, op = "natprefetch") => ({ op, id: sid, key: "k", sid, tag: String(sid), ranges: { "a.wiff.scan": [[0, 50]] }, members: [{ name: "a.wiff", blob: blob(10) }, { name: "a.wiff.scan", blob: blob(100) }] });
  const run = sid => pf.op(m(sid), copies, cancelled, o => posted.push(o)).then(() => "done", e => String(e.message));
  return { files, gates, stubs, pf, copies, cancelled, posted, started, run, m, deps, blob, peak: () => peak, sparseCalls: () => sparseCalls, open };
}
const tick = () => new Promise(r => setTimeout(r, 5));

{ // at most 2 at once, and the two of ONE pair run side by side (the dataset lock is shared): the other two wait and say so; all land under their own stores
  const w = world(); const rs = [1, 2, 3, 4].map(w.run); await tick();
  assert.strictEqual(w.copies.running, 2); assert.strictEqual(w.copies.queued, 2); assert.strictEqual(w.posted.filter(p => p.wait).length, 2);
  assert.strictEqual(w.peak(), 2, "two copies of one pair are not serialised by the dataset lock"); assert.strictEqual(peakShared, 2);
  for (const sid of [1, 2, 3, 4]) { w.open(`nat~k~${sid}~a.wiff.scan`).go(); await tick(); }
  assert.deepStrictEqual(await Promise.all(rs), ["done", "done", "done", "done"]); assert.ok(w.peak() <= 2, w.peak()); assert.ok(peakShared <= 2, peakShared);
  assert.ok([1, 2, 3, 4].every(s => w.files.has(`nat~k~${s}~a.wiff.scan`)));
}
{ // unticking sample 2 HALFWAY through its copy (its store exists, half written): the copy stops, the store is gone, the other one stays
  const w = world(); const rs = [1, 2].map(w.run); await tick();
  assert.ok(w.files.has("nat~k~2~a.wiff.scan"), "the copy had begun");
  const un = w.pf.op({ op: "natunprefetch", key: "k", sids: [2], end: false }, w.copies, w.cancelled, () => {}); w.open("nat~k~2~a.wiff.scan").go(); w.open("nat~k~1~a.wiff.scan").go(); await un;
  const out = await Promise.all(rs); assert.strictEqual(out[0], "done"); assert.strictEqual(out[1], "err.load.cancelled");
  assert.ok(w.files.has("nat~k~1~a.wiff.scan") && !w.files.has("nat~k~2~a.wiff.scan"));
}
{ // X: the running ones stop, the queued ones never start, nothing is left and no hold stays
  const w = world(); const rs = [1, 2, 3, 4].map(w.run); await tick();
  const un = w.pf.op({ op: "natunprefetch", key: "k", sids: null, end: true }, w.copies, w.cancelled, () => {}); for (const s of [1, 2]) w.open(`nat~k~${s}~a.wiff.scan`).go(); await un;
  const out = await Promise.all(rs); assert.ok(out.every(x => x !== "done"), out);
  assert.ok(![...w.files.keys()].some(n => n.endsWith(".scan")), [...w.files.keys()]); assert.strictEqual(w.pf.holds.size, 0);
  assert.ok(!w.started.some(s => s.includes("~3~") || s.includes("~4~")), w.started);
}
{ // a finished prefetch is held from the sweep; X removes the one no stub uses and keeps the one a stub points to
  const w = world(); const r1 = w.run(1), r2 = w.run(2); await tick(); w.open("nat~k~1~a.wiff.scan").go(); w.open("nat~k~2~a.wiff.scan").go(); await Promise.all([r1, r2]);
  assert.ok(w.pf.holds.has("nat~k~1~a.wiff.scan"));
  w.stubs.set("one.mzML", { members: [{ store: "nat~k~1~a.wiff.scan" }, { store: "nat~k~a.wiff" }] }); w.files.set("one.mzML", { size: 1 }); w.pf.landed(w.stubs.get("one.mzML").members);      // putHeld tells the prefetch when it writes the stub
  await w.pf.unprefetch({ key: "k", sids: null, end: true });
  assert.ok(w.files.has("nat~k~1~a.wiff.scan"), "a store a stub uses stays"); assert.ok(!w.files.has("nat~k~2~a.wiff.scan"), "an unused one goes"); assert.strictEqual(w.pf.holds.size, 0);
}
{ // Open while the copy is still running: the opening's copy of the same store waits for it (`writing`), writes nothing again and reports the whole member as done
  const w = world(); const r1 = w.run(1); await tick();
  assert.strictEqual(w.sparseCalls(), 1); const store = "nat~k~1~a.wiff.scan";
  let done = 0; const opening = copyMember(w.deps, w.blob(100), store, n => { done += n; }, [[0, 50]]).then(() => "opened");
  await tick(); assert.strictEqual(w.sparseCalls(), 1, "no second write while the first runs"); assert.strictEqual(done, 0);
  w.open(store).go(); assert.strictEqual(await opening, "opened"); assert.strictEqual(await r1, "done");
  assert.strictEqual(w.sparseCalls(), 1, "the store was written once"); assert.strictEqual(done, 100);
  const again = await copyMember(w.deps, w.blob(100), store, () => {}, [[0, 50]]); assert.strictEqual(again, undefined); assert.strictEqual(w.sparseCalls(), 1, "a finished copy is found in place");
  w.pf.landed([{ store }, { store: "nat~k~a.wiff" }]); assert.strictEqual(w.pf.holds.size, 0);      // the stub is written (or the opening failed): the sweep decides from the stubs now
}
console.log("ok");
