// @ts-check
// The rare paths of OPENNOW (open-now.js), loaded on first use: «Switch to …» of #hrmix (closes the open world, opens the other one, one undo entry)
// and the question «Open all / Choose» of a folder of more than 50 data files. Kept out of the start-up list (tests/test_peso_avvio.py).
/** «Switch to …»: closes the files of the open world and opens the other one; one «Removed · Undo» entry gives the old world back */
async function toWorld(/** @type {boolean} */ hr, /** @type {{ sent: Set<string>, q: string[], drain: () => Promise<void> }} */ o) {
  const { sent, q, drain } = o;
  const go = async (/** @type {boolean} */ to) => {
    loading(true);
    try {
      // the panels of the closed world point at its files: they go first (no late draw asks for a file that is gone)
      if (E.panels.length) closeAllPanels();
      await post("api/new", {});      // closes the open files, the uploaded ones stay
      sent.clear();
      const fs = ST.files.filter((/** @type {FileRiga} */ f) => f.kind in GORD);
      fs.forEach((/** @type {FileRiga} */ f) => { f.use = !!f.hr === to; });
      await bootSession();
      fs.filter((/** @type {FileRiga} */ f) => f.use).forEach((/** @type {FileRiga} */ f) => q.push(f.name));
      await drain();
    } catch (e_) { fail(e_); } finally { loading(false); renderFiles(); }
  };
  const n = E.files.filter((/** @type {FileAperto} */ g) => !g.gone).length;
  await go(hr);
  if (typeof STORIA !== "undefined" && STORIA) STORIA.tolto({ what: I18N.t("files.removeAll.what", { n }), dati: String(n), undo: () => go(!hr), redo: () => go(hr) });
}
/** a folder of more than 50 data files asks once: true = open all, false = choose (all unticked) */
function many(/** @type {number} */ n) {
  return new Promise(res => {
    const d = /** @type {HTMLDialogElement} */ (document.querySelector("#askdlg")), ok = /** @type {HTMLElement} */ (document.querySelector("#askok")), no = /** @type {HTMLElement} */ (document.querySelector("#askno")), inp = /** @type {HTMLElement} */ (document.querySelector("#askin"));
    const was = [ok.textContent, no.textContent];
    /** @type {HTMLElement} */ (document.querySelector("#asktxt")).textContent = I18N.t("load.many.text", { n });
    inp.hidden = true; no.hidden = false; ok.textContent = I18N.t("load.many.all"); no.textContent = I18N.t("load.many.choose");
    const done = (/** @type {boolean} */ v) => { d.close(); inp.hidden = false; [ok.textContent, no.textContent] = was; ok.onclick = no.onclick = null; d.oncancel = null; res(v); };
    ok.onclick = () => done(true); no.onclick = () => done(false); d.oncancel = () => done(false);
    d.showModal(); ok.focus();
  });
}
/** @type {any} */ (window).OPENRARO = { toWorld, many };
