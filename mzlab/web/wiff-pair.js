// @ts-check
"use strict";
// A SCIEX .wiff always travels with its .wiff.scan (docs/agenti/wiff-pair.md). When only one of them is loaded the page asks the user for the FOLDER (a web page cannot
// read the neighbours of a file), opens that dialog already in the right place when the browser gave a file handle, and takes ONLY the file of the pair from it
// (names are listed, no other file is read). Lazy (carica("wiff-pair.js")), called by upload() once a batch holds a .wiff or .wiff.scan that is still alone.
// The rule «which file is the companion of which» is the engine's (crates/mzlab-lettori/src/pair.rs); `half`, `companionName` and `pick` are its twin, tests/test_wiff_pair.py proves them equal.

const WIFFPAIR = (() => {
  const lc = (/** @type {string} */ s) => s.replace(/[A-Z]/g, c => c.toLowerCase());
  /** "wiff", "scan" or "" (any other name, also a .scan of another maker) */
  const half = (/** @type {string} */ n) => { const l = lc(n); return l.endsWith(".wiff.scan") ? "scan" : l.endsWith(".wiff") ? "wiff" : ""; };
  /** the name the other half has: X.wiff -> X.wiff.scan, X.wiff.scan -> X.wiff ("" for any other name) */
  const companionName = (/** @type {string} */ n) => { const h = half(n); return h === "wiff" ? n + ".scan" : h === "scan" ? n.slice(0, -5) : ""; };
  /** the name among `names` that is the companion of `name`: the exact spelling wins, else, of those equal ignoring case, the first in byte order (UTF-8, as the engine) */
  const enc = new TextEncoder(), cmp = (/** @type {string} */ a, /** @type {string} */ b) => { const x = enc.encode(a), y = enc.encode(b); for (let i = 0; i < Math.min(x.length, y.length); i++) if (x[i] !== y[i]) return x[i] - y[i]; return x.length - y.length; };
  function pick(/** @type {string} */ name, /** @type {Iterable<string>} */ names) {
    const want = companionName(name); if (!want) return null;
    const low = want.toLowerCase(); let loose = null;
    for (const n of names) { if (n === want) return n; if (n.toLowerCase() === low && (loose === null || cmp(n, loose) < 0)) loose = n; }
    return loose;
  }
  const stemOf = (/** @type {string} */ n) => n.replace(/\.wiff(\.scan)?$/i, "");
  const mb = (/** @type {number} */ n) => (n >= 1 << 20 ? (n / 1048576).toFixed(n >= 10 << 20 ? 0 : 1) + " MB" : Math.max(1, Math.round(n / 1024)) + " KB");
  const t = (/** @type {string} */ k, /** @type {any} */ p) => I18N.t(k, p);
  const g = /** @type {any} */ (typeof window !== "undefined" ? window : {});

  // file handles of the picker and of the drop (DROP.handles, shell.js): their only use is `startIn` of the folder dialog
  const handleOf = (/** @type {string} */ name) => (typeof DROP !== "undefined" && DROP.handles.get(lc(name))) || null;
  // the folder granted in this page session: the next lone file looks there first
  /** @type {{ h: any, name: string } | null} */
  let session = null;
  /** @type {{ el: HTMLElement | null, wanted: { file: File, want: string }[], busy: boolean }} */
  const S = { el: null, wanted: [], busy: false };

  /** top-level files of a folder handle whose names are the wanted ones: [{want, name, file}] (a name is listed, a file is read only when it is the companion) */
  async function look(/** @type {any} */ dir, /** @type {{ file: File, want: string }[]} */ wanted) {
    const names = new Map();
    for await (const [n, h] of dir.entries()) if (h.kind === "file") names.set(n, h);
    const found = [], missing = [];
    for (const w of wanted) {
      const n = pick(w.file.name, names.keys());
      if (n) found.push({ want: w.want, file: await names.get(n).getFile() }); else missing.push(w.want);
    }
    return { found, missing };
  }
  /** the same from the files of a webkitdirectory input (Safari, Firefox): only the top level, only the names that match */
  function lookInput(/** @type {FileList} */ list, /** @type {{ file: File, want: string }[]} */ wanted) {
    const top = [...list].filter(f => (f.webkitRelativePath || "").split("/").length === 2), by = new Map(top.map(f => [f.name, f]));
    const found = [], missing = [];
    for (const w of wanted) { const n = pick(w.file.name, by.keys()); if (n) found.push({ want: w.want, file: /** @type {File} */ (by.get(n)) }); else missing.push(w.want); }
    return { found, missing, folder: top.length ? (top[0].webkitRelativePath || "").split("/")[0] : "" };
  }

  // ---- the notice: a status region with the sentence and the buttons
  function card() {
    if (S.el && S.el.isConnected) return S.el;
    const el = document.createElement("div"); el.id = "wiffpair"; el.className = "wiffpair"; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite");
    document.body.appendChild(el); S.el = el; return el;
  }
  function say(/** @type {string} */ msg, /** @type {{ key: string, label: string, title: string, go?: boolean }[]} */ buttons) {
    const el = card(); el.textContent = "";
    const p = document.createElement("p"); p.className = "wp-msg"; p.textContent = msg; el.appendChild(p);
    const row = document.createElement("div"); row.className = "wp-act";
    for (const b of buttons) {
      const x = document.createElement("button"); x.type = "button"; x.dataset.wp = b.key; x.textContent = b.label; x.title = b.title; if (b.go) x.className = "go"; row.appendChild(x);
      x.onclick = () => act(b.key);
    }
    el.appendChild(row); el.hidden = false;
    const first = row.querySelector("button"); if (first && buttons.length && S.busy === false && !el.contains(document.activeElement)) /** @type {HTMLElement} */ (first).focus({ preventScroll: true });
  }
  function close() { if (S.el) { S.el.hidden = true; S.el.textContent = ""; } }
  const folderBtn = () => ({ key: "choose", label: t("wiffpair.choose"), title: t("wiffpair.choose.tip"), go: true });
  const skipBtn = () => ({ key: "skip", label: t("wiffpair.skip"), title: t("wiffpair.skip.tip") });
  const names = () => S.wanted.map(w => w.want).join(", ");
  const needText = () => t(S.wanted.length > 1 ? "wiffpair.needMany" : "wiffpair.need", { name: names() });

  /** the files found are loaded: the lone halves wait in the loader (WIFF_WAIT) and the pair opens as soon as its second file arrives */
  function open(/** @type {{ want: string, file: File }[]} */ found, /** @type {string} */ folder) {
    const size = found.reduce((s, x) => s + x.file.size, 0);
    say(t("wiffpair.found", { names: found.map(x => x.want).join(", "), size: mb(size), folder }), []);
    S.wanted = S.wanted.filter(w => !found.some(x => x.want === w.want));
    g.upload(found.map(x => x.file), "data");
    setTimeout(() => { if (!S.wanted.length && S.el) close(); }, 6000);
  }
  function notThere(/** @type {string[]} */ missing, /** @type {string} */ folder) {
    say(t("wiffpair.missing", { names: missing.join(", "), folder }), [folderBtn(), skipBtn()]);
  }

  async function act(/** @type {string} */ key) {
    if (key === "skip") { S.wanted = []; close(); return; }
    if (S.busy || !S.wanted.length) return;
    S.busy = true;
    try {
      if (key === "allow") { const p = await CartPP.S.handle.requestPermission({ mode: "read" }); if (p === "granted") { const r = await look(CartPP.S.handle, S.wanted); if (r.found.length) open(r.found, CartPP.S.name); if (r.missing.length) { S.wanted = S.wanted.filter(w => r.missing.includes(w.want)); say(needText(), [folderBtn(), skipBtn()]); } return; } say(needText(), [folderBtn(), skipBtn()]); return; }
      if (typeof g.showDirectoryPicker === "function") {
        const fh = handleOf(S.wanted[0].file.name), opts = { id: "mzlab-wiff-pair", mode: "read", ...(fh ? { startIn: fh } : session ? { startIn: session.h } : {}) };
        let dir; try { dir = await g.showDirectoryPicker(opts); } catch (e) { if (/** @type {any} */ (e).name === "AbortError") return; throw e; }
        const r = await look(dir, S.wanted); session = { h: dir, name: dir.name };
        if (r.found.length) open(r.found, dir.name);
        if (r.missing.length) { S.wanted = S.wanted.filter(w => r.missing.includes(w.want)); notThere(r.missing, dir.name); }
      } else {
        // Safari and Firefox: no showDirectoryPicker; a folder input gives the names of the folder (their files are not read), only the pair is taken
        const inp = document.createElement("input"); inp.type = "file"; inp.setAttribute("webkitdirectory", ""); inp.multiple = true;
        inp.onchange = () => { const r = lookInput(/** @type {FileList} */ (inp.files), S.wanted); if (r.found.length) open(r.found, r.folder); if (r.missing.length) { S.wanted = S.wanted.filter(w => r.missing.includes(w.want)); notThere(r.missing, r.folder); } };
        inp.click();
      }
    } catch (e) { say(t("wiffpair.error", { detail: String(/** @type {any} */ (e).message || e) }), [folderBtn(), skipBtn()]); }
    finally { S.busy = false; }
  }

  /** The halves of `files` that are still alone in the loader (no companion in the batch, in an earlier loading or in a folder loading): asks for the folder. */
  async function offer(/** @type {File[]} */ files) {
    if (g.MZLAB_DESKTOP) return;                      // the app opens the pair from the path (opening.rs)
    /** @type {{ file: File, want: string }[]} */
    const alone = [];
    for (const f of files) {
      const h = half(f.name); if (!h || CartCarica.dir(f)) continue;
      const e = WIFF_WAIT.get(CartCarica.pairKey(f, stemOf(f.name)));
      if (e && !(e.wiff && e.scan) && !alone.some(a => a.want === companionName(f.name))) alone.push({ file: f, want: companionName(f.name) });
    }
    if (!alone.length) return;
    S.wanted = [...S.wanted.filter(w => !alone.some(a => a.want === w.want)), ...alone];
    // 1) the folder of this session, then the folder the page remembers (pointed folder): a granted one is read at once, a remembered one needs one click
    for (const src of [session && { h: session.h, name: session.name, own: true }, typeof CartPP !== "undefined" && CartPP.S.handle && { h: CartPP.S.handle, name: CartPP.S.name, own: false }]) {
      if (!src) continue;
      try {
        const p = await src.h.queryPermission({ mode: "read" });
        if (p === "granted") { const r = await look(src.h, S.wanted); if (r.found.length) { open(r.found, src.name); if (!S.wanted.length) return; } }
        else if (!src.own && CartPP.S.items.some((/** @type {any} */ x) => !x.dir && S.wanted.some(w => pick(w.file.name, [x.name])))) { say(t("wiffpair.remembered", { name: names(), folder: src.name }), [{ key: "allow", label: t("wiffpair.allow", { folder: src.name }), title: t("wiffpair.allow.tip"), go: true }, folderBtn(), skipBtn()]); return; }
      } catch (_) { /* the folder is gone or not readable: the prompt below */ }
    }
    say(needText(), [folderBtn(), skipBtn()]);
  }

  return { half, companionName, pick, offer, handleOf, state: S };
})();
if (typeof module !== "undefined") module.exports = WIFFPAIR;
