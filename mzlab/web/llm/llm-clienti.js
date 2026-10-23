// @ts-check
// The clients of the assistant: the worker + iframe `trasporto.html` in the site, the native routes `llm/*` in the desktop app (the key stays in Rust), and the helpers that
// say which providers a page can reach. Loaded by llm-impostazioni.js only after the user turned the assistant on.

/** @typedef {{id: string, base_url: string, local: boolean, site_allowed: boolean, models: {id: string, price?: any}[]}} Provider */
/** @typedef {{ev: string, text?: string, key?: string, params?: Record<string, string>}} LlmEvent */

const RPC = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/rpc" : "mzlab://localhost/rpc";

/** The client of the worker + iframe (site). */
export class SiteClient {
  constructor() {
    this.kind = "site";
    this.worker = new Worker(new URL("./llm-worker.js", import.meta.url), { type: "module" });
    /** @type {Map<string, any>} */ this.calls = new Map();
    /** @type {Map<string, (e: LlmEvent) => void>} */ this.sinks = new Map();
    this.n = 0;
    /** @type {HTMLIFrameElement | null} */ this.frame = null;
    this.hasKey = false;
    /** @type {((has: boolean) => void) | null} */ this.onKey = null;
    this.worker.onmessage = ev => {
      const m = ev.data || {};
      if (m.event && this.sinks.has(m.id)) { /** @type {(e: LlmEvent) => void} */ (this.sinks.get(m.id))(JSON.parse(m.event)); return; }
      const c = this.calls.get(m.id);
      if (!c) return;
      if (m.done || m.ok) { this.calls.delete(m.id); this.sinks.delete(m.id); c.ok(m); } else if (m.ok === false) { this.calls.delete(m.id); this.sinks.delete(m.id); c.no({ key: m.error_key, params: m.params }); }
    };
    window.addEventListener("message", ev => {
      if (ev.origin !== location.origin || !this.frame || ev.source !== this.frame.contentWindow) return;
      if (ev.data && (ev.data.op === "ready" || ev.data.op === "key")) { this.hasKey = !!ev.data.hasKey; if (this.onKey) this.onKey(this.hasKey); }
    });
  }
  /** @param {string} op @param {any} [extra] @returns {Promise<any>} */
  call(op, extra) { return new Promise((ok, no) => { const id = "c" + (++this.n); this.calls.set(id, { ok, no }); this.worker.postMessage({ op, id, ...extra }); }); }
  /** @returns {Promise<Provider[]>} */
  async providers() { return JSON.parse((await this.call("init")).providers).providers; }
  /** @param {object} conv */
  async request(conv) { return JSON.parse((await this.call("request", { conv: JSON.stringify(conv) })).request); }
  /** @param {object} conv @param {(e: LlmEvent) => void} sink @param {boolean} replay */
  send(conv, sink, replay) {
    return new Promise((ok, no) => { const id = "c" + (++this.n); this.calls.set(id, { ok, no }); this.sinks.set(id, sink); this.lastId = id; this.worker.postMessage({ op: "send", id, conv: JSON.stringify(conv), transport: replay ? "replay" : "net" }); });
  }
  /** a request already built by Rust (a call of a subagent): the events come to `sink` like those of `send` @param {string} provider @param {string} model @param {string} request JSON of the request @param {(e: LlmEvent) => void} sink */
  sendRaw(provider, model, request, sink) {
    return new Promise((ok, no) => { const id = "c" + (++this.n); this.calls.set(id, { ok, no }); this.sinks.set(id, sink); this.lastId = id; this.worker.postMessage({ op: "send", id, provider, model, request, transport: "net" }); });
  }
  stop() { if (this.lastId) this.worker.postMessage({ op: "abort", id: this.lastId }); }
  /** @param {string} text @param {boolean} strict */
  async replay(text, strict) { return (await this.call("replay.load", { text, strict })).turns; }
  /** the iframe with the key field; created only here, after the consent @param {HTMLElement} box @param {Provider} p @param {Record<string, string>} labels @param {string} title */
  attach(box, p, labels, title) {
    if (this.frame) this.frame.remove();
    const f = document.createElement("iframe");
    f.src = new URL("./trasporto.html", import.meta.url).href; f.title = title; f.className = "llm-frame";
    f.setAttribute("style", "width:100%;height:150px;border:1px solid var(--line);border-radius:6px;background:var(--panel)");
    f.addEventListener("load", () => {
      const ch = new MessageChannel();
      this.worker.postMessage({ op: "port" }, [ch.port2]);
      /** @type {Window} */ (f.contentWindow).postMessage({ op: "init", provider: p.id, local: p.local, labels }, location.origin, [ch.port1]);
    });
    box.appendChild(f); this.frame = f;
  }
  close() { this.worker.terminate(); if (this.frame) this.frame.remove(); }
}

