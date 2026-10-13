// @ts-strict
"use strict";
// Desktop app side of the remembered folder (cartella-puntata.js asks for it when `window.MZLAB_DESKTOP`): the folder is chosen with the system dialog, kept by the native side
// (config.json) and listed by `data_list` (crates/mzlab-desktop/src/cartella.rs); the page only sees names and relative paths. A file is registered (`data_register`) when
// it is opened and arrives as the same stub File as a file of the system dialog (the engine reads it from the disk). There is no change notification: the list is asked again
// when the window is shown again and every 10 s.

const CartPuntDesk = (() => {
  const T = () => (window.__TAURI__ && window.__TAURI__.core) || null, POLL = 10000;
  let S = null, draw = null, timer = 0;
  const available = () => !!(window.MZLAB_DESKTOP && T());
  async function rescan() {
    if (!S) return;
    S.name = (await T().invoke("data_get_dir")) || "";
    S.perm = "granted";
    S.items = S.name ? await T().invoke("data_list") : [];
    for (const x of S.items) S.stats.set(x.path, { size: x.size, mtime: x.mtime });
    draw();
  }
  async function start(state, render) {
    S = state; draw = render; await rescan().catch(() => { S.items = []; });
    clearInterval(timer); timer = setInterval(() => { if (!document.hidden && S.name) rescan().catch(() => {}); }, POLL);
  }
  async function choose() { const n = await T().invoke("data_choose_dir"); if (n) { S.opened.clear(); await rescan(); } }
  async function forget() { await T().invoke("data_forget"); clearInterval(timer); }
  async function fileOf(x) {                                  // a stub File carrying its number in `type`, as the shim makes it for the system dialog
    const f = await T().invoke("data_register", { path: x.path });
    const file = new File([], f.name, { type: "application/x-mzlab-file;id=" + f.id });
    Object.defineProperty(file, "size", { value: f.size });
    return file;
  }
  return { available, start, rescan, choose, forget, fileOf };
})();
if (typeof module !== "undefined") module.exports = CartPuntDesk;
