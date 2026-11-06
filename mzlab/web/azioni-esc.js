// @ts-strict
// @ts-check
"use strict";
// ONE Esc for the whole data view. The things Esc can take back register here with a priority; one press runs ONE of them, the most urgent:
// menus and tooltips (10), dialogs (20) keep their own handlers (this file only steps aside), then 30 pending step / measure, 40 playback of the scans,
// 50 full screen of a panel. Lower number = first. The side drawer (barra.js) closes only when this file did nothing (it sees the event as defaultPrevented).
// Also: a dialog that closes leaving the focus on the body or on a hidden control (its opener was an item of a menu that hid itself) gives it to the nearest visible control.
const ESC = (() => {
  /** @type {{ id: string, prio: number, aperto: () => boolean, chiudi: () => void }[]} */
  const voci = [];
  // what owns Esc by itself: context menu, correction menu, small pop-ups, help box, settings, dialogs
  const PROPRI = "dialog[open],#ctx:not([hidden]),.corrm:not([hidden]),#subpop:not([hidden]),#helppop:not([hidden]),#uipset,#stmenu,#unopen:not([hidden])";
  /** @param {{ id: string, prio: number, aperto: () => boolean, chiudi: () => void }} v */
  function registra(v) {
    const i = voci.findIndex(x => x.id === v.id); if (i >= 0) voci.splice(i, 1);
    voci.push(v); voci.sort((a, b) => a.prio - b.prio);
  }
  /** The entry that the next Esc would run (null: nothing of the registry is open). */
  const prossimo = () => (document.querySelector(PROPRI) || (typeof PALETTE !== "undefined" && PALETTE.isOpen)) ? null : voci.find(v => v.aperto()) || null;
  /** @param {KeyboardEvent} e */
  function premi(e) {
    if (e.key !== "Escape" || e.defaultPrevented || e.isComposing) return;
    const v = prossimo(); if (!v) return;
    e.preventDefault(); v.chiudi();
  }
  if (typeof document !== "undefined") document.addEventListener("keydown", premi, true);
  return { registra, prossimo, premi, voci };
})();
// The focus after a dialog closes: the browser (or the dialog) gives it back to its opener; when that one is hidden or gone (an item of a menu that hid
// itself), the most recent control focused before it that is still visible takes it (the ⋯ button that opened the menu), else the first visible control of
// the opener's nearest visible container, so the keyboard keeps its place instead of starting again from the top of the page.
(() => {
  if (typeof document === "undefined") return;
  const CTL = "button:not([disabled]),[href],input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex='-1'])";
  const seen = (/** @type {Element} */ e) => e.isConnected && e.getClientRects().length > 0 && !e.closest("dialog:not([open])");
  /** @type {HTMLElement[]} */ const hist = [];
  document.addEventListener("focusin", e => {
    const t = e.target; if (!(t instanceof HTMLElement) || t.closest("dialog")) return;
    const i = hist.indexOf(t); if (i >= 0) hist.splice(i, 1); hist.push(t); if (hist.length > 8) hist.shift();
  }, true);
  document.addEventListener("close", e => {
    if (!(e.target instanceof HTMLDialogElement)) return;
    setTimeout(() => {
      const a = document.activeElement;
      if ((a && a !== document.body && seen(a)) || document.querySelector("dialog[open]")) return;
      let f = [...hist].reverse().find(seen);
      for (let c = /** @type {Element | null} */ (hist[hist.length - 1]); !f && c && c !== document.body; c = c.parentElement) if (seen(c)) f = c.matches(CTL) ? c : [...c.querySelectorAll(CTL)].find(seen);
      if (f) /** @type {HTMLElement} */ (f).focus({ preventScroll: true });
    }, 0);
  }, true);
})();
