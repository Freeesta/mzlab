// The CPU time of one request (the free Workers plan counts CPU, not waiting), printed so the PR can quote it: a short recorded answer (the fixed
// cost: body check, hashes, ledger) and a long one of 4000 text deltas (462 KB, more than any allowed max_tokens). The fake provider and the reading of
// the answer are measured alone and taken off; the first request (cold JIT) is printed apart.

import { test } from "node:test";
import assert from "node:assert/strict";
import { setup, post, drain, orchestratorBody, fakeProvider, longStream } from "./helpers.mjs";

const cpu = async (runs, f) => {
  const c0 = process.cpuUsage();
  for (let i = 0; i < runs; i++) await f(i);
  const c = process.cpuUsage(c0);
  return (c.user + c.system) / 1000 / runs;
};

async function measure(deltas) {
  const body = longStream(deltas);
  const s = await setup({ provider: fakeProvider({ body, chunk: 4096 }), rate: { limit: 1e9, period: 60_000 } });
  if (!s) return null;
  const alone = fakeProvider({ body, chunk: 4096 });
  const one = async (i) => {
    const r = await drain(await s.handle(post(orchestratorBody(), { session: `cpu-session-${i}` }), "192.0.2.60"));
    assert.equal(r.status, 200);
    assert.equal(Buffer.byteLength(r.text), body.length, "the stream passes whole");
  };
  const cold = await cpu(1, () => one("cold"));
  for (let i = 0; i < 20; i++) await one(`warm${i}`);
  const base = await cpu(40, () => alone.upstream("u", { headers: {}, body: "x" }).then(drain));
  const all = await cpu(40, one);
  assert.equal(JSON.parse(s.state()).c["s:cpu-session-0"][1], 0);
  return { kb: Math.round(body.length / 1024), warm: Math.max(0, all - base), cold };
}

test("CPU time per request, short and long recorded stream", async (t) => {
  const short = await measure(0);
  if (!short) return t.skip("no wasm");
  const long = await measure(4000);
  const line =
    `proxy CPU per request (node ${process.version}): ${short.warm.toFixed(1)} ms on a ${short.kb} KB answer, ${long.warm.toFixed(1)} ms on a ${long.kb} KB stream of 4000 deltas ` +
    `(warm, fake provider taken off); first request after start ${short.cold.toFixed(0)} ms`;
  process.stdout.write(`# ${line}\n`);
  t.diagnostic(line);
});
