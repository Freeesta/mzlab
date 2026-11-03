// The key goes to the provider and nowhere else: not in any response body, header or error, whatever happens (the patterns of `log::has_secret`).

import { test } from "node:test";
import assert from "node:assert/strict";
import { setup, post, drain, orchestratorBody, fakeProvider, sse, FAKE_KEY, OWNER } from "./helpers.mjs";

// mzlab_llm::log::has_secret: `sk-` or `AIza` at the start of a word, followed by a letter or digit
const HAS_SECRET = /(^|[^A-Za-z0-9])(sk-|AIza)[A-Za-z0-9]/;

function clean(r, what) {
  const all = r.text + "\n" + Object.entries(r.headers).map(([k, v]) => `${k}: ${v}`).join("\n");
  assert.ok(!all.includes(FAKE_KEY), `${what}: the key is in the response`);
  assert.ok(!HAS_SECRET.test(all), `${what}: something shaped like a key is in the response`);
}

test("the key never appears in a response, a header or an error", async (t) => {
  const echo = Buffer.from(JSON.stringify({ type: "error", error: { type: "authentication_error", message: `invalid x-api-key ${FAKE_KEY}` } }));
  const scenarios = [
    ["answer", {}, () => post(orchestratorBody())],
    ["truncated", { provider: fakeProvider({ body: sse("truncated") }) }, () => post(orchestratorBody())],
    ["tool call", { provider: fakeProvider({ body: sse("tool") }) }, () => post(orchestratorBody())],
    ["provider echoes the key", { provider: fakeProvider({ body: echo, status: 401 }) }, () => post(orchestratorBody())],
    ["provider 500", { provider: fakeProvider({ body: Buffer.from("{}"), status: 500 }) }, () => post(orchestratorBody())],
    ["network down", { provider: { upstream: async () => { throw new Error(`connect failed with ${FAKE_KEY}`); }, seen: [] } }, () => post(orchestratorBody())],
    ["closed", { vars: { PHASE: "0" } }, () => post(orchestratorBody())],
    ["owner", { vars: { PHASE: "0" } }, () => post(orchestratorBody(), { owner: OWNER })],
    ["bad body", {}, () => post(orchestratorBody({ temperature: 1 }))],
    ["origin", {}, () => post(orchestratorBody(), { origin: "https://evil.example" })],
    ["off", { vars: { PHASE: "1", ASSIST_OFF: "1" } }, () => post(orchestratorBody())],
    ["preflight", {}, () => new Request("https://proxy.example/v1/messages", { method: "OPTIONS", headers: { origin: "https://freeesta.github.io" } })],
  ];
  for (const [what, opts, req] of scenarios) {
    const s = await setup(opts);
    if (!s) return t.skip("no wasm");
    const r = await drain(await s.handle(req(), "192.0.2.50"));
    clean(r, what);
    assert.ok(!s.state().includes(FAKE_KEY), `${what}: the key is in the ledger`);
  }
});

test("the provider gets the secret key and the pinned API version, and the body byte for byte", async (t) => {
  const s = await setup();
  if (!s) return t.skip("no wasm");
  const body = JSON.stringify(orchestratorBody());
  await drain(await s.handle(post(body), "192.0.2.51"));
  const [seen] = s.provider.seen;
  assert.equal(seen.url, "https://api.anthropic.com/v1/messages");
  assert.equal(seen.headers["x-api-key"], FAKE_KEY);
  assert.equal(seen.headers["anthropic-version"], "2023-06-01");
  assert.equal(seen.body, body);
  for (const h of ["origin", "x-mzlab-session", "x-mzlab-owner", "cf-connecting-ip", "user-agent"]) assert.equal(seen.headers[h], undefined, h);
});
