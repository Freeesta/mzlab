// @ts-check
// @ts-strict
// Worker of the assistant (module worker): loads the Rust module `wasm-feat/llm/` (manifest, SHA-256 checked) on the first message and only then. Builds each request and decodes
// each answer with it; the bytes travel over a MessagePort to/from the iframe `llm/trasporto.html`, which owns the key. The transport `replay` answers from a recording and never uses the iframe.
// Page -> worker: init, port, request, send (a conversation, or a ready `request` of a subagent), abort, assist.files, assist.on, assist.request, assist.check, assist.readers, assist.state, assist.reading, sub.replay, replay.load, replay.reset, chart.views, chart.make, chart.presets (charts: grafico/1, docs/agenti/llm.md), inspect.build, params.propose (the facts without a model: ispezione/1, proposta-param/1), agent.new, agent.in (the loop of the assistant: llm-agente.js). Worker -> page: {id, ok, ...} per call and {id, event} per normalized event.
// The provider of the public demo (`mzlab-demo`): its conversations use the demo catalog `strumenti-demo-1.json`, and each of its requests leaves through `demo_send` of Rust (the host of
// proxy-host.js in place of the placeholder, the session header); with an empty host nothing leaves. The spend headers of the proxy reach the page as {id, spend}.
import { PROXY_HOST } from "./proxy-host.js";
/** @typedef {{request: (conv: string) => string, replay_load: (text: string, strict: boolean) => number, replay_respond: (conv: string) => string, replay_reset: () => void}} Llm */
/** @typedef {{push: (b: Uint8Array) => string, finish: () => string}} Dec */

/** @type {any} */ let mod = null;
/** @type {Llm | null} */ let llm = null;
let toolsText = "";
const DEMO = "mzlab-demo";
/** the demo catalog and its builder, fetched on the first conversation of the demo @type {Promise<{text: string, llm: Llm}> | null} */
let demoTools = null;
const loadDemo = () => demoTools || (demoTools = (async () => {
  const r = await fetch(new URL("strumenti-demo-1.json", import.meta.url));
  if (!r.ok) throw { key: "err.llm.module", params: { reason: "tools" } };
  const text = await r.text();
  return { text, llm: new mod.Llm(text) };
})().catch(e => { demoTools = null; throw e; }));
/** @type {any} */ let agent = null;
/** the readers of the subagents (`Assist` of the module), created by `assist.on` @type {any} */ let assist = null;
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
  toolsText = await tools.text();
  llm = new mod.Llm(toolsText);
};

/** the roles and prompt files of the subagents, fetched on first request (they are data beside the page, not part of the module) @type {Promise<{roles: string, files: [string, string][]}> | null} */
let assistFiles = null;
const loadAssist = () => assistFiles || (assistFiles = (async () => {
  const get = async (/** @type {string} */ name) => {
    const r = await fetch(new URL("assist/" + name, import.meta.url));
    if (!r.ok) throw { key: "err.llm.module", params: { reason: "assist" } };
    return r.text();
  };
  const roles = await get("assist-ruoli-1.json");
  const j = JSON.parse(roles), phase = j.orchestrator && j.orchestrator.phase_prompt;
  const names = [...new Set([...j.roles.map((/** @type {{prompt: string}} */ r) => r.prompt), ...(phase ? [phase] : [])])];
  return { roles, files: await Promise.all(names.map(async n => /** @type {[string, string]} */ ([n, await get(String(n))]))) };
})().catch(e => { assistFiles = null; throw e; }));

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
  // a call of a subagent comes as a request already built by Rust (`Assist.request`); a conversation is built here
  const conv = m.request ? m : JSON.parse(m.conv);
  let req = m.request ? m.request : (conv.provider === DEMO ? (await loadDemo()).llm : llm).request(m.conv);
  // the owner passphrase is added by the transport document, which alone holds it
  if (conv.provider === DEMO) req = mod.demo_send(req, DEMO, PROXY_HOST, m.demo && m.demo.session || "", "");
  live.set(m.id, { dec: new mod.LlmStream(conv.provider, conv.model), status: 0, body: [] });
  port.postMessage({ t: "send", id: m.id, request: JSON.parse(req) });
};

/** the request of a conversation as it will leave: the demo's with its catalog and, once the proxy has a host, the host and session in place (`demo_send`) @param {any} m */
const requestOf = async m => {
  if (!llm) throw { key: "err.llm.module", params: { reason: "load" } };
  if (JSON.parse(m.conv).provider !== DEMO) return llm.request(m.conv);
  const req = (await loadDemo()).llm.request(m.conv);
  return PROXY_HOST ? mod.demo_send(req, DEMO, PROXY_HOST, m.demo && m.demo.session || "", "") : req;
};

