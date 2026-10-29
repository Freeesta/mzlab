// Pins the order of the messages ordered() sends to the engine worker (module > policy > budget > down > commands) on a fake worker with simulated delays,
// and that the module goes out right after compile() without waiting for the growth probe. Prints one JSON object. See tests/test_supervisor_ordine.py.
import { ordered } from "../mzlab/web/rust-supervisor.js";

const sleep = ms => new Promise(r => setTimeout(r, ms));

async function run({ compileMs, probeMs }) {
  const t0 = performance.now(), log = [];
  const w = { postMessage: m => log.push({ op: m.op, at: Math.round(performance.now() - t0) }) };
  const mod = { fake: "module" };
  const send = ordered(w, () => sleep(compileMs).then(() => mod), () => 256, () => 7, () => ({ device_gb: 8 }), () => sleep(probeMs), () => ({ current: 192, strikes: 1 }));
  send({ op: "lr_open" });          // sent before anything is ready: must queue behind the setup messages
  send({ op: "lr_xic" });
  await sleep(compileMs + probeMs + 60);
  return log;
}

const out = {};
out.slowProbe = await run({ compileMs: 20, probeMs: 400 });
out.fastProbe = await run({ compileMs: 20, probeMs: 0 });

// startProbe asks for the idle callback with a timeout below the 800 ms whenProbed gives up at
let idle = null;
globalThis.self = globalThis;
globalThis.window = { requestIdleCallback: (_cb, o) => { idle = o; } };
const { whenProbed } = await import("../mzlab/web/rust-budget.js");
const t = performance.now();
await whenProbed();
out.idleTimeout = idle && idle.timeout;
out.whenProbedMs = Math.round(performance.now() - t);
console.log(JSON.stringify(out));
process.exit(0);
