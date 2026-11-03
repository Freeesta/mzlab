// The anonymous public proxy of the mzLab assistant for the Claude API (docs/agenti/assistente-proxy.md): paths, CORS and Origin, phase and switch,
// the closed body (only the prompts, schemas and catalog of mzLab pass), the reservation in the Rust ledger, forwarding with the secret key, the SSE
// stream passed through unchanged while only `message_start`/`message_delta` are read for the usage, and the settlement. Standard Request/Response
// only; the host gives the ledger, the secrets, the rate limit and the upstream fetch (adapters/). It never logs content.

export const UPSTREAM = "https://api.anthropic.com/v1/messages";
export const ORIGINS = new Set(["https://freeesta.github.io", "mzlab://localhost", "http://mzlab.localhost"]);
export const MODELS = new Set(["claude-sonnet-5-5", "claude-haiku-5-5"]);
export const LIMITS = { bodyBytes: 120 * 1024, messages: 40, userChars: 2000 };
const DOLLAR = 1_000_000;
/** Micro-dollars. `ip` 0 = no IP counter (the owner). */
export const CAPS = {
  owner: { session: 2 * DOLLAR, ip: 0, day: 10 * DOLLAR, month: 150 * DOLLAR },
  public: { session: DOLLAR / 2, ip: DOLLAR, day: 6 * DOLLAR, month: 150 * DOLLAR },
};
const SOFT_PERCENT = 80;
const SESSION = /^[A-Za-z0-9_-]{8,64}$/;
const SECRET_LIKE = /(^|[^A-Za-z0-9])(sk-|AIza)[A-Za-z0-9]/;
const TOP_KEYS = new Set(["model", "max_tokens", "stream", "thinking", "output_config", "cache_control", "system", "messages", "tools", "tool_choice"]);
const EXPOSED = "x-mzlab-spend, x-mzlab-soft, x-mzlab-limit, retry-after, request-id";
// Anthropic charges nothing for a refused request: the reservation is settled at zero
const ZERO_USAGE = { start: { input_tokens: 0, output_tokens: 0 }, delta: { output_tokens: 0 } };

const enc = new TextEncoder();

export async function sha256hex(text) {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", typeof text === "string" ? enc.encode(text) : text));
  let s = "";
  for (const b of d) s += b.toString(16).padStart(2, "0");
  return s;
}

