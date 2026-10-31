// @ts-strict
"use strict";
// The folder pointed at and remembered (docs/agenti/dati-cartella.md). The user points at a folder of the computer ONCE (showDirectoryPicker); the handle is kept in
// IndexedDB "qqq_lab" (store `kv`, key `cartella.puntata`) and found again at the next visit. The compatible files it holds appear GREY («not loaded») and a click
// loads one: nothing is copied before that, the file is read through the handle. Four kinds (kindOf): data (mzML, raw, wiff), method (.dam), library, table; data are listed
// under «Data», the others under «Others». The list follows the folder: FileSystemObserver where it exists, otherwise a light rescan when the tab is shown again and every 20 s.
// Pure parts (kindOf, collect, pairScan, visible) have no DOM and are tested in node (tests/test_cartella_puntata.py). Desktop app: cartella-puntata-desktop.js.

// the warm-up of the folder (riscalda.js), once it is loaded
const riscaldaDi = () => /** @type {any} */ (window).RISCALDA;

const CartPP = (() => {
  const KEY = "cartella.puntata", MAXD = 3, CAP = 3000, POLL = 20000;
  const HIDDEN = n => n[0] === "." || n === "__MACOSX";
  /** @type {[string, RegExp][]} */
  const TIPI = [["data", /\.(mzml|raw|wiff)$/i], ["method", /\.dam$/i], ["library", /\.(msp|mgf|mzlib|jsonl|ndjson)$/i], ["table", /\.(csv|tsv)$/i]];
  const kindOf = name => { for (const [t, re] of TIPI) if (re.test(name)) return t; return null; };

  // list of a directory handle (also a fake one in the tests): [{path, name, dir, kind, h}], hidden folders skipped, depth and number limited
  async function collect(h, base = "", depth = 0, out = []) {
    for await (const [name, e] of h.entries()) {
      if (out.length >= CAP) break;
      if (HIDDEN(name)) continue;
      if (e.kind === "directory") { if (depth < MAXD) await collect(e, base + name + "/", depth + 1, out); continue; }
      const t = kindOf(name);
      if (t) out.push({ path: base + name, name, dir: base, kind: t, h: e });
      else if (/\.wiff\.scan$/i.test(name)) out.push({ path: base + name, name, dir: base, kind: "scan", h: e });
    }
    return out;
  }
  // the .wiff.scan of a .wiff (same folder, same name); a .scan that has its .wiff is not a row of its own
  const pairScan = (items, w) => items.find(x => x.kind === "scan" && x.dir === w.dir && x.name.toLowerCase() === w.name.toLowerCase() + ".scan") || null;
  // one pass: the scans that pair with a .wiff are collected once (first scan per folder and name, as pairScan finds it), then dropped
  const visible = items => {
    const key = (dir, name) => dir + "\0" + name.toLowerCase(), scans = new Map(), paired = new Set();
    for (const x of items) if (x.kind === "scan" && !scans.has(key(x.dir, x.name))) scans.set(key(x.dir, x.name), x);
    for (const w of items) if (/\.wiff$/i.test(w.name)) { const sc = scans.get(key(w.dir, w.name + ".scan")); if (sc) paired.add(sc); }
    return items.filter(x => !paired.has(x));
  };
  const sameList = (a, b) => a.length === b.length && a.every((x, i) => x.path === b[i].path);

  /** @type {{ handle: any, name: string, perm: string, items: any[], opened: Set<any>, stats: Map<any, any>, busy: boolean, gen: number, opening: boolean, obs: any, timer: any, native: any, deb?: any }} */
  // `gen` changes with the folder (choose, forget): a scan that started on the old one drops its result; `opening` is "Load all" running
  const S = { handle: null, name: "", perm: "none", items: [], opened: new Set(), stats: new Map(), busy: false, gen: 0, opening: false, obs: null, timer: 0, native: null };
  const supported = () => typeof window !== "undefined" && !!window.CAPACITA && CAPACITA.ha("cartellaLocale") && !!/** @type {any} */ (window).showDirectoryPicker;
  return { kindOf, collect, pairScan, visible, sameList, S, supported };
})();

