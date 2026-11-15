// @ts-check
// Browser only, `?nav=cache` (sw.js): a newer version of the site is complete in its cache and takes over at the next page load. One polite line
// says so and offers the reload; it stays until then (no timer). Imported by browser.js only when the service worker announces it.

/** Shows the line once per page. */
export function mostra() {
  if (document.getElementById("sw-nuova")) return;
  const el = document.createElement("div");
  el.id = "sw-nuova"; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite");
  el.style.cssText = "position:fixed;left:12px;bottom:12px;z-index:7000;display:flex;gap:10px;align-items:center;background:var(--panel);color:var(--ink);border:1px solid var(--line);border-radius:6px;padding:6px 10px;font:14px system-ui,sans-serif";
  const t = document.createElement("span"), b = document.createElement("button");
  t.setAttribute("data-i18n", "update.siteNew"); t.textContent = I18N.t("update.siteNew");
  b.type = "button"; b.setAttribute("data-i18n", "update.siteReload"); b.textContent = I18N.t("update.siteReload");
  b.style.cssText = "min-height:24px;min-width:24px";
  b.onclick = () => location.reload();
  el.append(t, b);
  document.body.appendChild(el);
}