/** The client of the desktop app (routes `llm/*` of mzlab://; the key stays in Rust). */
export class DesktopClient {
  constructor() { this.kind = "desktop"; this.hasKey = false; /** @type {string} */ this.cur = ""; this.stopped = false; }
  /** @param {string} op @param {any} [p] @returns {Promise<any>} */
  async post(op, p) {
    const r = await fetch(RPC, { method: "POST", body: JSON.stringify({ op, ...p }) }), text = await r.text();
    if (r.status !== 200) { let e = /** @type {any} */ ({}); try { e = JSON.parse(text); } catch (_) { /* not JSON */ } throw { key: e.error_key || "err.llm.network", params: e.params || {} }; }
    return text ? JSON.parse(text) : {};
  }
  async providers() { const r = await this.post("llm/providers"); this.keys = r.has_key || {}; return JSON.parse(r.providers).providers; }
  /** @param {object} conv */
  async request(conv) { return this.post("llm/request", { conv: JSON.stringify(conv) }); }
  /** @param {object} conv @param {(e: LlmEvent) => void} sink @param {boolean} replay */
  async send(conv, sink, replay) {
    if (replay) throw { key: "err.llm.replay.format", params: {} };
    await this.poll(await this.post("llm/send", { conv: JSON.stringify(conv) }), sink);
  }
  /** a request already built by Rust (a call of a subagent), sent by the app with the key it holds @param {string} provider @param {string} _model @param {string} request JSON @param {(e: LlmEvent) => void} sink */
  async sendRaw(provider, _model, request, sink) { await this.poll(await this.post("llm/send", { provider, request }), sink); }
  /** @param {{id: string}} job @param {(e: LlmEvent) => void} sink */
  async poll({ id }, sink) {
    this.stopped = false;
    this.cur = id;
    for (;;) {
      const r = await this.post("llm/poll", { id });
      for (const l of r.events || []) sink(JSON.parse(l));
      if (r.done || this.stopped) break;
      await new Promise(ok => setTimeout(ok, 120));
    }
  }
  stop() { this.stopped = true; if (this.cur) this.post("llm/abort", { id: this.cur }).catch(() => {}); }
  /** @param {string} provider @param {string} key */
  async setKey(provider, key) { await this.post("llm/key/set", { provider, key }); this.hasKey = true; }
  /** @param {string} provider */
  async forgetKey(provider) { await this.post("llm/key/forget", { provider }); this.hasKey = false; }
  close() { /* nothing to release */ }
}

export const desktop = () => !!(/** @type {any} */ (window)).MZLAB_DESKTOP;
/** the hosts the transport document may reach, read from its own CSP: the providers outside the list are not offered in the site @returns {Promise<string[]>} */
export async function siteHosts() {
  try {
    const t = await (await fetch(new URL("./trasporto.html", import.meta.url))).text();
    const m = /connect-src([^;"]*)/.exec(t);
    return m ? m[1].trim().split(/\s+/) : [];
  } catch (_) { return []; }
}
/** @param {Provider} p @param {string[]} hosts */
export function reachable(p, hosts) {
  if (desktop()) return true;
  if (p.local) return hosts.some(h => /^http:\/\/(localhost|127\.0\.0\.1):\*$/.test(h));
  try { return p.site_allowed && hosts.includes(new URL(p.base_url).origin); } catch (_) { return false; }
}

/** one error shape for every client: {key, params} @param {any} e */
export const asError = e => (e && e.key ? e : { key: e && e.error_key || "err.llm.module", params: e && e.params || {} });
