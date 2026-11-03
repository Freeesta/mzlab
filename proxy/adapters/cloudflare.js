// The proxy on Cloudflare Workers: the Worker handles each request (assist-proxy.js) and one Durable Object `Ledger` holds the JSON state of the Rust
// ledger, serializing every reservation and settlement; its alarm starts each UTC day (drops the salt and the `s:`/`ip:` counters). Secrets come from
// `env` (ANTHROPIC_KEY, OWNER_SHA256, ASSIST_OFF); `RATE` is the ratelimit binding (12 requests / 60 s per IP hash). Built by build.sh (wasm/, allowlist.json).

import { DurableObject } from "cloudflare:workers";
import { makeProxy } from "../assist-proxy.js";
import { ledgerOps } from "../ledger-core.js";
import { bindRust } from "../rust.js";
import * as glue from "../wasm/mzlab_llm_wasm.js";
import wasm from "../wasm/mzlab_llm_wasm_bg.wasm";
import allow from "../allowlist.json";

let rust = null;
const bound = () => (rust ??= bindRust(glue, wasm));
const random = (n) => crypto.getRandomValues(new Uint8Array(n));
const nextMidnight = (t) => {
  const d = new Date(t);
  return Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate() + 1);
};

export class Ledger extends DurableObject {
  constructor(ctx, env) {
    super(ctx, env);
    this.ops = ledgerOps(bound(), random);
    this.state = "";
    ctx.blockConcurrencyWhile(async () => {
      this.state = (await ctx.storage.get("ledger")) || "";
    });
  }

  async run(f) {
    const r = f(this.state);
    this.state = r.state;
    await this.ctx.storage.put("ledger", this.state);
    if ((await this.ctx.storage.getAlarm()) == null) await this.ctx.storage.setAlarm(nextMidnight(Date.now()));
    return r.error ? { error: r.error } : { value: r.value };
  }

  ipKey(ip) {
    return this.run((s) => this.ops.ipKey(s, Date.now(), ip));
  }

  admit(a) {
    return this.run((s) => this.ops.admit(s, Date.now(), a));
  }

  settle(ticket, usage) {
    return this.run((s) => this.ops.settle(s, Date.now(), ticket, usage));
  }

  async alarm() {
    await this.run((s) => this.ops.rollover(s, Date.now()));
    await this.ctx.storage.setAlarm(nextMidnight(Date.now()));
  }
}

export default {
  async fetch(request, env, ctx) {
    const ns = env.LEDGER.jurisdiction("eu");
    const ledger = ns.get(ns.idFromName("ledger"));
    const proxy = makeProxy({
      allow,
      ledger,
      secret: (name) => env[name],
      vars: env,
      rateLimit: async (key) => (await env.RATE.limit({ key })).success,
      upstream: (url, init) => fetch(url, init),
      waitUntil: (p) => ctx.waitUntil(p),
    });
    return proxy.handle(request, request.headers.get("cf-connecting-ip") || "");
  },
};
