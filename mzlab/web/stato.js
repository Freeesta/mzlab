"use strict";
// Page state: the objects E (files and panels), S (view, session), ST (file list, methods), NB (notebook), and the only places that write
// E.tab, E.cur, E.active, E.panels, S.view and S.adding (tools/controlla.py refuses direct writes elsewhere). Classic script, loaded first.
// Every transition fires "stato:cambiato" on document ({motivo, ...}); STATO.verifica() lists the broken invariants (tests_e2e/lib.py ready()).
const E = { files: [], panels: [], seq: 1, key: "", cur: 0, browse: false, z: 10, tab: "full" };
const NB = { ui: null, session: null };   // notebook: state of the interface (+ drawing, see draw.js)
// addresses of the views: #/dati, #/disegno (and #/teoria, which the head script of index.html sends to the standalone Teoria on a cold start)
const VIEW_HASH = { data: "dati", draw: "disegno", theory: "teoria" }, HASH_VIEW = { dati: "data", disegno: "draw", teoria: "theory" };
const hashView = () => { const m = /^#\/(\w+)/.exec(location.hash); return m && HASH_VIEW[m[1]] || null; };
const S = { view: hashView() || "data", sess: null, adding: false, ref: { on: {}, k: "auto", factor: 1, norm: "none", istd: "", rttol: 0.1, clip: false } };
const ST = { files: [], methods: [] };

const STATO = (() => {
  const emit = (motivo, d) => document.dispatchEvent(new CustomEvent("stato:cambiato", { detail: { motivo, ...d } }));
  return {
    /** E.cur = the file k shown (sibling files, browse and the file list follow in the caller). */
    file(k, motivo) { E.cur = k; emit(motivo, { file: k }); return k; },
    /** E.tab = the experiment tab (setTab in tabs.js is the user-visible switch: it also restores the layout of the tab). */
    scheda(t, motivo) { E.tab = t; emit(motivo, { scheda: t }); return t; },
    /** Runs fn with E.tab = t and puts the tab back (building the starting layout of a tab that is not shown). */
    conScheda(t, fn, motivo) { const save = E.tab; STATO.scheda(t, motivo); try { return fn(); } finally { STATO.scheda(save, motivo); } },
    attiva(p, motivo) { E.active = p; emit(motivo, { pannello: p && p.id }); return p; },
    pannelli(lista, motivo) { E.panels = lista; emit(motivo, {}); return lista; },
    aggiungi(p, motivo) { E.panels.push(p); emit(motivo, { pannello: p.id }); return p; },
    rimuovi(p, motivo) { E.panels = E.panels.filter(x => x !== p); emit(motivo, { pannello: p.id }); },
    vista(v, motivo) { S.view = v; emit(motivo, { vista: v }); return v; },
    /** «All experiments together» (UIP.tog): the tab bar goes away and every panel is shown. */
    insieme(on, motivo) { UIP.tog = !!on; emit(motivo, { insieme: !!on }); return UIP.tog; },
    aggiunta(on, motivo) { S.adding = !!on; emit(motivo, { aggiunta: !!on }); return S.adding; },
    /** Broken invariants, as a list of sentences (empty = consistent). */
    verifica() {
      const r = [];
      if (E.active && !E.panels.includes(E.active)) r.push("E.active is not in E.panels");
      if (E.files.length && !(E.cur >= 0 && E.cur < E.files.length)) r.push("E.cur outside E.files: " + E.cur);
      const tutte = typeof allTabs === "function" && allTabs();
      if (!tutte) E.panels.forEach(p => { if (p.el && p.el.style.display !== "none" && p.tab !== E.tab) r.push("visible panel " + p.id + " of tab " + p.tab + " while E.tab is " + E.tab); });
      if (typeof lrTog === "function" && lrTog()) E.panels.forEach(p => { if (p.el && p.el.style.display === "none") r.push("panel " + p.id + " hidden while all experiments are shown together"); });
      return r;
    },
  };
})();
