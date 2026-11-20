// @ts-strict
"use strict";
// Lazy (loadScript("aggiorna-errore.js")) from aggiorna.js: an update error with the failed file under the message and a «Copy details» button
// (key, detail, commit and manifest version for a bug report).
const DESKUPDERR = (() => {
  let last = /** @type {Bag | null} */ (null), ctx = /** @type {Bag | null} */ (null);
  const KEYS = ["update.error.offline", "update.error.http", "update.error.manifest", "update.error.hash", "update.error.newapp"];
  const errText = (/** @type {Bag} */ r) => I18N.t(KEYS.includes(String(r.error_key)) ? String(r.error_key) : "update.error.io");
  const fail = (/** @type {Bag} */ c, /** @type {Bag} */ r) => {
    ctx = c; last = r;
    const d = r.detail ? `<div style="margin-top:4px;font-size:.85em;opacity:.8;word-break:break-all">${EH(String(r.detail))}</div>` : "";
    c.show(EH(errText(r)) + d, [["copy", I18N.t("update.copy")], c.closeBtn()]);
  };
  const details = () => ["mzLab " + (window.MZLAB_VERSION || "?") + " (" + (window.MZLAB_COMMIT || "?") + ")", "manifest: " + (ctx && ctx.ver() || "?"),
    "key: " + String(last && last.error_key || ""), "detail: " + String(last && last.detail || "")].join("\n");
  async function copy() {
    const b = ctx && ctx.box() && ctx.box().querySelector("[data-a=copy]");
    try { await navigator.clipboard.writeText(details()); if (b) b.textContent = I18N.t("update.copied"); } catch (_) { /* no clipboard: the text stays on screen */ }
  }
  return { fail, copy };
})();
