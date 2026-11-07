// @ts-check
// Room for a batch of files in the browser's storage, loaded on first use by shell.js upload0. The browser copies every file, with its index, into the
// share of free disk it gives this site (about ROOM times the file while it loads; not the RAM; a plain mzML that does not fit is read in place, its index alone): a batch bigger than that asks once before copying
// (`prima`), and the files refused for lack of room make ONE message with the count loaded / not loaded and «Retry the rest» (`nota`, `wire`).
const ROOM = 1.8, MB = 1048576;
const W = /** @type {any} */ (window);
/** @type {{ files: any[], into: string | undefined }} */
let last = { files: [], into: undefined };
const human = (/** @type {number} */ b) => b >= 1024 * MB ? `${(b / 1024 / MB).toFixed(1)} GB` : `${Math.ceil(b / MB)} MB`;
const isData = (/** @type {any} */ f) => !isMethodFile(f.name) && !fileKind(f.name) && !CartCarica.byproduct(f.name);
// a plain mzML with no room for its copy is read in place from the disk (rust-sul-posto.js): only its index takes room
const inPlace = (/** @type {any} */ f) => /\.mzml$/i.test(f.name) && !f.mzlabGz && !f.mzlabNat && f instanceof File && f.size >= 50 * MB;
async function room() {
  try { const e = await navigator.storage.estimate(); return e.quota ? Math.max(0, e.quota - (e.usage || 0)) : Infinity; } catch (_) { return Infinity; }
}
/** the dialog of the question, made once */
function dialog() {
  let d = /** @type {HTMLDialogElement | null} */ (document.getElementById("quotadlg"));
  if (d) return d;
  d = document.createElement("dialog"); d.id = "quotadlg"; d.setAttribute("aria-labelledby", "quotatxt");
  d.innerHTML = `<p id="quotatxt"></p><p class="muted" id="quotawhy"></p><div class="row"><button type="button" class="sec" data-q="cancel"></button><button type="button" class="sec" data-q="all"></button><button type="button" class="go" data-q="first"></button></div>`;
  document.body.appendChild(d);
  return d;
}
/** before copying: the files of this batch that go ahead (all, the first ones that fit, or none); the data files left out go to rep.quota.left */
async function prima(/** @type {any[]} */ files, /** @type {CaricaRep} */ rep, /** @type {string | undefined} */ into) {
  const data = files.filter(isData), free = await room();
  const need = data.reduce((s, f) => s + NATIVO.size(f) * ROOM, 0);
  if (need <= free) return files;
  const share = (/** @type {any} */ f) => NATIVO.size(f) * (inPlace(f) ? ROOM - 1 : ROOM);
  const least = data.reduce((s, f) => s + share(f), 0);
  if (least <= free) return files;
  let k = 0, s = 0;
  for (const f of data) { s += share(f); if (s > free) break; k++; }
  const T = I18N.t, d = dialog(), b = (/** @type {string} */ q) => /** @type {HTMLButtonElement} */ (d.querySelector(`[data-q=${q}]`));
  /** @type {HTMLElement} */ (d.querySelector("#quotatxt")).textContent = T("load.quota.ask", { n: data.length, serve: human(least), libero: human(free), k });
  /** @type {HTMLElement} */ (d.querySelector("#quotawhy")).textContent = T("load.quota.why");
  b("cancel").textContent = T("common.cancel"); b("all").textContent = T("load.quota.all"); b("first").textContent = T("load.quota.first", { k });
  b("first").hidden = k === 0;
  const pick = await new Promise(res => {
    const done = (/** @type {string} */ v) => { d.oncancel = null; d.close(); res(v); };
    for (const q of ["cancel", "all", "first"]) b(q).onclick = () => done(q);
    d.oncancel = ev => { ev.preventDefault(); done("cancel"); };
    d.showModal(); (k ? b("first") : b("cancel")).focus();
  });
  if (pick === "all") return files;
  if (pick === "cancel") return [];
  const out = data.slice(k);
  rep.quota = { ok: 0, left: out, into };
  return files.filter(f => !out.includes(f));
}
/** a file refused for lack of room while it loaded: no error of its own, it counts in the one message of the batch; `msg` (the MB written, needed and free) is shown when it is the only one */
function refused(/** @type {CaricaRep} */ rep, /** @type {any} */ f, /** @type {string | undefined} */ into, /** @type {string} */ key, /** @type {string} */ msg) {
  const q = rep.quota || (rep.quota = { ok: 0, left: [], into });
  q.left.push(f); q.msg = msg; if (key === "error.quotaWrite") q.write = true;
}
/** the message of a batch that ran out of room (HTML for reportLoad) */
function nota(/** @type {CaricaRep} */ rep, /** @type {(s: string) => string} */ esc) {
  const T = I18N.t, q = rep.quota, names = q.left.map((/** @type {any} */ f) => f.name);
  last = { files: q.left, into: q.into };
  return `<div class="quota-note" role="status"><p><b>${esc(T("load.quota.note", { ok: rep.ok || 0, no: names.length }))}</b> ${esc(names.length === 1 && q.msg ? q.msg : T(q.write ? "load.quota.write" : "load.quota.why"))}</p>`
    + `<details class="unopen-names"><summary>${esc(T("unopen.names"))}</summary><ul>${names.slice(0, 5).map((/** @type {string} */ n) => `<li>${esc(n)}</li>`).join("")}${names.length > 5 ? `<li class="muted">${esc(T("unopen.more", { n: names.length - 5 }))}</li>` : ""}</ul></details>`
    + `<div class="row"><button type="button" class="sec" data-a="quota-retry">${esc(T("load.quota.retry"))}</button></div></div>`;
}
/** «Retry the rest»: the files left out go in again (after the student removed some, the question tells how many fit now) */
function wire(/** @type {HTMLElement} */ u) {
  const x = /** @type {HTMLElement | null} */ (u.querySelector("[data-a=quota-retry]"));
  if (x) x.onclick = () => { const { files, into } = last; closeUnopen(); upload(files, into); };
}
/** @type {any} */ (window).QUOTALOTTO = { prima, refused, nota, wire };
