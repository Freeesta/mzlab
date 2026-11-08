// @ts-check
// First-visit notice in Safari (compat.js safariKind): Safari may delete the saved data after 7 days without a visit, the user's own files are never touched.
// On a Mac it recommends another browser; on an iPad every browser is WebKit, so it says instead that a page added to the Home Screen counts only the days
// of use (WebKit's rule for Home Screen web apps). Shown once per browser storage (flag in localStorage): a cleared storage shows it again, a storage that
// refuses shows it each visit and the page goes on. Calm and non-blocking: no focus is taken.
import { safariKind } from "./compat.js";

const KEY = "mzlab.safari.seen";

/** Shows the notice unless it was seen in this storage; returns the notice element, or null. @param {any} g */
export function avvisoSafari(g) {
  const d = g.document, kind = safariKind(g);
  if (!kind) return null;
  try { if (g.localStorage.getItem(KEY)) return null; } catch (_) { /* storage blocked: shown, nothing to remember */ }
  if (!d || !d.body || d.getElementById("safaribox") || !g.I18N) return null;
  const el = d.createElement("div");
  el.id = "safaribox"; el.dataset.kind = kind; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite");
  el.style.cssText = "position:fixed;right:12px;bottom:12px;z-index:7000;max-width:min(360px,calc(100vw - 24px));display:flex;gap:8px;align-items:flex-start;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;padding:8px 10px;box-shadow:0 2px 8px rgba(0,0,0,.2);font-size:13px;line-height:1.4";
  const msg = d.createElement("div"); msg.textContent = g.I18N.t(kind === "ipad" ? "safari.notice.ipad" : "safari.notice.mac");
  el.appendChild(msg);
  d.body.appendChild(el);
  try { g.localStorage.setItem(KEY, "1"); } catch (_) { /* optional */ }
  const close = typeof g.avvisoAuto === "function" ? g.avvisoAuto(el, { ms: 30000 }) : () => el.remove();
  el.addEventListener("keydown", (/** @type {KeyboardEvent} */ e) => { if (e.key === "Escape") { e.stopPropagation(); close(); } });
  return el;
}

avvisoSafari(globalThis);
