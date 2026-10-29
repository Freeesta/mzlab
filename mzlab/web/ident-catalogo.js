// @ts-strict
// The «Get libraries» menu of the identification tab (ident-pannello.js): the public libraries of `librerie-pubbliche.json` (name, version, source, size, polarity, level),
// each with a download link and a link to its site. The page's CSP (connect-src 'self') does not allow fetching from Zenodo or GNPS, so ↓ is a plain `<a download>`:
// the browser saves the file and «Load file…» reads it. Nothing is fetched until the menu is opened, and nothing is stored in the repository.

/** Formats the engine reads (libreria-worker.js): the others keep a disabled ↓ with a tooltip. */
const SUPPORTED = new Set(["msp", "mgf", "json", "jsonl", "ndjson", "mzlib"]);
export const isSupported = (/** @type {string} */ fmt) => SUPPORTED.has(String(fmt).toLowerCase());
const DOWN = '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M8 2v8M4.5 7 8 10.5 11.5 7M3 13.5h10"/></svg>';
const GLOBE = '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="1.4"><circle cx="8" cy="8" r="6"/><path d="M2 8h12M8 2c2 2 2 10 0 12M8 2c-2 2-2 10 0 12"/></svg>';
const css = `.cat-box{max-height:40vh;margin-top:4px;overflow:auto;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;padding:4px 8px}
.cat-box h5{margin:6px 0 2px;font-size:12px;color:var(--muted)}.cat-it{display:flex;gap:6px;align-items:center;border-bottom:1px solid var(--line);padding:2px 0}.cat-it span.nm{flex:1}
.cat-it a,.cat-it button{display:inline-flex;align-items:center;justify-content:center;min-width:28px;min-height:28px;color:var(--ink)}@media (pointer:coarse){.cat-it a,.cat-it button{min-width:40px;min-height:40px}}`;

/** @type {Promise<any[]> | null} */
let loaded = null;
const items = () => (loaded ||= fetch(new URL("librerie-pubbliche.json", import.meta.url), { cache: "no-store" }).then(r => r.json()).then(j => j.librerie));
const mb = (/** @type {number} */ n) => (n >= 1e9 ? (n / 1e9).toFixed(1) + " GB" : Math.max(1, Math.round(n / 1e6)) + " MB");

function listHtml(/** @type {any[]} */ L) {
  const groups = new Map();
  for (const e of L) { const g = `${e.fonte} · ${e.livello === 3 ? "MSn" : "MS2"} · ${e.polarita > 0 ? "+" : e.polarita < 0 ? "−" : "±"}`; (groups.get(g) || groups.set(g, []).get(g)).push(e); }
  return [...groups].map(([g, es]) => h`<h5>${g}</h5>` + es.map((/** @type {any} */ e) => {
    const ok = isSupported(e.formato), nm = `${e.nome} ${e.versione}`;
    const down = ok ? h`<a href="${e.url}" download rel="noopener noreferrer" aria-label="${I18N.t("bn.idn.dl", { name: nm })}" title="${I18N.t("bn.idn.dl", { name: nm })}">${raw(DOWN)}</a>`
      : h`<button type="button" disabled aria-label="${I18N.t("bn.idn.dl", { name: nm })}" title="${I18N.t("bn.idn.dlNo")}">${raw(DOWN)}</button>`;
    return h`<div class="cat-it"><span class="nm">${e.nome} <span class="muted">${e.versione} · ${e.licenza} · ${e.formato} · ${mb(e.dimensione)}</span></span>${raw(down)}<a href="${e.sito}" target="_blank" rel="noopener noreferrer" aria-label="${I18N.t("bn.idn.site", { name: nm })}" title="${I18N.t("bn.idn.site", { name: nm })}">${raw(GLOBE)}</a></div>`;
  }).join("")).join("");
}

/** Puts the «Get libraries» button (↓▾) and its list in `host`. */
export function menu(/** @type {HTMLElement} */ host) {
  if (!document.getElementById("cat-css")) { const st = document.createElement("style"); st.id = "cat-css"; st.textContent = css; document.head.appendChild(st); }
  host.innerHTML = h`<button type="button" id="cat-b" aria-expanded="false" aria-controls="cat-box">${raw(DOWN)}<span>${I18N.t("bn.idn.get")} ▾</span></button><div class="cat-box" id="cat-box" hidden></div>`;
  const btn = /** @type {HTMLButtonElement} */ (host.querySelector("#cat-b")), box = /** @type {HTMLElement} */ (host.querySelector("#cat-box"));
  const set = (/** @type {boolean} */ open) => { box.hidden = !open; btn.setAttribute("aria-expanded", String(open)); };
  btn.onclick = async () => {
    if (!box.hidden) return set(false);
    set(true); box.setAttribute("aria-busy", "true");
    try { box.innerHTML = listHtml(await items()); } catch (e) { loaded = null; box.textContent = I18N.t("bn.idn.catErr"); }
    box.removeAttribute("aria-busy");
  };
  host.onkeydown = e => { if (e.key === "Escape" && !box.hidden) { set(false); btn.focus(); e.stopPropagation(); } };
}
