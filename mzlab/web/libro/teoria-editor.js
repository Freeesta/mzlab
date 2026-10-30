// @ts-strict
// Teoria editor (administrator page): edit the Markdown source of a Teoria chapter, check it with the Rust converter, preview the page and send the change
// as one pull request with auto-merge (pr.js). Text and token live in memory; the token may be sealed with a passphrase (vault.js, own slot).
import { GhError } from "./gh.js";
import { BRANCH_PREFIX, makePrClient } from "./pr.js";
import * as vault from "./vault.js";
import { clearDraft, loadDraft, makeHistory, saveDraft } from "./storia.js";
import { loadConverter } from "./wasm.js";

const SLOT = "mzlab.libro.teoria.token";
/** @param {string} id */
const $ = (id) => /** @type {any} */ (document.getElementById(id));

/** @type {ReturnType<typeof makePrClient> | null} */
let client = null;
/** @type {import("./wasm.js").Converter | null} */
let conv = null;
/** @type {{name: string, md: {text: string, sha: string}, html: {text: string, sha: string}, index: {text: string, sha: string}, rawOk: boolean} | null} */
let cur = null;
/** @type {ReturnType<typeof makeHistory> | null} */
let hist = null;
/** @type {ReturnType<typeof setTimeout> | undefined} */
let commitTimer;

/** @param {string} t @param {boolean} [err] */
const say = (t, err = false) => {
  $("status").textContent = t;
  $("status").classList.toggle("err", err);
};
/** @param {string} t */
const fail = (t) => {
  $("msg").textContent = t;
};

const NOT_TOKEN = "This is not a token: it must start with github_pat_";
/** A fine-grained token starts with github_pat_, a classic one with ghp_. @param {string} t */
const looksLikeToken = (t) => /^(github_pat_|ghp_)\S+$/.test(t);

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
  $("btn-undo").disabled = !hist || !hist.canUndo();
  $("btn-redo").disabled = !hist || !hist.canRedo();
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
    say("Connected");
    $("status").title = `Converter ${c.version}`;
    showOpen();
    const first = nav.querySelector("button");
    if (first) openChapter(names[0], first);
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
    cur = { name, md: l.md, html: l.html, index: l.index, rawOk: l.rawOk };
    $("source").value = l.md.text;
    $("source").disabled = false;
    $("summary").value = "";
    hist = makeHistory(l.md.text);
    offerDraft(name, l.md);
    $("path").textContent = `mzlab/web/teoria/src/${name}`;
    preview();
    for (const b of document.querySelectorAll("#sections button")) b.removeAttribute("aria-current");
    btn.setAttribute("aria-current", "true");
    refresh();
  } catch (e) {
    fail(explain(e));
  }
}

/** The source the preview on screen was made from: an edit in the page goes back into exactly this text. */
let rendered = "";
/** @type {ReturnType<typeof setTimeout> | undefined} */
let timer;
/** Scroll position of the page before it was made again, put back when the new one has loaded. */
let pendingTop = 0;

const EDIT_CSS = "[data-src]{cursor:text}[data-src]:hover{outline:1px dashed #0b5cad;outline-offset:2px}[data-src][contenteditable=true]{outline:2px solid #0b5cad;outline-offset:2px}a{pointer-events:none}";

function preview() {
  if (!conv || !cur) return;
  const text = $("source").value;
  let page;
  try {
    page = conv.preview(text, cur.name, cur.rawOk);
  } catch (e) {
    return fail(String(e));
  }
  fail("");
  rendered = text;
  const frame = $("preview");
  pendingTop = frame.contentDocument?.scrollingElement?.scrollTop || 0;
  // the page runs no script here (the sandbox has no allow-scripts); <base> makes teoria.css load from the Teoria folder
  // questions are shown open so the answer can be edited too
  frame.srcdoc = page.replace("<head>", '<head><base href="../teoria/">').replaceAll('<details class="q">', '<details class="q" open>');
  frame.hidden = false;
}

