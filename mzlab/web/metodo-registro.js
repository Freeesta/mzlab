// @ts-check
// The error log of a Thermo .raw in the Method window (`metodo/1.registro`): what the instrument reported while it acquired, each entry with its time. A click on the time moves the cursor of the
// first chromatogram of the tab to that RT (the spectrum of the scan nearest in time follows, as for a click on the chromatogram). Only what the file says is shown, as it says it; every text goes through EH().
// Loaded as a module after the classic scripts; metodo-doc.js uses window.METODO_REGISTRO.

/** @typedef {{ t_min?: number, testo?: string, livello?: string }} Voce */

/** the rows that have a time (the entries of a .wiff have none: metodo-doc.js lists them as they are) */
const timed = (/** @type {Voce[]} */ reg) => reg.filter(x => typeof x.t_min === "number" && x.testo);

/** the table of the entries for the open item `f` */
function html(/** @type {Voce[]} */ reg, /** @type {{ k: number }} */ f) {
  const rows = timed(reg);
  if (!rows.length) return "";
  const tr = rows.map(x => {
    const t = /** @type {number} */ (x.t_min), s = (+t.toFixed(3)).toString();
    return `<tr><td class="num"><button type="button" class="bt" data-mreg-rt="${EH(s)}" data-mreg-k="${f.k}" aria-label="${EH(I18N.t("method.reg.go", { t: s }))}" title="${EH(I18N.t("method.reg.go", { t: s }))}">${EH(s)}</button></td><td>${EH(x.livello || "")}</td><td>${EH(String(x.testo))}</td></tr>`;
  }).join("");
  return `<table><tr><th scope="col" class="num">${EH(I18N.t("method.time"))}</th><th scope="col">${EH(I18N.t("method.reg.level"))}</th><th scope="col">${EH(I18N.t("method.reg.text"))}</th></tr>${tr}</table>`;
}

/** the cursor of the first chromatogram goes to `rt` (min) and the linked panels follow */
function go(/** @type {number} */ rt, /** @type {number} */ k) {
  const p = E.panels.find(q => q.type === "chrom");
  const d = /** @type {HTMLDialogElement | null} */ (Q("#bigdlg"));
  if (d && d.open) d.close();
  if (!p) return;
  p.cur = rt; draw(p);
  const dt = scanStep();
  pushLinked(p, rt - dt / 2, rt + dt / 2, k);
}

document.addEventListener("click", e => {
  const b = e.target instanceof Element ? e.target.closest("[data-mreg-rt]") : null;
  if (b instanceof HTMLElement) go(+(b.dataset.mregRt || 0), +(b.dataset.mregK || 0));
});

/** @type {any} */ (window).METODO_REGISTRO = { html, timed };