// the list in the page: rows, groups, messages (the buttons call CartPunt when they are pressed)
const CartPuntVista = (() => {
  const { visible, S, supported } = CartPP;
  const loadedNames = () => new Set((/** @type {any} */ (window).ST && ST.files || []).map(f => f.name.toLowerCase()));
  // a file counts as loaded by its name only when no other file of the folder has that name (pos/qc.mzML and neg/qc.mzML are told apart by path)
  const sameNames = () => { const m = new Map(); for (const x of S.items) { const k = x.name.toLowerCase(); m.set(k, (m.get(k) || 0) + 1); } return m; };
  const loadedTest = () => { const names = loadedNames(), same = sameNames(); return x => S.opened.has(x.path) || (names.has(x.name.toLowerCase()) && same.get(x.name.toLowerCase()) === 1); };
  // a click whose pointer-down and pointer-up straddle a redraw is lost: no redraw while the pointer is down on the list
  let down = false, late = false;
  // ---- page
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  function rowHtml(x, i, done) {
    const T = (k, p) => I18N.t(k, p), st = S.stats.get(x.path), size = st && typeof sz === "function" ? " · " + sz(st.size) : "";
    const state = T(done ? "cartp.loaded" : "cartp.notloaded");
    const one = done ? "" : `<button type="button" class="sec sm cpone" data-i="${i}" aria-label="${esc(T("cartp.one.aria", { name: x.name }))}">${T("cartp.one")}</button>`;
    return `<li class="cpli"><button type="button" class="cprow${done ? " done" : ""}" data-i="${i}" aria-label="${esc(T("cartp.row.label", { name: x.name, state }))}"><span class="cpn">${esc(x.dir + x.name)}</span><span class="cpm">${esc(state + size)}</span></button>${one}</li>`;
  }
  function render() {
    const box = document.getElementById("cartbody"), pick = document.getElementById("cartpick");
    if (!box || !pick) return;
    if (down) { late = true; return; }
    late = false;
    const T = (k, p) => I18N.t(k, p), nat = !!S.native;
    pick.hidden = !!window.QQQ_PHONE; box.hidden = !!window.QQQ_PHONE;
    // no folder yet: the icon is the way to choose one
    if ((!nat && (!supported() || !S.handle)) || (nat && !S.name)) { box.innerHTML = ""; return; }
    const head = `<div class="sm"><b>${T("cartp.where", { name: esc(S.name) })}</b> <button type="button" class="sec" id="cartforget">${T("cartp.forget")}</button> <button type="button" class="sec" id="cartrefresh">${T("cartp.refresh")}</button></div>`;
    // the visible list, the index of each item and the loaded files are computed once per redraw
    const vis = visible(S.items), idx = new Map(S.items.map((x, i) => [x, i])), isLoaded = loadedTest();
    let body;
    if (!nat && S.perm !== "granted") body = `${S.perm === "denied" ? `<p class="sm">${T("cartp.denied", { name: esc(S.name) })}</p>` : ""}<button type="button" class="go" id="cartgrant">${T("cartp.grant.btn")}</button>`;
    else {
      const data = vis.filter(x => x.kind === "data" || x.kind === "scan");
      const list = a => `<ul class="cplist">${a.map(x => rowHtml(x, idx.get(x), isLoaded(x))).join("")}</ul>`;
      body = !data.length ? `<p class="sm muted">${T("cartp.empty")}</p>`
        : `<h4>${T("cartp.data")} <span class="muted">${T("cartp.count", { n: data.length })}</span></h4>${list(data)}`;
    }
    const todo = vis.filter(x => x.kind === "data" && !isLoaded(x)), go = (nat || S.perm === "granted") && todo.length ? `<button type="button" class="go" id="cartgo"${S.opening ? " disabled" : ""}>${T("cartp.all")}</button>` : "";
    box.innerHTML = head + go + `<div role="status" aria-live="polite">${body}</div>`;
    const cg = box.querySelector("#cartgo"); if (cg) cg.onclick = async () => {
      if (S.opening) return;
      S.opening = true; cg.disabled = true;
      try { for (const x of todo) await CartPunt.open(x); } finally { S.opening = false; render(); }
    };
    box.querySelector("#cartforget").onclick = () => CartPunt.forget(); box.querySelector("#cartrefresh").onclick = () => CartPunt.rescan();
    const g = box.querySelector("#cartgrant"); if (g) g.onclick = () => CartPunt.grant();
    box.querySelectorAll(".cprow, .cpone").forEach(b => { b.onclick = () => CartPunt.open(S.items[+b.dataset.i]); });
  }
  if (typeof document !== "undefined") {
    const up = () => { if (!down) return; down = false; if (late) render(); };
    document.addEventListener("pointerdown", e => { const t = /** @type {any} */ (e.target); down = !!(t && t.closest && t.closest("#cartbody")); }, true);
    document.addEventListener("pointerup", up, true);
    document.addEventListener("pointercancel", up, true);
    addEventListener("blur", up);
  }
  return { render };
})();