/** @param {HTMLIFrameElement} frame @param {number} top */
function wirePreview(frame, top) {
  const doc = frame.contentDocument;
  if (!doc) return;
  const st = doc.createElement("style");
  st.textContent = EDIT_CSS;
  doc.head.append(st);
  if (doc.scrollingElement) doc.scrollingElement.scrollTop = top;
  /** @type {{el: HTMLElement, html: string} | null} */
  let editing = null;
  /** @param {boolean} keep */
  const finish = (keep) => {
    if (!editing) return;
    const { el, html } = editing;
    editing = null;
    el.contentEditable = "false";
    if (keep && el.innerHTML !== html) applyEdit(el.getAttribute("data-src") || "", el.innerHTML);
    else if (!keep) el.innerHTML = html;
  };
  doc.addEventListener("click", (ev) => {
    const t = /** @type {HTMLElement} */ (ev.target);
    ev.preventDefault();
    const el = t.closest?.("[data-src]");
    if (el) {
      if (editing && editing.el === el) return;
      finish(true);
      editing = { el: /** @type {HTMLElement} */ (el), html: el.innerHTML };
      editing.el.contentEditable = "true";
      editing.el.focus();
    } else if (t.closest?.("main")) {
      finish(true);
      fail("This block cannot be changed in the page: edit it in the source box below.");
    }
  });
  doc.addEventListener("keydown", (ev) => {
    if (!editing) return;
    if (ev.key === "Enter") {
      ev.preventDefault();
      finish(true);
    } else if (ev.key === "Escape") {
      ev.preventDefault();
      finish(false);
    } else if (ev.key === " " && editing.el.tagName === "SUMMARY") {
      // a space in a summary would fold it instead of being typed
      ev.preventDefault();
      doc.execCommand("insertText", false, " ");
    }
  });
  // a space typed in the summary of a question would fold it
  doc.addEventListener("keyup", (ev) => {
    if (editing && ev.key === " ") ev.preventDefault();
  });
  doc.addEventListener("paste", (ev) => {
    if (!editing) return;
    ev.preventDefault();
    doc.execCommand("insertText", false, (ev.clipboardData?.getData("text/plain") || "").replace(/\s+/g, " "));
  });
  doc.addEventListener("focusout", () => finish(true));
  for (const n of ["drop", "dragstart"]) doc.addEventListener(n, (ev) => ev.preventDefault());
}

/** An edit made in the page: the Rust converter puts it back into the source, or says why it cannot. @param {string} src @param {string} html */
function applyEdit(src, html) {
  if (!conv || !cur) return;
  let err = "";
  if ($("source").value !== rendered) {
    err = "The source changed after this page was made. The page was refreshed: make the change again.";
  } else {
    try {
      $("source").value = conv.edit(rendered, cur.name, cur.rawOk, src, html);
      commitText();
    } catch (e) {
      err = String(e);
    }
  }
  // the page is made again either way (a refused edit puts the old text back); the reason is shown after that
  preview();
  if (err) fail(err);
}

/** The text in the box is a new step of the history, and the draft is kept (a draft equal to the chapter is dropped). */
function commitText() {
  clearTimeout(commitTimer);
  if (!cur || !hist) return;
  const text = $("source").value;
  hist.record(text);
  if (text === cur.md.text) clearDraft(cur.name);
  else saveDraft(cur.name, { text, base: cur.md.sha, summary: $("summary").value, saved: new Date().toISOString() });
  refresh();
}

/** @param {string | null} text a state of the history, or null when there is none */
function goTo(text) {
  if (text === null) return;
  $("source").value = text;
  commitText();
  preview();
}

const undo = () => hist && goTo(hist.undo());
const redo = () => hist && goTo(hist.redo());

