// @ts-check
// «Report a bug» dialog of the header (button #bug-btn; shell.js loads this module on the first click). It collects the diagnostics without any name (bug-report-testo.js), shows the
// whole report in a read-only box, and offers two actions: copy it, or open a new issue of the public repository in a new tab with the text already in the form. Nothing is sent by
// the site: no request, no cookie; the link is only opened when the user clicks it, and the user still reads and edits everything on GitHub before sending.
// Keyboard: the button is a header button, Enter/Space open the dialog, Tab stays inside it, Esc closes and gives the focus back.
import { anonimizza, riassuntoFile, righeDiagnostica, rapporto, titolo, urlIssue } from "./bug-report-testo.js";

/** @type {any} */
const W = window;
// E (files), S (state) and DIAG are top-level `const` of classic scripts: global names, not properties of window
const glob = (/** @type {() => any} */ f) => { try { return f(); } catch (_) { return undefined; } };
const EE = () => glob(() => E), SS = () => glob(() => S), DG = () => glob(() => DIAG);
const t = (/** @type {string} */ k) => (W.I18N ? W.I18N.t(k) : k);
const STYLE_ID = "bug-style";
const CSS = `#bugdlg{position:fixed;inset:0;z-index:6200;display:flex;align-items:center;justify-content:center;padding:16px;background:color-mix(in srgb,var(--ink) 45%,transparent)}
#bugdlg[hidden]{display:none}
#bugdlg .bg-box{background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:10px;width:min(640px,100%);max-height:calc(100vh - 32px);overflow:auto;padding:16px 18px;display:flex;flex-direction:column;gap:10px;box-shadow:0 8px 32px color-mix(in srgb,var(--ink) 30%,transparent)}
#bugdlg h2{margin:0;font-size:1.0625rem}
#bugdlg label{display:flex;flex-direction:column;gap:4px;font-size:.875rem;font-weight:600}
#bugdlg label.bg-chk{flex-direction:row;align-items:center;gap:8px;font-weight:500}
#bugdlg textarea{font:inherit;font-weight:400;color:var(--ink);background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:6px 8px;resize:vertical;min-height:64px}
#bugdlg textarea.bg-prev{font:12px/1.4 ui-monospace,Menlo,Consolas,monospace;min-height:150px}
#bugdlg input[type=checkbox]{width:18px;height:18px;accent-color:var(--accent)}
#bugdlg .bg-note{margin:0;font-size:.8125rem;color:var(--muted)}
#bugdlg .bg-act{display:flex;flex-wrap:wrap;gap:8px;align-items:center}
#bugdlg .bg-act>*{min-height:36px;display:inline-flex;align-items:center;padding:0 14px;border-radius:6px;border:1px solid var(--line);background:var(--panel);color:var(--ink);font:inherit;font-weight:600;cursor:pointer;text-decoration:none}
#bugdlg .bg-act a.bg-go{background:var(--accent);border-color:var(--accent);color:var(--bg)}
#bugdlg :is(button,a,textarea,input):focus-visible{outline:2px solid var(--accent);outline-offset:2px}
#bugdlg .bg-st{min-height:1.2em;font-size:.8125rem}
@media (pointer:coarse){#bugdlg .bg-act>*{min-height:40px}}`;

/** @type {HTMLElement | null} */ let dlg = null;
/** @type {Element | null} */ let back = null;
let diagnostics = /** @type {string[]} */ ([]);

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

function costruisci() {
  if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
  const d = document.createElement("div");
  d.id = "bugdlg"; d.hidden = true;
  d.innerHTML = `<div class="bg-box" role="dialog" aria-modal="true" aria-labelledby="bug-h">
<h2 id="bug-h"></h2>
<label><span data-i18n="bug.what"></span><textarea id="bug-what" rows="3"></textarea></label>
<label><span data-i18n="bug.expected"></span><textarea id="bug-exp" rows="3"></textarea></label>
<label class="bg-chk"><input type="checkbox" id="bug-diag" checked><span data-i18n="bug.diag"></span></label>
<label><span data-i18n="bug.preview"></span><textarea id="bug-prev" class="bg-prev" readonly></textarea></label>
<p class="bg-note" data-i18n="bug.privacy"></p>
<div class="bg-act"><button type="button" id="bug-copy"></button><a id="bug-go" class="bg-go" target="_blank" rel="noopener noreferrer" href="#"></a><button type="button" id="bug-x"></button></div>
<div class="bg-st" id="bug-st" role="status" aria-live="polite"></div></div>`;
  document.body.appendChild(d);
  d.addEventListener("mousedown", ev => { if (ev.target === d) chiudi(); });
  d.addEventListener("keydown", ev => {
    if (ev.key === "Escape") { ev.preventDefault(); ev.stopPropagation(); chiudi(); return; }
    if (ev.key !== "Tab") return;
    const f = /** @type {HTMLElement[]} */ ([...d.querySelectorAll("textarea,input,button,a[href]")]).filter(e => !e.hasAttribute("disabled"));
    const first = f[0], last = f[f.length - 1], cur = document.activeElement;
    if (ev.shiftKey && cur === first) { ev.preventDefault(); last.focus(); } else if (!ev.shiftKey && cur === last) { ev.preventDefault(); first.focus(); }
  });
  const q = (/** @type {string} */ s) => /** @type {any} */ (d.querySelector(s));
  q("#bug-what").addEventListener("input", aggiorna); q("#bug-exp").addEventListener("input", aggiorna); q("#bug-diag").addEventListener("change", aggiorna);
  q("#bug-x").onclick = chiudi;
  q("#bug-copy").onclick = async () => {
    const st = q("#bug-st"), ta = q("#bug-prev");
    try { await navigator.clipboard.writeText(ta.value); st.textContent = t("bug.copied"); }
    catch (_) { ta.focus(); ta.select(); st.textContent = t("bug.copyFail"); }
  };
  return d;
}

function testi() {
  if (!dlg) return;
  const q = (/** @type {string} */ s) => /** @type {any} */ (dlg).querySelector(s);
  q("#bug-h").textContent = t("bug.title");
  if (W.I18N) W.I18N.apply(dlg);
  q("#bug-copy").textContent = t("bug.copy"); q("#bug-go").textContent = t("bug.open"); q("#bug-x").textContent = t("bug.close");
  q("#bug-prev").setAttribute("aria-label", t("bug.preview"));
}

function aggiorna() {
  if (!dlg) return;
  const q = (/** @type {string} */ s) => /** @type {any} */ (dlg).querySelector(s);
  const what = q("#bug-what").value, body = rapporto({ what, expected: q("#bug-exp").value, diag: q("#bug-diag").checked ? diagnostics : null }, t);
  q("#bug-prev").value = body;
  const u = urlIssue({ title: titolo(what, t("bug.title.empty")), body, ambiente: navigator.userAgent, cut: "\n\n" + t("bug.cut") });
  q("#bug-go").href = u.url;
  q("#bug-st").textContent = u.troncato ? t("bug.cutNote") : "";
}

export async function apri() {
  back = document.activeElement;
  dlg = dlg || costruisci();
  testi();
  await raccogli();
  aggiorna();
  dlg.hidden = false;
  /** @type {HTMLElement | null} */ (dlg.querySelector("#bug-what"))?.focus();
}

export function chiudi() {
  if (!dlg || dlg.hidden) return;
  dlg.hidden = true;
  if (back instanceof HTMLElement && back.isConnected) back.focus();
}

W.BUGREP = { apri, chiudi };