/** the iframe's side of a call: head (status), chunk (bytes), end, fail @param {MessageEvent} ev */
const onPort = ev => {
  const m = ev.data || {}, c = live.get(m.id);
  if (!c) return;
  if (m.t === "head") {
    c.status = m.status;
    // the spend of the demo session, as the proxy reports it on a 2xx answer (parsed by Rust)
    const spend = m.spend && m.status < 300 ? mod.demo_spend(String(m.spend), !!m.soft) : "";
    if (spend) postMessage({ id: m.id, spend });
  }
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

/** one move of the loop of the assistant; the lines are the outputs of the machine (`send`, `call`, `ask`, `show`, `log`, `done`) @param {any} m */
/** the subagents on: the module `Assist` (readers, with the registry of steps) and the router/phase prompts of the machine; the files are fetched once @param {any} m */
const assistOn = async m => {
  const f = await loadAssist();
  const prompts = JSON.stringify(f.files);
  if (!agent) throw { key: "err.llm.module", params: { reason: "agent" } };
  agent.enable_assist(f.roles, prompts, JSON.parse(f.roles).orchestrator.phase_prompt || "");
  assist = new mod.Assist(m.provider, f.roles, prompts, m.steps);
};
const needAssist = () => { if (!assist) throw { key: "err.llm.module", params: { reason: "assist" } }; return assist; };

const agentIn = m => {
  if (!agent) throw { key: "err.llm.module", params: { reason: "agent" } };
  const out = (/** @type {string} */ lines) => ({ lines, outcome: agent.outcome(), usage: Array.from(agent.usage()) });
  if (m.kind === "user") return out(agent.user(m.text, m.context || ""));
  if (m.kind === "event") return out(agent.event(m.line));
  if (m.kind === "result") return out(agent.result(m.callId, !!m.ok, m.content));
  if (m.kind === "confirm") return out(agent.confirm(m.callId, !!m.yes));
  if (m.kind === "stop") return out(agent.stop());
  if (m.kind === "sub") return out(agent.sub_result(m.role, !!m.ok, m.body));
  if (m.kind === "conv") return { ...out(""), conv: agent.conversation() };
  if (m.kind === "log") return { ...out(""), log: agent.log() };
  throw { key: "err.llm.module", params: { reason: "kind" } };
};

onmessage = async ev => {
  const m = ev.data || {};
  try {
    if (m.op === "port") { port = ev.ports[0]; port.onmessage = onPort; return; }
    if (m.op === "assist.files") { postMessage({ id: m.id, ok: true, ...(await loadAssist()) }); return; }
    await load();
    if (!llm) return;
    if (m.op === "init") postMessage({ id: m.id, ok: true, providers: mod.providers(), version: mod.version() });
    else if (m.op === "request") postMessage({ id: m.id, ok: true, request: await requestOf(m) });
    else if (m.op === "replay.load") postMessage({ id: m.id, ok: true, turns: llm.replay_load(m.text, !!m.strict) });
    else if (m.op === "replay.reset") { llm.replay_reset(); postMessage({ id: m.id, ok: true }); }
    else if (m.op === "send") await send(m);
    else if (m.op === "context") postMessage({ id: m.id, ok: true, context: mod.context_build(m.sources, m.budget || 4000) });
    else if (m.op === "chart.views") postMessage({ id: m.id, ok: true, views: mod.chart_views(m.sources) });
    else if (m.op === "chart.make") postMessage({ id: m.id, ok: true, chart: mod.chart_validate(m.spec, m.sources) });
    else if (m.op === "inspect.build") postMessage({ id: m.id, ok: true, inspection: mod.inspect_build(m.sources, m.budget || 30000) });
    else if (m.op === "params.propose") postMessage({ id: m.id, ok: true, proposal: mod.params_propose(m.inspection, m.answers || "", m.steps) });
    else if (m.op === "chart.presets") postMessage({ id: m.id, ok: true, charts: mod.chart_presets(m.sources) });
    else if (m.op === "agent.new") {
      agent = new mod.Agent(m.provider === DEMO ? (await loadDemo()).text : toolsText, m.provider, m.model, m.level, !!m.research);
      if (m.replay) agent.replay_load(m.replay, !!m.strict);
      postMessage({ id: m.id, ok: true });
    } else if (m.op === "agent.in") postMessage({ id: m.id, ok: true, ...agentIn(m) });
    else if (m.op === "assist.on") { await assistOn(m); postMessage({ id: m.id, ok: true }); }
    else if (m.op === "assist.request") postMessage({ id: m.id, ok: true, request: needAssist().request(m.role, m.input) });
    else if (m.op === "assist.check") postMessage({ id: m.id, ok: true, verdict: needAssist().check(m.role, m.input, m.body, m.attempt) });
    else if (m.op === "assist.readers") postMessage({ id: m.id, ok: true, readers: mod.assist_readers(m.body) });
    else if (m.op === "assist.digest") postMessage({ id: m.id, ok: true, digest: needAssist().digest(m.role) });
    else if (m.op === "assist.state") { if (!agent) throw { key: "err.llm.module", params: { reason: "agent" } }; agent.set_assist_state(m.state); postMessage({ id: m.id, ok: true }); }
    else if (m.op === "assist.reading") { if (!agent) throw { key: "err.llm.module", params: { reason: "agent" } }; agent.reading(m.role, m.summary); postMessage({ id: m.id, ok: true }); }
    else if (m.op === "sub.replay") { if (!agent) throw { key: "err.llm.module", params: { reason: "agent" } }; postMessage({ id: m.id, ok: true, replaying: agent.replaying(), lines: agent.replaying() ? agent.replay_sub(m.request) : "" }); }
    else if (m.op === "abort") { live.delete(m.id); if (port) port.postMessage({ t: "abort", id: m.id }); postMessage({ id: m.id, done: true }); }
  } catch (e) {
    const x = wrap(e);
    postMessage({ id: m.id, ok: false, error_key: x.key, params: x.params });
  }
};
