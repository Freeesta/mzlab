// @ts-strict
// The loop of the assistant on the page: one conversation = one worker (llm-worker.js) that holds the state machine of Rust (`Agent`), and this class, which carries out what the machine asks:
// show the text, call a tool of the Flow, ask the user, send a request. Every decision (levels, confirmations, limits) is Rust's; the page only executes and reports back.
// Used by flow/lazy/flow-llm.js, only when the assistant is on. A recording (tests) answers the requests inside the worker and no transport is used.
// The subagents (docs/agenti/llm.md): when the machine asks for the router (`sub`), the page sends its request, asks Rust which readers the answer names (`assist_readers`), runs each
// one (request, send, verdict of `Assist.check`, one retry with the problems, then a discard) and hands the accepted summaries to the machine as data. Every decision is Rust's;
// what a file says never becomes an instruction: a reading is checked against the data it saw and enters the conversation only as data.
import { DesktopClient, SiteClient, asError, desktop } from "./llm-clienti.js";
import { readSettings } from "./llm-impostazioni.js";
import { askConsent, costLine, hasConsent } from "./llm-consenso.js";
import { DEMO_CHIPS, DEMO_PROVIDER, DEMO_WORDS, demoActive, spend, startDemo } from "./llm-demo.js";
import { PROXY_HOST } from "./proxy-host.js";
import { conv, ownerLabels, pretty } from "./llm-sezione.js";


/** @typedef {{show: (text: string) => void, log: (line: any) => void, sending: (request: any) => void, call: (id: string, name: string, args: any) => Promise<{ok: boolean, content: any}>,
 *   ask: (id: string, summary: string, name: string, args: any) => Promise<boolean>, done: (outcome: string) => void,
 *   readerInput?: (role: string) => Promise<object | null> | object | null, reader?: (info: {role: string, status: "accepted" | "dropped" | "failed" | "skipped", problems?: any[], summary?: any}) => void, readerStage?: (role: string, stage: "queued" | "running" | "cancelled") => void}} Host
 * `readerInput(role)` gives the sources a reader may see, keyed by source (`{"ispezione/1": ...}`), or null when the page has none (the reader is skipped); `reader` hears how each one ended and `readerStage` where it is (queued, running, cancelled by a Stop). */

/** a recording for tests: `MZLAB_LLM_AGENT.replay(text, strict)` answers the requests of the next conversation */
/** @type {{text: string, strict: boolean} | null} */ let recording = null;
/** @type {any} */ (window).MZLAB_LLM_AGENT = { replay: (/** @type {string} */ text, strict = false) => { recording = { text, strict }; } };

/** without a limit set, the conversation asks before it goes past this many tokens */
const ASK_TOKENS = 50000;
/** the errors after which nothing more is sent in the turn (the proxy of the demo refused, or the spend limit was reached) */
const FINAL = /^err\.llm\.(proxy\.|limit\.spend$)/;

export class AgentRun {
  /** @param {Host} host @param {{hideNames?: boolean}} [opts] */
  constructor(host, opts = {}) {
    this.host = host;
    this.settings = readSettings();
    this.hideNames = !!opts.hideNames || this.settings.hideNames;
    this.worker = new Worker(new URL("./llm-worker.js", import.meta.url), { type: "module" });
    /** @type {Map<string, {ok: (m: any) => void, no: (e: any) => void}>} */ this.calls = new Map();
    this.n = 0;
    this.turns = 0;
    this.usage = [0, 0];
    /** tokens [in, out] of the subagents' calls, which the machine does not see */ this.subUsage = [0, 0];
    this.stopped = false;
    this.outcome = "";
    this.running = false;
    this.replaying = false;
    /** whether the user was already asked to go past ASK_TOKENS */ this.capAsked = false;
    this.assist = false;
    /** the spend of the demo session {spent, cap, soft} (micro-dollars), set by the panel @type {((s: {spent: number, cap: number, soft: boolean}) => void) | null} */ this.onSpend = null;
    /** @type {any} */ this.client = null;
    /** @type {Promise<string>} */ this.done = Promise.resolve("");
    /** @type {((o: string) => void) | null} */ this.finish = null;
    this.worker.onmessage = ev => {
      const m = ev.data || {}, c = this.calls.get(m.id);
      if (!c) return;
      this.calls.delete(m.id);
      if (m.ok) c.ok(m); else c.no({ key: m.error_key, params: m.params });
    };
  }
  /** @param {any} msg @returns {Promise<any>} */
  rpc(msg) { return new Promise((ok, no) => { const id = "a" + (++this.n); this.calls.set(id, { ok, no }); this.worker.postMessage({ ...msg, id }); }); }