// opening one file: the only moment its bytes are read (through the handle, or through the native side in the desktop app)
const CartPuntApri = (() => {
  const { pairScan, S } = CartPP;
  async function fileOf(x) { return S.native ? S.native.fileOf(x) : x.h.getFile(); }
  async function open(x) {
    const T = (k, p) => I18N.t(k, p);
    try {
      if (riscaldaDi()) await riscaldaDi().claim(x.path);       // a warm-up of this very file under way finishes first; one waiting for its turn is dropped
      const f = CartCarica.setDir(await fileOf(x), x.dir);
      const sc = /\.wiff$/i.test(x.name) ? pairScan(S.items, x) : null;
      await upload(sc ? [f, CartCarica.setDir(await fileOf(sc), x.dir)] : [f], "data");
      S.opened.add(x.path);
    } catch (e_) { const e = /** @type {Lanciato} */ (e_); CartCarica.note(T("cartp.error", { name: x.name, message: String(e.message || e) })); }
    CartPuntVista.render();
  }

  // the mzML of the folder are prepared in the background once the list is shown and the access is granted (riscalda.js, loaded here on first use)
  function warm() {
    if (window.QQQ_PHONE || !(S.native || S.perm === "granted") || !S.items.length) return;
    const items = CartPP.visible(S.items).filter(x => x.kind === "data" && /\.mzml$/i.test(x.name)).map(x => ({ key: x.path, name: x.name, size: (S.stats.get(x.path) || {}).size, get: () => fileOf(x) }));
    if (items.length) carica("m:riscalda.js").then(() => riscaldaDi().offer(items)).catch(() => {});
  }
  return { open, warm };
})();

// permission and scan of the folder (the native side of the desktop app scans on its own)
const CartPuntScan = (() => {
  const { collect, sameList, S } = CartPP, render = () => CartPuntVista.render();
  async function perm(ask) {
    if (!S.handle) return "none";
    try {
      let p = await S.handle.queryPermission({ mode: "read" });
      if (p !== "granted" && ask) p = await S.handle.requestPermission({ mode: "read" });
      return p;
    } catch (e) { return "denied"; }
  }
  async function rescan() {
    if (S.native) { await S.native.rescan(); return CartPuntApri.warm(); }
    if (!S.handle || S.busy) return;
    S.busy = true;
    const gen = S.gen, h = S.handle, stale = () => gen !== S.gen;
    try {
      const p = await perm(false);
      if (stale()) return;
      S.perm = p;
      if (S.perm === "granted") {
        const items = (await collect(h)).sort((a, b) => a.path.localeCompare(b.path));
        if (stale()) return;
        const changed = !sameList(items, S.items); S.items = items;
        if (changed) render();
        await fillStats(items, gen);
      } else S.items = [];
    } catch (e) { if (!stale()) { S.items = []; S.perm = "denied"; } }
    finally {
      S.busy = false;
      // the folder changed while this scan ran: its result was dropped, scan the new one
      if (stale()) rescan(); else { render(); CartPuntApri.warm(); }
    }
  }
  // size and date of the files seen for the first time (getFile reads no content)
  // getFile in parallel batches (order kept), a redraw after every 25 files
  async function fillStats(items, gen) {
    const BATCH = 25, todo = items.filter(x => !S.stats.has(x.path));
    for (let i = 0; i < todo.length; i += BATCH) {
      const batch = todo.slice(i, i + BATCH);
      const got = await Promise.all(batch.map(x => x.h.getFile().then(f => ({ size: f.size, mtime: f.lastModified }), () => ({ size: 0, mtime: 0 }))));
      if (gen !== S.gen) return;
      batch.forEach((x, j) => S.stats.set(x.path, got[j]));
      if (batch.length === BATCH) render();
    }
  }
  return { perm, rescan };
})();

