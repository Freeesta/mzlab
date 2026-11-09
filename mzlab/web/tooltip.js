// @ts-strict
// @ts-check
// Tooltips of the whole program: pointer rest, keyboard focus and touch long press (WCAG 1.4.13).
// Split from eventi.js: classic script, loaded right after it; TIP_DELAY comes from pannelli.js.
// ---- discreet tooltip on every button and control (the program's own, not the browser's): it appears when the pointer rests on one, or keyboard focus reaches it, for TIP_DELAY (1.25 s) and goes away when the pointer or the focus leaves, or with Esc.
// The text is the element's title; the title is moved to data-tip on the first hover so the browser's own (earlier, fixed-style) tooltip does not also show up.
(function () {
  const tip = document.createElement("div"); tip.id = "qtip"; tip.hidden = true; document.body.appendChild(tip);
  const SEL = "[title], [data-tip], [data-tiph]";
  let timer = null, cur = null;
  const hide = () => { clearTimeout(timer); timer = null; tip.hidden = true; cur = null; };
  // draws the tooltip of t: below, else above, else beside it (never over the element it describes); a tooltip made of HTML (data-tiph) wins over the plain text
  const show = (/** @type {HTMLElement} */ t) => {
    const txt = t.dataset.tip, html = t.dataset.tiph;
    if (html) tip.innerHTML = html; else tip.textContent = /** @type {string} */ (txt);
    tip.hidden = false;
    // a control of a toolbar keeps the tooltip clear of the whole toolbar (the outermost one), not only of itself: it must not hide the neighbouring buttons
    let zone = /** @type {HTMLElement} */ (t); for (let e = t.parentElement; e; e = e.parentElement) if (e.matches("#stbar, #tools, [role=toolbar]")) zone = e;
    const r = t.getBoundingClientRect(), z = zone.getBoundingClientRect(), w = tip.offsetWidth, h = tip.offsetHeight, m = 6, below = z.bottom + m + h + m <= innerHeight, above = z.top - m - h >= m;
    let x = Math.max(m, Math.min(innerWidth - w - m, r.left + r.width / 2 - w / 2)), y = below ? z.bottom + m : z.top - m - h;
    if (!below && !above) { y = Math.max(m, Math.min(innerHeight - h - m, r.top)); x = z.right + m + w + m <= innerWidth ? z.right + m : Math.max(m, z.left - m - w); }
    tip.style.left = x + "px"; tip.style.top = y + "px";
  };
  document.addEventListener("mouseover", e => {
    const el = /** @type {HTMLElement} */ (e.target), t = /** @type {HTMLElement | null} */ (el.closest && el.closest(SEL));
    if (t === cur) return;
    hide();
    if (!t || t.closest("dialog[open]")) return;
    if (t.hasAttribute("title")) { t.dataset.tip = /** @type {string} */ (t.getAttribute("title")); t.removeAttribute("title"); }
    const txt = t.dataset.tip, html = t.dataset.tiph; if (!txt && !html) return;
    cur = t;
    const delay = t.closest(".fl") ? 1000 : TIP_DELAY;
    timer = setTimeout(() => show(t), delay);
  });
  document.addEventListener("mouseout", e => { if (cur && !(e.relatedTarget && cur.contains(e.relatedTarget))) hide(); });
  // Esc with a tooltip up puts away only the tooltip (WCAG 1.4.13: dismissible without moving focus), never the panel under it
  addEventListener("keydown", e => { if (e.key === "Escape" && !tip.hidden) { hide(); e.stopImmediatePropagation(); e.preventDefault(); } }, true);
  ["mousedown", "keydown", "wheel"].forEach(ev => document.addEventListener(ev, hide, true));
  // keyboard focus shows the same tooltip as the pointer, after the same rest; a focus that follows a click or a touch does not (the pointer has its own path)
  let byKey = false;
  document.addEventListener("keydown", () => { byKey = true; }, true);
  document.addEventListener("pointerdown", () => { byKey = false; }, true);
  document.addEventListener("focusin", e => {
    const el = /** @type {HTMLElement} */ (e.target), t = /** @type {HTMLElement | null} */ (el.closest && el.closest(SEL));
    if (!byKey || !t || t === cur || t.closest("dialog[open]")) return;
    hide();
    if (t.hasAttribute("title")) { t.dataset.tip = /** @type {string} */ (t.getAttribute("title")); t.removeAttribute("title"); }
    if (!t.dataset.tip && !t.dataset.tiph) return;
    cur = t; timer = setTimeout(() => show(t), TIP_DELAY);
  });
  document.addEventListener("focusout", e => { if (cur && cur.contains(/** @type {Node} */ (e.target)) && !(e.relatedTarget && cur.contains(/** @type {Node} */ (e.relatedTarget)))) hide(); });
  // touch has no hover: a long press (0.6 s) on a button or control shows its tooltip for a moment; the click that follows the release is swallowed. Graphs and file rows keep their long press = right click (touch.js)
  let lpT = null, lpX = 0, lpY = 0, lpDone = 0, lpUp = false;      // lpDone: the lift that ends a press whose tooltip came up; the click a finger sends on lifting (iOS and Chromium, however long the press) is swallowed for 700 ms after it
  const lpStop = () => clearTimeout(lpT);
  document.addEventListener("pointerdown", e => {
    lpStop(); lpUp = false; if (e.pointerType !== "touch") return; if (!tip.hidden) hide();      // the next touch puts the tooltip of a long press away
    const el = /** @type {HTMLElement} */ (e.target), t = /** @type {HTMLElement | null} */ (el.closest && el.closest(SEL));
    if (!t || t.closest("dialog[open], canvas, .fl, .pnl .cv")) return;
    lpX = e.clientX; lpY = e.clientY;
    lpT = setTimeout(() => {
      if (t.hasAttribute("title")) { t.dataset.tip = /** @type {string} */ (t.getAttribute("title")); t.removeAttribute("title"); }
      if (!t.dataset.tip && !t.dataset.tiph) return;
      show(t); lpUp = true; cur = t; clearTimeout(timer); timer = setTimeout(hide, 2800);
    }, 600);
  }, true);
  document.addEventListener("pointermove", e => { if (lpT && Math.hypot(e.clientX - lpX, e.clientY - lpY) > 10) lpStop(); }, true);
  ["pointerup", "pointercancel"].forEach(ev => document.addEventListener(ev, () => { lpStop(); if (lpUp) { lpDone = Date.now(); lpUp = false; } }, true));
  document.addEventListener("click", e => { if (Date.now() - lpDone < 700) { e.stopPropagation(); e.preventDefault(); lpDone = 0; } }, true);
  addEventListener("scroll", hide, true);
})();
