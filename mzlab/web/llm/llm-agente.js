// @ts-check
// The loop of the assistant on the page: one conversation = one worker (llm-worker.js) that holds the state machine of Rust (`Agent`), and this class, which carries out what the machine asks:
// show the text, call a tool of the Flow, ask the user, send a request. Every decision (levels, confirmations, limits) is Rust's; the page only executes and reports back.
// Used by flow/lazy/flow-llm.js, only when the assistant is on. A recording (tests) answers the requests inside the worker and no transport is used.
import { DesktopClient, asError, desktop } from "./llm-clienti.js";
import { readSettings } from "./llm-impostazioni.js";
import { hasConsent } from "./llm-consenso.js";

/** @typedef {{show: (text: string) => void, log: (line: any) => void, sending: (request: any) => void, call: (id: string, name: string, args: any) => Promise<{ok: boolean, content: any}>,
 *   ask: (id: string, summary: string, name: string, args: any) => Promise<boolean>, done: (outcome: string) => void}} Host */

/** whether the assistant is on for this page: profile «ricerca», switched on in the settings and a consent for the provider (no file of llm/ is read before this is true) */
export function assistantOn() {
  const s = readSettings();
  return s.on && !!s.provider && hasConsent(s.provider);
}

/** a recording for tests: `MZLAB_LLM_AGENT.replay(text, strict)` answers the requests of the next conversation */
/** @type {{text: string, strict: boolean} | null} */ let recording = null;
/** @type {any} */ (window).MZLAB_LLM_AGENT = { replay: (/** @type {string} */ text, strict = false) => { recording = { text, strict }; } };

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
    this.outcome = "";
    this.running = false;
    this.replaying = false;
    /** @type {any} */ this.client = null;
    /** @type {Promise<void>} */ this.chain = Promise.resolve();
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

  /** @param {string} text the user's message @param {string} [context] a contesto/1 text that goes with it @returns {Promise<string>} how the turn ended */
  async user(text, context = "") {
    this.running = true; this.turns++;
    this.done = new Promise(ok => { this.finish = ok; });
    try { await this.pump(await this.rpc({ op: "agent.in", kind: "user", text, context })); } catch (e) { this.host.done("error:" + asError(e).key); this.end("error:" + asError(e).key); }
    return this.done;
  }
  /** @param {string} o */
  end(o) { this.running = false; this.outcome = o; if (this.finish) this.finish(o); }

  /** stops the turn: the model's stream is cut, a tool already running ends, the machine closes the turn */
  async stop() {
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
    else if (o.o === "call") {
      const r = await h.call(o.id, o.name, o.args);
      await this.pump(await this.rpc({ op: "agent.in", kind: "result", callId: o.id, ok: r.ok, content: JSON.stringify(r.content) }));
    } else if (o.o === "ask") {
      const yes = await h.ask(o.id, o.summary, o.name || "", o.args || null);
      await this.pump(await this.rpc({ op: "agent.in", kind: "confirm", callId: o.id, yes }));
    } else if (o.o === "done") { const out = o.outcome || outcome || "end"; h.done(out); this.end(out); }
  }

  /** the request of the last `send` goes out through the client of the page and its events come back one by one, in order */
  async transport() {
    const conv = JSON.parse((await this.rpc({ op: "agent.in", kind: "conv" })).conv);
    const client = this.client || (this.client = desktop() ? new DesktopClient() : (/** @type {any} */ (window).MZLAB_LLM && /** @type {any} */ (window).MZLAB_LLM.client));
    if (!client) throw { key: "err.llm.network", params: { reason: "settings" } };
    const sink = (/** @type {any} */ e) => { this.chain = this.chain.then(async () => { await this.pump(await this.rpc({ op: "agent.in", kind: "event", line: JSON.stringify(e) })); }); };
    try { await client.send(conv, sink, false); } catch (e) { const x = asError(e); sink({ ev: "error", key: x.key, params: x.params }); }
    await this.chain;
  }

  /** the log of the conversation (`llmlog/1` text) */
  async log() { return (await this.rpc({ op: "agent.in", kind: "log" })).log; }
  /** @param {string} sources JSON of the sources of the page @param {number} [budget] @returns {Promise<string>} a contesto/1 text */
  async context(sources, budget = 4000) { return (await this.rpc({ op: "context", sources, budget })).context; }
  close() { this.worker.terminate(); if (this.client && this.client.close) this.client.close(); }
}
