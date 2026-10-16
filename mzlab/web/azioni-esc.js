// @ts-strict
// @ts-check
"use strict";
// ONE Esc for the whole data view. The things Esc can take back register here with a priority; one press runs ONE of them, the most urgent:
// menus and tooltips (10), dialogs (20) keep their own handlers (this file only steps aside), then 30 pending step / measure, 40 playback of the scans,
// 50 full screen of a panel. Lower number = first. The side drawer (barra.js) closes only when this file did nothing (it sees the event as defaultPrevented).
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
