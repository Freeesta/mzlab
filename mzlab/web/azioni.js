// @ts-strict
"use strict";
// Registry of the actions of the panels (format azioni/1): every entry of the right-click menu, of the bar of a panel and of the command palette is one action.
// A panel composes its menu with AZIONI.per(c): the actions of its view, in the order of the groups, the ones not available now grey (never hidden).
// c = the context of the request: { vista: "map" | "map3d" | ..., p: the panel, ... } plus what the module that registered the action needs (the map passes its adapter).
// Classic script, loaded after esperimenti.js; the modules register their actions in their own files (mappa.js for the RT x m/z map).
/**
 * @typedef {{ vista: string, [k: string]: any }} Contesto
 * @typedef {{ id: string, gruppo: string, viste: string[], icona?: string, menu?: boolean, barra?: boolean,
 *   visibile?: (c: Contesto) => boolean, disponibile?: (c: Contesto) => boolean, attiva?: (c: Contesto) => boolean,
 *   nome: (c: Contesto) => string, voce?: (c: Contesto) => string, tip?: (c: Contesto) => string, svg?: (c: Contesto) => string,
 *   esegui: (c: Contesto) => any, [k: string]: any }} Azione
 * @typedef {{ id: string, label: string, aria: string, tip?: string, icon?: string, check: boolean | null, dim: boolean, fn: () => any }} Voce
 */
const AZIONI = (() => {
  // the order of the groups in every menu: one table for all the panels
  const GRUPPI = ["extract", "lock", "view", "zoom", "learn", "copy", "help"];
  /** @type {Azione[]} */
  const voci = [];
  /** @param {Azione} a */
  function registra(a) {
    if (voci.some(v => v.id === a.id)) throw new Error("AZIONI: id already registered: " + a.id);
    if (!GRUPPI.includes(a.gruppo)) throw new Error("AZIONI: unknown group " + a.gruppo + " of " + a.id);
    voci.push(a);
    return a;
  }
  /** @param {string} id */
  const voce = id => voci.find(v => v.id === id);
  /** @param {Azione} a @param {Contesto} c */
  const mostra = (a, c) => a.viste.includes(c.vista) && (!a.visibile || a.visibile(c));
  /** @param {Azione} a @param {Contesto} c */
  const pronta = (a, c) => !a.disponibile || a.disponibile(c);
  // the actions of the view in the order of the groups (registration order inside a group)
  /** @param {Contesto} c @param {"menu" | "barra"} [dove] */
  function elenco(c, dove = "menu") {
    const l = voci.filter(a => mostra(a, c) && (dove === "menu" ? a.menu !== false : a.barra === true));
    return l.sort((a, b) => GRUPPI.indexOf(a.gruppo) - GRUPPI.indexOf(b.gruppo) || voci.indexOf(a) - voci.indexOf(b));
  }
  // the entries of the right-click menu (shared component: menu() of explore.js), a separator between two groups
  /** @param {Contesto} c @returns {(Voce | "-")[]} */
  function per(c) {
    /** @type {(Voce | "-")[]} */
    const items = [];
    let last = null;
    for (const a of elenco(c)) {
      if (last && a.gruppo !== last) items.push("-");
      last = a.gruppo;
      const nome = a.nome(c);
      items.push({ id: a.id, label: a.voce ? a.voce(c) : nome, aria: nome, tip: a.tip && a.tip(c), icon: a.svg && a.svg(c),
        check: a.attiva ? a.attiva(c) : null, dim: !pronta(a, c), fn: () => esegui(a.id, c) });
    }
    return items;
  }
  /** @param {string} id @param {Contesto} c */
  function esegui(id, c) {
    const a = voce(id);
    if (a && mostra(a, c) && pronta(a, c)) return a.esegui(c);
  }
  return { formato: "azioni/1", GRUPPI, voci, registra, voce, elenco, per, esegui, pronta };
})();
