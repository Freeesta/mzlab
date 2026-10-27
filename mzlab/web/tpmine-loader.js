// @ts-strict
// Loader of Tabui (internal name "TP Mine", also called mzFinder in the code): the search for transformation products, shown in the header as a normal tab.
// This file is small and contains NO tool code. The tools are in the site in plain text (static/mzfinder/, written by tools/build_site.py
// from TP_Mine/) and are downloaded and run when the page starts (the tab shows a loading note until they are ready). No switch, no password.
// static/mzfinder/indice.json = {early: [names, run first, before the data files], js: [names, run in this order], files: [names, given to the scripts as QTOOLS.ctx.files[name]], py: zip name}
// The scripts register their tools with window.QTOOLS.register(...). A tool registered with nav:true gets its own tab in the page header
// (next to Teoria) and a full page; others get a button in the floating bar.
// A tool registered with admin:true exists only in the administrator mode (localStorage «qqq.admin» = «1», five clicks on the logo): without it the package is not
// even downloaded, no tab or button is made and nothing in the page points to the tool. The five clicks (logo-piccone.js) fire "mzlab:admin" on document and the
// tab appears at once; leaving the mode (settings.js) reloads the page, which drops the tabs, the results and the workers of the tools.
(() => {
  "use strict";
  const BASE = new URL(".", document.currentScript ? /** @type {HTMLScriptElement} */ (document.currentScript).src : location.href).href;
  const DIR = BASE + "mzfinder/";

  // ---- registry: tools register themselves after the scripts are loaded. {id, name, icon (svg string), desc, open(body, ctx)}
  const tools = [];
  // true while the main scripts run: a tool registered then gets its tab when they are all done (turnOn), so its markup is drawn with the help (TPHELP) and the other later scripts in place
  let settling = false;
  const adminOn = () => { try { return localStorage.getItem(NOMI.KEY("admin")) === "1" || localStorage.getItem(NOMI.KEY("teoria.esteso")) === "1"; } catch (_) { return false; } };
  /** a tool with admin:true is shown only in the administrator mode */
  const shown = t => !t.admin || adminOn();
  const reg = window.QTOOLS = {
    ctx: /** @type {any} */ (null),                                     // set before the scripts run: {base: URL of the site static folder, meta, files}
    list: () => on ? tools.filter(shown) : [],
    register(t) { if (!t || !t.id || typeof t.open !== "function") throw new Error("QTOOLS.register: id and open() are required"); const i = tools.findIndex(x => x.id === t.id); if (i >= 0) tools[i] = t; else tools.push(t); if (t.nav && on && shown(t) && !settling) addTab(t); render(); },
  };

  // scripts that are not needed at the start: TPLAZY.load("a.js", "b.js") runs them once, in this order, from static/mzfinder/lazy/ (the names are listed in indice.json under `lazy`)
  const lazyDone = /** @type {Map<string, Promise<void>>} */ (new Map());
  /** starts the downloads of all the names together with a preload (the script tags below then take the same download, even while it is still running), so a long list costs one round trip, not one per file; a `low` priority is for files that nobody waits for yet @param {string[]} names @param {boolean} [low] */
  const warmLazy = (names, low) => { for (const n of names) if (!lazyWarm.has(n)) { lazyWarm.add(n); const l = document.createElement("link"); l.rel = "preload"; l.as = "script"; l.href = DIR + "lazy/" + n; if (low) l.setAttribute("fetchpriority", "low"); l.onload = l.onerror = () => l.remove(); document.head.appendChild(l); } };
  const lazyWarm = /** @type {Set<string>} */ (new Set());
  /** @param {...string} names @returns {Promise<void>} */
  const lazyLoad = async (...names) => { warmLazy(names); for (const n of names) { let p = lazyDone.get(n); if (!p) { p = runScript("lazy/" + n); lazyDone.set(n, p); p.catch(() => lazyDone.delete(n)); } await p; } };
  /** like load, but all the tags go in now, in order (async=false): called early, they run before the page's own long list of scripts instead of one by one behind it @param {...string} names @returns {Promise<void>} */
  const lazyLoadNow = async (...names) => {
    warmLazy(names);
    const ps = names.map(n => { let p = lazyDone.get(n); if (!p) { p = runScript("lazy/" + n, true); lazyDone.set(n, p); p.catch(() => lazyDone.delete(n)); } return p; });
    for (const p of ps) await p;
  };
  /** @type {any} */ (window).TPLAZY = { load: lazyLoad, loadNow: lazyLoadNow, warm: warmLazy, whenLoaded: () => loaded || Promise.resolve() };

  function runScript(name, inOrder) {
    return new Promise((/** @type {any} */ res, rej) => {
      const s = document.createElement("script");
      if (inOrder) s.async = false;
      s.onload = () => { s.remove(); res(); };
      s.onerror = () => { s.remove(); rej(new Error("script " + name)); };
      s.src = DIR + name; document.head.appendChild(s);
    });
  }

  // ---- launcher bar and tool host
  let bar = null, host = null, on = false, loaded = null;
  const css = `#qt-bar{position:fixed;left:12px;bottom:12px;z-index:80;display:flex;gap:6px;align-items:center;background:var(--panel,#fff);border:1px solid var(--accent,#7a5c1e);border-radius:20px;padding:4px 10px;box-shadow:0 2px 10px rgba(0,0,0,.2);font:13px system-ui,sans-serif}
#qt-bar button{display:inline-flex;align-items:center;gap:5px;border:0;background:transparent;color:var(--accent,#7a5c1e);cursor:pointer;font:inherit;font-weight:600;padding:2px 4px}
#qt-bar button:hover{text-decoration:underline}
#qt-host{position:fixed;inset:0;z-index:90;background:var(--bg,#fff);display:none;flex-direction:column}
#qt-host.on{display:flex}
#qt-host .qt-hd{display:flex;align-items:center;gap:10px;padding:8px 14px;border-bottom:1px solid var(--line,#ccc)}
#qt-host .qt-hd b{font-size:15px}#qt-host .qt-hd .qt-x{margin-left:auto}
#qt-host .qt-bd{flex:1;overflow:auto;padding:14px}`;
  function ensureUi() {
    if (bar) return;
    const st = document.createElement("style"); st.textContent = css; document.head.appendChild(st);
    bar = document.createElement("div"); bar.id = "qt-bar"; bar.hidden = true; document.body.appendChild(bar);
    host = document.createElement("div"); host.id = "qt-host"; document.body.appendChild(host);
  }
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  function render() {
    if (!bar) return;
    const floating = tools.filter(t => !t.nav && shown(t));
    bar.hidden = !on || !floating.length;
    bar.innerHTML = floating.map(t => `<button data-t="${esc(t.id)}" title="${esc(t.desc || t.name)}">${t.icon || ""}${esc(t.name)}</button>`).join("");
    bar.querySelectorAll("button").forEach(b => b.onclick = () => openTool(tools.find(t => t.id === b.dataset.t)));
  }
  // a tab can be clicked before the page shell (shell.js) has run: the view is then shown here, and told to the shell as soon as it exists
  let early = /** @type {string | null} */ (null);
  const shellReady = () => typeof setView === "function" && typeof S !== "undefined" && typeof STATO !== "undefined";
  function syncView() {
    if (early === null) return;
    if (shellReady()) { const id = early; early = null; setView(id); } else setTimeout(syncView, 50);
  }
  /** @param {string} id */
  function openView(id) {
    if (shellReady()) { setView(id); return; }
    const first = early === null;
    early = id;
    for (const s of document.querySelectorAll("section[id^='v-']")) /** @type {HTMLElement} */ (s).hidden = s.id !== "v-" + id;
    for (const k of ["start", "cancadd"]) { const e = document.getElementById(k); if (e) e.hidden = true; }
    document.querySelectorAll("#nav button").forEach(x => x.classList.toggle("on", /** @type {HTMLElement} */ (x).dataset.v === id));
    document.dispatchEvent(new CustomEvent("tpview", { detail: { view: id } }));
    if (first) syncView();
  }
  // tool with its own tab: button in #nav + full page section after the other views; opened lazily on the first visit
  const tabs = new Map();         // id -> {b, sec, fn}
  /** the text of the tab button: icon and name */
  const label = t => (t.icon ? t.icon + " " : "") + esc(t.name);
  function addTab(t) {
    const nav = document.getElementById("nav"), anchor = document.getElementById("v-theory");
    const old = tabs.get(t.id);
    if (old && old.ph && !t.ph) {                                    // the real tool replaces the placeholder shown while loading
      old.t = t; old.ph = false; old.b.title = t.desc || t.name; old.b.innerHTML = label(t);
      if (old.opened) { old.sec.textContent = ""; openIn(old); }
      return;
    }
    if (!nav || !anchor || old || document.getElementById("v-" + t.id)) return;
    const b = document.createElement("button"); b.dataset.v = t.id; b.title = t.desc || t.name;
    b.innerHTML = label(t); b.style.display = "inline-flex"; b.style.alignItems = "center"; b.style.gap = "5px";
    nav.appendChild(b);
    const sec = document.createElement("section"); sec.id = "v-" + t.id; sec.hidden = true; anchor.after(sec);
    /** @type {any} */ const x = { b, sec, fn: null, t, ph: !!t.ph, opened: false };
    b.onclick = () => openView(t.id);
    x.fn = ev => {
      const show = ev.detail.view === t.id; sec.hidden = !show;
      if (show && !x.opened) { x.opened = true; openIn(x); }
      else if (show && x.t.onShow) try { x.t.onShow(); } catch (_) { /* ignore */ }
    };
    document.addEventListener("tpview", x.fn);
    tabs.set(t.id, x);
  }
  function openIn(x) { try { x.t.open(x.sec, reg.ctx); } catch (e_) { const e = /** @type {Lanciato} */ (e_); x.sec.textContent = "Tabui failed: " + (e && e.message || e); } }
  function openTool(t) {
    host.innerHTML = `<div class="qt-hd">${t.icon || ""}<b>${esc(t.name)}</b><span class="muted sm">${esc(t.desc || "")}</span><button class="qt-x" title="Close">Close</button></div><div class="qt-bd"></div>`;
    host.querySelector(".qt-x").onclick = () => { host.classList.remove("on"); if (t.close) try { t.close(); } catch (_) { /* ignore */ } host.innerHTML = ""; };
    host.classList.add("on");
    try { t.open(host.querySelector(".qt-bd"), reg.ctx); } catch (e_) { const e = /** @type {Lanciato} */ (e_); host.querySelector(".qt-bd").textContent = "Tabui failed: " + (e && e.message || e); }
  }

  // ---- loading (once per page): index, then extra files + the scripts' downloads in parallel, then the scripts run in order
  async function load() {
    const get = async (name, how) => { const r = await fetch(DIR + name); if (!r.ok) throw new Error(name + " missing"); return r[how](); };
    const idx = await get("indice.json", "json");
    const early = idx.early || [], js = idx.js || [], names = idx.files || [];
    const files = {};
    reg.ctx = { base: BASE, meta: idx.meta || {}, files };
    const warm = [...early, ...js].map(n => fetch(DIR + n).catch(() => null));     // starts all downloads now (early ones first); the script tags below then hit the cache
    try { for (const n of early) await runScript(n); }             // scripts that need neither the data files nor the other scripts (the Flow tab): they draw while the rest downloads
    finally { document.dispatchEvent(new Event("tpmine:early")); }   // carica.js starts the Dati scripts on this in administrator mode
    if (on) showTabs();                                           // their tabs (Flow) appear now, not after the whole package
    const texts = await Promise.all(names.map(n => get(n, "text")));
    names.forEach((n, i) => { files[n] = texts[i]; });
    await Promise.all(warm);
    settling = true;
    try { for (const n of js) await runScript(n); } finally { settling = false; }
  }
  // the tab appears at once with a short loading note; the real tool takes its place when its script registers
  const PLACEHOLDER = { id: "tpmine", nav: true, admin: true, ph: true, name: "Tabui", desc: "Loading…", open(/** @type {HTMLElement} */ sec) { sec.innerHTML = '<p class="muted" role="status" style="padding:14px">Loading…</p>'; } };
  function showTabs() {
    for (const t of tools) if (t.nav && shown(t)) addTab(t);
    render();
  }
  async function turnOn() {
    on = true; ensureUi();
    if (!loaded) loaded = load();
    if (!tabs.size) addTab(PLACEHOLDER);
    try { await loaded; } catch (e) {
      loaded = null; console.error("Tabui failed to load:", e);
      const x = tabs.get(PLACEHOLDER.id); if (x && x.ph) x.sec.textContent = "Tabui failed to load. Reload the page to try again.";
      return;
    }
    if (!on) return;
    showTabs();
  }
  const start = () => { if ((window.MZLAB_BROWSER || window.MZLAB_DESKTOP) && !on && adminOn()) turnOn(); };      // only in the built site and the desktop app (the plain web folder has no static/mzfinder/), only for the administrator
  document.addEventListener("mzlab:admin", start);
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start); else start();
})();
