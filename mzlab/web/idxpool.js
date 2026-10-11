// Pool of short-lived Rust workers that build the .mzidx of the uploaded mzML files (idx-worker.js). Used by browser-worker.js, the engine's worker.
//  - N workers at most (min(cores - 1, 4); 2 on Safari/iPad; at least 1), one file each, started in the order of the queue (prioritize() puts the list's order first);
//  - supervisor: a worker that dies is replaced and its file goes back to the front of the queue (once; a second death gives the file to Python);
//  - memory: a builder that cannot reserve memory (error.memoryBudget) halves the pool and the budget and retries once; the page is told (notice);
//  - no WASM / no SIMD / no OPFS: unavailable() is true and every job answers { ok: false, reason: "unavailable" }, so Python builds the index as before.
// A job's result: { ok: true, stats } or { ok: false, reason: "unavailable" | "cancelled" | "crash" | "memory" | "error", text }.
export function poolSize(small, cores) {
  const n = Math.max(1, Math.min((cores || 2) - 1, 4));
  return small ? Math.min(n, 2) : n;
}

// A tiny WebAssembly module that uses a 128-bit SIMD instruction: the browser validates it only when it supports SIMD (the Rust builder is built with it).
const SIMD_PROBE = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253, 15, 253, 98, 11]);
export const simdSupported = () => { try { return WebAssembly.validate(SIMD_PROBE); } catch (_) { return false; } };

export function createPool({ workerUrl, size, budget = 64, notify = () => {}, needSimd = false, WorkerClass = Worker }) {
  let limit = size, active = 0, off = needSimd && !simdSupported();
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
    const retry = (reason, text) => {                          // the worker is lost: once more at the front of the queue, then Python
      end();
      if (!job.tried) { job.tried = true; queue.unshift(job); } else finish(job, { ok: false, reason, text });
      pump();
    };
    try { w = new WorkerClass(workerUrl, { type: "module" }); } catch (e) { end(); off = true; finish(job, { ok: false, reason: "unavailable", text: String(e) }); return pump(); }
    running.set(job, w);
    w.onerror = e => { if (job.done) return; if (job.inited) retry("crash", e.message); else { end(); off = true; finish(job, { ok: false, reason: "unavailable", text: e.message }); pump(); } };
    w.onmessage = ev => {
      const m = ev.data;
      if (job.done) return;
      if (m.op === "ready") { job.inited = true; w.postMessage({ blob: job.blob, inputs: job.inputs, part: job.part, budget: job.budget || budget }, []); return; }
      if (m.op === "unavailable") { end(); off = true; finish(job, { ok: false, reason: "unavailable", text: m.text }); pump(); return; }
      if (m.op === "done") { end(); finish(job, { ok: true, stats: m.stats }); pump(); return; }
      if (m.op === "error") {
        if (m.fatal) return retry("crash", m.text);
        end();
        let key = ""; try { key = JSON.parse(m.text).error_key || ""; } catch (_) { /* not JSON */ }
        if (key === "error.memoryBudget") {                      // fewer workers and a smaller budget; the page is told why Python reads this file
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
    // job: { name, blob, inputs, part, budget? }
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
