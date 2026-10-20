// @ts-check
// Admin page of the Libro: visual editor (Milkdown) for the sections of the private repository, saved as Markdown with the GitHub API.
// Nothing of the Libro is part of the site: the text is fetched at run time with the user's token and kept in memory only.
import { GhError, makeClient, trio } from "./gh.js";
import * as vault from "./vault.js";
import { normalize } from "./norm.js";

const T = {
  idle: "Not connected", loading: "Loading…", saved: "All changes saved", dirty: "Unsaved changes", saving: "Saving…", conflict: "File changed by someone else",
  empty: "Nothing here yet. Write something and save: the file will be created.", noHistory: "No earlier versions.", wrongPass: "Wrong passphrase.",
  needToken: "Paste the token first.", needPass: "Choose a passphrase to keep the token, or use it for this tab only.", noStore: "This browser cannot keep the token. It works for this tab only.",
  leave: "You have unsaved changes. Leave without saving?",
};
const TABS = /** @type {const} */ (["text", "notes", "qa"]);
const LABEL = { text: "text", notes: "notes", qa: "Q&A" };

/** @param {string} id */
const $ = (id) => /** @type {HTMLElement} */ (document.getElementById(id));
/** @param {string} id */
const $i = (id) => /** @type {HTMLInputElement} */ (document.getElementById(id));
/** @param {string} id */
const $d = (id) => /** @type {HTMLDialogElement} */ (document.getElementById(id));

/** @type {{client: ReturnType<typeof makeClient>|null, current: string, tab: "text"|"notes"|"qa", sha: string, base: string, dirty: boolean, builder: any, mine: string}} */
const S = { client: null, current: "", tab: "text", sha: "", base: "", dirty: false, builder: null, mine: "" };

/** @param {string} msg @param {boolean} [dirty] */
function status(msg, dirty = false) {
  const s = $("status");
  s.textContent = msg;
  s.classList.toggle("dirty", dirty);
}

function setDirty(/** @type {boolean} */ d) {
  S.dirty = d;
  $("btn-save").toggleAttribute("disabled", !d || !S.client);
  status(d ? T.dirty : T.saved, d);
}

/** Mount the visual editor on `md`; `base` is the text the file has on GitHub, or null for "what the editor makes of md" (to know when there is something to save). @param {string} md @param {string|null} base */
async function mount(md, base) {
  if (S.builder) await S.builder.destroy();
  $("root").textContent = "";
  const { CrepeBuilder, features: F } = /** @type {any} */ (window).LibroEditor;
  const b = new CrepeBuilder({ root: $("root"), defaultValue: md });
  b.addFeature(F.listItem).addFeature(F.table).addFeature(F.codeMirror).addFeature(F.latex).addFeature(F.linkTooltip).addFeature(F.cursor);
  b.addFeature(F.toolbar).addFeature(F.blockEdit).addFeature(F.placeholder, { text: "Write here, or type / for the menu" });
  await b.create();
  S.builder = b;
  S.mine = normalize(b.getMarkdown());
  S.base = base ?? S.mine;
  b.on((/** @type {any} */ api) => api.markdownUpdated((/** @type {any} */ _c, /** @type {string} */ m) => { S.mine = normalize(m); setDirty(S.mine !== S.base); }));
  setDirty(false);
}

/** @param {"text"|"notes"|"qa"} tab */
async function loadTab(tab) {
  if (!S.client || !S.current) return;
  if (S.dirty && !confirm(T.leave)) return;
  S.tab = tab;
  for (const t of TABS) $("tab-" + t).setAttribute("aria-selected", String(t === tab));
  const path = trio(S.current)[tab];
  $("path").textContent = path;
  status(T.loading);
  try {
    const f = await S.client.read(path);
    S.sha = f ? f.sha : "";
    await mount(f ? f.text : "", null);
    $("btn-history").toggleAttribute("disabled", !f);
    if (!f) $("path").textContent = `${path} — ${T.empty}`;
    closeHistory();
  } catch (e) {
    fail(e);
  }
}

/** @param {unknown} e */
function fail(e) {
  if (e instanceof GhError && e.kind === "auth") {
    S.client = null;
    status(e.message);
    openToken(e.message);
  } else status(e instanceof Error ? e.message : String(e));
}

async function openSection(/** @type {string} */ p) {
  if (S.dirty && !confirm(T.leave)) return;
  S.dirty = false;
  S.current = p;
  for (const b of document.querySelectorAll("#sections button")) b.setAttribute("aria-current", String(/** @type {HTMLElement} */ (b).dataset.p === p));
  await loadTab("text");
}

/** @param {string[]} paths */
function renderSections(paths) {
  const nav = $("sections");
  nav.textContent = "";
  /** @type {Map<string, string[]>} */
  const groups = new Map();
  for (const p of paths) {
    const dir = p.includes("/") ? p.slice(0, p.lastIndexOf("/")) : "";
    groups.set(dir, [...(groups.get(dir) || []), p]);
  }
  for (const [dir, list] of groups) {
    const h = document.createElement("h2");
    h.textContent = dir || "Libro";
    nav.append(h);
    for (const p of list) {
      const b = document.createElement("button");
      b.type = "button";
      b.dataset.p = p;
      b.textContent = p.slice(p.lastIndexOf("/") + 1).replace(/\.md$/, "").replace(/_/g, " ");
      b.addEventListener("click", () => openSection(p));
      nav.append(b);
    }
  }
}

