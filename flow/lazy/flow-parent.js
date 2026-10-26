// @ts-check
// Flow (administrator mode): the parent compound of a plan (input type `structure`) written in place, a formula, a SMILES or a JSON {formula, smiles, rt}: the text lives in memory
// (S.parents[input name]) and also in S.samplesJson[input name], the map of the contents of the inputs that the desktop run is given. The input keeps its SHA-256, which the plan records.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** keeps `text` as the content of the parent input and its SHA-256 on the input @param {any} S @param {any} input @param {string} text */
  F.parentSet = async (S, input, text) => {
    const t = text.trim();
    S.parents = S.parents || {}; S.samplesJson = S.samplesJson || {};
    if (!t) { delete S.parents[input.name]; delete S.samplesJson[input.name]; input.sha256 = ""; return; }
    S.parents[input.name] = t; S.samplesJson[input.name] = t;
    input.sha256 = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(t)))].map(b => b.toString(16).padStart(2, "0")).join("");
  };

  /** the text of the parent input `name`, or "" @param {any} S @param {string} name */
  F.parentText = (S, name) => (S.parents && S.parents[name]) || "";

  /** the inline editor of the parent input @param {any} S @param {any} input the input of the plan */
  F.parentEditor = (S, input) => {
    const { el } = F, say = el("p", { class: "fl-note", role: "status" });
    const field = el("input", { type: "text", class: "flx-parent", value: F.parentText(S, input.name), spellcheck: "false", autocomplete: "off", "aria-label": "Parent compound: neutral formula or SMILES",
      placeholder: "C13H24N4O3S or a SMILES", onchange: async (/** @type {any} */ e) => { await F.parentSet(S, input, e.target.value); say.textContent = input.sha256 ? "SHA-256 " + input.sha256.slice(0, 12) + "…" : "No parent yet."; if (S.redraw) S.redraw(); } });
    say.textContent = input.sha256 ? "SHA-256 " + input.sha256.slice(0, 12) + "…" : "No parent yet.";
    return el("div", { class: "flx-smp" }, el("label", { class: "fl-note" }, "Parent compound ", field), say);
  };
})();
