// @ts-strict
// The text of a bug report or of a suggestion, no DOM and no network: what the «Report a bug» and «Suggest an improvement» groups of the settings (bug-report.js) show, copy,
// download (JSON) and put in the link to a new GitHub issue. Every diagnostic is a FIELD {key, label, value, lines}: the user sees them all and can untick any before sending.
// Privacy rule: the diagnostics hold only counts, formats and size classes of the loaded files, never a file name, a sample label or a path; every string of a field goes through
// `anonimizza` (names of the loaded files and file-like words become «<file>»). The user's own description is the only free text. Run by node in tests/test_bug_report.py.

export const REPO = "https://github.com/Freeesta/mzlab";
/** longest link we open (characters, already percent-encoded); the rest of the report is only in «Copy report» */
export const URL_MAX = 6000;
/** the two kinds of report: the form of .github/ISSUE_TEMPLATE, its label (the form adds it for everyone; `labels=` only for people with triage rights), the title prefix and the id of the form field that gets the text */
export const TIPI = {
  bug: { template: "bug.yml", label: "bug", prefix: "[Bug] ", field: "cosa", schema: "mzlab-bug-report/1", keys: null },
  suggestion: { template: "suggestion.yml", label: "suggestion", prefix: "Suggestion: ", field: "idea", schema: "mzlab-suggestion/1", keys: ["version", "view", "lang"] },
};
const FILE_LIKE = /[^\s"'<>()[\]{}|,;]*\.(?:mzml|mzxml|raw|wiff2?|scan|d|qgd|lcdx?|baf|tdf|tsf|yep|peg|smp|mzdata|cdf|dam|mzlib|mzidx|mzfeatf?|csv|tsv|xlsx|json|zip|txt)\b/gi;

/** @param {string} s */
const esc = s => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** One line of an error text without names: the names of the loaded files (whole and without extension), file-like words, and what `pulisci` cuts (paths).
 * @param {unknown} text @param {string[]} [nomi] names to hide @param {(s: string) => string} [pulisci] DIAG.pulisci @param {number} [max] */
export function anonimizza(text, nomi = [], pulisci = s => s, max = 160) {
  let s = String(text == null ? "" : text).split(/\r?\n/)[0];
  const hide = new Set();
  for (const n of nomi) {
    const full = String(n || "").split("#")[0];
    if (full.length >= 3) hide.add(full);
    const base = full.replace(/\.[A-Za-z0-9]{1,8}$/, "");
    if (base.length >= 3) hide.add(base);
  }
  for (const h of [...hide].sort((a, b) => b.length - a.length)) s = s.replace(new RegExp(esc(h), "gi"), "<file>");
  s = pulisci(s).replace(FILE_LIKE, "<file>");
  return s.length > max ? s.slice(0, max) + "…" : s;
}

/** @param {number} n bytes */
export function dimensione(n) {
  if (!(n > 0)) return "0 B";
  if (n < 1024) return n + " B";
  if (n < 1048576) return (n / 1024).toFixed(1) + " KB";
  if (n < 1073741824) return (n / 1048576).toFixed(1) + " MB";
  return (n / 1073741824).toFixed(2) + " GB";
}

/** size class of a total, so a size never identifies a file @param {number} n bytes */
export function classeDimensione(n) {
  return !(n > 0) ? "0" : n < 1e7 ? "<10 MB" : n < 1e8 ? "10-100 MB" : n < 1e9 ? "100 MB-1 GB" : ">1 GB";
}

/** Number of loaded files, their formats (counts) and the size class of the total, without any name or exact size. @param {Array<{file?: string, size?: number, format?: string, fmt?: string}>} files */
export function riassuntoFile(files) {
  const fs = files || [], by = new Map();
  let total = 0;
  for (const f of fs) {
    const ext = (String(f.file || "").split("#")[0].match(/\.([A-Za-z0-9]{1,8})(?:\.scan)?$/) || [, ""])[1], k = String(f.format || f.fmt || ext || "?");
    by.set(k, (by.get(k) || 0) + 1); total += +(f.size || 0);
  }
  return `${fs.length}` + (fs.length ? ` (${[...by].map(([k, n]) => `${k} ×${n}`).join(", ")}; total ${classeDimensione(total)})` : "");
}

/** `Error.stack` lines as «function script.js:line:col»: a site script keeps its name, any other file becomes «<file>», the query and the hash of an URL are cut.
 * @param {unknown} stack @param {number} [max] frames kept */
export function telai(stack, max = 8) {
  const out = /** @type {string[]} */ ([]);
  for (const raw of String(stack == null ? "" : stack).split(/\r?\n/)) {
    const m = raw.match(/^\s*(?:at\s+)?(.*?)\s*[(@]?\s*((?:\w+:\/\/|blob:)\S*?)(?::(\d+):(\d+))?\)?\s*$/);
    if (!m || out.length >= max) continue;
    const base = m[2].replace(/[?#].*$/, "").split("/").pop() || "", name = /^[\w.-]+\.(?:m?js|wasm)$/.test(base) ? base : "<file>";
    out.push(`${(m[1] || "<anonymous>").replace(/^async\s+/, "").slice(0, 60)} ${name}${m[3] ? `:${m[3]}:${m[4]}` : ""}`);
  }
  return out;
}

/** The diagnostic FIELDS, in the order they are shown (fixed English labels, like DIAG.testo), each only if it has something to tell. `lines` is the text form, `value` the JSON form.
 * Every string goes through `anonimizza` with `nomi` (names of the open session); the page gives raw values, nothing here is read from the page.
 * @param {any} d the snapshot of bug-report-dati.js @param {string[]} [nomi] @param {(s: string) => string} [pulisci] @param {string[] | null} [only] keys wanted (all when null) */
export function campiDiagnostica(d, nomi = [], pulisci = s => s, only = null) {
  const a = (/** @type {unknown} */ x, max = 160) => anonimizza(x, nomi, pulisci, max), list = (/** @type {unknown} */ x) => (Array.isArray(x) ? x : []);
  /** @type {Array<{key: string, label: string, value: any, lines: string[]}>} */
  const F = [], add = (/** @type {string} */ key, /** @type {string} */ label, /** @type {any} */ value, /** @type {string[]} */ lines) => { if (!only || only.includes(key)) F.push({ key, label, value, lines }); };
  const one = (/** @type {string} */ key, /** @type {string} */ label, /** @type {unknown} */ v, max = 160) => { if (v != null && v !== "") { const s = a(v, max); add(key, label, s, [`- ${label}: ${s}`]); } };
  one("version", "mzLab", `${d.version || "?"} (${d.commit || "?"})`);
  one("date", "Date", d.date);
  one("ua", "Browser and system", d.ua, 220);
  one("mode", "Mode", d.mode);
  one("view", "View", d.view);
  one("lang", "Language", d.lang);
  if (d.screen) { const s = d.screen; one("screen", "Screen and window", `screen ${s.sw}x${s.sh}, window ${s.iw}x${s.ih}, pixel ratio ${s.dpr}`); }
  if (d.memory != null || d.cores != null) one("device", "Device", [d.memory != null ? `${d.memory} GB memory` : "", d.cores != null ? `${d.cores} cores` : ""].filter(Boolean).join(", "));
  if (d.isolated != null) one("isolated", "Cross-origin isolated", d.isolated ? "yes" : "no");
  if (d.opfs != null) one("opfs", "OPFS available", d.opfs ? "yes" : "no");
  one("engine", "Engine", d.engine);
  one("tool", "Active tool", d.tool);
  one("cards", "Open cards", d.cards);
  one("files", "Loaded files", Array.isArray(d.files) ? riassuntoFile(d.files) : d.files);
  const st = list(d.settings).map(x => a(x, 40)); if (d.settings) add("settings", "Changed settings", st, [`- Changed settings: ${st.join(", ") || "none"}`]);
  one("sw", "Service worker", d.sw);
  one("storage", "Stored state", d.storage);
  if (d.errors) {
    const es = list(d.errors).map(e => ({ where: a(e.where || "", 20), message: a(e.message, 160), frames: telai(e.stack).map(x => a(x, 120)) }));
    add("errors", "Recent errors", es, [es.length ? `- Recent errors (last ${es.length}):` : "- Recent errors: none", ...es.flatMap(e => [`  - ${e.where ? `[${e.where}] ` : ""}${e.message}`, ...e.frames.map(f => `    - ${f}`)])]);
  }
  if (d.actions) {
    // only the shapes of the static ids of the page (#kebab-case, tag[data-v=word], a tag): anything else (an id made from a name) is «<id>»
    const ac = list(d.actions).map(x => (/^(#[a-z][a-z-]{0,30}|[a-z]{1,10}(\[data-(?:v|a|tab)=[a-z-]{1,20}\])?)$/.test(String(x)) ? a(x, 60) : "<id>")); add("actions", "Recent actions", ac, [ac.length ? `- Recent actions (oldest first): ${ac.join(" > ")}` : "- Recent actions: none"]);
  }
  return F;
}

/** The lines of the diagnostics block of the fields still ticked. @param {ReturnType<typeof campiDiagnostica>} fields @param {Set<string>} [off] keys the user unticked */
export function righe(fields, off = new Set()) {
  return fields.filter(f => !off.has(f.key)).flatMap(f => f.lines);
}

/** The JSON of a report (the same fields, by key), for «Download full report» and for agents. @param {string} kind @param {string} what the description @param {string} doing what the user was doing
 * @param {ReturnType<typeof campiDiagnostica>} fields @param {Set<string>} [off] */
export function jsonRapporto(kind, what, doing, fields, off = new Set()) {
  const t = TIPI[/** @type {keyof typeof TIPI} */ (kind)] || TIPI.bug;
  return JSON.stringify({ schema: t.schema, kind, what: (what || "").trim(), ...(kind === "suggestion" ? { doing: (doing || "").trim() } : {}), fields: Object.fromEntries(fields.filter(f => !off.has(f.key)).map(f => [f.key, f.value])) }, null, 2);
}

/** The Markdown report with FIXED headings: the description, for a suggestion «What I was doing», and the diagnostics.
 * @param {{what: string, doing?: string, diag: string[] | null}} r @param {(k: string) => string} t */
export function rapporto(r, t) {
  const L = [`## ${t(r.doing !== undefined ? "sug.r.what" : "bug.r.what")}`, "", (r.what || "").trim() || t("bug.r.none")];
  if (r.doing !== undefined) L.push("", `## ${t("sug.r.doing")}`, "", (r.doing || "").trim() || t("bug.r.none"));
  if (r.diag && r.diag.length) L.push("", `## ${t("bug.r.diag")}`, "", ...r.diag);
  return L.join("\n");
}

/** Title of the issue: the prefix of the kind («[Bug] », «Suggestion: ») + the first line of the answer, short. @param {string} what @param {string} empty @param {string} [prefix] */
export function titolo(what, empty, prefix = TIPI.bug.prefix) {
  const l = ((what || "").trim().split(/\r?\n/)[0] || "").replace(/\s+/g, " ");
  const s = l.length > 70 ? l.slice(0, 70).replace(/\s\S*$/, "") + "…" : l;
  return prefix + (s || empty);
}

/** The link that opens a new issue of the public repository with the form of its kind filled in (title, label, the report in the main field, and for a bug the browser and system), at most `URL_MAX`
 * characters: if the report is longer it is cut (and says so), and `troncato` is true. @param {{kind?: string, title: string, body: string, ambiente?: string, cut: string}} p */
export function urlIssue(p) {
  const t = TIPI[/** @type {keyof typeof TIPI} */ (p.kind || "bug")] || TIPI.bug;
  const mk = (/** @type {string} */ body) => `${REPO}/issues/new?template=${t.template}&labels=${t.label}&title=${encodeURIComponent(p.title)}${p.ambiente != null ? `&ambiente=${encodeURIComponent(p.ambiente.slice(0, 200))}` : ""}&${t.field}=${encodeURIComponent(body)}`;
  let url = mk(p.body);
  if (url.length <= URL_MAX) return { url, troncato: false };
  // never cut a pair of surrogates in two: encodeURIComponent would throw
  const cutAt = (/** @type {number} */ n) => { const h = p.body.slice(0, n); return /[\uD800-\uDBFF]$/.test(h) ? h.slice(0, -1) : h; };
  let lo = 0, hi = p.body.length;
  while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (mk(cutAt(mid) + p.cut).length <= URL_MAX) lo = mid; else hi = mid - 1; }
  url = mk(cutAt(lo) + p.cut);
  return { url, troncato: true };
}

/** longest mailto link (characters, already percent-encoded): many mail programs refuse longer ones */
export const MAIL_MAX = 1900;

/** The mailto link with the report as body, cut like the GitHub link when it is too long. The address comes in as two parts and is joined here, so it is never written whole in the page.
 * @param {{user: string, host: string, subject: string, body: string, cut: string}} p */
export function urlEmail(p) {
  const to = p.user + "@" + p.host;
  const mk = (/** @type {string} */ body) => `mailto:${to}?subject=${encodeURIComponent(p.subject)}&body=${encodeURIComponent(body)}`;
  let url = mk(p.body);
  if (url.length <= MAIL_MAX) return { url, troncato: false };
  const cutAt = (/** @type {number} */ n) => { const h = p.body.slice(0, n); return /[\uD800-\uDBFF]$/.test(h) ? h.slice(0, -1) : h; };
  let lo = 0, hi = p.body.length;
  while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (mk(cutAt(mid) + p.cut).length <= MAIL_MAX) lo = mid; else hi = mid - 1; }
  url = mk(cutAt(lo) + p.cut);
  return { url, troncato: true };
}
