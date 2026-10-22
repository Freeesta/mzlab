// Parity of the scheduler in WebAssembly with the native one: plays the shared cases of crates/mzlab-core/tests/data/sched_cases.json on the .wasm
// (wasm-bindgen --target nodejs output in argv[2]) and compares each answer with the one sched_cases.rs wrote. Prints "ok <n> calls" or the first mismatch.
"use strict";
const fs = require("fs");
const path = require("path");

const pkg = path.resolve(process.argv[2]);
const casesPath = process.argv[3];
const { FlowSched } = require(path.join(pkg, "mzlab_feat_wasm.js"));

const jid = (s) => {
  const i = s.indexOf("|");
  return JSON.stringify(i < 0 ? { instance: s, item: null } : { instance: s.slice(0, i), item: s.slice(i + 1) });
};
const name = (j) => (j.item === null ? j.instance : `${j.instance}|${j.item}`);
// the cases are written with sorted keys: compare without the order of the keys
const canon = (v) => (Array.isArray(v) ? v.map(canon) : v && typeof v === "object" ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])])) : v);
const same = (a, b) => JSON.stringify(canon(a)) === JSON.stringify(canon(b));

function answer(s, call) {
  const num = (k, d) => (call[k] === undefined ? d : call[k]);
  switch (call.op) {
    case "resources": s.set_resources(jid(call.job), JSON.stringify(call.res)); return undefined;
    case "start": if (!s.started(jid(call.job))) throw new Error(`${call.job} did not start`); return undefined;
    case "progress": s.progress(jid(call.job), num("frac", 0)); return undefined;
    case "cached": if (!s.cached(jid(call.job))) throw new Error(`${call.job} not cached`); return undefined;
    case "pause": s.pause(); return undefined;
    case "resume": s.resume(); return undefined;
    case "next": {
      const lim = { slots: num("slots", 99), mem_mb: num("mem", 1e9), mb: call.mb || {} };
      return JSON.parse(s.next(JSON.stringify(lim))).map(name);
    }
    case "finish": {
      const r = call.error === undefined ? { ms: num("ms", 1) } : { error_key: call.error, params: {} };
      return s.finished(jid(call.job), JSON.stringify(r)).split(":")[0];
    }
    case "update": return JSON.parse(s.update(JSON.stringify(call.plan))).map(name);
    case "states": return JSON.parse(s.jobs()).map((j) => `${name(j)}=${j.state}`);
    case "nodes":
      return JSON.parse(s.nodes()).map((n) => `${n.instance}=${n.state} ${n.parte}/${n.di} ${n.frac === null ? "?" : n.frac.toFixed(3)}`);
    case "events": {
      const ev = JSON.parse(s.events_since(num("since", 0)));
      if (call.full === true) return ev;
      return [ev.length, ev.length ? ev[ev.length - 1].seq : 0, s.rev()];
    }
    default: throw new Error(`unknown op ${call.op}`);
  }
}

const cases = JSON.parse(fs.readFileSync(casesPath, "utf8"));
let n = 0;
for (const c of cases) {
  const s = new FlowSched(JSON.stringify(c.plan), JSON.stringify(c.steps), "test-1", "{}");
  c.calls.forEach((call, i) => {
    const got = answer(s, call);
    if (!same(got, call.expect)) {
      console.log(`MISMATCH ${c.name}, call ${i} (${call.op}): got ${JSON.stringify(got)}, want ${JSON.stringify(call.expect)}`);
      process.exit(1);
    }
    n++;
  });
}
console.log(`ok ${n} calls`);
