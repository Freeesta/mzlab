// @ts-check
// «Assistant» section of the settings (profile «ricerca» only; settings.js imports this file when the section is shown, so a profile «corso» downloads nothing of llm/).
// Off by default: turning it on loads the provider list, asks for the consent (llm-consenso.js) and only then can anything leave the computer. The client talks to the
// worker + iframe `trasporto.html` in the site, to the native routes `llm/*` in the desktop app, or to a recording (`replay`, tests); the preview shows the exact request.
import { askConsent, costLine, hasConsent, readConsent, revoke } from "./llm-consenso.js";

const KEY = "mzlab.llm";
/** the levels of autonomy of the assistant (D5 uses them): L0 advises, L1 asks before each action, L2 acts alone and can undo */
const LEVELS = ["L0", "L1", "L2"];
/** catalog keys of each level: its name and what it does */
const LV = { L0: ["llm.level.L0", "llm.level.L0.title"], L1: ["llm.level.L1", "llm.level.L1.title"], L2: ["llm.level.L2", "llm.level.L2.title"] };
/** @typedef {{on: boolean, provider: string, model: string, level: string, l2max: number, cap: number, hideNames: boolean}} Settings */
/** @typedef {{id: string, base_url: string, local: boolean, site_allowed: boolean, models: {id: string, price?: any}[]}} Provider */
/** @typedef {{ev: string, text?: string, key?: string, params?: Record<string, string>}} LlmEvent */

/** @returns {Settings} */
export function readSettings() {
  /** @type {any} */ let o = {};
  try { o = JSON.parse(localStorage.getItem(KEY) || "{}") || {}; } catch (_) { /* blocked or damaged: the defaults */ }
  return { on: o.on === true, provider: String(o.provider || ""), model: String(o.model || ""), level: LEVELS.includes(o.level) ? o.level : "L0",
    l2max: Math.min(50, Math.max(1, Math.round(+o.l2max || 5))), cap: Math.max(0, +o.cap || 0), hideNames: o.hideNames === true };
}
/** @param {Settings} s */
export function writeSettings(s) { try { localStorage.setItem(KEY, JSON.stringify(s)); } catch (_) { /* the setting lasts until the page is closed */ } }

const desktop = () => !!(/** @type {any} */ (window)).MZLAB_DESKTOP;
const RPC = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/rpc" : "mzlab://localhost/rpc";

/** one error shape for every client: {key, params} @param {any} e */
const asError = e => (e && e.key ? e : { key: e && e.error_key || "err.llm.module", params: e && e.params || {} });

