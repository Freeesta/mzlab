# Anonymous public assistant proxy (`proxy/`)
Cloudflare Worker that forwards the site assistant's requests to the provider without a user key, with a spending cap per session, per IP, per day and per month. Phases, deploy steps and cost: sections «Phases and choices», «Owner steps», «Cost» at the bottom.

## Pieces
- `crates/mzlab-llm/src/spend/ledger.rs`: `Ledger` (spend ledger in micro-dollars, JSON state `ledger/1` written with `mzlab_core::json`, no serde): `rollover(day)`, `ip_hash(ip, entropy)`, `admit(prices, model, byte, max_tokens, &Keys, &Caps) -> Hold`, `settle(prices, &Hold, Option<&Usage>)`; `usage_of_stream` derives usage from the `message_start` and `message_delta` events (`provider::anthropic::start_usage`). Errors: `err.llm.limit.spend` (parameters `limit`, `cap`, `spent`), `err.llm.ledger` for an invalid state or key.
- `crates/mzlab-llm-wasm/src/ledger.rs`: wasm exports `ledger_rollover`, `ledger_ip_hash`, `ledger_admit`, `ledger_settle`, `max_cost`, `assist_schemas` (the closed schemas of `subcheck::closed_schemas`: `router/1`, `reading/1`, `param-choices/1`); the proxy uses the same `.wasm` `llm` as the site.
- `proxy/rust.js` binds the module (`{value}` or `{error}`); `proxy/ledger-core.js` (`ledgerOps`) always does `rollover` to the current UTC day before every operation; `proxy/assist-proxy.js` (`makeProxy(deps).handle(request, ip)`, `checkBody`, `usageSniffer`) holds the gates; `proxy/adapters/cloudflare.js` (Durable Object `Ledger` with RPC and an alarm at UTC midnight, jurisdiction `eu`, binding `RATE`) and `proxy/adapters/memoria.js` (serialized queue and sliding window, for tests) are the adapters.
- `proxy/build-allowlist.mjs` generates `allowlist.json` (`proxy-allow/1`): SHA-256 of every system prompt in `crates/mzlab-llm/data/assist/` (full text and without trailing whitespace) with the role, models and `max_tokens` of the demo (`demo_max_tokens` of `assist-ruoli-1.json`, read by `demoMaxTokens`: router 200, readers 2000, charts 800, orchestrator 1500, plan 3000).
- The list also carries `sistema-1.md` as orchestrator, the `06_piano` phase prompt, the closed schemas (`JSON.stringify` of the JSON read) and the tool catalog as `Llm.request` writes it from `docs/schemi/tools-demo-1.json` (`CATALOGS`: the 12 tools of the demo; the full catalog is rejected).
- `proxy/build.sh` (`[build]` command of `wrangler.toml`): compiles `mzlab-llm-wasm`, `wasm-bindgen` 0.2.129 into `proxy/wasm/`, `wasm_opt.sh` with the `limit_llm` ceiling, then `allowlist.json`; needs cargo, the wasm32 target, wasm-bindgen and node. `proxy/wasm/` and `proxy/allowlist.json` are not versioned.

