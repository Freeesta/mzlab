// @ts-strict
// «Report a bug» and «Suggest an improvement» groups of the settings panel (settings.js loads this module when a group is opened; `mount(host, kind)`, kind «bug» or «suggestion»).
// A free field (a suggestion also has «What I was doing», optional); the diagnostics (bug-report-dati.js, bug-report-testo.js, no names) are a list of fields, each with a tick the user can remove.
// Four actions: open a new issue of the public repository in a new tab with the text already in the form (nothing is sent by the site: the user reads and presses «Submit» on GitHub), copy it,
// write an email with it (the address is joined at click time), or download the same content as JSON. Keyboard and focus are those of the settings panel.
import { campiDiagnostica, righe, jsonRapporto, rapporto, titolo, urlIssue, urlEmail, TIPI } from "./bug-report-testo.js";
import { istantanea, nomi } from "./bug-report-dati.js";

/** @type {any} */
const W = window;
const glob = (/** @type {() => any} */ f) => { try { return f(); } catch (_) { return undefined; } };
const DG = () => glob(() => DIAG);
const t = (/** @type {string} */ k) => (W.I18N ? W.I18N.t(k) : k);
const STYLE_ID = "bug-style";
const CSS = `#uipset .bg-box{display:flex;flex-direction:column;gap:8px}
#uipset .bg-box textarea{font:inherit;color:var(--ink);background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:6px 8px;resize:vertical;min-height:88px;width:100%;box-sizing:border-box}
#uipset .bg-box pre{margin:4px 0 0;font:12px/1.4 ui-monospace,Menlo,Consolas,monospace;white-space:pre-wrap;color:var(--ink)}
#uipset .bg-box summary{cursor:pointer;min-height:24px;font-size:.8125rem;color:var(--muted)}
#uipset .bg-act{display:flex;flex-wrap:wrap;gap:8px;align-items:center}
#uipset .bg-act>*{min-height:32px;display:inline-flex;align-items:center;gap:6px;padding:0 12px;border-radius:6px;border:1px solid var(--line);background:var(--panel);color:var(--ink);font:inherit;font-weight:600;cursor:pointer;text-decoration:none}
#uipset .bg-act .bg-go{background:var(--accent);border-color:var(--accent);color:var(--bg)}
#uipset .bg-box :is(button,a,textarea,summary,input):focus-visible{outline:2px solid var(--accent);outline-offset:2px}
#uipset .bg-st{min-height:1.2em;font-size:.8125rem}
#uipset .bg-f{display:flex;gap:6px;align-items:center;font-size:.8125rem;min-height:24px}
#uipset .bg-f input{margin:0}
#uipset .bg-f.off+pre{opacity:.45;text-decoration:line-through}
#uipset .bg-box label.bg-l{font-size:.8125rem;color:var(--muted)}
@media (pointer:coarse){#uipset .bg-act>*{min-height:40px}}`;


/** one form per kind: its host, the fields, the keys unticked by the user and the text as it is now (for «Copy», the email and the download)
 * @typedef {{host: HTMLElement, p: string, fields: ReturnType<typeof campiDiagnostica>, off: Set<string>, corpo: string}} Form */
const forms = /** @type {Record<string, Form>} */ ({});
/** the catalog keys of each kind (written whole, so the i18n check sees them) */
const KEYS = { bug: { what: "bug.what", tip: "bug.what.tip", empty: "bug.title.empty" }, suggestion: { what: "sug.what", tip: "sug.what.tip", empty: "sug.title.empty" } };
const MAIL = { user: "federico.cristaudo", host: "unito.it" };
const ICO = {
  gh: '<svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 4H4.5v11.5H16V12M11 4h5v5M16 4l-7 7"/></svg>',
  copy: '<svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="7" y="7" width="9" height="9" rx="1.5"/><path d="M4 12V4.5A1.5 1.5 0 0 1 5.5 3H12"/></svg>',
  mail: '<svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="5" width="14" height="10" rx="1.5"/><path d="M3.5 6l6.5 5 6.5-5"/></svg>',
  dl: '<svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5v9M6.5 9.5L10 13l3.5-3.5M4 16.5h12"/></svg>',
};

/** @param {Form} f @param {string} s */
const q = (f, s) => /** @type {any} */ (f.host).querySelector(s);

/** @param {string} kind */
async function raccogli(kind) {
  const f = forms[kind], names = nomi(), pul = DG() ? (/** @type {string} */ s) => DG().pulisci(s, 400) : undefined;
  f.fields = campiDiagnostica(await istantanea(kind === "suggestion"), names, pul, TIPI[/** @type {keyof typeof TIPI} */ (kind)].keys);
  f.off = new Set();
  const box = q(f, "#" + f.p + "-diagtxt");
  box.textContent = "";
  for (const fl of f.fields) {
    const row = document.createElement("label"), cb = document.createElement("input"), nm = document.createElement("span"), pre = document.createElement("pre");
    row.className = "bg-f"; cb.type = "checkbox"; cb.checked = true; cb.dataset.k = fl.key; nm.textContent = fl.label; cb.title = t("bug.field.tip");
    cb.addEventListener("change", () => { if (cb.checked) f.off.delete(fl.key); else f.off.add(fl.key); row.classList.toggle("off", !cb.checked); aggiorna(kind); });
    row.append(cb, nm); pre.textContent = fl.lines.join("\n"); box.append(row, pre);
  }
}