/** @param {string} token */
async function connect(token) {
  S.client = makeClient({ token });
  status(T.loading);
  try {
    renderSections(await S.client.sections());
    status("Connected");
  } catch (e) {
    S.client = null;
    fail(e);
  }
}

async function save() {
  // the editor reports changes after a short pause: ask it directly so a quick Ctrl+S does not miss the last words
  if (S.builder) S.mine = normalize(S.builder.getMarkdown());
  if (!S.client || S.mine === S.base) return;
  const text = S.mine;
  const path = trio(S.current)[S.tab];
  status(T.saving);
  try {
    const r = await S.client.write(path, text, S.sha, `Libro ${S.current.replace(/\.md$/, "")}: edit ${LABEL[S.tab]} from the editor`);
    S.sha = r.sha;
    S.base = text;
    setDirty(false);
    $("btn-history").removeAttribute("disabled");
  } catch (e) {
    if (e instanceof GhError && e.kind === "conflict") {
      status(T.conflict, true);
      $i("c-mine").value = text;
      $d("dlg-conflict").showModal();
    } else fail(e);
  }
}

async function showHistory() {
  const box = $("history");
  if (!box.hidden) return closeHistory();
  box.hidden = false;
  box.textContent = "Loading…";
  try {
    const list = await /** @type {NonNullable<typeof S.client>} */ (S.client).history(trio(S.current)[S.tab]);
    box.textContent = "";
    const h = document.createElement("h2");
    h.textContent = "Earlier versions";
    const ul = document.createElement("ul");
    for (const c of list) {
      const li = document.createElement("li");
      const when = document.createElement("div");
      when.className = "when";
      when.textContent = new Date(c.date).toLocaleString();
      const msg = document.createElement("div");
      msg.textContent = c.message;
      const load = document.createElement("button");
      load.type = "button";
      load.textContent = "Load this version";
      load.addEventListener("click", () => loadOld(c.sha));
      const a = document.createElement("a");
      a.href = c.url;
      a.target = "_blank";
      a.rel = "noopener noreferrer";
      a.textContent = " Open on GitHub";
      li.append(when, msg, load, a);
      ul.append(li);
    }
    box.append(h, list.length ? ul : Object.assign(document.createElement("p"), { textContent: T.noHistory }));
  } catch (e) {
    fail(e);
  }
}

function closeHistory() {
  const box = $("history");
  box.hidden = true;
  box.textContent = "";
}

/** Put an older version in the editor; it is saved (as a new version) only if the user presses Save. @param {string} sha */
async function loadOld(sha) {
  if (!S.client) return;
  try {
    const f = await S.client.read(trio(S.current)[S.tab], sha);
    if (!f) return;
    await mount(f.text, S.base);
    setDirty(true);
  } catch (e) {
    fail(e);
  }
}

/** @param {string} [msg] */
function openToken(msg = "") {
  $("t-msg").textContent = msg;
  $i("in-token").value = "";
  $i("in-pass").value = "";
  $d("dlg-token").showModal();
  $i("in-token").focus();
}

async function useToken(/** @type {boolean} */ withPass) {
  const token = $i("in-token").value.trim();
  const pass = $i("in-pass").value;
  if (!token) return void ($("t-msg").textContent = T.needToken);
  if (withPass && !pass) return void ($("t-msg").textContent = T.needPass);
  const stored = withPass ? await vault.seal(token, pass) : vault.keepForTab(token);
  if (!stored) $("t-msg").textContent = T.noStore;
  $i("in-token").value = $i("in-pass").value = "";
  $d("dlg-token").close();
  await connect(token);
}

async function unlock() {
  const token = await vault.unseal($i("in-unlock").value);
  if (!token) return void ($("u-msg").textContent = T.wrongPass);
  $i("in-unlock").value = "";
  $d("dlg-unlock").close();
  await connect(token);
}

function bind() {
  $("btn-save").addEventListener("click", save);
  $("btn-history").addEventListener("click", showHistory);
  $("btn-token").addEventListener("click", () => openToken());
  for (const t of TABS) $("tab-" + t).addEventListener("click", () => loadTab(t));
  $("t-seal").addEventListener("click", () => useToken(true));
  $("t-tab").addEventListener("click", () => useToken(false));
  $("t-forget").addEventListener("click", () => { vault.forget(); S.client = null; status(T.idle); $d("dlg-token").close(); });
  $("t-close").addEventListener("click", () => $d("dlg-token").close());
  $("u-go").addEventListener("click", unlock);
  $("u-other").addEventListener("click", () => { $d("dlg-unlock").close(); openToken(); });
  $("c-copy").addEventListener("click", () => navigator.clipboard.writeText($i("c-mine").value).catch(() => $i("c-mine").select()));
  $("c-keep").addEventListener("click", () => $d("dlg-conflict").close());
  $("c-load").addEventListener("click", async () => { $d("dlg-conflict").close(); S.dirty = false; await loadTab(S.tab); });
  document.addEventListener("keydown", (e) => { if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); save(); } });
  addEventListener("beforeunload", (e) => { if (S.dirty) e.preventDefault(); });
}

bind();
const tab = vault.tabToken();
if (tab) connect(tab);
else if (vault.hasSealed()) { $d("dlg-unlock").showModal(); $i("in-unlock").focus(); }
else openToken();