/** A draft of this chapter found in the browser: restored when it starts from the chapter as it is now, else only offered as a copy. @param {string} name @param {{text: string, sha: string}} md */
function offerDraft(name, md) {
  const box = $("draft");
  box.replaceChildren();
  const d = loadDraft(name);
  if (!d || d.text === md.text) {
    box.hidden = true;
    return;
  }
  const fresh = d.base === md.sha;
  const when = d.saved ? new Date(d.saved).toLocaleString() : "earlier";
  const label = document.createElement("span");
  label.textContent = fresh ? `An unsent draft of this chapter (${when}) was found.` : `A draft from ${when} was found, but the chapter has changed since: it cannot be put back, only copied.`;
  /** @param {string} t @param {() => void} f */
  const btn = (t, f) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = t;
    b.addEventListener("click", f);
    box.append(b);
  };
  box.append(label);
  if (fresh) {
    btn("Restore the draft", () => {
      $("source").value = d.text;
      $("summary").value = d.summary;
      commitText();
      preview();
      box.hidden = true;
    });
  } else {
    btn("Copy the draft", () => navigator.clipboard?.writeText(d.text).then(() => (label.textContent = "Copied.")));
  }
  btn("Discard the draft", () => {
    clearDraft(name);
    box.hidden = true;
  });
  box.hidden = false;
}

function scheduleCommit() {
  clearTimeout(commitTimer);
  commitTimer = setTimeout(commitText, 600);
}

function schedulePreview() {
  clearTimeout(timer);
  timer = setTimeout(preview, 400);
}

async function send() {
  if (!client || !cur) return;
  fail("");
  const r = convert();
  if (r.error) return fail(r.error);
  // retitled sections are allowed (with their anchors); the search index of the chapter is then rebuilt by the same converter
  /** @type {{text: string, loadedSha: string} | undefined} */
  let index;
  try {
    if (conv?.retitled(cur.md.text, $("source").value, cur.name)) {
      index = { text: conv.reindex(cur.index.text, cur.name.replace(/\.md$/, ".html"), /** @type {string} */ (r.html)), loadedSha: cur.index.sha };
    }
  } catch (e) {
    return fail(String(e));
  }
  $("btn-send").disabled = true;
  say("Sending…");
  try {
    const pr = await client.submit({ name: cur.name, md: $("source").value, html: /** @type {string} */ (r.html), loaded: { md: cur.md, html: cur.html }, summary: $("summary").value.trim(), index });
    clearTimeout(commitTimer);
    clearDraft(cur.name);
    cur.md = { text: $("source").value, sha: "" };
    $("source").disabled = true;
    $("btn-preview").disabled = true;
    $("preview").hidden = true;
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
  $("preview").addEventListener("load", () => wirePreview($("preview"), pendingTop));
  $("source").addEventListener("input", () => {
    refresh();
    schedulePreview();
    scheduleCommit();
  });
  $("summary").addEventListener("input", () => {
    refresh();
    scheduleCommit();
  });
  $("btn-undo").addEventListener("click", undo);
  $("btn-redo").addEventListener("click", redo);
  document.addEventListener("keydown", (ev) => {
    const el = /** @type {HTMLElement} */ (ev.target);
    if (!(ev.ctrlKey || ev.metaKey) || (el.tagName === "INPUT" && el.id !== "source")) return;
    const k = (ev.key || "").toLowerCase();
    if (k === "z" && !ev.shiftKey) undo();
    else if ((k === "z" && ev.shiftKey) || k === "y") redo();
    else return;
    ev.preventDefault();
  });
  $("btn-preview").addEventListener("click", preview);
  $("btn-send").addEventListener("click", send);
  $("btn-token").addEventListener("click", () => $("dlg-token").showModal());
  $("t-close").addEventListener("click", () => $("dlg-token").close());
  $("t-tab").addEventListener("click", () => {
    const t = $("in-token").value.trim();
    if (!t) return ($("t-msg").textContent = "Paste the token first.");
    if (!looksLikeToken(t)) return ($("t-msg").textContent = NOT_TOKEN);
    vault.keepForTab(t, SLOT);
    useToken(t);
  });
  $("t-seal").addEventListener("click", async () => {
    const t = $("in-token").value.trim();
    const p = $("in-pass").value;
    if (!t || !p) return ($("t-msg").textContent = "Paste the token and invent a passphrase.");
    if (!looksLikeToken(t)) return ($("t-msg").textContent = NOT_TOKEN);
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
    if (!looksLikeToken(t)) {
      // what was sealed is not a token (the two fields were swapped): drop it and ask again
      vault.forget(SLOT);
      $("dlg-unlock").close();
      $("t-msg").textContent = "The saved text was not a token and was removed. Paste the github_pat_… key in the Token field.";
      return $("dlg-token").showModal();
    }
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
