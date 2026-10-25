// @ts-strict
// «Assistant» section of the settings (profile «ricerca» only; settings.js imports this file when the section is shown, so a profile «corso» downloads nothing of llm/).
// Off by default: turning it on loads the provider list, asks for the consent (llm-consenso.js) and only then can anything leave the computer. The client talks to the
// worker + iframe `trasporto.html` in the site, to the native routes `llm/*` in the desktop app, or to a recording (`replay`, tests); the preview shows the exact request.
import { askConsent, costLine, hasConsent, readConsent, revoke } from "./llm-consenso.js";
import { DesktopClient, SiteClient, asError, desktop, reachable, siteHosts } from "./llm-clienti.js";
import { bindTest, keyUi as keyArea, sectionHtml, LEVELS } from "./llm-sezione.js";

/** @typedef {import("./llm-clienti.js").Provider} Provider */
/** @typedef {import("./llm-clienti.js").LlmEvent} LlmEvent */

const KEY = "mzlab.llm";

/** @typedef {{on: boolean, provider: string, model: string, level: string, l2max: number, cap: number, hideNames: boolean}} Settings */

/** @returns {Settings} */
export function readSettings() {
  /** @type {any} */ let o = {};
  try { o = JSON.parse(localStorage.getItem(KEY) || "{}") || {}; } catch (_) { /* blocked or damaged: the defaults */ }
  return { on: o.on === true, provider: String(o.provider || ""), model: String(o.model || ""), level: LEVELS.includes(o.level) ? o.level : "L0",
    l2max: Math.min(50, Math.max(1, Math.round(+o.l2max || 5))), cap: Math.max(0, +o.cap || 0), hideNames: o.hideNames === true };
}
/** @param {Settings} s */
export function writeSettings(s) { try { localStorage.setItem(KEY, JSON.stringify(s)); } catch (_) { /* the setting lasts until the page is closed */ } }

/** Mounts the section in `box`; `startOn` = the user just flipped the switch (the consent is asked at once). @param {HTMLElement} box @param {{C: any, T: (k: string, p?: any) => string}} ui @param {boolean} [startOn] */
export async function mount(box, ui, startOn = false) {
  const { C, T } = ui, esc = /** @type {(s: string) => string} */ (C.esc);
  let s = readSettings();
  /** @type {SiteClient | DesktopClient | null} */ let client = null;
  /** @type {Provider[]} */ let provs = [], hosts = [];
  let replayOn = false;
  const api = { replay: async (/** @type {string} */ text, strict = false) => { if (!client || client.kind !== "site") throw new Error("replay needs the site client"); await /** @type {SiteClient} */ (client).replay(text, strict); replayOn = true; } };
  /** @type {any} */ (window).MZLAB_LLM = Object.defineProperty(api, "client", { get: () => client, enumerable: false });

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


  const render = () => {
    const html = sectionHtml(C, T, s, provs, provider(), model());
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
    $("#llm-revoke").onclick = () => { revoke(provider().id); s.on = false; writeSettings(s); off(); };
    keyArea(client, T, status, $("#llm-keybox"), provider());
    bindTest($, T, errText, () => ({ client, s, replay: replayOn }));
    $("#llm-st").textContent = "";
  };


  if (startOn) { off(); await turnOn(); return; }
  if (s.on && Object.keys(readConsent()).length) {
    try { await load(); if (hasConsent(s.provider || provs[0].id)) { render(); return; } } catch (e) { /* falls back to off */ }
    s.on = false; writeSettings(s);
  }
  off();
}
