// The engine .wasm is compiled once per page and shared by the peaks worker (compileOnce keyed by URL; qualita.js posts the module first). Prints one JSON object.
// See tests/test_modulo_condiviso.py.
const sleep = ms => new Promise(r => setTimeout(r, ms));
const counts = { compile: 0, fetch: 0 };
let failNext = 0;
const fakeModule = { fake: "module" };
globalThis.fetch = async () => { counts.fetch++; return { arrayBuffer: async () => new ArrayBuffer(8) }; };
globalThis.WebAssembly = {
  compileStreaming: async () => { counts.compile++; await sleep(5); if (failNext > 0) { failNext--; throw new Error("boom"); } return fakeModule; },
  compile: async () => { counts.compile++; if (failNext > 0) { failNext--; throw new Error("boom"); } return fakeModule; },
};
const { compileOnce } = await import("../mzlab/web/rust-supervisor.js");
const out = {};

// two callers, same URL (a distinct URL object each): one compile, one module
const u = () => new URL("file:///site/wasm/mzlab_wasm_bg.wasm");
const [a, b] = await Promise.all([compileOnce(u())(), compileOnce(u())()]);
const again = await compileOnce(u())();
out.shared = { compiles: counts.compile, same: a === b && b === again && a === fakeModule };
out.samePromise = (() => { const f = compileOnce(u()); return f() === f(); })();

// a failed compile (both attempts) gives null and the next call retries
counts.compile = 0; failNext = 2;
const other = new URL("file:///other/wasm/mzlab_wasm_bg.wasm");
const bad = await compileOnce(other)();
const retry = await compileOnce(other)();
out.retry = { bad, retryIsModule: retry === fakeModule, compiles: counts.compile };

// qualita.js: the first message to the worker is the module, the operations follow
counts.compile = 0;
const posted = [];
globalThis.Worker = class { constructor() { setTimeout(() => {}, 0); } postMessage(m) { posted.push(m.op); if (m.op !== "module") queueMicrotask(() => this.onmessage({ data: { id: m.id, v: [1, 0, 0] } })); } };
globalThis.window = {};
await import("../mzlab/web/qualita.js");
const Q = window.QUALITA;
const r = await Promise.all([Q.quality([0, 1], [1, 2], 0, 1), Q.flat ? Q.flat([1, 2], 0, 0) : null]);
await Q.quality([0, 1], [1, 2], 0, 1);
out.qualita = { posted, compiles: counts.compile, answered: r[0] !== null };
console.log(JSON.stringify(out));
process.exit(0);
