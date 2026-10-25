// @ts-strict
// Pieces of the «Assistant» section that keep no state of their own: the markup of the switched-on section, the text of «What will be sent», the test conversation and
// the area of the key. Used by llm-impostazioni.js.
import { readConsent } from "./llm-consenso.js";
import { DesktopClient, SiteClient, asError } from "./llm-clienti.js";

export const LEVELS = ["L0", "L1", "L2"];
/** catalog keys of each level: its name and what it does */
const LV = { L0: ["llm.level.L0", "llm.level.L0.title"], L1: ["llm.level.L1", "llm.level.L1.title"], L2: ["llm.level.L2", "llm.level.L2.title"] };

/** the request as shown to the user: URL, headers (the key only as a slot) and body @param {(k: string, p?: any) => string} T @param {any} r */
export const pretty = (T, r) => `${r.url}\n${(r.headers || []).map((/** @type {string[]} */ h) => `${h[0]}: ${h[1]}`).join("\n")}\n${r.auth && r.auth.header ? `${r.auth.header}: (${T("llm.test.keyslot")})\n` : ""}\n${(() => { try { return JSON.stringify(JSON.parse(r.body), null, 2); } catch (_) { return r.body; } })()}`;

/** the test conversation: one user message @param {{provider: string, model: string}} s @param {string} text */
export const conv = (s, text) => ({ schema: "llm-conv/1", provider: s.provider, model: s.model, system: "", max_output_tokens: 256, messages: [{ role: "user", parts: [{ text }] }] });

/** the markup of the switched-on section @param {any} C @param {(k: string, p?: any) => string} T @param {any} s @param {any[]} provs @param {any} p @param {any} m */
export function sectionHtml(C, T, s, provs, p, m) {
  const esc = /** @type {(s: string) => string} */ (C.esc);
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
  return html;
}

/** the key area: the iframe in the site (cloud providers; the key never reaches this document), a password field in the desktop app, nothing for a model on this computer
 * @param {any} client @param {(k: string, p?: any) => string} T @param {(k: string, p?: any) => void} status @param {HTMLElement} kb @param {import("./llm-clienti.js").Provider} p */
export const keyUi = (client, T, status, kb, p) => {
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

/** the test area: preview of the request, send and stop @param {(sel: string) => any} $ @param {(k: string, p?: any) => string} T @param {(e: {key: string, params?: any}) => string} errText
 * @param {() => {client: any, s: any, replay: boolean}} get the current client, settings and whether a recording answers */
export function bindTest($, T, errText, get) {
  const out = $("#llm-out"), pv = $("#llm-prevout"), text = () => $("#llm-msg").value || T("llm.test.ph");
  $("#llm-prev").onclick = async () => {
    const { client, s } = get();
    try { pv.textContent = pretty(T, await client.request(conv(s, text()))); pv.hidden = false; pv.focus(); } catch (e) { out.textContent = errText(asError(e)); }
  };
  $("#llm-send").onclick = async () => {
    const { client, s, replay } = get();
    out.textContent = ""; $("#llm-send").disabled = true; $("#llm-stop").hidden = false;
    try {
      await client.send(conv(s, text()), (/** @type {import("./llm-clienti.js").LlmEvent} */ e) => {
        if (e.ev === "text_delta") out.textContent += e.text || "";
        else if (e.ev === "error") out.textContent += "\n" + errText({ key: e.key || "", params: e.params });
      }, replay);
    } catch (e) { out.textContent += "\n" + errText(asError(e)); }
    $("#llm-send").disabled = false; $("#llm-stop").hidden = true;
  };
  $("#llm-stop").onclick = () => get().client.stop();
}
