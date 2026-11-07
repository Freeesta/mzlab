// @ts-strict
// @ts-check
// Pool of short-lived Rust workers that build the .mzidx of the uploaded mzML files (idx-worker.js). Used by rust-worker.js, the engine's worker.
//  - N workers at most (min(cores - 1, 4); 2 on Safari/iPad; at least 1), one file each, started in the order of the queue (prioritize() puts the list's order first);
//  - supervisor: a worker that dies is replaced and its file goes back to the front of the queue (once; a second death fails the file with reason "crash");
//  - memory: a builder that cannot reserve memory (error.memoryBudget without `fase`) halves the pool and the budget and retries once, until the pool is idle;
//    a planned refusal (with `fase`, the builder's estimate) fails the file at once with its numbers (its row says it); a lowering is told once (notice error.poolLowered);
//  - no WASM / no OPFS: unavailable() is true and every job answers { ok: false, reason: "unavailable" } (the upload fails with an error key).
// The same pool runs other short-lived workers (a worker of its own): a job may carry its own message to the worker (`msg`, with `transfer`), an `onProgress(m)`
// for the { op: "progress" } messages of the worker, and the worker's `done` message may carry `parts` (the result's `parts`).
//  - helpers: with cross-origin isolation a builder gets the places of the pool no other file uses, as helper workers that decode its arrays (idx-aiuti.js).
// A job's result: { ok: true, stats, parts } or { ok: false, reason: "unavailable" | "cancelled" | "crash" | "memory" | "error", text }.
export function poolSize(small, cores) {
  const n = Math.max(1, Math.min((cores || 2) - 1, 4));
  return small ? Math.min(n, 2) : n;
}

// Helper workers for a builder that starts while `used` workers (itself included) run in a pool of `size` (poolSize: cores - 1, at most 4): alone, it gets
// `size` helpers (one per core beside its own, at most 4); with others running, the free places; never one alone (it costs more than it gives), none without shared memory.
export function aiutiPer(size, used, isolated) {
  const n = !isolated ? 0 : used <= 1 ? size : size - used;
  return n >= 2 ? n : 0;
}

// The workers of calculation of ALL the pools of one engine (index builders, .wiff conversions) count against one gate: at most `size` run together
// (2 on iPad/phone), and a pool that frees a place wakes the others that wait for it.
export function makeGate(size) {
  return { size: Math.max(1, size), used: 0, pumps: /** @type {Array<() => void>} */ ([]) };
}

// The peak budget of one index builder in MB: its share of the engine's budget (`engineMb`, 0 = not chosen yet: undefined, the pool's default) with the
// `slots` workers of the pool, never below the default (32 on a tablet or phone, 64) nor above 512. Rounds read from the scratch file stay at 32 MB whatever
// it is (`ROUND_MAX`): a larger budget only raises the densest 1-Da range a file may have (`rust-mzidx.md`).
export function builderBudget(engineMb, small, slots) {
  return engineMb > 0 ? Math.min(512, Math.max(small ? 32 : 64, Math.floor(engineMb / (Math.max(1, slots) + 1)))) : undefined;
}

// The error of a builder: "memory" = a reservation failed (error.memoryBudget without an estimate: fewer workers and a smaller budget may pass, until the pool
// is idle again), "refused" = the builder's own estimate (with `fase`) says the file does not fit: the same at any budget not larger, so it fails at once with its numbers.
export function errorKind(text) {
  let err = /** @type {any} */ ({}); try { err = JSON.parse(text) || {}; } catch (_) { /* not JSON */ }
  if (err.error_key !== "error.memoryBudget") return "error";
  return err.params && err.params.fase ? "refused" : "memory";
}

