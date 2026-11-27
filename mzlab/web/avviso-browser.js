// @ts-check
// A closable notice (never a block) for a browser that runs the site worse: decided by feature checks of capacita.js, never by the user agent
// (the agent string only names the browser in the text). Two cases: the engine cannot run (SIMD128 or OPFS missing: update, or Chrome, Firefox,
// Edge), or the browser cannot point a folder (showDirectoryPicker: only Chrome and Edge have it, Firefox lacks it too, so it is NOT suggested).
// Closing is remembered in localStorage (in try). Loaded lazily by capacita.js, which makes the feature checks (none in the desktop app, no folder notice on phones/tablets).
const KEY = "mzlab.browserNotice";
const URL_OF = { chrome: "https://www.google.com/chrome/", firefox: "https://www.mozilla.org/firefox/new/", edge: "https://www.microsoft.com/edge" };
const g = /** @type {any} */ (globalThis);
const t = (/** @type {string} */ k, /** @type {Record<string, string>} */ p = {}) => (g.I18N ? g.I18N.t(k, p) : k);
const ESC = (/** @type {string} */ s) => s.replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] || c);

/** Name of the browser for the text only: "safari" | "firefox" | "other". */
export function nome(/** @type {string} */ ua = (g.navigator && g.navigator.userAgent) || "") {
  if (/Firefox\/|FxiOS/.test(ua)) return "firefox";
  if (/Safari\//.test(ua) && !/Chrome|Chromium|Edg|OPR|CriOS|Android/.test(ua)) return "safari";
  return "other";
}

const saved = () => { try { return g.localStorage.getItem(KEY) === "1"; } catch (_) { return false; } };
const remember = () => { try { g.localStorage.setItem(KEY, "1"); } catch (_) { /* private window: shown again next time */ } };

function css() {
  if (document.getElementById("bn-css")) return;
  const s = document.createElement("style"); s.id = "bn-css";
  s.textContent = "#browser-notice{position:fixed;top:12px;left:50%;transform:translateX(-50%);z-index:7100;box-sizing:border-box;width:min(460px,calc(100vw - 32px));max-height:80vh;overflow:auto;"
    + "padding:10px 12px;border:1px solid var(--line);border-left:3px solid var(--accent);border-radius:6px;background:var(--panel);color:var(--ink);font-size:.92em;box-shadow:0 4px 18px rgba(0,0,0,.25)}"
    + "#browser-notice h2{margin:0 28px 4px 0;font-size:1em}#browser-notice p{margin:4px 0}"
    + "#browser-notice .bn-x{position:absolute;top:4px;right:4px;min-width:28px;min-height:28px;cursor:pointer}"
    + "#browser-notice .bn-links{display:flex;flex-wrap:wrap;gap:4px 14px;margin-top:6px}"
    + "#browser-notice .bn-links a{color:var(--accent);display:inline-block;min-height:24px;min-width:24px;line-height:24px}";
  document.head.appendChild(s);
}

function close() {
  const n = document.getElementById("browser-notice"); if (n) n.remove();
  remember();
}

/** Shows (or, for the engine case, replaces) the notice. `kind` is "engine" or "folder". */
export function mostra(/** @type {string} */ kind, /** @type {boolean} */ touch = false) {
  if (typeof document === "undefined" || !document.body || saved()) return;
  const old = document.getElementById("browser-notice");
  if (old && old.dataset.kind === kind) return;
  css();
  const web = nome();
  const who = t(`cap.browser.name.${web}`);
  const names = kind === "engine" ? ["chrome", "firefox", "edge"] : ["chrome", "edge"];
  const links = touch ? "" : `<p class="bn-links">${names.map(k => `<a href="${URL_OF[/** @type {"chrome"} */ (k)]}" target="_blank" rel="noopener noreferrer">${ESC(t(`cap.browser.get.${k}`))}</a>`).join("")}</p>`;
  const n = old || document.createElement("div");
  n.id = "browser-notice"; n.dataset.kind = kind; n.setAttribute("role", "dialog"); n.setAttribute("aria-modal", "false");
  n.setAttribute("aria-labelledby", "bn-title"); n.setAttribute("aria-describedby", "bn-text");
  n.innerHTML = `<h2 id="bn-title">${ESC(t(`cap.browser.${kind}.title`, { browser: who }))}</h2><p id="bn-text">${ESC(t(`cap.browser.${kind}.text`, { browser: who }))}</p>${links}`
    + `<button type="button" class="bn-x" aria-label="${ESC(t("cap.browser.close"))}">×</button>`;
  /** @type {HTMLElement} */ (n.querySelector(".bn-x")).addEventListener("click", close);
  n.addEventListener("keydown", e => { if (e.key === "Escape") { e.stopPropagation(); close(); } });
  // first in the page, so the Tab key reaches it before the rest; it takes no focus by itself (the site stays usable behind it)
  if (!old) document.body.insertBefore(n, document.body.firstChild);
}
