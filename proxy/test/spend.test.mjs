// The money: the reservation through the Rust ledger, the settlement from the usage of the stream, the caps of session, IP, day and month, the
// rate limit and the day rollover. The ledger state is read back to check what the proxy keeps.

import { test } from "node:test";
import assert from "node:assert/strict";
import { setup, post, drain, orchestratorBody, fakeProvider, sse } from "./helpers.mjs";

const IP = "192.0.2.10";
const counters = (s) => JSON.parse(s.state());
const DAY1 = Date.UTC(2026, 10, 3, 23, 59);

/** The reservation of one orchestrator request (Rust `max_cost` of ceil(bytes / 3) prompt tokens and its max_tokens). */
function worst(s) {
  const bytes = Buffer.byteLength(JSON.stringify(orchestratorBody()));
  return s.rust.maxCost("claude-sonnet-5-5", Math.ceil(bytes / 3), 1500).value;
}

/** Public caps filled in after setup (the proxy reads the same object). */
function lateCaps() {
  return { owner: { session: 1, ip: 0, day: 1, month: 1 }, public: { session: 0, ip: 0, day: 0, month: 150_000_000 } };
}

test("two parallel requests of one session never pass the session cap together", async (t) => {
  const caps = lateCaps();
  const s = await setup({ caps });
  if (!s) return t.skip("no wasm");
  const w = worst(s);
  // the cap admits one worst case, never two at once
  Object.assign(caps.public, { session: Math.floor(1.5 * w), ip: 10 * w, day: 10 * w });
  const res = await Promise.all([s.handle(post(orchestratorBody()), IP), s.handle(post(orchestratorBody()), IP)]);
  const r = await Promise.all(res.map(drain));
  const codes = r.map((x) => x.status).sort();
  assert.deepEqual(codes, [200, 429]);
  const no = r.find((x) => x.status === 429);
  assert.equal(JSON.parse(no.text).params.limit, "session");
  assert.equal(no.headers["x-mzlab-limit"], "session");
  const c = counters(s).c["s:session-0001"];
  assert.ok(c[0] + c[1] <= Math.floor(1.5 * w), JSON.stringify(c));
  assert.equal(c[1], 0, "nothing stays reserved after the stream ended");
  // the true cost of the recorded answer (25 in, 4 cached, 12 out) replaced the reservation
  assert.equal(c[0], 50 + 1 + 120);
  assert.equal(s.provider.seen.length, 1);
});

test("a truncated stream keeps the whole reservation", async (t) => {
  const s = await setup({ provider: fakeProvider({ body: sse("truncated") }) });
  if (!s) return t.skip("no wasm");
  const r = await drain(await s.handle(post(orchestratorBody()), IP));
  assert.equal(r.status, 200);
  assert.ok(r.text.includes("Half an ans"), "the bytes pass unchanged");
  assert.equal(r.text, sse("truncated").toString("utf-8"));
  const reserved = Number(r.headers["x-mzlab-spend"].split("/")[0]);
  const c = counters(s).c["s:session-0001"];
  assert.deepEqual(c, [reserved, 0]);
  assert.ok(reserved >= 15_000, "1500 output tokens at least");
});

test("a client that goes away mid-stream is settled with what was read", async (t) => {
  const s = await setup();
  if (!s) return t.skip("no wasm");
  const res = await s.handle(post(orchestratorBody()), IP);
  const reader = res.body.getReader();
  await reader.read();
  await reader.cancel();
  const c = counters(s).c["s:session-0001"];
  assert.equal(c[1], 0);
  assert.ok(c[0] > 0);
});

test("an error of the provider passes through with its status and costs nothing", async (t) => {
  const err = Buffer.from(JSON.stringify({ type: "error", error: { type: "rate_limit_error", message: "Rate limited" } }));
  const s = await setup({ provider: fakeProvider({ body: err, status: 429, headers: { "retry-after": "7", "anthropic-ratelimit-requests-remaining": "0", "x-secret-upstream": "1" } }) });
  if (!s) return t.skip("no wasm");
  const r = await drain(await s.handle(post(orchestratorBody()), IP));
  assert.equal(r.status, 429);
  assert.equal(r.headers["retry-after"], "7");
  assert.equal(r.headers["request-id"], "req_1");
  assert.equal(r.headers["anthropic-ratelimit-requests-remaining"], undefined, "no other header of the provider");
  assert.equal(r.headers["x-secret-upstream"], undefined);
  assert.deepEqual(counters(s).c["s:session-0001"], [0, 0]);
});

