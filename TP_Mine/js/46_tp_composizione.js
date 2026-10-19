// @ts-check
// TPMINE-PRIVATE  TPCOMP: the «Elemental composition» block of the Identity tab of an open row (TPRIGA). On request it asks the engine for candidate compositions of the MS1 spectrum at the apex
// (`formula_id/1`: charge and pattern from the isotope finder, compositions from the exact mass) and shows them with the share each one has in the list and the reasons for the choice of the program.
// The program chooses (the first one); the chemist can pick another one and sees why. Candidates, never identifications. Nothing is computed here; texts in English only (no catalogue of languages).
(() => {
  "use strict";
  const G = /** @type {any} */ (window);

  /** the words of the reasons (`why` of the engine): a stable code and its number @type {Record<string, (v: number) => string>} */
  const WHY = {
    massError: v => `Mass error ${v >= 0 ? "+" : ""}${v.toFixed(2)} ppm`,
    rdb: v => `Double-bond equivalents ${v}`,
    isotopeFit: v => `Isotope pattern match ${v.toFixed(2)} (0 to 1)`,
    massOnly: () => "No isotope pattern in the spectrum: ranked by mass error only",
    rulesOk: v => `${v} of 10 chemical rules met`,
  };
  /** the chemical rules the engine checks (`rules_failed`) @type {Record<string, string>} */
  const RULE = { HC: "H/C ratio", NC: "N/C ratio", OC: "O/C ratio", PC: "P/C ratio", SC: "S/C ratio", FC: "F/C ratio", ClC: "Cl/C ratio", BrC: "Br/C ratio", LEWIS: "Lewis rule", SENIOR: "Senior rule" };
  const SRC = /** @type {Record<string, string>} */ ({ finder: "from the isotope pattern", given: "given", assumed: "assumed: no isotope pattern found" });
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  /** the reasons of one candidate as a list of sentences @param {any} c */
  function reasons(c) {
    const out = /** @type {string[]} */ ([]);
    for (const w of c.why || []) if (WHY[w.code]) out.push(WHY[w.code](w.value));
    if ((c.rules_failed || []).length) out.push("Rules not met: " + c.rules_failed.map((/** @type {string} */ r) => RULE[r] || r).join(", "));
    return out;
  }

  /** the table of the candidates, the line about the charge and the elements, and the reasons of the one picked @param {HTMLElement} box @param {any} r the answer `formula_id/1` */
  function show(box, r) {
    const cs = r.candidates || [], ch = r.charge || {};
    if (!cs.length) { box.innerHTML = `<p class="tpr-id">No composition fits this mass within the tolerance.</p>`; return; }
    let pick = r.chosen == null ? 0 : r.chosen;
    const found = (r.elements_found || []).map((/** @type {any} */ f) => `${esc(f.symbol)} (about ${f.atoms})`).join(", ");
    const head = `<p class="tpr-id"><b>Charge</b> ${esc(ch.value)} (${esc(SRC[ch.source] || ch.source)}${ch.probability != null ? `, p = ${(+ch.probability).toFixed(2)}` : ""})${found ? ` · <b>Heavy elements in the pattern</b> ${found}` : ""}</p>`;
    const hasNeutral = cs.some((/** @type {any} */ c) => c.neutral);
    const draw = () => {
      const rows = cs.map((/** @type {any} */ c, /** @type {number} */ i) => `<tr><td><button type="button" class="tpr-more" data-i="${i}" aria-pressed="${i === pick}" aria-label="Use ${esc(c.formula)}${i === r.chosen ? ", the choice of the program" : ""}">${i === pick ? "Using" : "Use"}</button></td>`
        + (hasNeutral ? `<td>${esc(c.neutral || "–")}</td>` : "") + `<td>${esc(c.formula)}</td><td class="n">${(+c.error_ppm).toFixed(2)}</td><td class="n">${c.rdb}</td><td class="n">${(+c.score).toFixed(2)}</td><td class="n">${Math.round(100 * c.confidence)} %</td></tr>`).join("");
      box.innerHTML = head + `<div class="tpr-tw"><table class="tpr tpr-cand"><caption class="sr-only">Candidate compositions</caption><thead><tr><th scope="col"><span class="sr-only">Choice</span></th>`
        + (hasNeutral ? `<th scope="col">Neutral formula</th>` : "") + `<th scope="col">Ion formula</th><th scope="col" class="n">Δppm</th><th scope="col" class="n">Double bonds</th><th scope="col" class="n">Score</th><th scope="col" class="n">Share</th></tr></thead><tbody>${rows}</tbody></table></div>`
        + `<p class="tpr-id"><b>${pick === r.chosen ? "Choice of the program" : "Your choice"}</b> ${esc(cs[pick].formula)}${pick === r.chosen ? "" : ` (the program chose ${esc(cs[r.chosen == null ? 0 : r.chosen].formula)})`}</p>`
        + `<ul class="tpr-pf" aria-label="Reasons">${reasons(cs[pick]).map(t => `<li><span></span><span>${esc(t)}</span></li>`).join("")}</ul>`
        + `<p class="tpr-id">The share is the part of the total score of the list: a measure of how the candidates compare, not an identification.</p>`;
      box.querySelectorAll("button[data-i]").forEach(b => b.addEventListener("click", () => { pick = +(/** @type {HTMLElement} */ (b).dataset.i || 0); draw(); const nb = box.querySelector(`button[data-i="${pick}"]`); if (nb) /** @type {HTMLElement} */ (nb).focus(); }));
    };
    draw();
  }

  /**
   * Adds the block to the panel of the Identity tab.
   * @param {HTMLElement} panel
   * @param {{ run: () => Promise<any> }} comp `run` asks the engine for the candidates of the spectrum of the row
   */
  function mount(panel, comp) {
    const sec = document.createElement("section"); sec.className = "tpr-comp";
    sec.innerHTML = `<h4 class="tpr-h">Elemental composition</h4><p class="tpr-id">Candidate compositions from the exact mass and the isotope pattern of the MS1 spectrum at the apex.</p>`
      + `<button type="button" class="tpr-more">Suggest compositions</button><div class="tpr-compbox" aria-live="polite"></div>`;
    const btn = /** @type {HTMLButtonElement} */ (sec.querySelector("button")), box = /** @type {HTMLElement} */ (sec.querySelector(".tpr-compbox"));
    btn.addEventListener("click", async () => {
      btn.disabled = true; box.textContent = "Computing…";
      try { show(box, await comp.run()); btn.textContent = "Suggest again"; } catch (/** @type {any} */ e) { box.innerHTML = `<p class="err">${esc(String((e && e.message) || e))}</p>`; }
      btn.disabled = false;
    });
    panel.appendChild(sec);
  }

  G.TPCOMP = { mount, reasons };
})();
