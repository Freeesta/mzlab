// @ts-check
// Teoria editor (administrator page): edit the Markdown source of a Teoria chapter, check it with the Rust converter, preview the page and send the change
// as one pull request with auto-merge (pr.js). Text and token live in memory; the token may be sealed with a passphrase (vault.js, own slot).
import { GhError } from "./gh.js";
import { BRANCH_PREFIX, headings, makePrClient } from "./pr.js";
import * as vault from "./vault.js";
import { loadConverter } from "./wasm.js";

const SLOT = "mzlab.libro.teoria.token";
/** @param {string} id */
const $ = (id) => /** @type {any} */ (document.getElementById(id));

/** @type {ReturnType<typeof makePrClient> | null} */
let client = null;
/** @type {import("./wasm.js").Converter | null} */
let conv = null;
/** @type {{name: string, md: {text: string, sha: string}, html: {text: string, sha: string}, rawOk: boolean} | null} */
let cur = null;

/** @param {string} t @param {boolean} [err] */
const say = (t, err = false) => {
  $("status").textContent = t;
  $("status").classList.toggle("err", err);
};
/** @param {string} t */
const fail = (t) => {
  $("msg").textContent = t;
};

/** @param {unknown} e */
function explain(e) {
  if (e instanceof GhError) return e.message;
  return e instanceof Error ? e.message : String(e);
}

/** Convert the text in the box; the error text (file:line: message) when the converter refuses it. */
function convert() {
  if (!conv || !cur) return { error: "The converter is not loaded." };
  try {
    return { html: conv.convert($("source").value, cur.name, cur.rawOk) };
  } catch (e) {
    return { error: String(e) };
  }
}

function refresh() {
  const dirty = !!cur && $("source").value !== cur.md.text;
  $("btn-preview").disabled = !cur;
  $("btn-send").disabled = !cur || !dirty || !$("summary").value.trim();
}

async function connect(/** @type {string} */ token) {
  client = makePrClient({ token });
  say("Connecting…");
  try {
    const [names, c] = await Promise.all([client.chapters(), loadConverter()]);
    conv = c;
    const nav = $("sections");
    nav.replaceChildren();
    for (const n of names) {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = n.replace(/\.md$/, "");
      b.addEventListener("click", () => openChapter(n, b));
      nav.append(b);
    }
    say(`Connected · converter ${c.version}`);
    showOpen();
  } catch (e) {
    client = null;
    say(explain(e), true);
  }
}

async function showOpen() {
  if (!client) return;
  try {
    const open = await client.openPrs();
    $("result").textContent = open.length ? `Pull request #${open[0].number} of the editor is open: ${open[0].url}` : "";
  } catch (_) {
    /* the list is only a hint; submit() checks again */
  }
}

/** @param {string} name @param {HTMLElement} btn */
async function openChapter(name, btn) {
  if (!client || !conv) return;
  if (cur && $("source").value !== cur.md.text && !confirm("Discard your changes to this chapter?")) return;
  fail("");
  try {
    const l = await client.load(name);
    // the converter must reproduce the page on main from the source on main, or the page it makes for an edit would carry unrelated changes
    let same = false;
    try {
      same = conv.convert(l.md.text, name, l.rawOk) === l.html.text;
    } catch (_) {
      same = false;
    }
    if (!same) return fail(`The converter does not reproduce ${name}.html from its source: not editable here.`);
    cur = { name, md: l.md, html: l.html, rawOk: l.rawOk };
    $("source").value = l.md.text;
    $("source").disabled = false;
    $("summary").value = "";
    $("path").textContent = `mzlab/web/teoria/src/${name}`;
    $("preview").hidden = true;
    for (const b of document.querySelectorAll("#sections button")) b.removeAttribute("aria-current");
    btn.setAttribute("aria-current", "true");
    refresh();
  } catch (e) {
    fail(explain(e));
  }
}

function preview() {
  const r = convert();
  if (r.error) return fail(r.error);
  fail("");
  // the page runs no script here (the sandbox has no allow-scripts): it shows the text and the styles; <base> makes teoria.css load from the Teoria folder
  const frame = $("preview");
  frame.srcdoc = /** @type {string} */ (r.html).replace("<head>", '<head><base href="../teoria/">');
  frame.hidden = false;
}

async function send() {
  if (!client || !cur) return;
  fail("");
  const r = convert();
  if (r.error) return fail(r.error);
  if (JSON.stringify(headings($("source").value)) !== JSON.stringify(headings(cur.md.text))) {
    return fail("The headings changed. Titles and sections feed the search index, which this page cannot rebuild: change headings with a normal pull request.");
  }
  $("btn-send").disabled = true;
  say("Sending…");
  try {
    const pr = await client.submit({ name: cur.name, md: $("source").value, html: /** @type {string} */ (r.html), loaded: { md: cur.md, html: cur.html }, summary: $("summary").value.trim() });
    cur.md = { text: $("source").value, sha: "" };
    $("source").disabled = true;
    $("btn-preview").disabled = true;
    $("result").textContent = pr.autoMerge
      ? `Pull request #${pr.number} opened and set to merge by itself when the checks are green: ${pr.url}`
      : `Pull request #${pr.number} opened, but auto-merge could not be switched on (is it allowed in the repository settings?). Merge it by hand: ${pr.url}`;
    say("Sent");
  } catch (e) {
    say("Not sent", true);
    fail(explain(e));
    refresh();
  }
}

/** @param {string} token */
function useToken(token) {
  $("dlg-token").close?.();
  $("dlg-unlock").close?.();
  connect(token);
}

function wire() {
  $("source").addEventListener("input", refresh);
  $("summary").addEventListener("input", refresh);
  $("btn-preview").addEventListener("click", preview);
  $("btn-send").addEventListener("click", send);
  $("btn-token").addEventListener("click", () => $("dlg-token").showModal());
  $("t-close").addEventListener("click", () => $("dlg-token").close());
  $("t-tab").addEventListener("click", () => {
    const t = $("in-token").value.trim();
    if (!t) return ($("t-msg").textContent = "Paste the token first.");
    vault.keepForTab(t, SLOT);
    useToken(t);
  });
  $("t-seal").addEventListener("click", async () => {
    const t = $("in-token").value.trim();
    const p = $("in-pass").value;
    if (!t || !p) return ($("t-msg").textContent = "Paste the token and invent a passphrase.");
    if (!(await vault.seal(t, p, SLOT))) $("t-msg").textContent = "This browser refuses to store it; using it for this tab only.";
    useToken(t);
  });
  $("t-forget").addEventListener("click", () => {
    vault.forget(SLOT);
    client = null;
    say("Not connected");
    $("t-msg").textContent = "Forgotten.";
  });
  $("u-go").addEventListener("click", async () => {
    const t = await vault.unseal($("in-unlock").value, SLOT);
    if (!t) return ($("u-msg").textContent = "Wrong passphrase.");
    useToken(t);
  });
  $("u-other").addEventListener("click", () => {
    $("dlg-unlock").close();
    $("dlg-token").showModal();
  });
}

wire();
const tab = vault.tabToken(SLOT);
if (tab) connect(tab);
else if (vault.hasSealed(SLOT)) $("dlg-unlock").showModal();
else $("dlg-token").showModal();
// the branches of this editor all start with this prefix; shown so a failed run can be found on GitHub
$("path").title = `Branches: ${BRANCH_PREFIX}…`;