## Gates, in order
- Path other than `POST`/`OPTIONS /v1/messages` → 404 `err.llm.proxy.body` (`field: path`); origin outside `https://freeesta.github.io`, `mzlab://localhost`, `http://mzlab.localhost` → 403 `err.llm.proxy.origin`; `OPTIONS` → 204 preflight.
- `ASSIST_OFF` (variable or secret) → 503 `err.llm.proxy.off`; with `PHASE` other than `"1"` only the owner passes (header `x-mzlab-owner` whose SHA-256 matches the secret `OWNER_SHA256` in constant time), otherwise 403 `err.llm.proxy.closed`.
- `x-mzlab-session` must be `[A-Za-z0-9_-]{8,64}`; body at most 120 KB, valid UTF-8 JSON; every body rejection is 400 `err.llm.proxy.body` with the field (`session`, `size`, `json`, `model`, `system`, `tools`, ...).
- `checkBody`: closed top-level keys; models `claude-sonnet-5-5` and `claude-haiku-5-5` according to the role; SHA-256 of the system prompt in the list; `max_tokens` not above the role's ceiling (the phase prompt raises it to 3000); at most 40 messages; blocks only `text`, `tool_use`, `tool_result`; every user-written text at most 2000 characters; tools only for the orchestrator, without a `type` key and with a hash in the list; `tool_choice`, `thinking`, `cache_control` and `output_config` (schema in the list) within the shapes the site sends.
- For non-owners: IP key = HMAC-SHA256 of the IP with a daily salt (16 random bytes, changes at UTC midnight, never stored beyond the day), first 16 bytes in hexadecimal; rate limit 12 requests per minute per IP key → 429 `err.llm.limit.spend` with `limit: rate`, `retry-after: 60`.
- `admit` reserves the worst case (`max_cost(model, ceil(byte/3), max_tokens)`) on session, IP, day and month; public caps $0.5 per session, $1 per IP, $6 per day, $150 per month; owner $2 per session, $10 per day, $150 per month, no per-IP cap. Day or month exceeded → 503 `err.llm.proxy.budget`; session or IP → 429 `err.llm.limit.spend`; both with `x-mzlab-limit`.

## Forwarding and settlement
- Forwards with `x-api-key` (secret `ANTHROPIC_KEY`) and `anthropic-version: 2023-06-01`; a network error → 502 `err.llm.network` and the reservation stays spent.
- Non-2xx provider response: settled with zero usage, body passed through as is (replaced if it looks like a key), of the headers only `retry-after` and `request-id` remain.
- 2xx response: the SSE bytes pass unchanged; `usageSniffer` decodes only the lines with `data:` and `"message_` in the first 48 bytes; at end of stream, error or cancellation (`waitUntil`) `settle` charges the true cost, or the whole reservation if usage did not fully arrive (truncated stream).
- The true cost exceeds the reservation only if the input exceeds the `byte/3` estimate: the ledger records the true cost, so a cap can be exceeded by that estimation error.
- Exposed headers: `x-mzlab-spend` `spent/cap` of the session in micro-dollars, `x-mzlab-soft: 1` from 80 %, `x-mzlab-limit`, `retry-after`, `request-id`; errors as `{error, error_key, params}`; the `err.llm.proxy.*` keys do not yet have texts in `lang/` (they belong to the assistant panel).
- No logs: in `wrangler.toml` `observability` and `logs` are off, `logpush = false`; the code writes only `console.error("err.llm.proxy.settle")` if settlement fails, with no request data.

## Measurements and tests
- CPU per request (node 22, warm, fake provider removed): about 2.5 ms for a 1 KB response, about 5 ms for a 462 KB stream with 4000 deltas; first request about 3 ms.
- `cargo test -p mzlab-llm --test spend` (caps never exceeded, two admissions in parallel, truncated stream, day and month change, HMAC pinned to the Python reference); `node --test proxy/test/` (`body`, `spend`, `secret`, `nolog`, `cpu`; the Rust from `MZLAB_LLM_WASM_DIR` or compiled in a temporary folder); `tests/test_proxy.py` launches them and checks that `proxy/` contains no `sk-ant`.

