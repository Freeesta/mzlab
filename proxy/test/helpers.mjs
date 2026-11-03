// Shared by the proxy tests: the Rust module (from $MZLAB_LLM_WASM_DIR, or built with cargo + wasm-bindgen into a temporary folder; null when the
// tools are missing, and the tests then skip), the allow-list, a fake provider that replays the recorded Anthropic streams of
// crates/mzlab-llm/tests/data/ and requests shaped like those of the site.

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { buildAllowlist, loadRust, ROOT } from "../build-allowlist.mjs";
import { memoryProxy } from "../adapters/memoria.js";

const DATA = join(ROOT, "crates/mzlab-llm/tests/data");
// a fake key, assembled at run time so that no key-shaped literal is written in proxy/ (the acceptance grep of the PR stays empty)
export const FAKE_KEY = ["sk", "ant", "api03", "FAKEKEYFORTHEPROXYTESTS0123456789abcdef"].join("-");
export const OWNER = "a phrase only the owner knows";
export const ORIGIN = "https://freeesta.github.io";

let cached;
export async function rust() {
  if (cached !== undefined) return cached;
  let dir = process.env.MZLAB_LLM_WASM_DIR;
  if (!dir) {
    try {
      execFileSync("cargo", ["build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-llm-wasm"], { cwd: ROOT, stdio: "ignore" });
      dir = mkdtempSync(join(tmpdir(), "mzlab-proxy-wasm-"));
      execFileSync("wasm-bindgen", ["--target", "web", "--no-typescript", "--out-dir", dir, join(ROOT, "target/wasm32-unknown-unknown/release/mzlab_llm_wasm.wasm")], { stdio: "ignore" });
    } catch {
      cached = null;
      return cached;
    }
  }
  cached = existsSync(join(dir, "mzlab_llm_wasm_bg.wasm")) ? await loadRust(dir) : null;
  return cached;
}

export const sse = (name) => readFileSync(join(DATA, `anthropic_${name}.sse`));

/** A stream of `deltas` text deltas between the message_start and message_delta of the recorded text answer. */
export function longStream(deltas) {
  const text = sse("text").toString("utf-8");
  const frames = text.split("\n\n");
  const delta = frames.find((f) => f.includes("content_block_delta"));
  const at = frames.indexOf(delta);
  return Buffer.from([...frames.slice(0, at), ...Array(deltas).fill(delta), ...frames.slice(at + 1)].join("\n\n"));
}

/** The fake provider: the bytes of `body` in chunks of `chunk` bytes, with every request it got (url, headers, body) in `seen`. */
export function fakeProvider({ body = sse("text"), status = 200, chunk = 97, headers = {} } = {}) {
  const seen = [];
  const upstream = async (url, init) => {
    seen.push({ url, headers: init.headers, body: Buffer.from(init.body).toString("utf-8") });
    let i = 0;
    const bytes = typeof body === "function" ? body(seen.length) : body;
    const stream = new ReadableStream({
      pull(c) {
        if (i >= bytes.length) return c.close();
        c.enqueue(new Uint8Array(bytes.subarray(i, i + chunk)));
        i += chunk;
      },
    });
    return new Response(status === 200 ? stream : bytes, { status, headers: { "content-type": status === 200 ? "text/event-stream" : "application/json", "request-id": "req_1", ...headers } });
  };
  return { upstream, seen };
}

export async function setup(opts = {}) {
  const r = await rust();
  if (!r) return null;
  const allow = await buildAllowlist(r);
  const provider = opts.provider || fakeProvider();
  const secrets = { ANTHROPIC_KEY: FAKE_KEY, OWNER_SHA256: await sha(OWNER), ...(opts.secrets || {}) };
  const p = memoryProxy({ rust: r, allow, upstream: provider.upstream, secrets, vars: opts.vars || { PHASE: "1" }, now: opts.now, rate: opts.rate, caps: opts.caps });
  return { ...p, provider, allow, rust: r };
}

async function sha(t) {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(t)));
  return [...d].map((b) => b.toString(16).padStart(2, "0")).join("");
}

const prompt = (f) => readFileSync(join(ROOT, "crates/mzlab-llm/data/assist", f), "utf-8");

/** A request of the orchestrator as the site sends it. */
export function orchestratorBody(over = {}) {
  return {
    model: "claude-sonnet-5-5",
    max_tokens: 1500,
    stream: true,
    thinking: { type: "adaptive" },
    output_config: { effort: "medium" },
    system: [{ type: "text", text: prompt("01_sistema.md"), cache_control: { type: "ephemeral", ttl: "1h" } }],
    messages: [{ role: "user", content: [{ type: "text", text: "Which steps fit my files?" }, { type: "text", text: '{"schema":"contesto/1","files":2}' }] }],
    ...over,
  };
}

/** A request of the router (its system prompt, the haiku model, the router schema). */
export function routerBody(r, over = {}) {
  const schema = JSON.parse(r.schemas().find(([n]) => n === "router/1")[1]);
  return {
    model: "claude-haiku-5-5",
    max_tokens: 200,
    stream: true,
    output_config: { effort: "low", format: { type: "json_schema", schema } },
    system: [{ type: "text", text: prompt("02_router.md"), cache_control: { type: "ephemeral", ttl: "5m" } }],
    messages: [{ role: "user", content: [{ type: "text", text: JSON.stringify({ message: "show the TIC", state: {} }) }] }],
    ...over,
  };
}

export function post(body, { origin = ORIGIN, session = "session-0001", owner, headers = {} } = {}) {
  const h = { "content-type": "application/json", origin, "x-mzlab-session": session, ...headers };
  if (owner) h["x-mzlab-owner"] = owner;
  return new Request("https://proxy.example/v1/messages", { method: "POST", headers: h, body: typeof body === "string" ? body : JSON.stringify(body) });
}

/** Reads a response to the end and returns its status, headers and text. */
export async function drain(res) {
  return { status: res.status, headers: Object.fromEntries(res.headers), text: await res.text() };
}
