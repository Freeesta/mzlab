// @ts-strict
// @ts-check
// Desktop app only (window.MZLAB_DESKTOP): files opened from outside the window (double click in the file manager, «Open with», a second launch while the app
// is open) and the notice «Make mzLab the default app for these files?». The native side keeps the queue, the alternating counter and the answer
// (crates/mzlab-desktop/src/apertura.rs, config.json in the app data folder); this file takes the files into the Data view and shows the dialog.
// In a browser nothing here runs.
window.DESKDEF = (() => {
  const T = () => window.__TAURI__;
  const live = () => !!(window.MZLAB_DESKTOP && T() && T().core);
  const os = () => /Windows/i.test(navigator.userAgent) ? "win" : /Mac/i.test(navigator.userAgent) ? "mac" : "linux";

  /** Empties the native queue into the Data view; the dialog follows when the native counter says it is due. */
  async function take() {
    if (!live() || window.QQ_RESETTING) return;                       // a cold start reloads the page: the files wait in the queue
    let r;
    try { r = await T().core.invoke("take_opened"); } catch (_) { return; }       // a binary without the command: nothing arrives
    const files = (r && r.files) || [];
    if (!files.length || typeof window.MZLAB_TOFILE !== "function") return;
    for (let i = 0; i < 300 && typeof window.upload !== "function"; i++) await new Promise(ok => setTimeout(ok, 100));
    if (typeof window.upload === "function") window.upload(files.map(window.MZLAB_TOFILE), "data");
    if (r.ask) ask();
  }

  const el = (/** @type {string} */ tag, /** @type {string} */ text = "") => { const e = document.createElement(tag); if (text) e.textContent = text; return e; };

  /** The dialog: Yes (shows how to do it on this system), Not now, Don't ask again. Esc is «Not now». */
  function ask() {
    if (!live()) return;
    document.getElementById("predefdlg")?.remove();
    const back = /** @type {HTMLElement | null} */ (document.activeElement);
    const d = /** @type {HTMLDialogElement} */ (el("dialog"));
    d.id = "predefdlg"; d.setAttribute("aria-labelledby", "predef-t"); d.setAttribute("aria-describedby", "predef-d");
    d.style.cssText = "max-width:440px;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;padding:14px 16px";
    const h = el("h3", I18N.t("predef.title")); h.id = "predef-t"; h.style.margin = "0 0 8px";
    const p = el("p", I18N.t("predef.text")); p.id = "predef-d"; p.style.margin = "0 0 12px";
    const bar = el("div"); bar.style.cssText = "display:flex;flex-wrap:wrap;gap:8px;justify-content:flex-end";
    d.append(h, p, bar);
    const btn = (/** @type {string} */ key, /** @type {() => void} */ fn) => {
      const b = /** @type {HTMLButtonElement} */ (el("button", I18N.t(key))); b.type = "button"; b.style.minHeight = "32px"; b.onclick = fn; bar.appendChild(b); return b;
    };
    const answer = (/** @type {string} */ a) => { try { return T().core.invoke("assoc_answer", { answer: a }); } catch (_) { return Promise.resolve(); } };
    const end = () => { d.close(); };
    const yes = btn("predef.yes", async () => {
      await answer("si");
      p.textContent = I18N.t({ win: "predef.guide.win", mac: "predef.guide.mac", linux: "predef.guide.linux" }[os()]);
      if (os() === "linux") { const c = el("code", I18N.t("predef.guide.linux.cmd")); c.style.cssText = "display:block;margin-top:6px;user-select:all;overflow-wrap:anywhere"; p.appendChild(c); }
      h.textContent = I18N.t("predef.guide.title");
      bar.replaceChildren();
      btn("predef.done", end).focus();
    });
    btn("predef.later", async () => { await answer("no"); end(); });
    btn("predef.never", async () => { await answer("mai"); end(); });
    d.addEventListener("cancel", () => { answer("no"); });            // Esc
    d.addEventListener("close", () => { d.remove(); if (back && back.isConnected) back.focus(); });
    document.body.appendChild(d);
    d.showModal(); yes.focus();
  }

  // row of the settings popup (only in the desktop app): forgets the answer and asks again at once
  const row = () => live() ? `<div class="imp-fld" title="${I18N.t("predef.settings.title")}"><span>${I18N.t("predef.settings.label")}</span><button id="uip-predef" type="button">${I18N.t("predef.settings.button")}</button></div>` : "";
  const bind = (/** @type {HTMLElement} */ s) => {
    const b = /** @type {HTMLButtonElement | null} */ (s.querySelector("#uip-predef"));
    if (b) b.onclick = async () => { s.remove(); try { await T().core.invoke("assoc_reset"); } catch (_) { return; } ask(); };
  };

  if (live()) {
    addEventListener("load", () => setTimeout(take, 0));
    if (T().event) T().event.listen("mzlab-open", () => { take(); });
  }
  return { row, bind, ask, take };
})();
