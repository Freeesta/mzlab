// The copy ahead of the «Choose samples» dialog (mzlab/web/rust-prefetch.js) with a fake OPFS and the engine's real copy gate: at most 2 copies at once, unticking one sample stops only its copy,
// X stops the running ones and drops the queued ones, a store a stub points to is never removed, a prefetch that landed is kept from the sweep until its stub is written.
import assert from "assert";
import { pathToFileURL } from "url";
const root = pathToFileURL(process.argv[2]).href;
const { createPrefetch } = await import(root + "/rust-prefetch.js");
const { createCopyGate } = await import(root + "/rust-carica.js");

function world() {
  const files = new Map(), gates = new Map(), stubs = new Map();
  let live = 0, peak = 0; const started = [];
  const ARCH = { get: async (_k, n) => files.get(n) || null, remove: async (_k, n) => { files.delete(n); }, list: async () => [...files.keys()].map(name => ({ name })) };
  const open = n => { if (!gates.has(n)) { let go; const p = new Promise(r => { go = r; }); gates.set(n, { p, go }); } return gates.get(n); };
  const copy = async (_d, blob, store, bump, ranges) => {
    live++; peak = Math.max(peak, live); started.push(store);
    try { bump(1); if (ranges) await open(store).p; bump(blob.size); files.set(store, { size: blob.size }); } finally { live--; }
  };
  const posted = [];
  const pf = createPrefetch({ ARCH, jerr: k => { throw new Error(k); }, held: (_k, fn) => fn(), copy, deps: { writing: new Map() },
    storeOf: (nat, x) => nat.ranges[x.name] ? `nat~${nat.key}~${nat.tag}~${x.name}` : `nat~${nat.key}~${x.name}`, partOf: (nat, x) => nat.ranges[x.name], workOf: (_n, x) => x.blob.size,
    missing: async () => 0, ensureRoom: async () => {}, release: () => {}, stubOf: async n => stubs.get(n) || null, now: () => 0 });
  const copies = createCopyGate(), cancelled = () => new Error("cancelled");
  const m = (sid, op = "natprefetch") => ({ op, id: sid, key: "k", sid, tag: String(sid), ranges: { "a.wiff.scan": [[0, 50]] }, members: [{ name: "a.wiff", blob: { size: 10 } }, { name: "a.wiff.scan", blob: { size: 100 } }] });
  const run = sid => pf.op(m(sid), copies, cancelled, o => posted.push(o)).then(() => "done", e => String(e.message));
  return { files, gates, stubs, pf, copies, cancelled, posted, started, run, m, peak: () => peak, open };
}
const tick = () => new Promise(r => setTimeout(r, 5));

{ // at most 2 at once; the other two wait and say so; all land under their own stores
  const w = world(); const rs = [1, 2, 3, 4].map(w.run); await tick();
  assert.strictEqual(w.copies.running, 2); assert.strictEqual(w.copies.queued, 2); assert.strictEqual(w.posted.filter(p => p.wait).length, 2);
  for (const sid of [1, 2, 3, 4]) { w.open(`nat~k~${sid}~a.wiff.scan`).go(); await tick(); }
  assert.deepStrictEqual(await Promise.all(rs), ["done", "done", "done", "done"]); assert.ok(w.peak() <= 2, w.peak());
  assert.ok([1, 2, 3, 4].every(s => w.files.has(`nat~k~${s}~a.wiff.scan`)));
}
{ // unticking sample 2: its copy stops, its store is gone, the others stay
  const w = world(); const rs = [1, 2].map(w.run); await tick();
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
console.log("ok");