// isolated: workers can share memory (crossOriginIsolated); aiuti: a fixed number of helpers per builder in place of the free places (tests).
// moduleUrl: the .wasm of the builder, compiled ONCE here (compileStreaming) and handed to every worker as a WebAssembly.Module, so that a worker that lives for one file
// does not compile it again (without it the worker compiles by itself, as the test workers do).
// module: the WebAssembly.Module the engine already compiled (used as is); gate: the shared limit of workers (makeGate).
export function createPool({ workerUrl, size, budget = 64, isolated = false, aiuti = /** @type {number | null} */ (null), notify = /** @type {(key?: string, params?: object) => void} */ (() => {}), WorkerClass = Worker, moduleUrl = /** @type {any} */ (null), module = /** @type {any} */ (null), gate = /** @type {any} */ (null), offMs = 5000 }) {
  let limit = size, active = 0, off = false, offAt = 0, compiled = module ? Promise.resolve(module) : null;
  const getModule = () => compiled || (compiled = (async () => {
    try { return await WebAssembly.compileStreaming(fetch(moduleUrl)); }
    catch (_) { try { return await WebAssembly.compile(await (await fetch(moduleUrl)).arrayBuffer()); } catch (e) { compiled = null; return null; } }
  })());
  const queue = [], running = new Map();          // running: job -> worker
  const finish = (job, res) => { if (job.done) return; job.done = true; job.resolve(res); };
  function wake() { pump(); if (gate) for (const p of gate.pumps) if (p !== pump) p(); }
  function pump() {
    if (!active && !queue.length) limit = size;      // idle: a lowering after a failed reservation lasts only while that loading runs
    while (active < limit && (!gate || gate.used < gate.size) && queue.length) {
      if (off && performance.now() - offAt <= offMs) { finish(queue.shift(), { ok: false, reason: "unavailable" }); continue; }
      off = false;       // a start that failed twice is not final: after offMs the next job tries a fresh worker
      run(queue.shift());
    }
  }
  function run(job) {
    active++; if (gate) gate.used++;
    let w, ended = false, inited = false;           // per run: a late event of a worker already ended must not end it twice
    const end = () => { if (ended) return; ended = true; if (running.get(job) === w) running.delete(job); try { w.terminate(); } catch (_) { /* gone */ } active--; if (gate) gate.used--; };
    // the worker is lost: once more at the front of the queue, then the error
    const retry = (reason, text) => {
      end();
      if (!job.tried) { job.tried = true; queue.unshift(job); } else finish(job, { ok: false, reason, text });
      wake();
    };
    const noStart = (/** @type {string} */ text) => { end(); if (!job.started) { job.started = true; queue.unshift(job); } else { off = true; offAt = performance.now(); finish(job, { ok: false, reason: "unavailable", text }); } wake(); };      // a worker that does not start (module or .wasm not loaded): one more try on a fresh worker, then the pool is off for offMs
    try { w = new WorkerClass(workerUrl, { type: "module" }); } catch (e) { end(); off = true; offAt = performance.now(); finish(job, { ok: false, reason: "unavailable", text: String(e) }); return wake(); }
    running.set(job, w);
    (moduleUrl || module ? getModule() : Promise.resolve(null)).then(module => { if (!job.done) { try { w.postMessage({ op: "module", module }); } catch (_) { /* gone */ } } });
    w.onerror = e => { if (job.done || ended) return; if (inited) retry("crash", e.message); else noStart(e.message); };
    w.onmessage = ev => {
      const m = ev.data;
      if (job.done || ended) return;
      if (m.op === "ready") { inited = true; w.postMessage(job.msg || { blob: job.blob, inputs: job.inputs, prep: job.prep, part: job.part, partCen: job.partCen, budget: job.budget || budget, cap: job.cap, nat: job.nat, name: job.name, aiuti: job.nat ? 0 : aiuti ?? aiutiPer(limit, gate ? gate.used : active, isolated) }, job.transfer || []); return; }   // aiuti: the helpers of an mzML builder (aiutiPer)
      if (m.op === "progress") { if (job.onProgress) job.onProgress(m); return; }
      if (m.op === "unavailable") return noStart(m.text);
      if (m.op === "done") { end(); finish(job, { ok: true, stats: m.stats, ms: m.ms, parts: m.parts }); wake(); return; }
      if (m.op === "error") {
        if (m.fatal) return retry("crash", m.text);
        end();
        const kind = errorKind(m.text);
        if (kind !== "memory" && kind !== "refused") { finish(job, { ok: false, reason: "error", text: m.text }); return wake(); }
        if (kind === "refused") { finish(job, { ok: false, reason: "memory", text: m.text }); return wake(); }      // the file's own row gives the numbers: no notice
        const was = limit; limit = Math.max(1, limit >> 1);
        if (!job.shrunk && was > 1) { job.shrunk = true; job.budget = Math.max(8, (job.budget || budget) >> 1); queue.unshift(job); }
        else finish(job, { ok: false, reason: "memory", text: m.text });
        if (limit < was) notify("error.poolLowered", { from: was, to: limit, mb: job.budget || budget });      // one calm notice per lowering (logged in Diagnostics)
        wake();
      }
    };
  }
  if (gate) gate.pumps.push(pump);
  return {
    // job: { name, blob, inputs | prep: { size, prints }, part, budget? } (prep: the worker makes the scan table itself) or { name, nat, part } (a dataset of a maker, rust-nativo.js) or { name, msg, transfer?, onProgress? }
    build(job) {
      return new Promise(resolve => {
        const j = { ...job, resolve, tried: false, done: false };
        queue.push(j); wake();
        // no free worker: the page says the file is queued, not stuck
        if (queue.includes(j) && j.onProgress) j.onProgress({ op: "progress", wait: true });
      });
    },
    // the order of the list: the files named first are the first to start (the queued ones; those already running continue)
    prioritize(names) {
      const rank = n => { const i = names.indexOf(n); return i < 0 ? names.length : i; };
      queue.sort((a, b) => rank(a.name) - rank(b.name));
    },
    cancel(name) {
      for (let i = queue.length - 1; i >= 0; i--) if (queue[i].name === name) finish(queue.splice(i, 1)[0], { ok: false, reason: "cancelled" });
      for (const [job, w] of [...running]) if (job.name === name) { running.delete(job); try { w.terminate(); } catch (_) { /* gone */ } active--; if (gate) gate.used--; finish(job, { ok: false, reason: "cancelled" }); }
      wake();
    },
    unavailable: () => off,
    get limit() { return limit; },
    get active() { return active; },
    get queued() { return queue.length; },
  };
}
