// @ts-strict
// «Report a bug» group of the settings panel (settings.js loads this module when the group is opened). One free field; the diagnostics (bug-report-testo.js, no names) are always added.
// Three actions: open a new issue of the public repository in a new tab with the text already in the form, copy it, or write an email with it (the address is joined here, at click time).
// Nothing is sent by the site: no request, no cookie; the user still reads and edits everything on GitHub or in the mail program before sending. Keyboard and focus are those of the settings panel.
import { anonimizza, riassuntoFile, righeDiagnostica, rapporto, titolo, urlIssue, urlEmail } from "./bug-report-testo.js";

/** @type {any} */
const W = window;
// E (files), S (state) and DIAG are top-level `const` of classic scripts: global names, not properties of window
const glob = (/** @type {() => any} */ f) => { try { return f(); } catch (_) { return undefined; } };
const EE = () => glob(() => E), SS = () => glob(() => S), DG = () => glob(() => DIAG);
const t = (/** @type {string} */ k) => (W.I18N ? W.I18N.t(k) : k);
const STYLE_ID = "bug-style";
const CSS = `#uipset .bg-box{display:flex;flex-direction:column;gap:8px}
#uipset .bg-box textarea{font:inherit;color:var(--ink);background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:6px 8px;resize:vertical;min-height:88px;width:100%;box-sizing:border-box}
#uipset .bg-box pre{margin:4px 0 0;font:12px/1.4 ui-monospace,Menlo,Consolas,monospace;white-space:pre-wrap;color:var(--ink)}
#uipset .bg-box summary{cursor:pointer;min-height:24px;font-size:.8125rem;color:var(--muted)}
#uipset .bg-act{display:flex;flex-wrap:wrap;gap:8px;align-items:center}
#uipset .bg-act>*{min-height:32px;display:inline-flex;align-items:center;gap:6px;padding:0 12px;border-radius:6px;border:1px solid var(--line);background:var(--panel);color:var(--ink);font:inherit;font-weight:600;cursor:pointer;text-decoration:none}
#uipset .bg-act .bg-go{background:var(--accent);border-color:var(--accent);color:var(--bg)}
#uipset .bg-box :is(button,a,textarea,summary):focus-visible{outline:2px solid var(--accent);outline-offset:2px}
#uipset .bg-st{min-height:1.2em;font-size:.8125rem}
@media (pointer:coarse){#uipset .bg-act>*{min-height:40px}}`;

/** @type {HTMLElement | null} */ let box = null;
let diagnostics = /** @type {string[]} */ ([]);
/** the report as it is now, for «Copy» and the email */
let corpo = "";
const MAIL = { user: "federico.cristaudo", host: "unito.it" };
const ICO = {
  gh: '<svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 4H4.5v11.5H16V12M11 4h5v5M16 4l-7 7"/></svg>',
  copy: '<svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="7" y="7" width="9" height="9" rx="1.5"/><path d="M4 12V4.5A1.5 1.5 0 0 1 5.5 3H12"/></svg>',
  mail: '<svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="5" width="14" height="10" rx="1.5"/><path d="M3.5 6l6.5 5 6.5-5"/></svg>',
};

/** names of everything the user may have called something: files and labels of the open session */
function nomi() {
  const out = /** @type {string[]} */ ([]);
  for (const f of (EE() && EE().files) || []) for (const k of ["file", "label", "name", "sample", "nome"]) if (typeof f[k] === "string") out.push(f[k]);
  return out;
}

async function swState() {
  if (!("serviceWorker" in navigator)) return "unsupported";
  try {
    const r = await navigator.serviceWorker.getRegistration();
    const s = r ? (r.active ? "active" : r.waiting ? "waiting" : r.installing ? "installing" : "registered") : "none";
    return s + (navigator.serviceWorker.controller ? ", controlling" : "");
  } catch (_) { return "unknown"; }
}

async function raccogli() {
  const names = nomi(), pul = DG() ? (/** @type {string} */ s) => DG().pulisci(s, 400) : undefined;
  const hash = (location.hash.match(/^#\/[A-Za-z0-9/_-]*/) || ["#/"])[0];
  const errs = W.ERRORI && W.ERRORI.recenti ? W.ERRORI.recenti(5) : [];
  diagnostics = righeDiagnostica({
    version: W.MZLAB_VERSION, commit: W.MZLAB_COMMIT, date: new Date().toISOString(), ua: navigator.userAgent,
    mode: W.MZLAB_DESKTOP ? "desktop" : "web", view: `${(SS() && SS().view) || "?"} (${hash})`,
    files: riassuntoFile((EE() && EE().files) || []), sw: await swState(), errors: errs.map((/** @type {string} */ e) => anonimizza(e, names, pul)),
  });
}

const q = (/** @type {string} */ s) => /** @type {any} */ (box).querySelector(s);

/** Fills `host` (the body of the settings group) with the field, the diagnostics note and the actions.
 * @param {HTMLElement} host */
export async function mount(host) {
  if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
  box = host;
  host.innerHTML = `<div class="bg-box">
<label for="bug-what" class="sr-only" data-i18n="bug.what"></label>
<textarea id="bug-what" rows="4" data-i18n-placeholder="bug.what"></textarea>
<p class="hint" data-i18n="bug.info"></p>
<details><summary data-i18n="bug.info.btn"></summary><pre id="bug-diagtxt"></pre></details>
<div class="bg-act"><a id="bug-go" class="bg-go" target="_blank" rel="noopener noreferrer" href="#">${ICO.gh}<span data-i18n="bug.open"></span></a><button type="button" id="bug-copy">${ICO.copy}<span data-i18n="bug.copy"></span></button><button type="button" id="bug-mail">${ICO.mail}<span data-i18n="bug.mail"></span></button></div>
<div class="bg-st" id="bug-st" role="status" aria-live="polite"></div></div>`;
  if (W.I18N) W.I18N.apply(host);
  q("#bug-what").addEventListener("input", aggiorna);
  q("#bug-copy").onclick = async () => {
    const st = q("#bug-st");
    try { await navigator.clipboard.writeText(corpo); st.textContent = t("bug.copied"); }
    catch (_) { st.textContent = t("bug.copyFail"); }
  };
  q("#bug-mail").onclick = () => {
    const a = document.createElement("a");
    a.href = mailUrl().url; a.click();
  };
  await raccogli();
  aggiorna();
}

function aggiorna() {
  if (!box) return;
  const what = q("#bug-what").value, body = rapporto({ what, diag: diagnostics }, t);
  corpo = body;
  q("#bug-diagtxt").textContent = diagnostics.join("\n");
  const u = urlIssue({ title: titolo(what, t("bug.title.empty")), body, ambiente: navigator.userAgent, cut: "\n\n" + t("bug.cut") });
  q("#bug-go").href = u.url;
  q("#bug-st").textContent = u.troncato || mailUrl().troncato ? t("bug.cutNote") : "";
}

/** the mailto link for the report now on screen (also used by the tests) */
function mailUrl() {
  const what = box ? q("#bug-what").value : "";
  return urlEmail({ ...MAIL, subject: titolo(what, t("bug.title.empty")), body: corpo, cut: "\n\n" + t("bug.cut") });
}

W.BUGREP = { mount, mailUrl, corpo: () => corpo };
