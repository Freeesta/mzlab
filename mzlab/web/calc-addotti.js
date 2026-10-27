// @ts-check
// @ts-strict
"use strict";
// Adduct table of the m/z calculator, the only adduct panel: catalogue, ion m/z, ppm and implied neutral masses come from the Rust engine
// (api/adduct_table); this file lays them out and keeps the choices (polarity, tolerance, fold). Classic script, after calcola.js.

/** @typedef {{ n: string, id: string, pol: number, common: boolean, k: number, z: number, add: string, sub: string, shift: number, mz4: number | null, mz5: number | null, mz1: number | null, nominal: number | null, ppm: number | null, m_if: number | null }} AdRow */
/** @typedef {{ formula: string | null, neutral: number | null, tol: number, rows: AdRow[], best: number | null, diffs: { pair: string, a: string, b: string, pol: number, d: number }[], delta?: { value: number, hits: { a: string, b: string, d: number, err_mda: number }[] } }} AdTable */

const CALCADD = (() => {
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
  const sgn = (/** @type {number} */ x) => (x >= 0 ? "+" : "−");
  /** @param {AdRow} r */
  const shiftText = r => (r.k > 1 ? r.k + "M " : "") + sgn(r.shift) + Math.abs(r.shift).toFixed(4) + (Math.abs(r.z) > 1 ? " ÷" + Math.abs(r.z) : "");
  const pairName = (/** @type {string} */ a, /** @type {string} */ b) => `${fmtAdduct(a)} − ${fmtAdduct(b)}`;

  // decimals of the m/z: those of the profile of a high-resolution file, four without one
  const decimals = () => {
    const hrf = window.HR ? E.files.filter((/** @type {any} */ f) => !f.gone && (HR.isHr(f, 1) || HR.isHr(f, 2))) : [];
    return hrf.length ? Math.max(HR.dec(hrf, 1), HR.dec(hrf, 2)) : 4;
  };

  /** @param {AdTable} t @param {boolean} hasM @param {boolean} hasObs */
  function mainHtml(t, hasM, hasObs) {
    const dd = decimals(), mzOf = (/** @type {AdRow} */ r) => (r.mz5 == null || r.mz4 == null ? "" : dd > 4 ? r.mz5.toFixed(dd) : r.mz4.toFixed(4));
    const idx = (/** @type {AdRow} */ r) => t.rows.indexOf(r), mine = t.rows.filter(r => r.pol === st.pol), best = t.best == null ? null : t.rows[t.best];
    const cols = 2 + (hasM ? 2 : 0) + (hasM && hasObs ? 1 : 0) + (hasObs ? 1 : 0) + (hasM ? 1 : 0);
    const head = `<tr><th scope="col">${I18N.t("calc.col.adduct")}</th><th scope="col" class="num">${I18N.t("calc.col.delta")}</th>`
      + (hasM ? `<th scope="col" class="num">${I18N.t("calc.col.exact")}</th><th scope="col" class="num">${I18N.t("calc.col.nominal")}</th>` : "")
      + (hasM && hasObs ? `<th scope="col" class="num">${I18N.t("calc.col.ppm")}</th>` : "") + (hasObs ? `<th scope="col" class="num">${I18N.t("calc.col.mif")}</th>` : "") + (hasM ? '<td></td>' : "") + "</tr>";
    const row = (/** @type {AdRow} */ r) => {
      const hit = best === r, mz = mzOf(r);
      return `<tr class="${hit ? "hit" : ""}"><th scope="row">${hit ? `<span aria-hidden="true">✓ </span><span class="sr-only">${I18N.t("calc.add.match")} </span>` : ""}${fmtAdduct(r.n)}</th><td class="num">${shiftText(r)}</td>`
        + (hasM ? `<td class="num">${mz}</td><td class="num">${r.nominal}</td>` : "")
        + (hasM && hasObs ? `<td class="num">${r.ppm == null ? "" : sgn(r.ppm) + Math.abs(r.ppm).toFixed(1)}</td>` : "")
        + (hasObs ? `<td class="num">${r.m_if == null ? "" : r.m_if.toFixed(4)}</td>` : "")
        + (hasM ? `<td><button type="button" class="cxic" data-i="${idx(r)}" title="${I18N.t("calc.xic.title")}" aria-label="${EH(I18N.t("calc.xic.aria", { adduct: r.n, mz }))}">XIC</button></td>` : "") + "</tr>";
    };
    const common = mine.filter(r => r.common), rare = mine.filter(r => !r.common), open = st.more || (best != null && rare.includes(best));
    const more = `<tr class="cmore"><td colspan="${cols}"><button type="button" class="cadd-more" aria-expanded="${open}">${I18N.t("tbl.add.more")}</button></td></tr>`;
    let line = "";
    if (hasM) line = `<p>${I18N.t("calc.neutral", { formula: fmtFormula(t.formula || ""), mass: (t.neutral || 0).toFixed(4), nominal: Math.round(t.neutral || 0) })}</p>`;
    if (hasM && hasObs) {
      line += best
        ? `<p class="cadd-match">${I18N.t("calc.add.hit", { adduct: fmtAdduct(best.n), ppm: sgn(best.ppm || 0) + Math.abs(best.ppm || 0).toFixed(1) })}${best.pol === st.pol ? "" : " (" + (best.pol > 0 ? "ESI+" : "ESI−") + ")"}</p>`
        : `<p class="cadd-match muted">${I18N.t("calc.add.none", { tol: t.tol })}</p>`;
    }
    return `${line}<div class="cadd-wrap" role="region" tabindex="0" aria-label="${I18N.t("calc.add.region")}"><table class="cadt"><thead>${head}</thead><tbody>${common.map(row).join("")}${more}${open ? rare.map(row).join("") : ""}</tbody></table></div>`;
  }

  /** @param {AdTable} t */
  function diffsHtml(t) {
    const rows = t.diffs.map(d => `<tr><th scope="row" class="num">${sgn(d.d)}${Math.abs(d.d).toFixed(4)}</th><td>${fmtFormula(d.pair)}</td><td>${d.pol > 0 ? "ESI+" : "ESI−"}</td></tr>`).join("");
    return `<details class="cadd-diffs" id="cadd-diffs"${st.diffs ? " open" : ""}><summary>${I18N.t("tbl.diff.title")}</summary>
      <table class="cadt"><thead><tr><th scope="col" class="num">Δ</th><th scope="col">${I18N.t("calc.diff.name")}</th><th scope="col">${I18N.t("calc.diff.pol")}</th></tr></thead><tbody>${rows}</tbody></table>
      <label class="cadd-f"><span>${I18N.t("calc.delta")}</span><input id="cadd-delta" inputmode="decimal" autocomplete="off" value="${EH(st.delta)}" title="${I18N.t("calc.delta.title")}"></label>
      <div id="cadd-dres" aria-live="polite">${deltaHtml(t)}</div></details>`;
  }
  /** @param {AdTable} t */
  function deltaHtml(t) {
    if (!t.delta) return "";
    if (!t.delta.hits.length) return `<p class="muted">${I18N.t("calc.delta.none", { tol: t.tol })}</p>`;
    return `<ul>${t.delta.hits.slice(0, 6).map(h => `<li>${pairName(h.a, h.b)} <span class="muted">(${sgn(h.d)}${Math.abs(h.d).toFixed(4)}, ${h.err_mda >= 0 ? "+" : "−"}${Math.abs(h.err_mda).toFixed(2)} mDa)</span></li>`).join("")}</ul>`;
  }

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
      out.innerHTML = (hasM || hasObs ? mainHtml(r, hasM, hasObs) : "") + diffsHtml(r);
    } catch (e_) { if (my !== seq) return; const e = /** @type {Lanciato} */ (e_); sum.innerHTML = ""; out.innerHTML = `<p class="muted">${EH(e.message)}</p>`; }
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
      table("", Q("#calcobs").value, st.tol, st.delta).then(r => { const d = Q("#cadd-dres"); if (d && i.value.trim() === st.delta) d.innerHTML = deltaHtml(r); }).catch(() => { /* the field is the student's: keep what is shown */ });
    });
  }

  /** Opens the differences section with a difference already typed (a pair of peaks of a spectrum). */
  function showDelta(/** @type {number} */ v) {
    st.diffs = true; st.delta = String(v); redraw();
  }
  return { render, bind, catalog, table, shiftText, showDelta, state: st };
})();

// eventi.js loads first and owns calcRun; the table hooks in once both exist
CALCADD.bind(calcRun);
