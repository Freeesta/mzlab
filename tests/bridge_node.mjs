// Drives createBridge (mzlab/web/rust-bridge.js) with a scripted fake Worker: D1 (a stale open must not touch a respawned slot) and V8 (xic/mrm requests overlap).
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const workers = [];
class FakeWorker {
  constructor(url) { this.log = []; this.terminated = false; if (String(url).endsWith("rust-worker.js")) workers.push(this); }   // the memory probe starts workers of its own
  postMessage(m) { this.log.push(m); if (this.auto) this.auto(m, this); }
  terminate() { this.terminated = true; }
  reply(m) { this.onmessage({ data: m }); }
}
Object.assign(globalThis, { Worker: FakeWorker, window: globalThis, self: globalThis, MZLAB_DESKTOP: true, fetch: () => Promise.reject(new Error("no net")) });
const { createBridge } = await import(pathToFileURL(path.join(root, "mzlab/web/rust-bridge.js")).href);

const sleep = ms => new Promise(r => setTimeout(r, ms));
const until = async f => { for (let i = 0; i < 400 && !f(); i++) await sleep(5); assert.ok(f(), "timeout"); };
const sent = (w, op) => w.log.filter(m => m.op === op);
const blob = new Blob(["x"]);
function make() {
  workers.length = 0;
  const notes = [];
  const b = createBridge({ workerUrl: new URL("file:///x/rust-worker.js"), notify: k => notes.push(k) });
  b.start();
  return { b, notes };
}
const session = (b, n) => { b.noteUpload("a.mzML", blob); b.noteExplore({ samples: Array.from({ length: n }, () => ({ file: "a.mzML" })) }); };

const out = {};

// D1a: the worker dies while idxopen is pending; the stale answer must not open the mzML nor mark the slot bad
{
  const { b } = make();
  session(b, 1);
  const w1 = workers[0];
  await until(() => sent(w1, "idxopen").length === 1);
  w1.onerror({ message: "boom" });                                   // died(): restart -> respawn() -> new open on worker 2
  const w2 = workers[1];
  await until(() => sent(w2, "idxopen").length === 1);
  await sleep(20);
  const opens = sent(w2, "open").length;
  w2.auto = m => { if (m.op === "open") w2.reply({ id: m.id, error: JSON.stringify({ error_key: "error.memoryBudget" }) }); };
  w2.reply({ id: sent(w2, "idxopen")[0].id, info: JSON.stringify({ scans: 3 }) });
  assert.equal(await b.ready([0]), true, "D1: the new open finds the index, the slot must be usable");
  out.d1_opens = opens + sent(w2, "open").length;
  assert.equal(out.d1_opens, 0, "D1: no mzML open for a stale answer");

}

// D1b: the engine is switched off for good: the stale open keeps its fallback (a dead rpc marks the slot bad, the mzML open is still attempted)
{
  const { b, notes } = make();
  session(b, 1);
  const w1 = workers[0];
  await until(() => sent(w1, "idxopen").length === 1);
  w1.onerror({ message: "boom" });
  const w2 = workers[1];
  await until(() => sent(w2, "idxopen").length === 1);
  w2.onerror({ message: "boom again" });                             // second death within a minute: off
  await sleep(20);
  assert.ok(notes.includes("error.engineFallback"));
  assert.equal(b.stats.mzml, 1, "D1: the fallback of the open that survives is kept");
  out.d1_off_fallback = b.stats.mzml;
}

// V8: the requests of /api/xic and /api/mrm overlap, the parts keep the order of k, the first error gives null
for (const route of ["xic", "mrm"]) {
  const { b } = make();
  session(b, 4);
  const w = workers[0];
  const op = route === "xic" ? "idxxic" : "mrm";
  await until(() => sent(w, "idxopen").length === 4);
  for (const m of sent(w, "idxopen")) w.reply({ id: m.id, info: JSON.stringify({ scans: 1 }) });
  await b.ready([0, 1, 2, 3]);
  let inflight = 0, peak = 0;
  const held = [];
  w.auto = m => { if (m.op === op) { inflight++; peak = Math.max(peak, inflight); held.push(m); } };
  const url = route === "xic" ? "api/xic?k=3,0,2,1&mz=100&tol=0.35" : "api/mrm?k=3,0,2,1";
  const p = b.handle("GET", url, {});
  await until(() => held.length > 0);
  await sleep(30);
  out[route + "_peak_inflight"] = peak;
  assert.equal(peak, 4, `V8: all four ${op} requests must be in flight together`);
  for (const m of [...held].reverse()) { inflight--; w.reply({ id: m.id, info: route === "xic" ? `{"s":${m.slot}}` : `[${m.slot}]` }); }   // answers out of order
  const r = await p;
  const want = route === "xic"
    ? `{"mz":100,"tol":0.35,"traces":[{"k":3,"s":3},{"k":0,"s":0},{"k":2,"s":2},{"k":1,"s":1}]}`
    : `{"files":[{"k":3,"transitions":[3]},{"k":0,"transitions":[0]},{"k":2,"transitions":[2]},{"k":1,"transitions":[1]}]}`;
  assert.equal(r.text, want);
  // first error -> null
  held.length = 0;
  const p2 = b.handle("GET", url, {});
  await until(() => held.length === 4);
  held.forEach((m, i) => w.reply(i === 1 ? { id: m.id, error: "x" } : { id: m.id, info: "[1]" }));
  assert.equal(await p2, null);
}
console.log(JSON.stringify(out));
process.exit(0);