/** The client of the worker + iframe (site). */
class SiteClient {
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
class DesktopClient {
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
    this.stopped = false;
    const { id } = await this.post("llm/send", { conv: JSON.stringify(conv) });
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

/** the hosts the transport document may reach, read from its own CSP: the providers outside the list are not offered in the site @returns {Promise<string[]>} */
async function siteHosts() {
  try {
    const t = await (await fetch(new URL("./trasporto.html", import.meta.url))).text();
    const m = /connect-src([^;"]*)/.exec(t);
    return m ? m[1].trim().split(/\s+/) : [];
  } catch (_) { return []; }
}
/** @param {Provider} p @param {string[]} hosts */
function reachable(p, hosts) {
  if (desktop()) return true;
  if (p.local) return hosts.some(h => /^http:\/\/(localhost|127\.0\.0\.1):\*$/.test(h));
  try { return p.site_allowed && hosts.includes(new URL(p.base_url).origin); } catch (_) { return false; }
}

/** Mounts the section in `box`; `startOn` = the user just flipped the switch (the consent is asked at once). @param {HTMLElement} box @param {{C: any, T: (k: string, p?: any) => string}} ui @param {boolean} [startOn] */
export async function mount(box, ui, startOn = false) {
  const { C, T } = ui, esc = /** @type {(s: string) => string} */ (C.esc);
  let s = readSettings();
  /** @type {SiteClient | DesktopClient | null} */ let client = null;
  /** @type {Provider[]} */ let provs = [], hosts = [];
  let replayOn = false;
  const api = { replay: async (/** @type {string} */ text, strict = false) => { if (!client || client.kind !== "site") throw new Error("replay needs the site client"); await /** @type {SiteClient} */ (client).replay(text, strict); replayOn = true; } };
  /** @type {any} */ (window).MZLAB_LLM = api;

  const status = (/** @type {string} */ k, /** @type {any} */ p = {}) => { const el = box.querySelector("#llm-st"); if (el) el.textContent = k ? T(k, p) : ""; };
  const errText = (/** @type {{key: string, params?: any}} */ e) => { const k = String(e.key || "err.llm.module"); return k.startsWith("err.llm.") ? I18N.t(`err.llm.${k.slice(8)}`, e.params || {}) : T("err.llm.module"); };
  const provider = () => provs.find(p => p.id === s.provider) || provs[0];
  const model = () => { const p = provider(); return p && (p.models.find(m => m.id === s.model) || p.models[0]); };
  const host = (/** @type {Provider} */ p) => { try { return new URL(p.base_url).host; } catch (_) { return p.base_url; } };

  const off = () => {
    if (client) client.close();
    client = null; replayOn = false;
    const html = C.sw(T("llm.on.label"), T("llm.on.title"), 'id="llm-on"', false, "") + `<div class="hint" id="llm-st" role="status" aria-live="polite">${T("llm.off.hint")}</div>`;
    box.innerHTML = html;
    /** @type {HTMLInputElement} */ (box.querySelector("#llm-on")).onchange = e => { if (/** @type {HTMLInputElement} */ (e.target).checked) turnOn(); };
  };

  const load = async () => {
    status("llm.loading");
    client = desktop() ? new DesktopClient() : new SiteClient();
    if (!desktop()) hosts = await siteHosts();
    provs = (await client.providers()).filter(p => reachable(p, hosts));
    if (!provs.length) throw { key: "err.llm.provider", params: {} };
    if (!provs.some(p => p.id === s.provider)) { s.provider = provs[0].id; s.model = provs[0].models[0].id; }
    if (!provider().models.some(m => m.id === s.model)) s.model = provider().models[0].id;
  };

  const turnOn = async () => {
    const was = readSettings();
    try {
      await load();
      const p = provider(), ok = hasConsent(p.id) || await askConsent({ provider: p.id, host: host(p), model: s.model, local: p.local, cost: costLine(/** @type {any} */ (model()), T), T,
        opener: /** @type {HTMLElement | null} */ (box.querySelector("#llm-on")) });
      if (!ok) { s = was; off(); return; }
      s.on = true; writeSettings(s); render();
    } catch (e) { s = was; off(); status("", {}); const el = box.querySelector("#llm-st"); if (el) el.textContent = errText(asError(e)); }
  };

  const pretty = (/** @type {any} */ r) => `${r.url}\n${(r.headers || []).map((/** @type {string[]} */ h) => `${h[0]}: ${h[1]}`).join("\n")}\n${r.auth && r.auth.header ? `${r.auth.header}: (${T("llm.test.keyslot")})\n` : ""}\n${(() => { try { return JSON.stringify(JSON.parse(r.body), null, 2); } catch (_) { return r.body; } })()}`;
  const conv = (/** @type {string} */ text) => ({ schema: "llm-conv/1", provider: s.provider, model: s.model, system: "", max_output_tokens: 256, messages: [{ role: "user", parts: [{ text }] }] });

  const render = () => {
    const p = provider(), m = model();
    const sel = (/** @type {string} */ id, /** @type {string} */ label, /** @type {string[][]} */ opts, /** @type {string} */ cur) => `<div class="imp-fld"><label for="${id}">${label}</label><select id="${id}">${opts.map(o => `<option value="${esc(o[0])}"${o[0] === cur ? " selected" : ""}>${esc(o[1])}</option>`).join("")}</select></div>`;
    const num = (/** @type {string} */ id, /** @type {string} */ label, /** @type {number} */ v, /** @type {number} */ min, /** @type {number} */ max) => `<div class="imp-fld"><label for="${id}">${label}</label><input type="number" id="${id}" min="${min}" max="${max}" step="1" value="${v}" style="width:84px"></div>`;
    const html = C.sw(T("llm.on.label"), T("llm.on.title"), 'id="llm-on"', true, "")
      + sel("llm-prov", T("llm.provider"), provs.map(x => [x.id, x.id + (x.local ? " · " + T("llm.local") : "")]), p.id)
      + sel("llm-model", T("llm.model"), p.models.map(x => [x.id, x.id]), m ? m.id : "")
      + `<div class="imp-fld"><span>${T("llm.level")}</span>${C.opts("seg3", C.radios("llm-level", LEVELS.map(l => [l, T(LV[/** @type {"L0"} */ (l)][0]), T(LV[/** @type {"L0"} */ (l)][1])]), s.level, "seg"))}</div>`
      + num("llm-l2max", T("llm.l2max"), s.l2max, 1, 50) + num("llm-cap", T("llm.cap"), s.cap, 0, 100000)
      + C.sw(T("llm.hideNames"), T("llm.hideNames.title"), 'id="llm-hide"', s.hideNames, "")
      + `<div class="imp-fld"><span>${T("llm.consent.state", { n: Object.keys(readConsent()).length })}</span><button type="button" id="llm-revoke">${T("llm.consenso.revoke")}</button></div>`
      + `<div id="llm-keybox"></div>`
      + `<div class="imp-fld"><label for="llm-msg">${T("llm.test.msg")}</label><input type="text" id="llm-msg" style="width:100%" placeholder="${esc(T("llm.test.ph"))}"></div>`
      + `<div class="row"><button type="button" id="llm-prev">${T("llm.test.preview")}</button><button type="button" id="llm-send">${T("llm.test.send")}</button><button type="button" id="llm-stop" hidden>${T("llm.test.stop")}</button></div>`
      + `<pre id="llm-prevout" tabindex="0" hidden aria-label="${esc(T("llm.test.preview"))}" style="white-space:pre-wrap;max-height:220px;overflow:auto"></pre><div id="llm-out" role="status" aria-live="polite" style="white-space:pre-wrap"></div>`
      + `<div class="hint" id="llm-st" role="status" aria-live="polite"></div>`;
    box.innerHTML = html;
    const $ = (/** @type {string} */ id) => /** @type {any} */ (box.querySelector(id));
    $("#llm-on").onchange = (/** @type {any} */ e) => { if (!e.target.checked) { s.on = false; writeSettings(s); off(); } };
    $("#llm-prov").onchange = async (/** @type {any} */ e) => {
      const was = s.provider, q = provs.find(x => x.id === e.target.value);
      if (!q) return;
      if (!hasConsent(q.id) && !(await askConsent({ provider: q.id, host: host(q), model: q.models[0].id, local: q.local, cost: costLine(/** @type {any} */ (q.models[0]), T), T, opener: $("#llm-prov") }))) { s.provider = was; render(); return; }
      s.provider = q.id; s.model = q.models[0].id; writeSettings(s); render();
    };
    $("#llm-model").onchange = (/** @type {any} */ e) => { s.model = e.target.value; writeSettings(s); };
    box.querySelectorAll('input[name="llm-level"]').forEach(r => r.addEventListener("change", e => { if (/** @type {HTMLInputElement} */ (e.target).checked) { s.level = /** @type {HTMLInputElement} */ (e.target).value; writeSettings(s); } }));
    $("#llm-l2max").onchange = (/** @type {any} */ e) => { s.l2max = Math.min(50, Math.max(1, Math.round(+e.target.value || 5))); e.target.value = s.l2max; writeSettings(s); };
    $("#llm-cap").onchange = (/** @type {any} */ e) => { s.cap = Math.max(0, +e.target.value || 0); e.target.value = s.cap; writeSettings(s); };
    $("#llm-hide").onchange = (/** @type {any} */ e) => { s.hideNames = e.target.checked; writeSettings(s); };
    $("#llm-revoke").onclick = () => { revoke(p.id); s.on = false; writeSettings(s); off(); };
    keyUi($("#llm-keybox"), p);
    const out = $("#llm-out"), pv = $("#llm-prevout");
    $("#llm-prev").onclick = async () => {
      try { pv.textContent = pretty(await /** @type {any} */ (client).request(conv($("#llm-msg").value || T("llm.test.ph")))); pv.hidden = false; pv.focus(); } catch (e) { out.textContent = errText(asError(e)); }
    };
    $("#llm-send").onclick = async () => {
      out.textContent = ""; $("#llm-send").disabled = true; $("#llm-stop").hidden = false;
      try {
        await /** @type {any} */ (client).send(conv($("#llm-msg").value || T("llm.test.ph")), (/** @type {LlmEvent} */ e) => {
          if (e.ev === "text_delta") out.textContent += e.text || "";
          else if (e.ev === "error") out.textContent += "\n" + errText({ key: e.key || "", params: e.params });
        }, replayOn);
      } catch (e) { out.textContent += "\n" + errText(asError(e)); }
      $("#llm-send").disabled = false; $("#llm-stop").hidden = true;
    };
    $("#llm-stop").onclick = () => /** @type {any} */ (client).stop();
    $("#llm-st").textContent = "";
  };

  /** the key area: the iframe in the site (cloud providers; the key never reaches this document), a password field in the desktop app, nothing for a model on this computer
   * @param {HTMLElement} kb @param {Provider} p */
  const keyUi = (kb, p) => {
    if (client instanceof SiteClient) {
      client.attach(kb, p, { key: T("llm.key.label"), use: T("llm.key.use"), forget: T("llm.key.forget"), remember: T("llm.key.remember"), hint: T("llm.key.hint"), local: T("llm.key.local"),
        none: T("llm.key.none"), set: T("llm.key.set") }, T("llm.key.frame"));
      return;
    }
    const hint = `<div class="hint">${T("llm.key.local")}</div>`;
    if (p.local) { kb.innerHTML = hint; return; }
    const html = `<div class="imp-fld"><label for="llm-key">${T("llm.key.label")}</label><input type="password" id="llm-key" autocomplete="off" spellcheck="false" style="width:16em"><button type="button" id="llm-key-use">${T("llm.key.use")}</button><button type="button" id="llm-key-forget">${T("llm.key.forget")}</button></div><div class="hint">${T("llm.key.hint.desktop")}</div>`;
    kb.innerHTML = html;
    const inp = /** @type {HTMLInputElement} */ (kb.querySelector("#llm-key")), dc = /** @type {DesktopClient} */ (client);
    /** @type {HTMLElement} */ (kb.querySelector("#llm-key-use")).onclick = async () => { const k = inp.value.trim(); inp.value = ""; if (k) { try { await dc.setKey(p.id, k); status("llm.key.set"); } catch (e) { status("", {}); } } };
    /** @type {HTMLElement} */ (kb.querySelector("#llm-key-forget")).onclick = async () => { try { await dc.forgetKey(p.id); status("llm.key.none"); } catch (e) { status("", {}); } };
  };

  if (startOn) { off(); await turnOn(); return; }
  if (s.on && Object.keys(readConsent()).length) {
    try { await load(); if (hasConsent(s.provider || provs[0].id)) { render(); return; } } catch (e) { /* falls back to off */ }
    s.on = false; writeSettings(s);
  }
  off();
}
