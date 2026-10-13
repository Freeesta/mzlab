// @ts-strict
// Loader of mzFinder (internal name "TP Mine"): the tools that give answers, only for the course owner.
// This file is small and contains NO tool code. The tools are in the site in plain text (static/mzfinder/, written by tools/build_site.py
// from TP_Mine/) but are downloaded and run ONLY after the switch is turned on: with mzFinder off nothing of it is fetched.
//
// Switch: 5 hits on the logo of the page header (logo-piccone.js, within 3 s of each other) turn mzFinder on; 5 more turn it off. No password.
// State in localStorage "qqq.mzfinder" ("1" = on): after a reload it stays on.
// static/mzfinder/indice.json = {js: [names, run in this order], files: [names, given to the scripts as QTOOLS.ctx.files[name]], py: zip name}
// The scripts register their tools with window.QTOOLS.register(...). A tool registered with nav:true gets its own tab in the page header
// (next to Teoria) and a full page; others get a button in the floating bar.
(() => {
  "use strict";
  const BASE = new URL(".", document.currentScript ? /** @type {HTMLScriptElement} */ (document.currentScript).src : location.href).href;
  const DIR = BASE + "mzfinder/";
  const KEY = "qqq.mzfinder";

  // ---- registry: tools register themselves after the scripts are loaded. {id, name, icon (svg string), desc, open(body, ctx)}
  const tools = [];
  const reg = window.QTOOLS = {
    ctx: /** @type {any} */ (null),                                     // set before the scripts run: {pyZip: Uint8Array, base: URL of the site static folder, files}
    list: () => on ? tools.slice() : [],
    register(t) { if (!t || !t.id || typeof t.open !== "function") throw new Error("QTOOLS.register: id and open() are required"); const i = tools.findIndex(x => x.id === t.id); if (i >= 0) tools[i] = t; else tools.push(t); if (t.nav && on) addTab(t); render(); },
  };

  const getOn = () => { try { return localStorage.getItem(KEY) === "1"; } catch (_) { return false; } };
  const setOn = v => { try { if (v) localStorage.setItem(KEY, "1"); else localStorage.removeItem(KEY); } catch (_) { /* private mode: only for this page */ } };

  function runScript(name) {
    return new Promise((/** @type {any} */ res, rej) => {
      const s = document.createElement("script");
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
    const floating = tools.filter(t => !t.nav);
    bar.hidden = !on || !floating.length;
    bar.innerHTML = floating.map(t => `<button data-t="${esc(t.id)}" title="${esc(t.desc || t.name)}">${t.icon || ""}${esc(t.name)}</button>`).join("");
    bar.querySelectorAll("button").forEach(b => b.onclick = () => openTool(tools.find(t => t.id === b.dataset.t)));
  }
  // tool with its own tab: button in #nav + full page section after the other views; opened lazily on the first visit
  const tabs = new Map();         // id -> {b, sec, fn}
  function addTab(t) {
    const nav = document.getElementById("nav"), anchor = document.getElementById("v-theory");
    if (!nav || !anchor || tabs.has(t.id) || document.getElementById("v-" + t.id)) return;
    const b = document.createElement("button"); b.dataset.v = t.id; b.title = t.desc || t.name;
    b.innerHTML = (t.icon ? t.icon + " " : "") + esc(t.name); b.style.display = "inline-flex"; b.style.alignItems = "center"; b.style.gap = "5px";
    nav.appendChild(b);
    const sec = document.createElement("section"); sec.id = "v-" + t.id; sec.hidden = true; anchor.after(sec);
    let opened = false;
    b.onclick = () => setView(t.id);
    const fn = ev => {
      const show = ev.detail.view === t.id; sec.hidden = !show;
      if (show && !opened) { opened = true; try { t.open(sec, reg.ctx); } catch (e) { sec.textContent = "Tabui failed: " + (e && e.message || e); } }
      else if (show && t.onShow) try { t.onShow(); } catch (_) { /* ignore */ }
    };
    document.addEventListener("tpview", fn);
    tabs.set(t.id, { b, sec, fn });
  }
  function removeTabs() {
    for (const x of tabs.values()) {
      const shown = !x.sec.hidden;
      document.removeEventListener("tpview", x.fn); x.b.remove(); x.sec.remove();
      if (shown && typeof setView === "function") setView("data");
    }
    tabs.clear();
  }
  function openTool(t) {
    host.innerHTML = `<div class="qt-hd">${t.icon || ""}<b>${esc(t.name)}</b><span class="muted sm">${esc(t.desc || "")}</span><button class="qt-x" title="Close">Close</button></div><div class="qt-bd"></div>`;
    host.querySelector(".qt-x").onclick = () => { host.classList.remove("on"); if (t.close) try { t.close(); } catch (_) { /* ignore */ } host.innerHTML = ""; };
    host.classList.add("on");
    try { t.open(host.querySelector(".qt-bd"), reg.ctx); } catch (e) { host.querySelector(".qt-bd").textContent = "Tabui failed: " + (e && e.message || e); }
  }

  // ---- loading (once per page): index, extra files, Python zip, then the scripts in order
  async function load() {
    const get = async (name, how) => { const r = await fetch(DIR + name); if (!r.ok) throw new Error(name + " missing"); return r[how](); };
    const idx = await get("indice.json", "json");
    const files = {};
    for (const n of idx.files || []) files[n] = await get(n, "text");
    const pyZip = idx.py ? new Uint8Array(await get(idx.py, "arrayBuffer")) : null;
    reg.ctx = { pyZip, base: BASE, meta: idx.meta || {}, files };
    for (const n of idx.js || []) await runScript(n);
  }
  async function turnOn() {
    on = true; ensureUi();
    if (!loaded) loaded = load();
    try { await loaded; } catch (e) { loaded = null; console.error("Tabui failed to load:", e); return; }
    if (!on) return;
    for (const t of tools) if (t.nav) addTab(t);
    render();
  }
  function turnOff() {
    on = false; removeTabs();
    if (host) { host.classList.remove("on"); host.innerHTML = ""; }
    render();
  }

  // ---- switch: logo-piccone.js (the five hits on the logo of the header) calls QTOOLS.flip()
  /** @type {any} */ (reg).flip = () => { if (on) { setOn(false); turnOff(); return false; } setOn(true); turnOn(); return true; };      // gives the new state (the logo follows it)
  const start = () => { if (getOn()) turnOn(); };
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start); else start();
})();
