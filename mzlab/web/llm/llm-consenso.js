// @ts-check
// Consent of the assistant: what the user agreed to, per provider and class of data (localStorage `mzlab.llm.consenso`, revocable in the settings), and the dialog that asks for it.
// Nothing leaves the computer before a consent exists for the provider and every class the request carries. Loaded by llm-impostazioni.js, only in the profile «ricerca».

/** the classes of data a request may carry; a class that is not in the stored consent asks again */
export const CLASSES = ["plan", "summary", "names"];
const KEY = "mzlab.llm.consenso";
/** catalog keys of each class: its name and what it carries */
const WORDS = { plan: ["llm.consenso.c.plan", "llm.consenso.c.plan.d"], summary: ["llm.consenso.c.summary", "llm.consenso.c.summary.d"], names: ["llm.consenso.c.names", "llm.consenso.c.names.d"] };

/** @returns {Record<string, {classes: string[]}>} */
export function readConsent() {
  try { const o = JSON.parse(localStorage.getItem(KEY) || "{}"); return o && typeof o === "object" && !Array.isArray(o) ? o : {}; } catch (_) { return {}; }
}
/** @param {Record<string, {classes: string[]}>} o */
function writeConsent(o) { try { localStorage.setItem(KEY, JSON.stringify(o)); } catch (_) { /* storage blocked: the consent lasts until the page is closed */ memory = o; } }
/** @type {Record<string, {classes: string[]}> | null} */ let memory = null;
const current = () => memory || readConsent();

/** @param {string} provider @param {string[]} classes */
export function hasConsent(provider, classes = CLASSES) {
  const c = current()[provider];
  return !!c && classes.every(k => c.classes.includes(k));
}
/** @param {string} provider @param {string[]} classes */
export function grant(provider, classes) {
  const o = current(); o[provider] = { classes: [...new Set([...(o[provider] ? o[provider].classes : []), ...classes])] }; writeConsent(o);
}
/** @param {string} provider */
export function revoke(provider) { const o = current(); delete o[provider]; writeConsent(o); }

/** the price line of the model, or the words «unknown» when the program holds no price (it never invents one)
 * @param {{price?: {in?: number, out?: number, as_of?: string, currency?: string}}} model @param {(k: string, p?: any) => string} T */
export function costLine(model, T) {
  const p = model && model.price;
  if (!p || !p.in || !p.out) return T("llm.consenso.cost.unknown");
  return T("llm.consenso.cost.price", { in: p.in, out: p.out, cur: p.currency || "USD", date: p.as_of || "?" });
}

/** The dialog. Resolves true when the user agrees (and stores the consent), false on «Cancel» or Esc.
 * @param {{provider: string, host: string, model: string, local: boolean, cost: string, classes?: string[], T: (k: string, p?: any) => string, opener?: HTMLElement | null}} o */
export function askConsent(o) {
  const { T } = o, classes = o.classes || CLASSES;
  return new Promise(done => {
    const d = document.createElement("dialog");
    d.setAttribute("aria-labelledby", "llm-c-title");
    const li = classes.map(k => `<li><b>${T(WORDS[/** @type {"plan"} */ (k)][0])}</b> &mdash; ${T(WORDS[/** @type {"plan"} */ (k)][1])}</li>`).join("");
    const esc = (/** @type {string} */ s) => s.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
    const html = `<h2 id="llm-c-title">${T("llm.consenso.title")}</h2>`
      + `<p>${T(o.local ? "llm.consenso.where.local" : "llm.consenso.where", { host: esc(o.host) })}</p>`
      + `<p>${T("llm.consenso.model", { model: esc(o.model), provider: esc(o.provider) })}</p>`
      + `<p>${T("llm.consenso.classes")}</p><ul>${li}</ul>`
      + `<p><b>${T("llm.consenso.raw")}</b></p><p>${esc(o.cost)}</p><p>${T("llm.consenso.preview")}</p>`
      + `<div class="row"><button type="button" id="llm-c-no">${T("llm.consenso.cancel")}</button><button type="button" id="llm-c-ok">${T("llm.consenso.accept")}</button></div>`;
    d.innerHTML = html;
    const finish = (/** @type {boolean} */ ok) => { if (ok) grant(o.provider, classes); d.close(); d.remove(); if (o.opener && o.opener.isConnected) o.opener.focus(); done(ok); };
    // Esc closes only the dialog, not the settings behind it
    d.addEventListener("keydown", ev => { if (ev.key === "Escape") { ev.stopPropagation(); ev.preventDefault(); finish(false); } });
    d.addEventListener("cancel", ev => { ev.preventDefault(); finish(false); });
    /** @type {HTMLElement} */ (d.querySelector("#llm-c-no")).onclick = () => finish(false);
    /** @type {HTMLElement} */ (d.querySelector("#llm-c-ok")).onclick = () => finish(true);
    document.body.appendChild(d);
    d.showModal();
    /** @type {HTMLElement} */ (d.querySelector("#llm-c-no")).focus();
  });
}
