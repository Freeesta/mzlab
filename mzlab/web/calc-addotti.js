// @ts-check
// @ts-strict
"use strict";
// Adduct table of the m/z calculator, the only adduct panel: catalogue, ion m/z, ppm and implied neutral masses come from the Rust engine
// (api/adduct_table); this file lays them out and keeps the choices (polarity, tolerance, fold). Classic script, after calc-addotti-html.js.

const CALCADD = (() => {
  const H = CALCADDHTML;
  const st = { pol: 1, tol: "5", more: false, diffs: false, delta: "" };
  const KEY = () => NOMI.KEY("calc");
  let redraw = () => {}, seq = 0, curF = "", cur = /** @type {AdRow[]} */ ([]), catP = /** @type {Promise<AdRow[]> | null} */ (null);

  try {
    const s = JSON.parse(localStorage.getItem(KEY()) || "null");
    if (s && (s.pol === 1 || s.pol === -1)) st.pol = s.pol;
    if (s && typeof s.tol === "string") st.tol = s.tol;
  } catch (_) { /* blocked or unreadable: the defaults */ }
  const save = () => { try { localStorage.setItem(KEY(), JSON.stringify({ pol: st.pol, tol: st.tol })); } catch (_) { /* blocked */ } };
  // first visit: the polarity of the files shown (negative only = ESI-)
  const polFromFiles = () => (typeof E !== "undefined" && E.files.some((/** @type {any} */ f) => !f.gone && f.polarity === "negative") && !E.files.some((/** @type {any} */ f) => !f.gone && f.polarity === "positive") ? -1 : 1);
  let remembered = false;
  try { remembered = localStorage.getItem(KEY()) !== null; } catch (_) { /* blocked */ }
  if (!remembered) st.pol = polFromFiles();

  /** @param {string} f @param {string} obs @param {string} tol @param {string} delta @returns {Promise<AdTable>} */
  const table = (f, obs, tol, delta) => J(`api/adduct_table?f=${encodeURIComponent(f)}&obs=${encodeURIComponent(obs)}&tol=${encodeURIComponent(tol)}&delta=${encodeURIComponent(delta)}`);
  // the catalogue alone (no formula): the search box and the palette read it
  const catalog = () => catP || (catP = table("", "", "", "").then(t => t.rows));

  /** The table of the formula `t` (or of the observed m/z alone when `t` is empty), the differences under it. */
  async function render(/** @type {string} */ t) {
    const out = Q("#calcout"), sum = Q("#calcsum"), obsT = Q("#calcobs").value, my = ++seq;
    const hasObs = Number.isFinite(numIt(obsT)) && numIt(obsT) > 0;
    try {
      const r = await table(t, hasObs ? obsT : "", st.tol, st.delta);
      if (my !== seq) return;
      cur = r.rows; curF = r.formula || "";
      const hasM = r.neutral != null;
      sum.innerHTML = r.formula && t.replace(/\s+/g, "") !== r.formula ? I18N.t("calc.interpreted", { formula: fmtFormula(r.formula) }) : "";
      out.innerHTML = (hasM || hasObs ? H.mainHtml(r, hasM, hasObs, st) : "") + H.diffsHtml(r, st);
    } catch (e_) { if (my !== seq) return; const e = /** @type {Lanciato} */ (e_); sum.innerHTML = ""; out.innerHTML = h`<p class="muted">${e.message}</p>`; }
  }

  /** @param {() => void} run  the calculator's redraw */
  function bind(run) {
    redraw = run;
    const pol = Q("#calcpol"), tol = /** @type {HTMLInputElement} */ (Q("#calctol")), out = Q("#calcout");
    const paint = () => pol.querySelectorAll("button").forEach((/** @type {HTMLButtonElement} */ b) => { const on = +(b.dataset.p || 0) === st.pol; b.classList.toggle("on", on); b.setAttribute("aria-pressed", String(on)); });
    paint(); tol.value = st.tol;
    pol.querySelectorAll("button").forEach((/** @type {HTMLButtonElement} */ b) => b.onclick = () => { st.pol = +(b.dataset.p || 1); save(); paint(); run(); });
    tol.oninput = () => { st.tol = tol.value; save(); run(); };
    Q("#calcobs").oninput = run;
    out.addEventListener("click", ev => {
      const el = /** @type {HTMLElement} */ (ev.target);
      const more = el.closest(".cadd-more"), x = /** @type {HTMLElement | null} */ (el.closest(".cxic"));
      if (more) { st.more = more.getAttribute("aria-expanded") !== "true"; run(); const nb = out.querySelector(".cadd-more"); if (nb instanceof HTMLElement) nb.focus(); return; }
      if (!x) return;
      const r = cur[+(x.dataset.i || 0)], f = curF;
      Q("#calcdlg").close();
      const panel = [...tabPanels()].reverse().find((/** @type {any} */ q) => q.type === "xic") || null;
      // the adducts the XIC window knows travel with the formula; the others as the m/z of the table
      openXic(panel, XIC_ADDUCTS.includes(r.n) && f ? { formula: f, adduct: r.n } : { mz: r.mz5 });
    });
    out.addEventListener("toggle", ev => { const d = /** @type {HTMLDetailsElement} */ (ev.target); if (d.id === "cadd-diffs") st.diffs = d.open; }, true);
    out.addEventListener("input", ev => {
      const i = /** @type {HTMLInputElement} */ (ev.target);
      if (i.id !== "cadd-delta") return;
      st.delta = i.value.trim();
      table("", Q("#calcobs").value, st.tol, st.delta).then(r => { const d = Q("#cadd-dres"); if (d && i.value.trim() === st.delta) d.innerHTML = H.deltaHtml(r); }).catch(() => { /* the field is the student's: keep what is shown */ });
    });
  }

  /** Opens the differences section with a difference already typed (a pair of peaks of a spectrum). */
  function showDelta(/** @type {number} */ v) {
    st.diffs = true; st.delta = String(v); redraw();
  }
  return { render, bind, catalog, table, shiftText: H.shiftText, showDelta, state: st };
})();

// eventi.js loads first and owns calcRun; the table hooks in once both exist
CALCADD.bind(calcRun);
