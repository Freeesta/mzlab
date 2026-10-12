"use strict";
// The folder pointed at and remembered (docs/agenti/dati-cartella.md). The user points at a folder of the computer ONCE (showDirectoryPicker); the handle is kept in
// IndexedDB "qqq_lab" (store `kv`, key `cartella.puntata`) and found again at the next visit. The compatible files it holds appear GREY («not loaded») and a click
// loads one: nothing is copied before that, the file is read through the handle. Four kinds (kindOf): data (mzML, raw, wiff), method (.dam), library, table; data are listed
// under «Data», the others under «Others». The list follows the folder: FileSystemObserver where it exists, otherwise a light rescan when the tab is shown again and every 20 s.
// Pure parts (kindOf, collect, pairScan, visible) have no DOM and are tested in node (tests/test_cartella_puntata.py). Desktop app: cartella-puntata-desktop.js.

const CartPunt = (() => {
  const KEY = "cartella.puntata", MAXD = 3, CAP = 3000, POLL = 20000;
  const HIDDEN = n => n[0] === "." || n === "__MACOSX";
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
  const visible = items => items.filter(x => x.kind !== "scan" || !items.some(w => /\.wiff$/i.test(w.name) && pairScan(items, w) === x));
  const sameList = (a, b) => a.length === b.length && a.every((x, i) => x.path === b[i].path);

  // ---- state and storage
  const S = { handle: null, name: "", perm: "none", items: [], opened: new Set(), stats: new Map(), busy: false, obs: null, timer: 0, native: null };
  const supported = () => typeof window !== "undefined" && !!window.showDirectoryPicker;
  async function db() { const { lab } = await import("./qqq-db.js"); return lab()(); }
  const idb = (mode, f) => db().then(d => new Promise((ok, no) => { const r = f(d.transaction("kv", mode).objectStore("kv")); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); }));
  const saved = () => idb("readonly", s => s.get(KEY)).catch(() => null);
  const keep = h => idb("readwrite", s => (h ? s.put(h, KEY) : s.delete(KEY))).catch(() => null);

  // ---- permission, scan, follow
  async function perm(ask) {
    if (!S.handle) return "none";
    try {
      let p = await S.handle.queryPermission({ mode: "read" });
      if (p !== "granted" && ask) p = await S.handle.requestPermission({ mode: "read" });
      return p;
    } catch (e) { return "denied"; }
  }
  async function rescan() {
    if (S.native) return S.native.rescan();
    if (!S.handle || S.busy) return;
    S.busy = true;
    try {
      S.perm = await perm(false);
      if (S.perm === "granted") {
        const items = (await collect(S.handle)).sort((a, b) => a.path.localeCompare(b.path));
        const changed = !sameList(items, S.items); S.items = items;
        if (changed) render();
        await fillStats(items);
      } else S.items = [];
    } catch (e) { S.items = []; S.perm = "denied"; }
    finally { S.busy = false; render(); }
  }
  async function fillStats(items) {          // size and date of the files seen for the first time (getFile reads no content)
    let n = 0;
    for (const x of items) {
      if (S.stats.has(x.path)) continue;
      try { const f = await x.h.getFile(); S.stats.set(x.path, { size: f.size, mtime: f.lastModified }); } catch (e) { S.stats.set(x.path, { size: 0, mtime: 0 }); }
      if (++n % 25 === 0) render();
    }
  }
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
    if (S.native) { await S.native.start(S, render); return render(); }
    if (supported()) { S.handle = await saved(); if (S.handle) { S.name = S.handle.name; await rescan(); follow(); } }
    render();
  }
  async function choose() {
    if (S.native) return S.native.choose();
    try {
      const h = await window.showDirectoryPicker({ id: "mzlab-dati", mode: "read" });
      S.handle = h; S.name = h.name; S.items = []; S.stats.clear(); S.opened.clear(); await keep(h); await rescan(); follow();
    } catch (e) { /* cancelled */ }
  }
  async function forget() { stop(); if (S.native) await S.native.forget(); S.handle = null; S.name = ""; S.items = []; S.perm = "none"; S.stats.clear(); S.opened.clear(); await keep(null); render(); }
  async function grant() { S.perm = await perm(true); if (S.perm === "granted") { await rescan(); follow(); } else render(); }

  // ---- loading one file (the only moment its bytes are read)
  const loadedNames = () => new Set((window.ST && ST.files || []).map(f => f.name.toLowerCase()));
  const isLoaded = x => S.opened.has(x.path) || loadedNames().has(x.name.toLowerCase());
  async function fileOf(x) { return S.native ? S.native.fileOf(x) : x.h.getFile(); }
  async function open(x) {
    const T = (k, p) => I18N.t(k, p);
    try {
      if (x.kind === "table") { CartCarica.note(T("cartp.tab.msg")); return; }
      const f = CartCarica.setDir(await fileOf(x), x.dir);
      if (x.kind === "method") await upload([f], "dam");
      else if (x.kind === "library") { await LIB.open(); await LIB.addFiles([f]); }
      else {
        const sc = /\.wiff$/i.test(x.name) ? pairScan(S.items, x) : null;
        await upload(sc ? [f, CartCarica.setDir(await fileOf(sc), x.dir)] : [f], "data");
      }
      S.opened.add(x.path);
    } catch (e) { CartCarica.note(T("cartp.error", { name: x.name, message: String(e.message || e) })); }
    render();
  }

  // ---- page
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const KIND = { method: "cartp.kind.dam", library: "cartp.kind.lib", table: "cartp.kind.tab" };
  function rowHtml(x, i) {
    const T = (k, p) => I18N.t(k, p), done = isLoaded(x), st = S.stats.get(x.path), size = st && typeof sz === "function" ? " · " + sz(st.size) : "";
    const kind = KIND[x.kind] ? ` · ${T(KIND[x.kind])}` : "", state = T(done ? "cartp.loaded" : "cartp.notloaded");
    return `<li><button type="button" class="cprow${done ? " done" : ""}" data-i="${i}" aria-label="${esc(T("cartp.row.label", { name: x.name, state }))}"><span class="cpn">${esc(x.dir + x.name)}</span><span class="cpm">${esc(state + kind + size)}</span></button></li>`;
  }
  function render() {
    const box = document.getElementById("cartbody"), card = document.getElementById("cartcard");
    if (!box || !card) return;
    const T = (k, p) => I18N.t(k, p), nat = !!S.native;
    card.hidden = !!window.QQQ_PHONE;
    const intro = `<p class="sm muted">${T("cartp.intro")}</p>`;
    if (!nat && !supported()) { box.innerHTML = intro + `<p class="sm" id="cartnosup">${T("cartp.nosupport")}</p>`; return; }
    if (!S.handle && !nat) { box.innerHTML = intro + `<button type="button" class="sec" id="cartchoose">${T("cartp.choose")}</button>`; box.querySelector("#cartchoose").onclick = choose; return; }
    if (nat && !S.name) { box.innerHTML = intro + `<button type="button" class="sec" id="cartchoose">${T("cartp.choose")}</button>`; box.querySelector("#cartchoose").onclick = choose; return; }
    const head = `<div class="sm"><b>${T("cartp.where", { name: esc(S.name) })}</b> <button type="button" class="sec" id="cartchange">${T("cartp.change")}</button> <button type="button" class="sec" id="cartforget">${T("cartp.forget")}</button> <button type="button" class="sec" id="cartrefresh">${T("cartp.refresh")}</button></div>`;
    let body;
    if (!nat && S.perm !== "granted") body = `<p class="sm">${T(S.perm === "denied" ? "cartp.denied" : "cartp.grant", { name: esc(S.name) })}</p><button type="button" class="go" id="cartgrant">${T("cartp.grant.btn")}</button>`;
    else {
      const vis = visible(S.items), idx = x => S.items.indexOf(x), data = vis.filter(x => x.kind === "data" || x.kind === "scan"), other = vis.filter(x => x.kind !== "data" && x.kind !== "scan");
      const list = a => `<ul class="cplist">${a.map(x => rowHtml(x, idx(x))).join("")}</ul>`;
      body = !vis.length ? `<p class="sm muted">${T("cartp.empty")}</p>`
        : `<h4>${T("cartp.data")} <span class="muted">${T("cartp.count", { n: data.length })}</span></h4>${data.length ? list(data) : ""}`
          + `<h4>${T("cartp.others")} <span class="muted">${T("cartp.count", { n: other.length })}</span></h4><p class="sm muted">${T("cartp.others.hint")}</p>${other.length ? list(other) : ""}`;
    }
    box.innerHTML = intro + head + `<div role="status" aria-live="polite">${body}</div>`;
    box.querySelector("#cartchange").onclick = choose; box.querySelector("#cartforget").onclick = forget; box.querySelector("#cartrefresh").onclick = rescan;
    const g = box.querySelector("#cartgrant"); if (g) g.onclick = grant;
    box.querySelectorAll(".cprow").forEach(b => { b.onclick = () => open(S.items[+b.dataset.i]); });
  }
  if (typeof document !== "undefined") {
    addEventListener("load", () => CartPunt.start());
    document.addEventListener("visibilitychange", () => { if (!document.hidden && S.handle) rescan(); });
    addEventListener("focus", () => { if (S.handle && !S.obs) rescan(); });
  }
  return { kindOf, collect, pairScan, visible, sameList, state: S, start, choose, forget, rescan, render, open };
})();
if (typeof module !== "undefined") module.exports = CartPunt;
