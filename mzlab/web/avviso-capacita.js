// @ts-check
// The common notice of a function this browser cannot run (capacita.js): the command gets aria-disabled and a short note with the link to the app next to it;
// one summary line per session goes to a polite live region. Colors only from the CSS variables of the page.
const APP = "https://github.com/Freeesta/mzlab/releases/latest";
const g = /** @type {any} */ (globalThis);
/** @type {Set<string>} */ const told = g.__capacitaTold || (g.__capacitaTold = new Set());

const t = (/** @type {string} */ k, /** @type {Record<string, string>} */ p = {}) => (g.I18N ? g.I18N.t(k, p) : k);
const ESC = (/** @type {string} */ s) => s.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] || c);

function css() {
  if (document.getElementById("cap-css")) return;
  const s = document.createElement("style"); s.id = "cap-css";
  s.textContent = ".cap-note{display:block;margin:4px 0;padding:4px 8px;border:1px solid var(--line);border-left:3px solid var(--accent);border-radius:4px;background:var(--panel);color:var(--ink);font-size:.9em}"
    + ".cap-note a{color:var(--accent);display:inline-block;min-height:24px;min-width:24px;line-height:24px}"
    + "#cap-live{position:fixed;left:12px;bottom:12px;z-index:7000;max-width:min(420px,90vw)}"
    + "[aria-disabled=true].cap-off{opacity:.55;cursor:not-allowed}";
  document.head.appendChild(s);
}

// the line for one function: "{function}: not available in this browser. Use the app" + the link
const line = (/** @type {string} */ funzione) => `${ESC(t("cap.missing", { funzione: t(`cap.fn.${funzione}`) }))} <a href="${APP}" target="_blank" rel="noopener noreferrer">${ESC(t("cap.app"))}</a>`;

/** Shows the notice for `funzione`; `comando` (if any) is marked aria-disabled and gets the note right after it. */
export function segnala(/** @type {string} */ funzione, /** @type {HTMLElement | null} */ comando = null) {
  if (typeof document === "undefined" || !document.body) return;
  css();
  const id = "cap-note-" + funzione;
  if (comando) {
    comando.setAttribute("aria-disabled", "true"); comando.classList.add("cap-off");
    if (!document.getElementById(id)) {
      const n = document.createElement("div"); n.id = id; n.className = "cap-note"; n.setAttribute("role", "note"); n.innerHTML = line(funzione);
      comando.insertAdjacentElement("afterend", n); comando.setAttribute("aria-describedby", id);
    }
  }
  if (told.has(funzione)) return;
  told.add(funzione);
  let live = document.getElementById("cap-live");
  if (!live) { live = document.createElement("div"); live.id = "cap-live"; live.setAttribute("role", "status"); live.setAttribute("aria-live", "polite"); document.body.appendChild(live); }
  // one line for the session, listing the functions turned off
  live.className = "cap-note"; live.innerHTML = [...told].map(line).join("<br>");
}
