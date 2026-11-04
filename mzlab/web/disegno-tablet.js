// Ketcher on tablets (Draw tab): its hidden clipboard textarea must neither scroll the page nor be a keyboard stop, and the top bar
// must stay reachable in a narrow window (iPad split view); at a coarse pointer its small controls grow to 24 px, a long press on a tool
// shows its name, and a skip link jumps over its ~38 Tab stops. Mounted into the Ketcher iframe document by draw.js.
const CSS =
  // the textarea sits at the bottom of the iframe: focusing it made the parent scroll to it; fixed at the corner it stays invisible and harmless
  '.cliparea,[data-cliparea]{position:fixed!important;top:0!important;left:0!important}' +
  // below 600 px the top bar wraps to a second row (undo and redo stay in the first); the height change is read back by fit() on resize
  '@media (max-width:599px){html [class*="App-module_top"],html [class*="App-module_top"]>*{flex-wrap:wrap!important;row-gap:2px}html [class*="App-module_top"],html [class*="App-module_top"]>*{height:auto!important}html [class*="App-module_app"]{grid-template-rows:auto minmax(0,1fr)!important}}';   // html: wins over the nowrap of draw.js, which is added later

// targets of 24 px (WCAG 2.5.8) and the long-press tooltip (colours copied from the page's CSS variables when it opens)
const TOUCH_CSS =
  '@media (pointer:coarse){[data-testid=copy-button-dropdown-triangle]{min-width:24px!important;min-height:24px!important;display:inline-flex;align-items:center;justify-content:center}' +
  '[data-testid=close-window-button]{min-width:24px;min-height:24px;display:inline-flex;align-items:center;justify-content:center}' +
  // the drag-a-file text and the image-recognition tile (never available here) say nothing a touch user can use
  '[data-testid=open-from-image-button],[data-testid=open-from-file-button] button>div{display:none!important}}' +
  '#tp-tip{position:fixed;z-index:2147483647;max-width:80vw;padding:6px 10px;border-radius:6px;border:1px solid var(--line,#888);background:var(--panel,#fff);color:var(--ink,#222);font:14px/1.3 system-ui,sans-serif;pointer-events:none;opacity:0;visibility:hidden;transition:opacity .15s,visibility .15s}' +
  '#tp-tip[data-on]{opacity:1;visibility:visible}@media (prefers-reduced-motion:reduce){#tp-tip{transition:none}}';
const LONG = 500, MOVE = 10;       // ms and px: a press that stays put this long names the tool instead of activating it

/** Long press on a toolbar button (Ketcher's or the drop-down bar's proxy) shows its name; the tap that follows is swallowed.
 * @param {Document} d */
function longPress(d) {
  const tip = d.createElement("div"); tip.id = "tp-tip"; tip.setAttribute("role", "status"); tip.setAttribute("aria-live", "polite"); d.body.appendChild(tip);
  let timer = 0, x0 = 0, y0 = 0, fired = 0, hideT = 0;
  const hide = () => { tip.removeAttribute("data-on"); };
  const show = (/** @type {Element} */ b, /** @type {string} */ name) => {
    const cs = getComputedStyle(document.documentElement);
    for (const v of ["--panel", "--ink", "--line"]) tip.style.setProperty(v, cs.getPropertyValue(v));
    tip.textContent = name; const r = b.getBoundingClientRect(), w = d.defaultView, tw = tip.offsetWidth, th = tip.offsetHeight; if (!w) return;
    const below = r.bottom + 8 + th <= w.innerHeight;
    tip.style.left = Math.max(4, Math.min(w.innerWidth - tw - 4, r.left + r.width / 2 - tw / 2)) + "px"; tip.style.top = (below ? r.bottom + 8 : Math.max(4, r.top - 8 - th)) + "px";
    tip.setAttribute("data-on", ""); clearTimeout(hideT); hideT = window.setTimeout(hide, 2500);
  };
  d.addEventListener("touchstart", e => {
    clearTimeout(timer); fired = 0; const t = /** @type {Element | null} */ (e.target), b = e.touches.length === 1 && t && t.closest ? t.closest("button") : null;
    if (!b || !b.closest('[data-testid$="-toolbar"],#tp-bar')) return;
    const name = b.getAttribute("title") || b.getAttribute("aria-label") || ""; if (!name) return;
    x0 = e.touches[0].clientX; y0 = e.touches[0].clientY; timer = window.setTimeout(() => { fired = Date.now(); show(b, name); }, LONG);
  }, { passive: true });
  d.addEventListener("touchmove", e => { const t = e.touches[0]; if (timer && Math.hypot(t.clientX - x0, t.clientY - y0) > MOVE) { clearTimeout(timer); timer = 0; } }, { passive: true });
  d.addEventListener("touchend", e => { clearTimeout(timer); timer = 0; if (fired && e.cancelable) e.preventDefault(); }, { passive: false });     // no emulated mouse events, so no click
  d.addEventListener("touchcancel", () => { clearTimeout(timer); timer = 0; fired = 0; });
  d.addEventListener("click", e => { if (fired && Date.now() - fired < 1500) { e.preventDefault(); e.stopImmediatePropagation(); } fired = 0; }, true);       // a click that slipped through
  d.addEventListener("contextmenu", e => { if (fired && Date.now() - fired < 1500) e.preventDefault(); }, true);                                         // Android's own menu
}

// the skip link (index.html) moves focus over Ketcher's toolbars to Quick draw
document.addEventListener("click", e => { if (e.target instanceof Element && e.target.closest(".tp-skip")) { const f = document.getElementById("ex-smi"); if (f) f.focus(); } });

/** @param {Document} d  the document of the Ketcher iframe */
export function mountTablet(d) {
  if (!d.head || !d.body || d.getElementById("tp-tablet")) return;
  const st = d.createElement("style"); st.id = "tp-tablet"; st.textContent = CSS; d.head.appendChild(st);
  const tc = d.createElement("style"); tc.id = "tp-touch"; tc.textContent = TOUCH_CSS; d.head.appendChild(tc); longPress(d);
  // not a Tab stop, and no on-screen keyboard after a tap; again whenever Ketcher re-creates the element
  const fix = () => { for (const c of d.querySelectorAll(".cliparea,[data-cliparea]")) { if (c.getAttribute("tabindex") !== "-1") c.setAttribute("tabindex", "-1"); if (c.getAttribute("inputmode") !== "none") c.setAttribute("inputmode", "none"); } };
  // the copy menu's triangle has no name of its own
  const name = () => { const tr = d.querySelector("[data-testid=copy-button-dropdown-triangle]"), w = /** @type {any} */ (window); if (tr && w.I18N) { const n = w.I18N.t("draw.copy.options"); if (tr.getAttribute("aria-label") !== n) { tr.setAttribute("aria-label", n); tr.setAttribute("title", n); } } };
  const all = () => { fix(); name(); };
  all(); new MutationObserver(all).observe(d.body, { childList: true, subtree: true });
}
