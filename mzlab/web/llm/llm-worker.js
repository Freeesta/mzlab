// @ts-check
// Worker of the assistant (module worker): loads the Rust module `wasm-feat/llm/` (manifest, SHA-256 checked) on the first message and only then. Builds each request and decodes
// each answer with it; the bytes travel over a MessagePort to/from the iframe `llm/trasporto.html`, which owns the key. The transport `replay` answers from a recording and never uses the iframe.
// Page -> worker: init, port, request, send, abort, replay.load, replay.reset. Worker -> page: {id, ok, ...} per call and {id, event} per normalized event.
/** @typedef {{request: (conv: string) => string, replay_load: (text: string, strict: boolean) => number, replay_respond: (conv: string) => string, replay_reset: () => void}} Llm */
/** @typedef {{push: (b: Uint8Array) => string, finish: () => string}} Dec */

/** @type {any} */ let mod = null;
/** @type {Llm | null} */ let llm = null;
/** @type {MessagePort | null} */ let port = null;
/** @type {Map<string, {dec: Dec, status: number, body: string[]}>} */
const live = new Map();

/** @param {ArrayBuffer} bytes */
const sha256 = async bytes => Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)), b => b.toString(16).padStart(2, "0")).join("");

const load = async () => {
  if (llm) return;
  const root = new URL("../wasm-feat/", import.meta.url);
  const res = await fetch(new URL("manifest.json", root), { cache: "no-store" });
  const entry = res.ok ? (await res.json()).llm : null;
  if (!entry) throw { key: "err.llm.module", params: { reason: "manifest" } };
  const bytes = await (await fetch(new URL(entry.wasm, root))).arrayBuffer();
  if ((await sha256(bytes)) !== entry.sha256) throw { key: "err.llm.module", params: { reason: "sha256" } };
  mod = await import(/* @vite-ignore */ new URL(entry.js, root).href);
  await mod.default({ module_or_path: bytes });
  const tools = await fetch(new URL("strumenti-1.json", import.meta.url));
  if (!tools.ok) throw { key: "err.llm.module", params: { reason: "tools" } };
  llm = new mod.Llm(await tools.text());
};

/** a Rust error is a JSON text {error_key, params}; anything else is a failure of the module @param {any} e */
const wrap = e => {
  if (e && e.key) return e;
  try { const j = JSON.parse(String(e)); return { key: j.error_key || "err.llm.module", params: j.params || {} }; } catch (_) { return { key: "err.llm.module", params: { reason: String(e && e.message || e).slice(0, 80) } }; }
};
const emit = (/** @type {string} */ id, /** @type {string} */ lines) => { for (const l of lines.split("\n")) if (l) postMessage({ id, event: l }); };

/** @param {any} m */
const send = async m => {
  if (!llm) return;
  if (m.transport === "replay") { emit(m.id, llm.replay_respond(m.conv)); postMessage({ id: m.id, done: true }); return; }
  if (!port) throw { key: "err.llm.network", params: { reason: "noport" } };
  const conv = JSON.parse(m.conv);
  live.set(m.id, { dec: new mod.LlmStream(conv.provider, conv.model), status: 0, body: [] });
  port.postMessage({ t: "send", id: m.id, request: JSON.parse(llm.request(m.conv)) });
};

/** the iframe's side of a call: head (status), chunk (bytes), end, fail @param {MessageEvent} ev */
const onPort = ev => {
  const m = ev.data || {}, c = live.get(m.id);
  if (!c) return;
  if (m.t === "head") c.status = m.status;
  else if (m.t === "chunk") {
    if (c.status >= 400) c.body.push(new TextDecoder().decode(m.bytes));
    else emit(m.id, c.dec.push(new Uint8Array(m.bytes)));
  } else if (m.t === "end") {
    if (c.status >= 400) emit(m.id, mod.http_error(c.status, c.body.join("")) + "\n");
    else emit(m.id, c.dec.finish());
    live.delete(m.id); postMessage({ id: m.id, done: true });
  } else if (m.t === "fail") {
    postMessage({ id: m.id, event: JSON.stringify({ ev: "error", key: m.key, params: m.params || {} }) });
    live.delete(m.id); postMessage({ id: m.id, done: true });
  }
};

onmessage = async ev => {
  const m = ev.data || {};
  try {
    if (m.op === "port") { port = ev.ports[0]; port.onmessage = onPort; return; }
    await load();
    if (!llm) return;
    if (m.op === "init") postMessage({ id: m.id, ok: true, providers: mod.providers(), version: mod.version() });
    else if (m.op === "request") postMessage({ id: m.id, ok: true, request: llm.request(m.conv) });
    else if (m.op === "replay.load") postMessage({ id: m.id, ok: true, turns: llm.replay_load(m.text, !!m.strict) });
    else if (m.op === "replay.reset") { llm.replay_reset(); postMessage({ id: m.id, ok: true }); }
    else if (m.op === "send") await send(m);
    else if (m.op === "abort") { live.delete(m.id); if (port) port.postMessage({ t: "abort", id: m.id }); postMessage({ id: m.id, done: true }); }
  } catch (e) {
    const x = wrap(e);
    postMessage({ id: m.id, ok: false, error_key: x.key, params: x.params });
  }
};