/** Equal hex digests, in a time that does not depend on where they differ. */
function sameDigest(a, b) {
  if (typeof a !== "string" || typeof b !== "string" || a.length !== 64 || b.length !== 64) return false;
  let diff = 0;
  for (let i = 0; i < 64; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

function cors(origin) {
  return ORIGINS.has(origin) ? { "access-control-allow-origin": origin, vary: "Origin", "access-control-expose-headers": EXPOSED } : {};
}

function fail(status, key, params, origin, extra) {
  const body = JSON.stringify({ error: key, error_key: key, params: params || {} });
  return new Response(body, { status, headers: { "content-type": "application/json", "cache-control": "no-store", ...cors(origin), ...(extra || {}) } });
}

const bodyError = (field, origin) => fail(400, "err.llm.proxy.body", { field }, origin);

/** Code points of a string (the 2 000-character limit counts characters, not UTF-16 units). */
function chars(s) {
  let n = 0;
  for (const _ of s) n++;
  return n;
}

/** What the person typed in the last user message: plain text blocks, or the `message` of a JSON block (the router's input); data blocks count 0. */
function typedChars(m) {
  const blocks = typeof m.content === "string" ? [{ type: "text", text: m.content }] : m.content;
  let n = 0;
  for (const b of blocks) {
    if (b.type !== "text" || typeof b.text !== "string") continue;
    const t = b.text.trimStart();
    if (t.startsWith("{")) {
      try {
        const v = JSON.parse(t);
        if (v && typeof v.message === "string") n += chars(v.message);
        continue;
      } catch {
        // not JSON: typed text
      }
    }
    n += chars(b.text);
  }
  return n;
}

const BLOCKS = new Set(["text", "tool_use", "tool_result"]);

function checkBlocks(content) {
  if (typeof content === "string") return true;
  if (!Array.isArray(content) || content.length === 0) return false;
  for (const b of content) {
    if (!b || typeof b !== "object" || !BLOCKS.has(b.type)) return false;
    if (b.type === "tool_result" && typeof b.content !== "string") {
      if (!Array.isArray(b.content) || b.content.some((c) => !c || c.type !== "text")) return false;
    }
  }
  return true;
}

function systemText(system) {
  if (typeof system === "string") return system;
  if (Array.isArray(system) && system.length === 1 && system[0] && system[0].type === "text" && typeof system[0].text === "string") return system[0].text;
  return null;
}

const plainObject = (v) => v !== null && typeof v === "object" && !Array.isArray(v);

/** The closed body: `{role, model, maxTokens}` when every field is one mzLab sends, else `{field}` (the first field that is not). */
export async function checkBody(b, allow) {
  if (!plainObject(b)) return { field: "json" };
  for (const k of Object.keys(b)) if (!TOP_KEYS.has(k)) return { field: k };
  if (b.stream !== true) return { field: "stream" };
  if (!MODELS.has(b.model)) return { field: "model" };
  const text = systemText(b.system);
  const role = text === null ? undefined : allow.system[await sha256hex(text)];
  if (!role) return { field: "system" };
  if (!role.models.includes(b.model)) return { field: "model" };
  let cap = role.max_tokens;
  if (!Array.isArray(b.messages) || b.messages.length === 0 || b.messages.length > LIMITS.messages) return { field: "messages" };
  let lastUser = null;
  for (const m of b.messages) {
    if (!plainObject(m)) return { field: "messages" };
    if (m.role === "system") {
      const phase = typeof m.content === "string" && role.role === "orchestrator" ? allow.phase[await sha256hex(m.content)] : undefined;
      if (!phase) return { field: "messages.system" };
      cap = Math.max(cap, phase.max_tokens);
      continue;
    }
    if (m.role !== "user" && m.role !== "assistant") return { field: "messages.role" };
    if (!checkBlocks(m.content)) return { field: "messages.content" };
    if (m.role === "user") lastUser = m;
  }
  if (lastUser && typedChars(lastUser) > LIMITS.userChars) return { field: "messages.text" };
  if (!Number.isInteger(b.max_tokens) || b.max_tokens < 1 || b.max_tokens > cap) return { field: "max_tokens" };
  if (b.tools !== undefined) {
    if (role.role !== "orchestrator" || !Array.isArray(b.tools)) return { field: "tools" };
    // server tools (web_search, web_fetch, code_execution, MCP) carry a `type`: never, whatever the list says
    if (b.tools.some((t) => !plainObject(t) || "type" in t)) return { field: "tools" };
    if (!allow.tools.includes(await sha256hex(JSON.stringify(b.tools)))) return { field: "tools" };
  }
  if (b.tool_choice !== undefined) {
    const c = b.tool_choice;
    if (b.tools === undefined || !plainObject(c) || !["auto", "any", "tool", "none"].includes(c.type)) return { field: "tool_choice" };
    if (Object.keys(c).some((k) => !["type", "name", "disable_parallel_tool_use"].includes(k))) return { field: "tool_choice" };
  }
  if (b.thinking !== undefined) {
    const t = b.thinking;
    if (!plainObject(t) || Object.keys(t).length !== 1 || !["adaptive", "disabled"].includes(t.type)) return { field: "thinking" };
  }
  if (b.cache_control !== undefined) {
    const c = b.cache_control;
    if (!plainObject(c) || c.type !== "ephemeral" || Object.keys(c).some((k) => k !== "type" && k !== "ttl")) return { field: "cache_control" };
  }
  if (b.output_config !== undefined) {
    const o = b.output_config;
    if (!plainObject(o) || Object.keys(o).some((k) => k !== "effort" && k !== "format")) return { field: "output_config" };
    if (o.effort !== undefined && !["low", "medium", "high"].includes(o.effort)) return { field: "output_config" };
    if (o.format !== undefined) {
      const f = o.format;
      if (!plainObject(f) || f.type !== "json_schema" || Object.keys(f).length !== 2) return { field: "output_config" };
      if (!allow.schemas.includes(await sha256hex(JSON.stringify(f.schema)))) return { field: "output_config" };
    }
  }
  return { role: role.role, model: b.model, maxTokens: b.max_tokens };
}

const DATA = enc.encode("data:");
const MARK = enc.encode('"message_');
// the event type is the first key of an Anthropic frame: a frame whose type comes later is not read, and its call keeps the whole reservation
const HEAD = 48;

function at(bytes, pat, from) {
  for (let j = 0; j < pat.length; j++) if (bytes[from + j] !== pat[j]) return false;
  return true;
}

/** Is this line (bytes, no newline) the `data:` of a `message_start` or `message_delta`? Only its first HEAD bytes are looked at. */
function candidate(line) {
  if (line.length < DATA.length || !at(line, DATA, 0)) return false;
  const end = Math.min(line.length, HEAD) - MARK.length;
  for (let i = DATA.length; i <= end; i++) if (line[i] === 0x22 && at(line, MARK, i)) return true;
  return false;
}

/** Reads `message_start.message.usage` and the last `message_delta.usage` from the SSE bytes as they pass: newlines are found in the bytes and only
 * the lines that are those two events are decoded and parsed; nothing else of the answer is looked at. */
function usageSniffer() {
  const dec = new TextDecoder();
  const usage = { start: null, delta: null };
  let carry = null;
  let skip = false;
  const line = (bytes) => {
    if (!candidate(bytes)) return;
    try {
      const v = JSON.parse(dec.decode(bytes.subarray(DATA.length)));
      if (v.type === "message_start") usage.start = (v.message && v.message.usage) || null;
      else if (v.type === "message_delta" && v.usage) usage.delta = { ...(usage.delta || {}), ...v.usage };
    } catch {
      // a broken frame gives no usage: the reservation stays
    }
  };
  const join = (a, b) => {
    const out = new Uint8Array(a.length + b.length);
    out.set(a);
    out.set(b, a.length);
    return out;
  };
  return {
    push(chunk) {
      let from = 0;
      for (let i = chunk.indexOf(0x0a); i !== -1; i = chunk.indexOf(0x0a, from)) {
        if (!skip) line(carry ? join(carry, chunk.subarray(from, i)) : chunk.subarray(from, i));
        carry = null;
        skip = false;
        from = i + 1;
      }
      if (from >= chunk.length || skip) return;
      carry = carry ? join(carry, chunk.subarray(from)) : chunk.slice(from);
      // a long line that is not one of the two events is not kept until its end
      if (carry.length >= HEAD && !candidate(carry)) {
        carry = null;
        skip = true;
      }
    },
    usage: () => (usage.start && usage.delta ? usage : null),
  };
}

function spendHeaders(session, cap) {
  const used = session[0] + session[1];
  const h = { "x-mzlab-spend": `${used}/${cap}` };
  if (used * 100 >= cap * SOFT_PERCENT) h["x-mzlab-soft"] = "1";
  return h;
}

const BLANK_ERROR = JSON.stringify({ type: "error", error: { type: "api_error", message: "" } });

/** Of the provider's headers only these reach the page. */
function keptHeaders(up) {
  const keep = {};
  for (const h of ["retry-after", "request-id"]) {
    const v = up.headers.get(h);
    if (v) keep[h] = v;
  }
  return keep;
}

function preflight(origin) {
  return new Response(null, {
    status: 204,
    headers: { ...cors(origin), "access-control-allow-methods": "POST", "access-control-allow-headers": "content-type, x-mzlab-session, x-mzlab-owner", "access-control-max-age": "600" },
  });
}

/** The request body as bytes and JSON, or the `field` that rejects it (`size`, `json`). */
async function readJson(request) {
  if (Number(request.headers.get("content-length") || 0) > LIMITS.bodyBytes) return { field: "size" };
  const bytes = new Uint8Array(await request.arrayBuffer());
  if (bytes.length > LIMITS.bodyBytes) return { field: "size" };
  try {
    return { bytes, parsed: JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) };
  } catch {
    return { field: "json" };
  }
}

/** A refused reservation: the global caps close the demo for everybody (503), the session and IP caps only for this visitor (429). */
function admitFail(error, origin) {
  const limit = error.params && error.params.limit;
  if (error.error_key !== "err.llm.limit.spend") return fail(500, error.error_key, error.params, origin);
  if (limit === "day" || limit === "month") return fail(503, "err.llm.proxy.budget", { limit }, origin, { "x-mzlab-limit": limit });
  return fail(429, "err.llm.limit.spend", error.params, origin, { "x-mzlab-limit": limit });
}

/** The bytes of a 2xx answer pass unchanged; `finish()` (the settlement, run once) runs at the end, on a read error or on a cancel. */
function passThrough(source, sniff, finish, waitUntil) {
  const reader = source.getReader();
  return new ReadableStream({
    async pull(ctrl) {
      let r;
      try {
        r = await reader.read();
      } catch (e) {
        await finish();
        ctrl.error(e);
        return;
      }
      if (r.done) {
        await finish();
        ctrl.close();
        return;
      }
      sniff.push(r.value);
      ctrl.enqueue(r.value);
    },
    async cancel(reason) {
      const p = finish();
      waitUntil?.(p);
      await reader.cancel(reason).catch(() => {});
      await p;
    },
  });
}

/**
 * deps: `allow` (allowlist.json), `ledger` ({ipKey(ip), admit(args), settle(ticket, usage)} → {value}|{error}), `secret(name)`, `vars` (PHASE,
 * ASSIST_OFF), `rateLimit(key)` → true when allowed, `upstream(url, init)` (fetch), optional `waitUntil(promise)` and `caps`.
 */
export function makeProxy(deps) {
  const caps = deps.caps || CAPS;
  const flag = (name) => deps.vars?.[name] === "1" || deps.secret(name) === "1";

  async function isOwner(request) {
    const phrase = request.headers.get("x-mzlab-owner");
    const want = deps.secret("OWNER_SHA256");
    if (!phrase || !want) return false;
    return sameDigest(await sha256hex(phrase), want.trim().toLowerCase());
  }

  async function settle(ticket, usage) {
    const r = await deps.ledger.settle(ticket, usage);
    if (r.error) console.error("err.llm.proxy.settle");
    return r;
  }

  async function forward(bytes, origin, ticket, head) {
    let up;
    try {
      up = await deps.upstream(UPSTREAM, {
        method: "POST",
        headers: { "content-type": "application/json", accept: "text/event-stream", "x-api-key": deps.secret("ANTHROPIC_KEY") || "", "anthropic-version": "2023-06-01" },
        body: bytes,
      });
    } catch {
      await settle(ticket, null);
      return fail(502, "err.llm.network", {}, origin);
    }
    const keep = keptHeaders(up);
    if (!up.ok || !up.body) {
      const text = await up.text().catch(() => "");
      const r = await settle(ticket, ZERO_USAGE);
      return new Response(SECRET_LIKE.test(text) ? BLANK_ERROR : text, {
        status: up.status,
        headers: { "content-type": "application/json", "cache-control": "no-store", ...cors(origin), ...keep, ...spendHeaders(r.value ? r.value.session : head.session, head.cap) },
      });
    }
    const sniff = usageSniffer();
    let done = null;
    const finish = () => (done ??= settle(ticket, sniff.usage()));
    return new Response(passThrough(up.body, sniff, finish, deps.waitUntil), {
      status: up.status,
      headers: { "content-type": up.headers.get("content-type") || "text/event-stream", "cache-control": "no-store", ...cors(origin), ...keep, ...spendHeaders(head.session, head.cap) },
    });
  }

  async function handle(request, ip) {
    const url = new URL(request.url);
    const origin = request.headers.get("origin") || "";
    if (url.pathname !== "/v1/messages" || (request.method !== "POST" && request.method !== "OPTIONS")) return fail(404, "err.llm.proxy.body", { field: "path" }, origin);
    if (!ORIGINS.has(origin)) return fail(403, "err.llm.proxy.origin", {}, "");
    if (request.method === "OPTIONS") return preflight(origin);
    if (flag("ASSIST_OFF")) return fail(503, "err.llm.proxy.off", {}, origin);
    const owner = await isOwner(request);
    if (deps.vars?.PHASE !== "1" && !owner) return fail(403, "err.llm.proxy.closed", {}, origin);
    const session = request.headers.get("x-mzlab-session") || "";
    if (!SESSION.test(session)) return bodyError("session", origin);
    const { bytes, parsed, field } = await readJson(request);
    if (field) return bodyError(field, origin);
    const c = await checkBody(parsed, deps.allow);
    if (c.field) return bodyError(c.field, origin);
    const mine = owner ? caps.owner : caps.public;
    let ipHash = null;
    if (!owner) {
      const k = await deps.ledger.ipKey(ip || "");
      if (k.error) return fail(500, k.error.error_key, k.error.params, origin);
      ipHash = k.value;
      if (!(await deps.rateLimit(ipHash))) return fail(429, "err.llm.limit.spend", { limit: "rate" }, origin, { "x-mzlab-limit": "rate", "retry-after": "60" });
    }
    const a = await deps.ledger.admit({ model: c.model, inBytes: bytes.length, maxTokens: c.maxTokens, session, ipHash, caps: mine });
    if (a.error) return admitFail(a.error, origin);
    return forward(bytes, origin, a.value.ticket, { session: a.value.session, cap: mine.session });
  }

  return { handle };
}