  /** starts the conversation with the level and model of the settings; `research` = the profile is «ricerca» (the machine refuses the course profile) */
  async open(research = true) {
    const rec = recording;
    this.replaying = !!rec;
    await this.rpc({ op: "agent.new", provider: this.settings.provider, model: this.settings.model, level: this.settings.level, research, replay: rec ? rec.text : "", strict: rec ? rec.strict : false });
  }

  /** turns the subagents on: `steps` = the `step/1` descriptors of the registry of the page (the limits that `set_param` is checked against) */
  async enableAssist(steps) { await this.rpc({ op: "assist.on", provider: this.settings.provider, steps: JSON.stringify(steps) }); this.assist = true; }
  /** what the router sees of the session before a user message: {phase, files_open, plan, pending_questions} @param {object} state */
  async assistState(state) { await this.rpc({ op: "assist.state", state: JSON.stringify(state) }); }
  /** tokens [in, out] of the machine and of the subagents together */
  totalUsage() { return [this.usage[0] + this.subUsage[0], this.usage[1] + this.subUsage[1]]; }

  /** @param {string} text the user's message @param {string} [context] a contesto/1 text that goes with it @returns {Promise<string>} how the turn ended */
  async user(text, context = "") {
    this.running = true; this.turns++; this.stopped = false;
    this.done = new Promise(ok => { this.finish = ok; });
    try { await this.pump(await this.rpc({ op: "agent.in", kind: "user", text, context })); } catch (e) { this.host.done("error:" + asError(e).key); this.end("error:" + asError(e).key); }
    return this.done;
  }
  /** @param {string} o */
  end(o) { this.running = false; this.outcome = o; if (this.finish) this.finish(o); }

  /** stops the turn: the model's stream is cut, a tool already running ends, the machine closes the turn */
  async stop() {
    this.stopped = true;
    if (this.client && this.client.stop) this.client.stop();
    try { await this.pump(await this.rpc({ op: "agent.in", kind: "stop" })); } catch (_) { /* the turn ends by itself */ }
  }

  /** @param {{lines: string, outcome: string, usage: number[]}} r */
  async pump(r) {
    this.usage = r.usage || this.usage;
    for (const l of r.lines.split("\n")) if (l) await this.handle(JSON.parse(l), r.outcome);
  }

  /** @param {any} o @param {string} outcome */
  async handle(o, outcome) {
    const h = this.host;
    if (o.o === "show") h.show(o.text);
    else if (o.o === "log") h.log(o.line);
    else if (o.o === "send") { h.sending(o.request); if (!this.replaying) await this.transport(); }
    else if (o.o === "sub") await this.runSub(o);
    else if (o.o === "call") {
      const r = await h.call(o.id, o.name, o.args);
      await this.pump(await this.rpc({ op: "agent.in", kind: "result", callId: o.id, ok: r.ok, content: JSON.stringify(r.content) }));
    } else if (o.o === "ask") {
      const yes = await h.ask(o.id, o.summary, o.name || "", o.args || null);
      await this.pump(await this.rpc({ op: "agent.in", kind: "confirm", callId: o.id, yes }));
    } else if (o.o === "done") { const out = o.outcome || outcome || "end"; h.done(out); this.end(out); }
  }