// state, permission, scan, follow and loading
const CartPunt = (() => {
  const { kindOf, collect, pairScan, visible, sameList, S, supported } = CartPP, { perm, rescan } = CartPuntScan, render = () => CartPuntVista.render();
  const KEY = "cartella.puntata", POLL = 20000;
  async function db() { const { lab } = await import("./qqq-db.js"); return lab(() => window.dispatchEvent(new Event("mzlab:db-blocked")))(); }
  const idb = (mode, f) => db().then(d => new Promise((ok, no) => { const r = f(d.transaction("kv", mode).objectStore("kv")); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); }));
  const saved = () => idb("readonly", s => s.get(KEY)).catch(() => null);
  const keep = h => idb("readwrite", s => (h ? s.put(h, KEY) : s.delete(KEY))).catch(() => null);

  // ---- follow
  function follow() {
    stop();
    if (!S.handle || S.perm !== "granted") return;
    if (typeof FileSystemObserver === "function") {
      try { S.obs = new FileSystemObserver(() => { clearTimeout(S.deb); S.deb = setTimeout(rescan, 600); }); S.obs.observe(S.handle, { recursive: true }).catch(() => { S.obs = null; poll(); }); return; } catch (e) { S.obs = null; }
    }
    poll();
  }
  const poll = () => { S.timer = setInterval(() => { if (!document.hidden) rescan(); }, POLL); };
  function stop() { clearInterval(S.timer); clearTimeout(S.deb); if (S.obs) { try { S.obs.disconnect(); } catch (e) { /* gone */ } S.obs = null; } }

  async function start() {
    if (S.native === null) S.native = (typeof CartPuntDesk !== "undefined" && CartPuntDesk.available()) ? CartPuntDesk : false;
    if (S.native) { await S.native.start(S, render); render(); return CartPuntApri.warm(); }
    if (supported()) { S.handle = await saved(); if (S.handle) { S.name = S.handle.name; await rescan(); follow(); } }
    render();
  }
  async function choose() {
    if (S.native) return S.native.choose();
    try {
      const h = await /** @type {any} */ (window).showDirectoryPicker({ id: "mzlab-dati", startIn: "desktop", mode: "read" });
      S.gen++; S.handle = h; S.name = h.name; S.items = []; S.stats.clear(); S.opened.clear(); await keep(h); await rescan(); follow();
    } catch (e) { /* cancelled */ }
  }
  async function forget() { stop(); S.gen++; if (riscaldaDi()) riscaldaDi().stop(); if (S.native) await S.native.forget(); S.handle = null; S.name = ""; S.items = []; S.perm = "none"; S.stats.clear(); S.opened.clear(); await keep(null); render(); }
  async function grant() { S.perm = await perm(true); if (S.perm === "granted") { await rescan(); follow(); } else render(); }

  if (typeof document !== "undefined") {
    addEventListener("load", () => CartPunt.start());
    document.addEventListener("visibilitychange", () => { if (!document.hidden && S.handle) rescan(); });
    addEventListener("focus", () => { if (S.handle && !S.obs) rescan(); });
  }
  return { kindOf, collect, pairScan, visible, sameList, state: S, start, choose, forget, rescan, grant, render, open: CartPuntApri.open };
})();
if (typeof module !== "undefined") module.exports = CartPunt;
