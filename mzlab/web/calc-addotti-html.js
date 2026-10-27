// @ts-check
// @ts-strict
"use strict";
// Markup of the adduct table of the m/z calculator (calc-addotti.js): the table, the differences section and the delta result, from the rows of
// api/adduct_table. Pure functions of the rows and of the choices passed in. Classic script, before calc-addotti.js.

const CALCADDHTML = (() => {
  const sgn = (/** @type {number} */ x) => (x >= 0 ? "+" : "−");
  /** @param {AdRow} r */
  const shiftText = r => (r.k > 1 ? r.k + "M " : "") + sgn(r.shift) + Math.abs(r.shift).toFixed(4) + (Math.abs(r.z) > 1 ? " ÷" + Math.abs(r.z) : "");
  const pairName = (/** @type {string} */ a, /** @type {string} */ b) => `${fmtAdduct(a)} − ${fmtAdduct(b)}`;

  // decimals of the m/z: those of the profile of a high-resolution file, four without one
  const decimals = () => {
    const hrf = window.HR ? E.files.filter((/** @type {any} */ f) => !f.gone && (HR.isHr(f, 1) || HR.isHr(f, 2))) : [];
    return hrf.length ? Math.max(HR.dec(hrf, 1), HR.dec(hrf, 2)) : 4;
  };

  /** @param {AdTable} t @param {boolean} hasM @param {boolean} hasObs @param {{ pol: number, more: boolean, diffs: boolean, delta: string }} st */
  function mainHtml(t, hasM, hasObs, st) {
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

  /** @param {AdTable} t @param {{ diffs: boolean, delta: string }} st */
  function diffsHtml(t, st) {
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
  return { sgn, shiftText, mainHtml, diffsHtml, deltaHtml };
})();
