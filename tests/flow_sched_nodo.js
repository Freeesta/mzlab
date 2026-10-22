// Parity of the scheduler in the engine .wasm with the native one: plays the shared cases of crates/mzlab-core/tests/data/sched_cases.json through
// flow_sched(op, body) (wasm-bindgen --target nodejs output in argv[2]) and compares each answer with the one sched_cases.rs wrote. Prints "ok <n> calls" or the first mismatch.
"use strict";
const fs = require("fs");
const path = require("path");

const pkg = path.resolve(process.argv[2]);
const casesPath = process.argv[3];
const { flow_sched } = require(path.join(pkg, "mzlab_wasm.js"));

const jid = (s) => {
  const i = s.indexOf("|");
  return i < 0 ? { instance: s, item: null } : { instance: s.slice(0, i), item: s.slice(i + 1) };
};
const call = (op, body) => JSON.parse(flow_sched(op, JSON.stringify(body)));
// the registry of the cases ("name*" = a port that takes many; the port types do not matter to the scheduler) written as step/1 documents, as the page sends it
const port = (t) => ({ type: "raw", many: t.endsWith("*") });
const ports = (l) => Object.fromEntries(l.map((t) => [t.replace("*", ""), port(t)]));
const stepDoc = (d) => ({
  id: d.id, version: 1, in: ports(d.ins), out: ports(d.outs), per_item: d.per_item === true,
  params: { properties: Object.fromEntries(Object.entries(d.params || {}).map(([k, v]) => [k, { type: "number", default: v }])) },
  runs_in: { native: true, wasm: "hr" },
});
const name = (j) => (j.item === null ? j.instance : `${j.instance}|${j.item}`);
// the cases are written with sorted keys: compare without the order of the keys
const canon = (v) => (Array.isArray(v) ? v.map(canon) : v && typeof v === "object" ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])])) : v);
const same = (a, b) => JSON.stringify(canon(a)) === JSON.stringify(canon(b));

function answer(c) {
  const num = (k, d) => (c[k] === undefined ? d : c[k]);
  switch (c.op) {
    case "resources": call("set_resources", { job: jid(c.job), res: c.res }); return undefined;
    case "start": if (!call("started", { job: jid(c.job) })) throw new Error(`${c.job} did not start`); return undefined;
    case "progress": call("progress", { job: jid(c.job), frac: num("frac", 0) }); return undefined;
    case "cached": if (!call("cached", { job: jid(c.job) })) throw new Error(`${c.job} not cached`); return undefined;
    case "pause": case "resume": call(c.op, {}); return undefined;
    case "next": return call("next", { slots: num("slots", 99), mem_mb: num("mem", 1e9), mb: c.mb || {} }).map(name);
    case "finish": {
      const r = c.error === undefined ? { ms: num("ms", 1) } : { error_key: c.error, params: {} };
      return call("finished", { job: jid(c.job), result: r }).split(":")[0];
    }
    case "update": return call("update", { plan: c.plan }).map(name);
    case "states": return call("jobs", {}).map((j) => `${name(j)}=${j.state}`);
    case "nodes":
      return call("nodes", {}).map((n) => `${n.instance}=${n.state} ${n.parte}/${n.di} ${n.frac === null ? "?" : n.frac.toFixed(3)}`);
    case "events": {
      const ev = call("events", { since: num("since", 0) });
      if (c.full === true) return ev;
      return [ev.length, ev.length ? ev[ev.length - 1].seq : 0, call("rev", {})];
    }
    default: throw new Error(`unknown op ${c.op}`);
  }
}

const cases = JSON.parse(fs.readFileSync(casesPath, "utf8"));
let n = 0;
for (const c of cases) {
  call("new", { plan: c.plan, registry: c.steps.map(stepDoc), version: "test-1", identities: {} });
  c.calls.forEach((step, i) => {
    const got = answer(step);
    if (!same(got, step.expect)) {
      console.log(`MISMATCH ${c.name}, call ${i} (${step.op}): got ${JSON.stringify(got)}, want ${JSON.stringify(step.expect)}`);
      process.exit(1);
    }
    n++;
  });
}
console.log(`ok ${n} calls`);
