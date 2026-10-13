// @ts-strict
// @ts-check
// Pool of short-lived Rust workers that build the .mzidx of the uploaded mzML files (idx-worker.js). Used by rust-worker.js, the engine's worker.
//  - N workers at most (min(cores - 1, 4); 2 on Safari/iPad; at least 1), one file each, started in the order of the queue (prioritize() puts the list's order first);
//  - supervisor: a worker that dies is replaced and its file goes back to the front of the queue (once; a second death fails the file with reason "crash");
//  - memory: a builder that cannot reserve memory (error.memoryBudget) halves the pool and the budget and retries once; the page is told (notice);
//  - no WASM / no OPFS: unavailable() is true and every job answers { ok: false, reason: "unavailable" } (the upload fails with an error key).
// The same pool runs other short-lived workers (wiff-worker.js, the .wiff -> mzML conversion): a job may carry its own message to the worker (`msg`, with `transfer`), an `onProgress(m)`
// for the { op: "progress" } messages of the worker, and the worker's `done` message may carry `parts` (the result's `parts`).
// A job's result: { ok: true, stats, parts } or { ok: false, reason: "unavailable" | "cancelled" | "crash" | "memory" | "error", text }.
export function poolSize(small, cores) {
  const n = Math.max(1, Math.min((cores || 2) - 1, 4));
  return small ? Math.min(n, 2) : n;
}

// moduleUrl: the .wasm of the builder, compiled ONCE here (compileStreaming) and handed to every worker as a WebAssembly.Module, so that a worker that lives for one file
// does not compile it again (without it the worker compiles by itself, as the test workers do).
export function createPool({ workerUrl, size, budget = 64, notify = /** @type {(key?: string) => void} */ (() => {}), WorkerClass = Worker, moduleUrl = /** @type {any} */ (null) }) {
  let limit = size, active = 0, off = false, compiled = null;
  const getModule = () => compiled || (compiled = (async () => {
    try { return await WebAssembly.compileStreaming(fetch(moduleUrl)); }
    catch (_) { try { return await WebAssembly.compile(await (await fetch(moduleUrl)).arrayBuffer()); } catch (e) { compiled = null; return null; } }
  })());
  const queue = [], running = new Map();          // running: job -> worker
  const finish = (job, res) => { if (job.done) return; job.done = true; job.resolve(res); };

  function pump() {
    while (active < limit && queue.length) {
      if (off) { finish(queue.shift(), { ok: false, reason: "unavailable" }); continue; }
      run(queue.shift());
    }
  }
  function run(job) {
    active++;
    let w;
    const end = () => { if (running.get(job) === w) running.delete(job); try { w.terminate(); } catch (_) { /* gone */ } active--; };
    const retry = (reason, text) => {                          // the worker is lost: once more at the front of the queue, then the error
      end();
      if (!job.tried) { job.tried = true; queue.unshift(job); } else finish(job, { ok: false, reason, text });
      pump();
    };
    try { w = new WorkerClass(workerUrl, { type: "module" }); } catch (e) { end(); off = true; finish(job, { ok: false, reason: "unavailable", text: String(e) }); return pump(); }
    running.set(job, w);
    (moduleUrl ? getModule() : Promise.resolve(null)).then(module => { if (!job.done) { try { w.postMessage({ op: "module", module }); } catch (_) { /* gone */ } } });
    w.onerror = e => { if (job.done) return; if (job.inited) retry("crash", e.message); else { end(); off = true; finish(job, { ok: false, reason: "unavailable", text: e.message }); pump(); } };
    w.onmessage = ev => {
      const m = ev.data;
      if (job.done) return;
      if (m.op === "ready") { job.inited = true; w.postMessage(job.msg || { blob: job.blob, inputs: job.inputs, part: job.part, budget: job.budget || budget }, job.transfer || []); return; }
      if (m.op === "progress") { if (job.onProgress) job.onProgress(m); return; }
      if (m.op === "unavailable") { end(); off = true; finish(job, { ok: false, reason: "unavailable", text: m.text }); pump(); return; }
      if (m.op === "done") { end(); finish(job, { ok: true, stats: m.stats, ms: m.ms, parts: m.parts }); pump(); return; }
      if (m.op === "error") {
        if (m.fatal) return retry("crash", m.text);
        end();
        let key = ""; try { key = JSON.parse(m.text).error_key || ""; } catch (_) { /* not JSON */ }
        if (key === "error.memoryBudget") {                      // fewer workers and a smaller budget; the page is told why the file fails
          notify(key);
          const was = limit; limit = Math.max(1, limit >> 1);
          if (!job.shrunk && was > 1) { job.shrunk = true; job.budget = Math.max(8, (job.budget || budget) >> 1); queue.unshift(job); }
          else finish(job, { ok: false, reason: "memory", text: m.text });
        } else finish(job, { ok: false, reason: "error", text: m.text });
        pump();
      }
    };
  }

  return {
    // job: { name, blob, inputs, part, budget? } or { name, msg, transfer?, onProgress? }
    build(job) {
      return new Promise(resolve => { queue.push({ ...job, resolve, tried: false, done: false }); pump(); });
    },
    // the order of the list: the files named first are the first to start (the queued ones; those already running continue)
    prioritize(names) {
      const rank = n => { const i = names.indexOf(n); return i < 0 ? names.length : i; };
      queue.sort((a, b) => rank(a.name) - rank(b.name));
    },
    cancel(name) {
      for (let i = queue.length - 1; i >= 0; i--) if (queue[i].name === name) finish(queue.splice(i, 1)[0], { ok: false, reason: "cancelled" });
      for (const [job, w] of [...running]) if (job.name === name) { running.delete(job); try { w.terminate(); } catch (_) { /* gone */ } active--; finish(job, { ok: false, reason: "cancelled" }); }
      pump();
    },
    unavailable: () => off,
    get limit() { return limit; },
    get active() { return active; },
    get queued() { return queue.length; },
  };
}
