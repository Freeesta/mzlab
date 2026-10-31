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
  window.MZLAB_TOFILE = toFile;                                                       // for the files opened from outside (predefinito.js)
  async function deliver(files, into) {
    if (!files.length || typeof window.upload !== "function") return;
    window.upload(files.map(toFile), into);
  }

  // «Scarica mzML»: the native save dialog for a converted mzML (by its number on the disk); resolves to the chosen path or null
  window.MZLAB_SAVE_MZML = (id, name) => T.core.invoke("save_mzml", { id, name });
  window.QQQ_SAVE_MZML = window.MZLAB_SAVE_MZML;
  // «Save the mzML files…»: the system folder dialog, then every mzML the background queue has ready is put there; resolves to how many, null if the user cancelled
  window.MZLAB_SAVE_MZML_DIR = () => T.core.invoke("save_mzml_folder");

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

  // macOS: the title bar is an overlay, so the header is the title bar (desktop-titlebar.css): the empty part of it drags the window, a double click zooms it,
  // and in full screen (traffic lights hidden) the room kept for them is given back
  if (/Macintosh/.test(navigator.userAgent)) {
    const de = document.documentElement, W = T.window && T.window.getCurrentWindow();
    de.classList.add("desk-mac");
    const fs = async () => {
      let on = !!document.fullscreenElement;
      try { on = on || await W.isFullscreen(); } catch (_) { /* optional */ }
      de.classList.toggle("desk-fs", on);
    };
    document.addEventListener("fullscreenchange", fs);
    try { W.onResized(fs); } catch (_) { /* optional */ }
    fs();
    const bare = e => e.target === e.currentTarget || !(e.target.closest && e.target.closest("button,a,input,select,textarea,label,nav,#stbar,.hq"));
    addEventListener("DOMContentLoaded", () => {
      const h = document.querySelector("header");
      if (!h) return;
      h.addEventListener("mousedown", e => { if (e.button === 0 && e.detail === 1 && bare(e)) W.startDragging().catch(() => {}); });
      h.addEventListener("dblclick", e => { if (bare(e)) W.toggleMaximize().catch(() => {}); });
    });
  }

  // the window title is the page's (the name is written only in appname.js)
  const title = () => { try { T.window.getCurrentWindow().setTitle(document.title); } catch (_) { /* optional */ } };
  addEventListener("DOMContentLoaded", () => {
    title();
    const el = document.querySelector("title");
    if (el) new MutationObserver(title).observe(el, { childList: true });
  });
})();