  /** one call of a subagent: the request (built by Rust) goes out and the text of the answer comes back; a failure is {ok: false, key, params} @param {any} request @returns {Promise<{ok: boolean, body?: string, key?: string, params?: any}>} */
  async subCall(request) {
    const json = typeof request === "string" ? request : JSON.stringify(request);
    /** @type {any[]} */ const events = [];
    /** @type {any} */ let rep;
    try { rep = await this.rpc({ op: "sub.replay", request: json }); } catch (e) { const x = asError(e); return { ok: false, key: x.key, params: x.params }; }
    if (rep.replaying) for (const l of rep.lines.split("\n")) { if (l) events.push(JSON.parse(l)); }
    else {
      const client = await this.getClient(), p = this.settings.provider, sink = (/** @type {any} */ e) => { events.push(e); };
      try { await this.guard(client); await client.sendRaw(p, "", json, sink); } catch (e) { const x = asError(e); events.push({ ev: "error", key: x.key, params: x.params }); }
    }
    let body = "";
    for (const e of events) {
      if (e.ev === "text_delta") body += e.text || "";
      else if (e.ev === "usage") { this.subUsage[0] += (e.in || 0) + (e.cache_read || 0); this.subUsage[1] += e.out || 0; }
      else if (e.ev === "error") return { ok: false, key: e.key, params: e.params };
    }
    return { ok: true, body };
  }

  /** the router, then the readers it names, then the router's answer to the machine (the readings must be in before it builds the request of the orchestrator) @param {{role: string, request: any}} o */
  async runSub(o) {
    const r = await this.subCall(o.request);
    // a refusal of the demo proxy or of the spend limit holds for the orchestrator too: the turn ends here, nothing more is sent
    if (!r.ok && FINAL.test(r.key || "")) {
      this.stopped = true;
      try { await this.rpc({ op: "agent.in", kind: "stop" }); } catch (_) { /* the turn ends below */ }
      this.host.done("error:" + r.key); this.end("error:" + r.key);
      return;
    }
    if (r.ok && this.assist && !this.stopped) {
      const readers = JSON.parse((await this.rpc({ op: "assist.readers", body: r.body })).readers);
      const stage = (/** @type {string} */ role, /** @type {"queued" | "running" | "cancelled"} */ st) => { if (this.host.readerStage) this.host.readerStage(role, st); };
      for (const role of readers) stage(role, "queued");
      for (const role of readers) { if (this.stopped) stage(role, "cancelled"); else await this.runReader(role); }
    }
    await this.pump(await this.rpc({ op: "agent.in", kind: "sub", role: o.role, ok: r.ok, body: r.ok ? r.body : r.key }));
  }

  /** a reader: request, send, verdict; the problems go back once, then the reading is discarded; an accepted summary is given to the machine @param {string} role */
  async runReader(role) {
    const h = this.host, say = (/** @type {any} */ info) => { if (h.reader) h.reader({ role, ...info }); };
    const src = h.readerInput ? await h.readerInput(role) : null;
    if (!src) return say({ status: "skipped" });
    const input = JSON.stringify(src);
    if (h.readerStage) h.readerStage(role, "running");
    try {
      let req = (await this.rpc({ op: "assist.request", role, input })).request;
      for (let attempt = 0; attempt < 2; attempt++) {
        const r = await this.subCall(req);
        if (!r.ok) return say({ status: "failed", problems: [{ key: r.key, params: r.params }] });
        const v = JSON.parse((await this.rpc({ op: "assist.check", role, input, body: r.body, attempt })).verdict);
        if (v.verdict === "accepted") { await this.rpc({ op: "assist.reading", role, summary: JSON.stringify(v.summary) }); return say({ status: "accepted", summary: v.summary }); }
        if (v.verdict === "dropped") return say({ status: "dropped", problems: v.problems });
        req = v.request;
      }
    } catch (e) { const x = asError(e); say({ status: "failed", problems: [{ key: x.key, params: x.params }] }); }
  }