## Phases and choices
- Phase 0 (`PHASE = "0"` in `wrangler.toml`): only the owner passes, with the secret phrase (`x-mzlab-owner`); the phrase is typed once in the panel, lives only in the IndexedDB of the transport iframe (never in the main page) and is erased with «Forget». It is not an account, it is a switch.
- Phase 1 (`PHASE = "1"`): every visitor, only in the Flow behind «Try the assistant», never in the course pages; opens only after the demo is proven and the data protection officer's opinion (the IP is personal data even if processed only in memory). Off: `ASSIST_OFF`, or a key revoked in the Console.
- Host: Cloudflare Workers, free plan, one Durable Object as ledger; paid plan only if the CPU per request is not enough (measurement in «Measurements and tests»). Deno Deploy is the second choice; Vercel Hobby, Netlify and AI Gateway are not enough (non-commercial use, duration, no per-IP-hash cap nor closed prompt).
- Key: only secret `ANTHROPIC_KEY` of a dedicated workspace «mzlab-demo» with a monthly spend limit of $180; the workspace-limit 400 is `err.llm.limit.workspace` and stops without retrying. Rotation = new secret + revocation of the old one. Never in the repository, in chats, in the page or in the app.
- Models: Sonnet 5.5 effort medium for readers and orchestrator, Haiku 5.5 effort low for router and charts; Opus never in the product.
- Closed body: no server-side tools (`web_search`, `web_fetch`, `code_execution`, MCP), no `service_tier`, `temperature`, `top_p`, `top_k`, images or documents. The system prompt refuses what does not concern mzLab and open data: the proxy is not a free chatbot.
- A non-browser client can lie about the `Origin`: the caps count, not CORS. Bots and rotating IPs cost at most the daily cap ($6).
- Consent before the first call of every visit: what is sent (the reduced facts, with «What will be sent»), to whom (mzLab proxy on Cloudflare, then Anthropic), that mzLab stores nothing (no text, no files, no IP: only a daily counter on the mixed IP), that Anthropic processes the data under its commercial API terms and does not train, that raw files are not sent and that everything else works without consent.
- Site/app parity: the app uses the same provider `mzlab-demo` of `fornitori-1.json` (proxy url, no key) and sends the same body byte for byte (`tools_demo.rs`, `provider/demo.rs`). In the demo the panel's cap is $0.5 and does not rise: the real one belongs to the proxy.
- Proposal not implemented: Turnstile, only if bots exhaust the daily cap twice (adds a host to the CSP).

## Owner steps (no chat does them)
1) Anthropic Console: workspace «mzlab-demo», monthly limit $180, one key for that workspace only.
2) Cloudflare (free plan): from the `proxy/` folder `npx wrangler secret put ANTHROPIC_KEY`, `npx wrangler secret put OWNER_SHA256` (SHA-256 of the secret phrase, computed locally), then `npx wrangler deploy`.
3) Only the proxy's public address goes to the Coordinator (for the transport CSP), never key or phrase.
4) Phase 1: `PHASE = "1"` in `wrangler.toml` or in the dashboard. To turn off: `ASSIST_OFF = 1` or key revocation.

## Cost
- Prices (USD per million tokens, input / cache write 5 min / 1 h / read / output): Sonnet 5.5 2 / 2.5 / 4 / 0.10 / 10; Haiku 5.5 0.10 / 0.125 / 0.20 / 0.01 / 0.50 (prompt ≤ 100 000 tokens). `inference_geo: "us"` costs ×1.1 and is not used. Reasoning is paid as output; `max_tokens` per role limits it.
- Cache: one `ttl: "1h"` point on the last system block (after the tools) because pauses beyond 5 minutes are normal, automatic 5-minute cache for the tail; catalog with stable bytes, tools in fixed order, no variable date or id in the system prompt; mid-conversation instructions enter as a `role: "system"` message without touching the prefix. Minimum prefix 512 tokens.
- Admission: maximum cost = `ceil(byte/3)` × full price + `max_tokens` × output price; if spent + maximum exceeds the cap the call does not go out. At 80 % the assistant says so once, at 100 % no new call and the Rust parts remain.
- The rate 429 with `retry-after` is retried at most 2 times; the 429 without `retry-after` (`enforced_spend_limit_reached`) never.
- Cost targets per task (spend / completed golden tasks): metadata reading ≤ $0.05, parameter proposal ≤ $0.10, table reading ≤ $0.10, chart ≤ $0.01, whole session ≤ $1.50 median and ≤ $5 maximum. Typical session with own key (12 files, one run, 25 turns) ≈ $0.77; the cache is worth about 60 % of it.
- With $150 per month: ≈ 535 typical public sessions ($0.28) or 300 at the session cap, ≈ 12-21 per day with the $6 cap. Expiring credits: phase 1 lasts as long as the credit.
