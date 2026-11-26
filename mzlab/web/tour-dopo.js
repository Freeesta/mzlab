// @ts-strict
"use strict";
// The first-visit tour invitation of tour.js (TOURINVITO): built here, centred under the side panel (TOURDOPO), gone by itself after INVITO_MS; after «Not now» a bubble on the gear
// says where the tour can be taken again (gone by itself after 10 s, on a click or on Esc). Classic script.
const TOURDOPO = (() => {
  /** Makes the invitation as wide as the side panel, with equal margins at both sides; follows the panel while it opens or is resized. @param {HTMLElement} div */
  function centra(div) {
    const side = document.getElementById("dfiles");
    if (!side) return;
    const apply = () => {
      if (!div.isConnected) { stop(); return; }
      const sr = side.getBoundingClientRect();
      if (sr.width <= 180 || sr.left >= window.innerWidth / 2) return;
      div.style.left = (sr.left + 12) + "px";
      div.style.width = (sr.width - 24) + "px";
    };
    const stop = FRAME.ro(side, apply);
    apply();
  }
  /** Shows the bubble under the gear of the header (once; the invitation itself appears once per browser). */
  function mostra() {
    const gear = document.getElementById("np-set");
    if (!gear || document.getElementById("tour-later")) return;
    const b = document.createElement("div");
    b.id = "tour-later";
    b.setAttribute("role", "status");
    b.setAttribute("aria-live", "polite");
    b.textContent = I18N.t("tour.invite.later");
    document.body.appendChild(b);
    const g = gear.getBoundingClientRect();
    const w = b.offsetWidth;
    const left = Math.max(8, Math.min(window.innerWidth - w - 8, g.right - w));
    b.style.left = left + "px";
    b.style.top = (g.bottom + 10) + "px";
    b.style.setProperty("--tip", Math.max(8, Math.min(w - 20, left + w - (g.left + g.width / 2) - 5)) + "px");
    FRAME.next(() => b.classList.add("on"));
    let gone = false;
    const off = () => {
      if (gone) return;
      gone = true;
      clearTimeout(timer);
      document.removeEventListener("pointerdown", off, true);
      document.removeEventListener("keydown", onKey, true);
      b.classList.remove("on");
      const reduced = FRAME.calm();
      setTimeout(() => b.remove(), reduced ? 0 : 320);
    };
    /** @param {KeyboardEvent} e */
    const onKey = e => { if (e.key === "Escape") off(); };
    const timer = setTimeout(off, 10000);
    // the click that closed the invitation is already over: listen from the next turn
    setTimeout(() => { document.addEventListener("pointerdown", off, true); document.addEventListener("keydown", onKey, true); }, 0);
  }
  return { centra, mostra };
})();
const TOURINVITO = (() => {
  // the invitation leaves by itself: long enough to read two buttons, short enough not to sit over the data for good
  const INVITO_MS = 30000;
  /** Shows the non-modal invitation; «Start» calls `start`. Returns the function that closes it (idempotent: timer, key listener and node go).
   * @param {() => void} start @returns {() => void} */
  function apri(start) {
    const div = document.createElement("div");
    div.id = "tour-invito";
    div.setAttribute("role", "dialog");
    div.setAttribute("aria-modal", "false");
    div.setAttribute("aria-label", I18N.t("tour.settings.title"));
    div.style.cssText = "position:fixed;bottom:12px;left:12px;z-index:7000;width:260px;max-width:calc(100vw - 24px);background:var(--panel,#fff);color:var(--ink,#25282c);border:1px solid var(--line,#e2e5ea);border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,0.18);padding:12px 14px;box-sizing:border-box;font:13px/1.4 system-ui,-apple-system,sans-serif";
    div.innerHTML = `
      <p style="margin:0 0 10px;line-height:1.45">${I18N.t("tour.invite.text")}</p>
      <div style="display:flex;gap:8px;justify-content:flex-end">
        <button type="button" id="tour-inv-no" style="background:var(--panel,#fff);color:var(--ink,#25282c);border:1px solid var(--line,#e2e5ea);border-radius:5px;padding:4px 10px;cursor:pointer">${I18N.t("tour.invite.no")}</button>
        <button type="button" id="tour-inv-si" style="background:var(--accent,#2b5c8a);color:#fff;border:1px solid var(--accent,#2b5c8a);border-radius:5px;padding:4px 12px;font-weight:500;cursor:pointer">${I18N.t("tour.invite.yes")}</button>
      </div>
    `;
    document.body.appendChild(div); TOURDOPO.centra(div);
    let idle = 0, left = INVITO_MS, since = 0;
    const close = () => {
      clearTimeout(idle); idle = 0;
      if (div.parentNode) div.remove();
      document.removeEventListener("keydown", onKey);
    };
    // no answer counts as «Not now» (the invitation is already marked as offered): same bubble on the gear, focus left where it is
    const run = () => { if (idle || !div.parentNode) return; since = Date.now(); idle = setTimeout(() => { close(); TOURDOPO.mostra(); }, left); };
    const pause = () => { if (!idle) return; clearTimeout(idle); idle = 0; left = Math.max(1000, left - (Date.now() - since)); };
    /** @param {KeyboardEvent} e */
    const onKey = e => { if (e.key === "Escape") { close(); TOURDOPO.mostra(); } };
    run(); document.addEventListener("keydown", onKey);
    try { /** @type {HTMLElement} */ (div.querySelector("#tour-inv-si")).focus({ preventScroll: true }); } catch (_) { /* focus is only a help: the invitation works without it */ }
    // whoever is reading or choosing keeps it: the countdown stops while the pointer or a focus move is inside and goes on after
    // (the initial programmatic focus above is not an interaction, or a keyboard user who never moves would keep it forever)
    let hover = false, focusIn = false;
    const sync = () => (hover || focusIn) ? pause() : run();
    div.addEventListener("pointerenter", () => { hover = true; sync(); });
    div.addEventListener("pointerleave", () => { hover = false; sync(); });
    div.addEventListener("focusin", () => { focusIn = true; sync(); });
    div.addEventListener("focusout", e => { focusIn = div.contains(/** @type {Node|null} */ (e.relatedTarget)); sync(); });
    /** @type {HTMLElement} */ (div.querySelector("#tour-inv-si")).onclick = () => { close(); start(); };
    /** @type {HTMLElement} */ (div.querySelector("#tour-inv-no")).onclick = () => {
      close(); TOURDOPO.mostra();
      try { document.getElementById("np-set")?.focus({ preventScroll: true }); } catch (_) { /* focus is only a help: the settings panel works without it */ }
    };
    return close;
  }
  return { apri };
})();
// The question at the end of a tour that loaded the example file itself: keep playing with it, or close it and start fresh.
const TOURSCELTA = (() => {
  /** Opens the modal question. `keep` and `remove` run after the dialog is gone; Esc (or a click outside) is «Keep exploring», the choice that destroys nothing.
   * @param {{ keep: () => void, remove: () => Promise<void> | void }} o @returns {() => void} closes it without choosing */
  function apri(o) {
    const back = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const wrap = document.createElement("div");
    wrap.id = "tour-scelta";
    wrap.style.cssText = "position:fixed;inset:0;z-index:7100;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,0.35);padding:12px;box-sizing:border-box";
    const bs = "border-radius:5px;padding:6px 12px;cursor:pointer;font:inherit;border:1px solid var(--line,#e2e5ea);background:var(--panel,#fff);color:var(--ink,#25282c)";
    const T = (/** @type {string} */ k) => I18N.t(k);
    const esc = (/** @type {string} */ s) => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
    wrap.innerHTML = `<div id="tour-scelta-dlg" role="dialog" aria-modal="true" aria-labelledby="tour-scelta-ttl" aria-describedby="tour-scelta-txt" style="width:340px;max-width:100%;background:var(--panel,#fff);color:var(--ink,#25282c);border:1px solid var(--line,#e2e5ea);border-radius:8px;box-shadow:0 8px 28px rgba(0,0,0,0.3);padding:16px 18px;box-sizing:border-box;font:13px/1.45 system-ui,-apple-system,sans-serif">
      <h3 id="tour-scelta-ttl" style="margin:0 0 6px;font-size:15px">${T("tour.choice.title")}</h3>
      <p id="tour-scelta-txt" style="margin:0 0 14px">${T("tour.choice.text")}</p>
      <div style="display:flex;gap:8px;justify-content:flex-end;flex-wrap:wrap">
        <button type="button" id="tour-sc-via" style="${bs}" title="${esc(T("tour.choice.leave.tip"))}">${T("tour.choice.leave")}</button>
        <button type="button" id="tour-sc-resta" style="${bs};background:var(--accent,#2b5c8a);color:#fff;border-color:var(--accent,#2b5c8a);font-weight:500" title="${esc(T("tour.choice.stay.tip"))}">${T("tour.choice.stay")}</button>
      </div></div>`;
    document.body.appendChild(wrap);
    const dlg = /** @type {HTMLElement} */ (wrap.querySelector("#tour-scelta-dlg"));
    const resta = /** @type {HTMLButtonElement} */ (wrap.querySelector("#tour-sc-resta")), via = /** @type {HTMLButtonElement} */ (wrap.querySelector("#tour-sc-via"));
    let done = false;
    const close = () => {
      if (done) return;
      done = true;
      document.removeEventListener("keydown", onKey, true);
      wrap.remove();
      try { if (back && back.isConnected) back.focus({ preventScroll: true }); } catch (_) { /* focus is only a help */ }
    };
    /** @param {KeyboardEvent} e */
    const onKey = e => {
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close(); o.keep(); return; }
      if (e.key !== "Tab") return;
      // the dialog is modal: Tab stays between its two buttons
      const els = [resta, via], i = els.indexOf(/** @type {any} */ (document.activeElement));
      e.preventDefault();
      els[(i + (e.shiftKey ? -1 : 1) + els.length) % els.length].focus();
    };
    document.addEventListener("keydown", onKey, true);
    wrap.addEventListener("pointerdown", e => { if (e.target === wrap) { close(); o.keep(); } });
    resta.onclick = () => { close(); o.keep(); };
    via.onclick = async () => {
      resta.disabled = via.disabled = true; dlg.setAttribute("aria-busy", "true");
      try { await o.remove(); } finally { close(); }
    };
    resta.focus();
    return close;
  }
  return { apri };
})();