/** Fills `host` (the body of the settings group) with the fields, the diagnostics list and the actions.
 * @param {HTMLElement} host @param {string} [kind] "bug" (default) or "suggestion" */
export async function mount(host, kind = "bug") {
  if (!document.getElementById(STYLE_ID)) { const st = document.createElement("style"); st.id = STYLE_ID; st.textContent = CSS; document.head.appendChild(st); }
  const sg = kind === "suggestion", p = sg ? "sug" : "bug", K = KEYS[/** @type {keyof typeof KEYS} */ (kind)], f = (forms[kind] = { host, p, fields: [], off: new Set(), corpo: "" });
  host.innerHTML = `<div class="bg-box">
<label for="${p}-what" class="sr-only"></label>
<textarea id="${p}-what" rows="4"></textarea>
${sg ? `<label for="sug-doing" class="bg-l" data-i18n="sug.doing"></label><textarea id="sug-doing" rows="2" data-i18n-title="sug.doing.tip"></textarea>` : ""}
<details><summary data-i18n="bug.info.btn" data-i18n-title="bug.info.tip"></summary><div id="${p}-diagtxt"></div></details>
<div class="bg-act"><a id="${p}-go" class="bg-go" target="_blank" rel="noopener noreferrer" href="#" data-i18n-title="bug.open.tip">${ICO.gh}<span data-i18n="bug.open"></span></a><button type="button" id="${p}-copy" data-i18n-title="bug.copy.tip">${ICO.copy}<span data-i18n="bug.copy"></span></button><button type="button" id="${p}-mail" data-i18n-title="bug.mail.tip">${ICO.mail}<span data-i18n="bug.mail"></span></button><button type="button" id="${p}-dl" data-i18n-title="bug.dl.tip">${ICO.dl}<span data-i18n="bug.dl"></span></button></div>
<div class="bg-st" id="${p}-st" role="status" aria-live="polite"></div></div>`;
  // the keys of the kind are set here (written whole in KEYS), not in the template: the tooltip inventory reads literal keys only
  q(f, `label[for=${p}-what]`).dataset.i18n = K.what;
  Object.assign(q(f, `#${p}-what`).dataset, { i18nPlaceholder: K.what, i18nTitle: K.tip });
  if (W.I18N) W.I18N.apply(host);
  for (const id of ["what", "doing"]) { const el = q(f, `#${p}-${id}`); if (el) el.addEventListener("input", () => aggiorna(kind)); }
  q(f, `#${p}-copy`).onclick = async () => {
    const st = q(f, `#${p}-st`);
    try { await navigator.clipboard.writeText(f.corpo); st.textContent = t("bug.copied"); }
    catch (_) { st.textContent = t("bug.copyFail"); }
  };
  q(f, `#${p}-mail`).onclick = () => { const a = document.createElement("a"); a.href = mailUrl(kind).url; a.click(); };
  q(f, `#${p}-dl`).onclick = () => {
    const a = document.createElement("a"), u = URL.createObjectURL(new Blob([json(kind)], { type: "application/json" }));
    a.href = u; a.download = `mzlab-${sg ? "suggestion" : "bug-report"}.json`; a.click(); setTimeout(() => URL.revokeObjectURL(u), 2000);
    q(f, `#${p}-st`).textContent = t("bug.downloaded");
  };
  await raccogli(kind);
  aggiorna(kind);
}

/** the user's two answers @param {string} kind */
function risposte(kind) {
  const f = forms[kind], doing = q(f, "#sug-doing");
  return { what: q(f, `#${f.p}-what`).value, doing: kind === "suggestion" && doing ? doing.value : undefined };
}

/** @param {string} kind */
function aggiorna(kind) {
  const f = forms[kind];
  if (!f || !f.host.isConnected) return;
  const { what, doing } = risposte(kind), body = rapporto({ what, doing, diag: righe(f.fields, f.off) }, t);
  f.corpo = body;
  const tp = TIPI[/** @type {keyof typeof TIPI} */ (kind)];
  const u = urlIssue({ kind, title: titolo(what, t(KEYS[/** @type {keyof typeof KEYS} */ (kind)].empty), tp.prefix), body, ambiente: kind === "bug" ? navigator.userAgent : undefined, cut: "\n\n" + t("bug.cut") });
  q(f, `#${f.p}-go`).href = u.url;
  q(f, `#${f.p}-st`).textContent = u.truncated || mailUrl(kind).truncated ? t("bug.cutNote") : "";
}

/** the mailto link for the report now on screen (also used by the tests) @param {string} [kind] */
function mailUrl(kind = "bug") {
  const f = forms[kind], tp = TIPI[/** @type {keyof typeof TIPI} */ (kind)], what = f ? risposte(kind).what : "";
  return urlEmail({ ...MAIL, subject: titolo(what, t(KEYS[/** @type {keyof typeof KEYS} */ (kind)].empty), tp.prefix), body: f ? f.corpo : "", cut: "\n\n" + t("bug.cut") });
}

/** the JSON of the report now on screen: the same fields, by key, with no cut @param {string} [kind] */
function json(kind = "bug") {
  const f = forms[kind], r = risposte(kind);
  return jsonRapporto(kind, r.what, r.doing || "", f.fields, f.off);
}

W.BUGREP = { mount, mailUrl, json, corpo: (/** @type {string} */ kind = "bug") => (forms[kind] ? forms[kind].corpo : "") };
