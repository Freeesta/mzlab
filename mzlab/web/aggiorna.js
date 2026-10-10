// Desktop app only (window.MZLAB_DESKTOP, set by desktop-shim.js): update of the web part. The native side does the work
// (crates/mzlab-desktop/src/update.rs: manifest, only the changed files, SHA-256, atomic swap); this file only asks and shows.
// In a browser nothing here runs. A silent check at start (at most once a day) shows a small notice, never a dialog.
window.DESKUPD = (() => {
  const KEY = "qqq.upd.last", DAY = 864e5, REL = "https://github.com/Freeesta/mzlab/actions/workflows/desktop.yml";
  const T = () => window.__TAURI__;
  const live = () => !!(window.MZLAB_DESKTOP && T() && T().core);
  const mb = b => I18N.fix(b / 1e6, 1);
  let box = null, busy = false;

  function show(html, buttons, title) {
    if (!box || !box.isConnected) {
      box = document.createElement("div"); box.id = "updbox"; box.setAttribute("role", "status"); box.setAttribute("aria-live", "polite");
      box.style.cssText = "position:fixed;left:12px;bottom:12px;z-index:7000;max-width:360px;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;padding:8px 10px;box-shadow:0 2px 8px rgba(0,0,0,.2)";
      document.body.appendChild(box);
    }
    box.innerHTML = (title ? `<div style="font-weight:600;margin-bottom:4px">${title}</div>` : "") + `<div>${html}</div>` +
      `<div style="display:flex;gap:6px;margin-top:6px;justify-content:flex-end">${buttons.map(b => `<button type="button" data-a="${b[0]}">${b[1]}</button>`).join("")}</div>`;
    box.querySelectorAll("button").forEach(b => { b.onclick = () => ACT[b.dataset.a](); });
    return box;
  }
  const close = () => { if (box) { box.remove(); box = null; } };
  const closeBtn = () => ["close", I18N.t("update.close")];
  const errText = r => {
    const k = String(r.error_key || "");
    return I18N.t(k === "update.error.offline" ? "update.error.offline" : k === "update.error.http" ? "update.error.http" : k === "update.error.manifest" ? "update.error.manifest"
      : k === "update.error.hash" ? "update.error.hash" : k === "update.error.newapp" ? "update.error.newapp" : "update.error.io");
  };

  function offer(r, silent) {
    if (r.status === "uptodate") { if (silent) return; show(EH(I18N.t("update.uptodate")), [closeBtn()]); }
    else if (r.status === "available") {
      show(EH(I18N.t("update.available", { files: r.files, size: mb(r.bytes) })), [["go", I18N.t("update.download")], closeBtn()], silent ? I18N.t("update.notice") : "");
    } else if (r.status === "newapp") {
      show(`${EH(I18N.t("update.newapp", { min: r.min }))}<br><a href="${REL}" target="_blank" rel="noopener noreferrer">${REL}</a>`, [closeBtn()]);
    } else if (!silent) show(EH(errText(r)), [closeBtn()]);
  }

  async function check(silent) {
    if (!live() || busy) return;
    busy = true;
    try { localStorage.setItem(KEY, String(Date.now())); } catch (_) { /* optional */ }
    if (!silent) show(EH(I18N.t("update.checking")), []);
    try { offer(JSON.parse(await T().core.invoke("update_check")), silent); }
    catch (_) { if (!silent) show(EH(I18N.t("update.error.io")), [closeBtn()]); }
    busy = false;
  }

  async function go() {
    if (busy) return;
    busy = true;
    show(EH(I18N.t("update.checking")), []);
    let un = null;
    try {
      if (T().event) un = await T().event.listen("update-progress", ev => {
        const p = ev.payload || {};
        show(EH(I18N.t("update.progress", { done: p.done, total: p.total, size: mb(p.bytes || 0) })), []);
      });
      const r = JSON.parse(await T().core.invoke("update_apply"));
      if (r.status === "ready") show(EH(I18N.t("update.ready")), [["restart", I18N.t("update.restart")], closeBtn()]);
      else show(EH(errText(r)), [closeBtn()]);
    } catch (_) { show(EH(I18N.t("update.error.io")), [closeBtn()]); }
    if (un) un();
    busy = false;
  }

  const ACT = { close, go, restart: () => T().core.invoke("update_restart") };

  // row of the settings popup (only in the desktop app)
  const row = () => live() ? `<div class="row" title="${I18N.t("update.settings.title")}"><span>${I18N.t("update.settings.label")}</span> <button id="uip-upd" type="button">${I18N.t("update.settings.button")}</button></div>` : "";
  const bind = d => { const b = d.querySelector("#uip-upd"); if (b) b.onclick = () => { d.remove(); check(false); }; };

  function auto() {
    if (!live()) return;
    let last = 0; try { last = +localStorage.getItem(KEY) || 0; } catch (_) { /* optional */ }
    if (Date.now() - last > DAY) setTimeout(() => check(true), 4000);
  }
  addEventListener("load", auto);
  return { row, bind, check };
})();
