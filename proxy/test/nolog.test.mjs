// The proxy writes nothing about a request anywhere: no `console.` call in its modules except `console.error` with a fixed key, and the Worker
// configuration keeps logs off.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../build-allowlist.mjs";

const DIR = join(ROOT, "proxy");
const modules = () =>
  [...readdirSync(DIR), ...readdirSync(join(DIR, "adapters")).map((f) => `adapters/${f}`)].filter((f) => /\.(m?js)$/.test(f));

test("no console call except console.error of a fixed key", () => {
  const files = modules();
  assert.ok(files.includes("assist-proxy.js") && files.includes("adapters/cloudflare.js"));
  for (const f of files) {
    const text = readFileSync(join(DIR, f), "utf-8");
    // the allow-list builder prints the list it builds to stdout and nothing else; it never runs in the Worker
    for (const m of text.matchAll(/console\s*\.\s*(\w+)\s*\(([^)]*)\)/g)) {
      assert.equal(m[1], "error", `${f}: console.${m[1]}`);
      assert.match(m[2], /^"err\.[a-z.]+"$/, `${f}: console.error with something other than a fixed key`);
    }
    assert.ok(!/console\s*\[/.test(text), `${f}: console[...]`);
    assert.ok(!/\bconsole\b(?!\s*\.\s*error\s*\(")/.test(text), `${f}: console used another way`);
  }
});

test("wrangler.toml: observability and logs off, no secret, phase 0", () => {
  const w = readFileSync(join(DIR, "wrangler.toml"), "utf-8");
  assert.match(w, /\[observability\]\s*\nenabled = false/);
  assert.match(w, /\[observability\.logs\]\s*\nenabled = false/);
  assert.match(w, /^logpush = false$/m);
  assert.match(w, /^PHASE = "0"$/m);
  assert.ok(!/ANTHROPIC_KEY\s*=|OWNER_SHA256\s*=|sk-/.test(w));
});