  async getClient() {
    const client = this.client || (this.client = desktop() ? new DesktopClient() : (/** @type {any} */ (window).MZLAB_LLM && /** @type {any} */ (window).MZLAB_LLM.client));
    if (!client) throw { key: "err.llm.network", params: { reason: "settings" } };
    // the spend the proxy of the demo reports reaches the panel
    client.onSpend = (/** @type {any} */ s) => { if (this.onSpend) this.onSpend(s); };
    return client;
  }

  /** the checks before any byte leaves, at every send: the assistant still switched on, the consent for the provider still stored (it may have been revoked since), a key for a provider that needs one
   * @param {{needsKey: (id: string) => Promise<boolean>}} client */
  async guard(client) {
    const s = readSettings();
    if (!s.on || !s.provider || !hasConsent(s.provider)) throw { key: "err.llm.consent", params: {} };
    if (await client.needsKey(s.provider)) throw { key: "err.llm.key.missing", params: {} };
    await this.capCheck(s.cap);
    if (s.spendCap > 0) await this.spendCheck(client, s);
  }

  /** the spending cap of the demo (USD, not editable): the estimated cost of the tokens so far, with the price of the provider list, stops the assistant at the cap
   * @param {any} client @param {{provider: string, model: string, spendCap: number}} s */
  async spendCheck(client, s) {
    const p = (await client.providers()).find((/** @type {any} */ x) => x.id === s.provider), m = p && p.models.find((/** @type {any} */ x) => x.id === s.model);
    const usd = spend(this.totalUsage(), m && m.price_per_mtok);
    if (usd === null || usd >= s.spendCap) throw { key: "err.llm.limit.spend", params: { limit: "session" } };
  }

  /** the token limit of the conversation (machine + subagents, counted before each send): a limit stops the assistant; without one, the first crossing of ASK_TOKENS asks the user once @param {number} cap */
  async capCheck(cap) {
    const used = this.totalUsage(), tokens = used[0] + used[1];
    if (cap > 0 ? tokens < cap : tokens < ASK_TOKENS || this.capAsked) return;
    if (cap === 0) {
      this.capAsked = true;
      if (await this.host.ask("cap", `${tokens} tokens used in this conversation so far. Go on?`, "continue", null)) return;
    }
    throw { key: "err.llm.limit.spend", params: { limit: "tokens" } };
  }

  /** the request of the last `send` goes out through the client of the page and its events come back one by one, in order */
  async transport() {
    const conv = JSON.parse((await this.rpc({ op: "agent.in", kind: "conv" })).conv);
    const client = await this.getClient();
    // one chain per send: the events of this send are handled in order, and a send started from inside one of them (after a tool result) waits only for its own events
    let chain = Promise.resolve();
    const sink = (/** @type {any} */ e) => { chain = chain.then(async () => { await this.pump(await this.rpc({ op: "agent.in", kind: "event", line: JSON.stringify(e) })); }); };
    try { await this.guard(client); await client.send(conv, sink, false); } catch (e) { const x = asError(e); sink({ ev: "error", key: x.key, params: x.params }); }
    await chain;
  }

