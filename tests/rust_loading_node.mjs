// Node checks of the loading helpers of the Rust engine (rust-centroidi.js, rust-xicbin.js, rust-recycle.js) against fake bridge handles; prints one JSON object. See tests/test_rust_loading.py.
globalThis.self = globalThis;
Object.defineProperty(globalThis, "navigator", { configurable: true, value: { storage: { estimate: async () => ({ quota: 1, usage: 0 }), getDirectory: async () => { throw new Error("no OPFS"); } } } });
const { makeConvertCentroids } = await import("../mzlab/web/rust-centroidi.js");
const { indexXicBin } = await import("../mzlab/web/rust-xicbin.js");
const { makeRecycler } = await import("../mzlab/web/rust-recycle.js");
const out = {};
const outcome = p => p.then(v => ({ ok: true, v }), e => ({ ok: false, cancelled: !!e.cancelled, key: e.json && e.json.error, msg: e.message }));

// C-D (1): Cancel pressed while the quota check is awaited must not start the conversion
{
  const w = {}, calls = { rpc: 0, abort: 0 };
  const h = { current: () => w, base: n => n, abort: () => { calls.abort++; }, rpc: async () => { calls.rpc++; return { none: true }; } };
  const ac = new AbortController();
  const p = outcome(makeConvertCentroids(h)("a.mzML", 1, null, ac.signal));
  ac.abort();
  out.abortInWindow = { ...(await p), ...calls };
}
// C-D (2): the listener asks for the worker that is current when Cancel is pressed, not the one read at the start
{
  const w1 = {}, w2 = {}; let cur = w1; const aborted = [];
  let release;
  const h = { current: () => cur, base: n => n, abort: w => aborted.push(w === w1 ? "w1" : "w2"), rpc: () => new Promise(r => { release = r; }) };
  const ac = new AbortController();
  const p = outcome(makeConvertCentroids(h)("a.mzML", 1, null, ac.signal));
  await new Promise(r => setTimeout(r, 5));
  cur = w2; ac.abort(); release({ cancelled: true });
  out.abortStaleWorker = { ...(await p), aborted };
}
// C-D (3): a failure of the quota check that is not a JSON string (here a TypeError from the size) comes out as itself, not as a SyntaxError of JSON.parse
{
  const h = { current: () => ({}), base: n => n, abort() {}, rpc: async () => ({ none: true }) };
  out.quotaJson = await outcome(makeConvertCentroids(h)("a.mzML", 1e9, null));
  const plain = await makeConvertCentroids(h)("a.mzML", 1n, null).catch(e => e);
  out.plainError = { name: plain.name, json: plain.json || null };
}

// C-F + P8: tol, parallel RPCs, buffers always go back to the pool
const mkBridge = (replies, log) => {
  const slots = { 0: { opening: null, idx: {}, bad: false }, 1: { opening: null, idx: {}, bad: false }, 2: { opening: null, idx: {}, bad: false } };
  let i = 0, inFlight = 0, maxInFlight = 0;
  return {
    slots, stats: { rust: 0 },
    recycle: buf => log.recycled.push(buf),
    rpc: async msg => { log.rpc.push(msg.slot); inFlight++; maxInFlight = Math.max(maxInFlight, inFlight); log.maxInFlight = maxInFlight; await Promise.resolve(); inFlight--; return replies[msg.slot] || { error: "none" }; },
    handle: async () => null,
  };
};
const rep = n => ({ n, len: 2 * n, buf: new Float64Array(Array.from({ length: 2 * n }, (_, j) => j + n * 100)).buffer });
const url = q => new URL("http://x/api/xicbin?" + q);
{
  const log = { rpc: [], recycled: [] };
  const b = mkBridge({}, log);
  out.tolNaN = { res: await indexXicBin(b, url("k=0&mz=300&tol=abc"), 1, 0), rpc: log.rpc.length };
  const log2 = { rpc: [], recycled: [] }, r1 = rep(2), r2 = rep(3);
  const b2 = mkBridge({ 0: r1, 1: { dead: true }, 2: r2 }, log2);
  const res2 = await indexXicBin(b2, url("k=0,1,2&mz=300"), 1, 0);
  out.failureRecycles = { res: res2, recycled: log2.recycled.length, same: log2.recycled.every(x => x === r1.buf || x === r2.buf) };
  const log3 = { rpc: [], recycled: [] }, q1 = rep(2), q2 = rep(1);
  const b3 = mkBridge({ 0: q1, 1: q2, 2: rep(0) }, log3);
  const res3 = await indexXicBin(b3, url("k=2,0,1&mz=300"), 1, 0);
  const dv = new DataView(res3.buf), hl = dv.getUint32(0, true), head = JSON.parse(new TextDecoder().decode(new Uint8Array(res3.buf, 4, hl)));
  out.success = { order: head.traces.map(t => t.k), n: head.traces.map(t => t.n), recycled: log3.recycled.length, rust: b3.stats.rust, parallel: log3.maxInFlight };
}

// C-I: now() honours disabled() and clears the idle timer
{
  const timers = []; const realSet = globalThis.setTimeout, realClear = globalThis.clearTimeout;
  globalThis.setTimeout = fn => { const t = { fn, cleared: false }; timers.push(t); return t; };
  globalThis.clearTimeout = t => { if (t) t.cleared = true; };
  let recycled = 0, off = false;
  const r = makeRecycler({ pending: new Set(), worker: () => ({}), recycleNow: () => { recycled++; }, disabled: () => off, limitMb: () => 1 });
  r.seen(300 * 1048576);
  const armed = timers.length === 1;
  r.now();
  const cleared = timers[0].cleared;
  off = true; r.now();
  globalThis.setTimeout = realSet; globalThis.clearTimeout = realClear;
  out.recycler = { armed, cleared, recycled };
}
console.log(JSON.stringify(out));
