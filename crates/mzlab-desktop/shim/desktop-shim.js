// Desktop app only (prepare.py puts it before browser.js): tells the page that the engine is the native one (desktop-worker.js) and replaces
// the browser file chooser with the system dialog, so that mzML, .dam and .raw files are opened from the disk by the native engine.
// The page itself is the same as the site's. Without Tauri (a normal browser) this file does nothing.
(() => {
  const T = window.__TAURI__;
  if (!T || !T.core) return;
  window.MZLAB_DESKTOP = true;
  const R = window.MZRESET || window.QQRESET;
  if (R && R.cold()) return;
  const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
  const tag = id => "application/x-mzlab-file;id=" + id;

  // A file of the disk as the page's File: an empty stub that carries its number in `type` (the native engine reads the file from the disk, the page never
  // holds its bytes) and says its real size, which is all the page looks at.
  function toFile(f) {
    const file = new File([], f.name, { type: tag(f.id) });
    Object.defineProperty(file, "size", { value: f.size });
    return file;
  }
  async function deliver(files, into) {
    if (!files.length || typeof window.upload !== "function") return;
    window.upload(files.map(toFile), into);
  }

  // «Scarica mzML»: the native save dialog for a converted mzML (by its number on the disk); resolves to the chosen path or null
  window.MZLAB_SAVE_MZML = (id, name) => T.core.invoke("save_mzml", { id, name });
  window.QQQ_SAVE_MZML = window.MZLAB_SAVE_MZML;

  document.addEventListener("click", async e => {
    const t = e.target.closest && e.target.closest("#drop, #dropdam");
    if (!t) return;
    e.preventDefault(); e.stopImmediatePropagation();
    const dam = t.id === "dropdam";
    deliver(await T.core.invoke("pick_files", { dam }), dam ? "dam" : "data");
  }, true);

  // files dropped on the window arrive as paths (the webview does not give File objects)
  if (T.event) T.event.listen("tauri://drag-drop", async ev => {
    const d = document.getElementById("drop");
    if (!d || !d.offsetParent) return;                                                // only on the loading screen
    const files = await T.core.invoke("register_paths", { paths: (ev.payload && ev.payload.paths) || [] });
    await deliver(files.filter(f => !/\.dam$/i.test(f.name)), "data");
    await deliver(files.filter(f => /\.dam$/i.test(f.name)), "dam");
  });

  // the window title is the page's (the name is written only in appname.js)
  const title = () => { try { T.window.getCurrentWindow().setTitle(document.title); } catch (_) { /* optional */ } };
  addEventListener("DOMContentLoaded", () => {
    title();
    const el = document.querySelector("title");
    if (el) new MutationObserver(title).observe(el, { childList: true });
  });
})();
