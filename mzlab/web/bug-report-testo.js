// @ts-check
// The text of a bug report, no DOM and no network: what the dialog of bug-report.js shows, copies and puts in the link to a new GitHub issue.
// Privacy rule: the diagnostics hold only counts, formats and sizes of the loaded files, never a file name, a sample label or a path; the recent errors are cut to
// one line, with every name of the loaded files and every file-like word replaced by «<file>». The user's own two answers are the only free text.
// Run by node in tests/test_bug_report.py.

export const REPO = "https://github.com/Freeesta/mzlab";
/** longest link we open (characters, already percent-encoded); the rest of the report is only in «Copy report» */
export const URL_MAX = 6000;
const FILE_LIKE = /[^\s"'<>()[\]{}|,;]*\.(?:mzml|mzxml|raw|wiff2?|scan|d|qgd|lcd|cdf|dam|mzlib|mzidx|mzfeatf?|csv|tsv|xlsx|json|zip|txt)\b/gi;

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

/** Format and size of each loaded file, in the order of the list, without any name. @param {Array<{file?: string, size?: number, format?: string, fmt?: string}>} files */
export function riassuntoFile(files) {
  const rows = (files || []).slice(0, 20).map(f => {
    const ext = (String(f.file || "").split("#")[0].match(/\.([A-Za-z0-9]{1,8})(?:\.scan)?$/) || [, ""])[1];
    return `${f.format || f.fmt || ext || "?"} ${dimensione(+(f.size || 0))}`;
  });
  const more = (files || []).length - rows.length;
  return `${(files || []).length}` + (rows.length ? ` (${rows.join(", ")}${more > 0 ? `, +${more}` : ""})` : "");
}

/** The lines of the diagnostics block (fixed English labels, like DIAG.testo).
 * @param {{version?: string, commit?: string, date: string, ua: string, mode: string, view: string, files: string, sw: string, errors: string[]}} d */
export function righeDiagnostica(d) {
  const L = [`- mzLab: ${d.version || "?"} (${d.commit || "?"})`, `- Date: ${d.date}`, `- Browser and system: ${d.ua}`, `- Mode: ${d.mode}`, `- View: ${d.view}`, `- Loaded files: ${d.files}`, `- Service worker: ${d.sw}`];
  L.push(d.errors.length ? "- Recent errors:" : "- Recent errors: none");
  for (const e of d.errors) L.push(`  - ${e}`);
  return L;
}

/** The Markdown report. @param {{what: string, expected: string, diag: ReturnType<typeof righeDiagnostica> | null}} r
 * @param {(k: string) => string} t */
export function rapporto(r, t) {
  const none = t("bug.r.none");
  const L = [`## ${t("bug.r.what")}`, "", (r.what || "").trim() || none, "", `## ${t("bug.r.expected")}`, "", (r.expected || "").trim() || none];
  if (r.diag) L.push("", `## ${t("bug.r.diag")}`, "", ...r.diag);
  return L.join("\n");
}

/** Title of the issue: «[Bug] » + the first line of the answer, short. @param {string} what @param {string} empty */
export function titolo(what, empty) {
  const l = ((what || "").trim().split(/\r?\n/)[0] || "").replace(/\s+/g, " ");
  const s = l.length > 70 ? l.slice(0, 70).replace(/\s\S*$/, "") + "…" : l;
  return "[Bug] " + (s || empty);
}

/** The link that opens a new issue of the public repository with the form «bug.yml» filled in (title, label «bug», the report in the first field, browser and system), at most `URL_MAX`
 * characters: if the report is longer it is cut (and says so), and `troncato` is true. @param {{title: string, body: string, ambiente: string, cut: string}} p */
export function urlIssue(p) {
  const mk = (/** @type {string} */ body) => `${REPO}/issues/new?template=bug.yml&labels=bug&title=${encodeURIComponent(p.title)}&ambiente=${encodeURIComponent(p.ambiente.slice(0, 200))}&cosa=${encodeURIComponent(body)}`;
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