test("the IP cap stops new sessions of one IP, the day cap stops everybody with 503 budget", async (t) => {
  const caps = lateCaps();
  const s = await setup({ caps, provider: fakeProvider({ body: sse("truncated") }) });
  if (!s) return t.skip("no wasm");
  const w = worst(s);
  Object.assign(caps.public, { session: 10 * w, ip: 2 * w, day: 3 * w });
  const send = (session, ip) => s.handle(post(orchestratorBody(), { session }), ip).then(drain);
  assert.equal((await send("visitor-aaaa1", IP)).status, 200);
  assert.equal((await send("visitor-aaaa2", IP)).status, 200);
  const third = await send("visitor-aaaa3", IP);
  assert.equal(third.status, 429);
  assert.equal(JSON.parse(third.text).params.limit, "ip");
  const other = await send("visitor-bbbb1", "192.0.2.99");
  assert.equal(other.status, 200);
  const closed = await send("visitor-cccc1", "192.0.2.100");
  assert.equal(closed.status, 503);
  assert.equal(JSON.parse(closed.text).error_key, "err.llm.proxy.budget");
  assert.equal(closed.headers["x-mzlab-limit"], "day");
  const st = counters(s);
  assert.ok(st["g:day"][0] <= 3 * w && st["g:month"][0] <= 150_000_000);
  for (const [k, c] of Object.entries(st.c)) if (k.startsWith("ip:")) assert.ok(c[0] + c[1] <= 2 * w, k);
});

test("the soft notice: x-mzlab-soft from 80 % of the session cap", async (t) => {
  const caps = lateCaps();
  const s = await setup({ caps, provider: fakeProvider({ body: sse("truncated") }) });
  if (!s) return t.skip("no wasm");
  const w = worst(s);
  Object.assign(caps.public, { session: Math.ceil(2.4 * w), ip: 10 * w, day: 10 * w });
  const a = await drain(await s.handle(post(orchestratorBody()), IP));
  assert.equal(a.headers["x-mzlab-soft"], undefined);
  const b = await drain(await s.handle(post(orchestratorBody()), IP));
  assert.equal(b.status, 200);
  assert.equal(b.headers["x-mzlab-soft"], "1");
  assert.equal(b.headers["x-mzlab-spend"], `${2 * w}/${Math.ceil(2.4 * w)}`);
});

test("the rate limit: the 13th request of an IP within 60 s is refused before any reservation", async (t) => {
  let now = DAY1;
  const s = await setup({ now: () => now });
  if (!s) return t.skip("no wasm");
  for (let i = 0; i < 12; i++) assert.equal((await drain(await s.handle(post(orchestratorBody()), IP))).status, 200);
  const before = counters(s)["g:day"];
  const r = await drain(await s.handle(post(orchestratorBody()), IP));
  assert.equal(r.status, 429);
  assert.equal(JSON.parse(r.text).params.limit, "rate");
  assert.equal(r.headers["retry-after"], "60");
  assert.deepEqual(counters(s)["g:day"], before);
  assert.equal((await drain(await s.handle(post(orchestratorBody()), "192.0.2.11"))).status, 200, "other IPs go on");
  now += 61_000;
  assert.equal((await drain(await s.handle(post(orchestratorBody()), IP))).status, 200);
});

test("the day rollover drops the IP and session counters and the salt, keeps the month", async (t) => {
  let now = DAY1;
  const s = await setup({ now: () => now, provider: fakeProvider({ body: sse("truncated") }) });
  if (!s) return t.skip("no wasm");
  await drain(await s.handle(post(orchestratorBody()), IP));
  const d1 = counters(s);
  const ipKey = Object.keys(d1.c).find((k) => k.startsWith("ip:"));
  assert.ok(ipKey && d1.salt);
  assert.ok(!s.state().includes(IP), "the IP is never stored");
  now = Date.UTC(2026, 10, 4, 0, 0, 5);
  await s.ledger.alarm();
  const d2 = counters(s);
  assert.equal(d2.day, "2026-11-04");
  assert.deepEqual(d2.c, {});
  assert.equal(d2.salt, undefined);
  assert.deepEqual(d2["g:day"], [0, 0]);
  assert.deepEqual(d2["g:month"], d1["g:month"]);
  // the same IP has a different key on the new day
  await drain(await s.handle(post(orchestratorBody()), IP));
  const d3 = counters(s);
  assert.equal(d3.c[ipKey], undefined);
  assert.ok(Object.keys(d3.c).some((k) => k.startsWith("ip:")));
  // without the alarm, the first request of a day rolls over by itself
  now = Date.UTC(2026, 10, 5, 8);
  await drain(await s.handle(post(orchestratorBody()), IP));
  assert.equal(counters(s).day, "2026-11-05");
});