  /** the log of the conversation (`llmlog/1` text) */
  async log() { return (await this.rpc({ op: "agent.in", kind: "log" })).log; }
  /** @param {string} sources JSON of the sources of the page @param {number} [budget] @returns {Promise<string>} a contesto/1 text */
  async context(sources, budget = 4000) { return (await this.rpc({ op: "context", sources, budget })).context; }
  /** the `ispezione/1` of the sources the page collected (Rust, no model), as Rust wrote it @param {string} sources JSON @param {number} [budget] tokens @returns {Promise<string>} */
  async inspectText(sources, budget = 30000) { return (await this.rpc({ op: "inspect.build", sources, budget })).inspection; }
  /** the `proposta-param/1` of the rules for an inspection and the user's answers (Rust, no model), as Rust wrote it @param {string} inspection JSON @param {string} answers JSON or "" @param {string} steps JSON of the `step/1` descriptors of the page @returns {Promise<string>} */
  async paramsProposeText(inspection, answers, steps) { return (await this.rpc({ op: "params.propose", inspection, answers, steps })).proposal; }
  /** @param {string} sources JSON of the sources of the charts @returns {Promise<any[]>} the views that exist now: {ref, description, fields, rows} */
  async chartViews(sources) { return JSON.parse((await this.rpc({ op: "chart.views", sources })).views); }
  /** a `grafico/1` of the model, checked and filled with its rows by Rust; a rejection is an error {key, params} @param {string} spec @param {string} sources @returns {Promise<any>} */
  async chartMake(spec, sources) { return JSON.parse((await this.rpc({ op: "chart.make", spec, sources })).chart); }
  /** the charts that need no model @param {string} sources @returns {Promise<any[]>} */
  async chartPresets(sources) { return JSON.parse((await this.rpc({ op: "chart.presets", sources })).charts); }
  close() { this.worker.terminate(); if (this.client && this.client.close) this.client.close(); }
}

/** whether the assistant is on for this page: profile «ricerca», switched on in the settings and a consent for the provider (no conversation is opened before this is true; the facts of `factsRun` need none) */
export function assistantOn() {
  const s = readSettings();
  return s.on && !!s.provider && hasConsent(s.provider);
}

/** the public demo (profile «demo»): the provider list of Rust, the consent for `mzlab-demo` (asked once), the client of the site with its transport (no key field) and the fixed
 * settings of llm-demo.js; false when the visitor says no. Nothing is sent here. @param {HTMLElement | null} opener */
export async function demoOpen(opener) {
  const T = (/** @type {string} */ k, /** @type {any} */ p = {}) => I18N.t(k, p), client = new SiteClient();
  try {
    const p = (await client.providers()).find(x => x.id === DEMO_PROVIDER);
    if (!p) throw { key: "err.llm.provider", params: {} };
    const m = p.models[0], host = new URL(PROXY_HOST || p.base_url).host;
    // the exact request of the first chip, built by Rust as it would leave (the summary of the files goes with the message when it is sent)
    const preview = async () => { try { return pretty(T, await client.request(conv({ provider: p.id, model: m.id }, DEMO_CHIPS[0][1]))); } catch (e) { return T(asError(e).key); } };
    if (!hasConsent(p.id) && !(await askConsent({ provider: p.id, host, model: m.id, local: false, cost: costLine({ price: m.price_per_mtok }, T), preview: await preview(), T, opener }))) { client.close(); return false; }
    // the transport document, with the field of the owner passphrase: hidden until the proxy answers «closed» (the panel shows it) or the administrator mode is on
    const box = document.createElement("div"); box.id = "llm-owner-box"; box.hidden = !(typeof (/** @type {any} */ (globalThis)).uipAdminOn === "function" && /** @type {any} */ (globalThis).uipAdminOn()); document.body.append(box);
    client.attach(box, p, ownerLabels(T), T("llm.owner.frame"));
    client.session = /** @type {any} */ (startDemo(m.id)).session;
    /** @type {any} */ (window).MZLAB_LLM = { client };
    return true;
  } catch (e) { client.close(); throw asError(e); }
}
export { DEMO_CHIPS, DEMO_WORDS, demoActive };

/** a worker for the facts that need no model (inspection, proposals of the parameters, charts without the model): no conversation, no consent, nothing leaves the site */
export function factsRun() {
  return new AgentRun({ show: () => {}, log: () => {}, sending: () => {}, call: async () => ({ ok: false, content: null }), ask: async () => false, done: () => {} });
}
