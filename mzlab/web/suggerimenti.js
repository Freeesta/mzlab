// @ts-check
// @ts-strict
"use strict";
// One queue for the hints that point at something: the tour (and its invitation), «the tools are here» (barra.js) and «drag here to place side by side» (affianca.js).
// Never two at once: a hint asks with SUGG.chiedi(id, mostra); while another is on screen it waits (a new request with the same id only renews `mostra`) and is shown,
// in order of request, when the screen is free. `mostra()` says whether it showed the hint; a hint that no longer applies returns false and leaves the queue. Classic script.
const SUGG = (() => {
  /** The elements of the hints that are on screen now (the tour dialog is `TOUR.active`). */
  const SEL = ["#tour-invito", "#bar-hint", "#aff-hint"];
  /** @type {{ id: string, mostra: () => boolean, fine: number, dopo: number }[]} */
  let coda = [];
  let timer = 0;
  const TENUTA = 120000;       // a hint that did not find the screen free for two minutes is no longer worth showing
  const aperto = () => SEL.some(s => !!document.querySelector(s)) || !!(typeof TOUR !== "undefined" && TOUR.active);
  function giro() {
    timer = 0;
    const adesso = Date.now();
    coda = coda.filter(r => r.fine > adesso);
    while (!aperto()) {
      const i = coda.findIndex(r => r.dopo <= adesso);
      if (i < 0) break;
      if (coda.splice(i, 1)[0].mostra()) break;
    }
    if (coda.length) timer = window.setTimeout(giro, 1000);
  }
  /** @param {string} id @param {() => boolean} mostra @param {number} [attesa] ms before the first try (a hint that is not about the action of the user waits for the tour invitation, which comes just after loading)
   *  @returns {boolean} true when the hint is on screen now */
  function chiedi(id, mostra, attesa = 0) {
    const gia = coda.find(r => r.id === id);
    if (gia) { gia.mostra = mostra; return false; }       // asked again (the page calls it after every layout): it keeps its place and its delay
    if (!attesa && !aperto() && !coda.length && mostra()) return true;
    coda.push({ id, mostra, fine: Date.now() + TENUTA, dopo: Date.now() + attesa });
    if (!timer) timer = window.setTimeout(giro, 1000);
    return false;
  }
  /** @param {string} id the request leaves the queue (the hint no longer applies) */
  const annulla = id => { coda = coda.filter(r => r.id !== id); };
  return { chiedi, annulla, aperto, get inCoda() { return coda.map(r => r.id); } };
})();
window.SUGG = SUGG;
