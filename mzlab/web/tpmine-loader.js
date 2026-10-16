// @ts-strict
// Loader of Tabui (internal name "TP Mine", also called mzFinder in the code): the search for transformation products, shown in the header as a normal tab.
// This file is small and contains NO tool code. The tools are in the site in plain text (static/mzfinder/, written by tools/build_site.py
// from TP_Mine/) and are downloaded and run when the page starts (the tab shows a loading note until they are ready). No switch, no password.
// static/mzfinder/indice.json = {js: [names, run in this order], lazy: [names of lazy/, loaded on request by TPLAZY.load], files: [names, given to the scripts as QTOOLS.ctx.files[name]], py: zip name}
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
  const adminOn = () => { try { return localStorage.getItem("qqq.admin") === "1" || localStorage.getItem("qqq.teoria.esteso") === "1"; } catch (_) { return false; } };
  /** a tool with admin:true is shown only in the administrator mode */
  const shown = t => !t.admin || adminOn();
  const reg = window.QTOOLS = {
    ctx: /** @type {any} */ (null),                                     // set before the scripts run: {pyZip: Uint8Array, base: URL of the site static folder, files}
    list: () => on ? tools.filter(shown) : [],
    register(t) { if (!t || !t.id || typeof t.open !== "function") throw new Error("QTOOLS.register: id and open() are required"); const i = tools.findIndex(x => x.id === t.id); if (i >= 0) tools[i] = t; else tools.push(t); if (t.nav && on && shown(t)) addTab(t); render(); },
  };

  function runScript(name, sub) {
    return new Promise((/** @type {any} */ res, rej) => {
      const s = document.createElement("script");
      s.onload = () => { s.remove(); res(); };
      s.onerror = () => { s.remove(); rej(new Error("script " + name)); };
      s.src = DIR + (sub || "") + name; document.head.appendChild(s);
    });
  }

  // ---- lazy files (static/mzfinder/lazy/, listed in indice.json under "lazy"): not downloaded when Tabui opens, one <script> each on the first TPLAZY.load(name)
  /** @type {string[]} */ let lazyNames = [];
  /** @type {Map<string, Promise<void>>} */ const lazyDone = new Map();
  /** @type {any} */ (window).TPLAZY = {
    /** loads a file of lazy/ once; a second call returns the same promise. A name not in the list or a failed download rejects (a failed one can be asked again) */
    load(/** @type {string} */ name) {
      const old = lazyDone.get(name); if (old) return old;
      if (!lazyNames.includes(name)) return Promise.reject(new Error("lazy file " + name + " missing"));
      const p = runScript(name, "lazy/"); lazyDone.set(name, p); p.catch(() => { lazyDone.delete(name); });
      return p;
    },
  };

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
    b.onclick = () => setView(t.id);
    x.fn = ev => {
      const show = ev.detail.view === t.id; sec.hidden = !show;
      if (show && !x.opened) { x.opened = true; openIn(x); }
      else if (show && x.t.onShow) try { x.t.onShow(); } catch (_) { /* ignore */ }
    };
    document.addEventListener("tpview", x.fn);
    tabs.set(t.id, x);
  }
  function openIn(x) { try { x.t.open(x.sec, reg.ctx); } catch (e) { x.sec.textContent = "Tabui failed: " + (e && e.message || e); } }
  function openTool(t) {
    host.innerHTML = `<div class="qt-hd">${t.icon || ""}<b>${esc(t.name)}</b><span class="muted sm">${esc(t.desc || "")}</span><button class="qt-x" title="Close">Close</button></div><div class="qt-bd"></div>`;
    host.querySelector(".qt-x").onclick = () => { host.classList.remove("on"); if (t.close) try { t.close(); } catch (_) { /* ignore */ } host.innerHTML = ""; };
    host.classList.add("on");
    try { t.open(host.querySelector(".qt-bd"), reg.ctx); } catch (e) { host.querySelector(".qt-bd").textContent = "Tabui failed: " + (e && e.message || e); }
  }

  // ---- loading (once per page): index, then extra files + Python zip + the scripts' downloads in parallel, then the scripts run in order
  async function load() {
    const get = async (name, how) => { const r = await fetch(DIR + name); if (!r.ok) throw new Error(name + " missing"); return r[how](); };
    const idx = await get("indice.json", "json");
    const js = idx.js || [], names = idx.files || [];
    lazyNames = idx.lazy || [];
    const warm = js.map(n => fetch(DIR + n).catch(() => null));     // starts all downloads now; the script tags below then hit the cache
    const [texts, pyBuf] = await Promise.all([Promise.all(names.map(n => get(n, "text"))), idx.py ? get(idx.py, "arrayBuffer") : null]);
    const files = {};
    names.forEach((n, i) => { files[n] = texts[i]; });
    reg.ctx = { pyZip: pyBuf ? new Uint8Array(pyBuf) : null, base: BASE, meta: idx.meta || {}, files };
    await Promise.all(warm);
    for (const n of js) await runScript(n);
  }
  // the tab appears at once with a short loading note; the real tool takes its place when its script registers
  const PLACEHOLDER = { id: "tpmine", nav: true, admin: true, ph: true, name: "Tabui", desc: "Loading…", open(/** @type {HTMLElement} */ sec) { sec.innerHTML = '<p class="muted" role="status" style="padding:14px">Loading…</p>'; } };
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
    for (const t of tools) if (t.nav && shown(t)) addTab(t);
    render();
  }
  const start = () => { if ((window.MZLAB_BROWSER || window.MZLAB_DESKTOP) && !on && adminOn()) turnOn(); };      // only in the built site and the desktop app (the plain web folder has no static/mzfinder/), only for the administrator
  document.addEventListener("mzlab:admin", start);
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start); else start();
})();
