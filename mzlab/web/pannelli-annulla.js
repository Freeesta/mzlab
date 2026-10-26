// @ts-strict
// Opening a panel as one undoable step (comandi.js / storia.js); used by the buttons and dialogs that add panels. Classic script, loaded after pannelli.js.
/** A panel the student opened (a button, a double click, the XIC window): one undo step that closes it again and a redo that opens it as it was then. @param {any} p */
function apriAnnullabile(p) {
  if (!STORIA || !p) return p;
  let cur = p, dati = "", rel = /** @type {any} */ ({});
  const after = () => { relayout(); fitHost(); uiSave(); if (tabFiles("ms2").length) renderFileList(); };
  STORIA.aperto({ what: ptitle(p.title) || I18N.t("storia.pannello"), tab: p.tab, dati: JSON.stringify(panelRec(p)),
    segue: (/** @type {any} */ v, /** @type {any} */ n) => { if (cur === v) cur = n; },
    undo: () => {
      if (!E.panels.includes(cur)) return;
      dati = JSON.stringify(panelRec(cur)); rel = { link: cur.link, src: cur.src, dda: cur.dda, dock: cur.dock, duo: cur.duo };
      closePanel(cur); after();
    },
    redo: () => {
      if (!dati) return;
      const q = riapriPannello(JSON.parse(dati));
      for (const k of /** @type {const} */ (["link", "src", "dda", "dock", "duo"])) q[k] = rel[k] != null && E.panels.some(x => x.id === rel[k]) ? rel[k] : null;
      STORIA.segui(cur, q); cur = q; after(); ctl(q); redrawAll();
    } });
  return p;
}

// the title of a panel is one undoable action (comandi.js)
COMANDI.gestore("panel.rename", { leggi: p => JSON.stringify(p.title ?? null), applica: (p, snap) => { p.title = JSON.parse(snap); ctl(p); } });
// the traces of an XIC panel (add, remove, label, window, unit) are one undoable action (comandi.js); the split into panels is not (it creates panels)
COMANDI.gestore("xic.traces", {
  leggi: p => JSON.stringify([p.traces, p._xu ?? null, p.ints]),
  applica: (p, snap) => { const [t, u, i] = JSON.parse(snap); p.traces = t; p._xu = u; p.ints = i; p.isel = null; ctl(p); },
});
