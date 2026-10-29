// @ts-check
// Flow (administrator mode): the parent compound of a plan (input type `structure`) written in place. The box is the search box of the molecules database (molecole-campo.js):
// a name, a synonym, a formula, an InChIKey or an m/z, with the ranked list (one slip of a letter is forgiven). A formula or an m/z is asked what it is, the neutral molecule M
// (the default) or an ion of the adduct table of the engine, and the plan keeps the NEUTRAL molecule: S.parents[input name] is a JSON {name, formula, smiles, inchikey, mass, typed:{text, adduct}}
// (also in S.samplesJson, the map of the contents of the inputs that the desktop run is given). A SMILES, or a formula/SMILES/JSON written by hand, is still accepted as text.
// The input keeps its SHA-256, which the plan records. Docs: docs/agenti/flow-web.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  /** the app module `name` (loaded once, the same instance as the app's) @param {string} name */
  const app = name => import(/* webpackIgnore: true */ new URL(name, G.QTOOLS && G.QTOOLS.ctx ? G.QTOOLS.ctx.base : document.baseURI).href);

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

  /** the JSON object a parent text holds, or null for a formula or a SMILES written as plain text @param {string} text */
  const savedOf = text => { try { const j = JSON.parse(text); return j && typeof j === "object" && !Array.isArray(j) ? j : null; } catch (_) { return null; } };

  /** the line that says what the node holds @param {any} j the saved parent @param {string} sha */
  const summary = (j, sha) => {
    const t = j.typed || {}, bits = [];
    if (j.name) bits.push(j.name);
    if (j.formula) bits.push("Neutral formula " + j.formula);
    if (typeof j.mass === "number") bits.push("M = " + j.mass.toFixed(4) + " Da");
    if (t.adduct && t.adduct !== "neutral" && t.text) bits.push(`read from ${t.text} as ${t.adduct}`);
    else if (!j.name && t.text) bits.push("typed as the neutral molecule (M)");
    if (!j.formula) bits.push("no formula yet: pick a molecule or type its formula, tp.generate needs one");
    return bits.join(" · ") + (sha ? " · SHA-256 " + sha.slice(0, 12) + "…" : "");
  };

  /** the plain box of the first version, for a page that cannot give the search box @param {any} S @param {any} input @param {HTMLElement} say */
  const plainBox = (S, input, say) => {
    const { el } = F;
    const shaLine = () => { say.textContent = input.sha256 ? "SHA-256 " + input.sha256.slice(0, 12) + "…" : "No parent yet."; };
    const field = el("input", { type: "text", class: "flx-parent", value: F.parentText(S, input.name), spellcheck: "false", autocomplete: "off", "aria-label": "Parent compound: neutral formula or SMILES",
      placeholder: "C13H24N4O3S or a SMILES", onchange: async (/** @type {any} */ e) => { await F.parentSet(S, input, e.target.value); shaLine(); if (S.redraw) S.redraw(); } });
    shaLine();
    return el("label", { class: "fl-note" }, "Parent compound ", field);
  };

  /** the inline editor of the parent input @param {any} S @param {any} input the input of the plan */
  F.parentEditor = (S, input) => {
    const { el } = F, say = el("p", { class: "fl-note", role: "status" }), host = el("div", { class: "flx-parent-host" });
    const sel = /** @type {HTMLSelectElement} */ (el("select", { class: "flx-parent-adduct", "aria-label": "What the typed formula or m/z is" }));
    const row = el("label", { class: "fl-note flx-parent-kind" }, "The typed formula or m/z is ", sel);
    row.hidden = true;
    const box = el("div", { class: "flx-smp flx-parent-box" }, host, row, say);
    const old = savedOf(F.parentText(S, input.name));
    let text = old && old.typed ? String(old.typed.text || "") : "", chosen = old && old.name ? String(old.name) : "";
    const show = () => { const j = savedOf(F.parentText(S, input.name)); say.textContent = j ? summary(j, input.sha256) : F.parentText(S, input.name) ? "SMILES or text kept as typed (neutral) · SHA-256 " + input.sha256.slice(0, 12) + "…" : "No parent yet."; };
    app("molecole-campo.js").then(async c => {
      const a = await app("molecole-archivio.js");
      const names = await a.addotti();
      sel.append(el("option", { value: "neutral" }, "the neutral molecule (M)"), ...names.map((/** @type {string} */ n) => el("option", { value: n }, "an ion " + n)));
      sel.value = old && old.typed && names.includes(old.typed.adduct) ? old.typed.adduct : "neutral";
      /** the reading of `t` as a formula or an m/z under `adduct`, or null when it is neither @param {string} t @param {string} adduct */
      const ionOf = async (t, adduct) => {
        if (!/^[\d.]+$/.test(t) && !(/^[A-Za-z0-9()]+$/.test(t) && /\d/.test(t))) return null;
        try { return await a.ione(t, adduct); } catch (e) { return /** @type {any} */ (e).key === "err.ion.notHeld" ? { error: `${t} has no ${adduct} atoms to take away, so it is not that ion.` } : null; }
      };
      const store = async (/** @type {any} */ j) => { await F.parentSet(S, input, JSON.stringify(j)); show(); if (S.redraw) S.redraw(); };
      const commit = async (/** @type {string} */ raw) => {
        const t = raw.trim(); text = t;
        if (!t) { chosen = ""; row.hidden = true; await F.parentSet(S, input, ""); show(); return; }
        const r = await ionOf(t, sel.value);
        row.hidden = !r;
        if (r && "error" in r) { say.textContent = r.error; return; }
        if (r) { chosen = ""; await store({ ...(r.formula ? { formula: r.formula } : {}), mass: r.massa, typed: { text: t, adduct: sel.value } }); return; }
        if (t === chosen) return;
        const hit = (await a.cerca(t, 1, 10, true))[0];
        if (hit && hit.nome.toLowerCase() === t.toLowerCase()) { await pick(hit); return; }
        if (/[\d()=#[\]@]/.test(t) || /^[A-Z]+$/.test(t)) { chosen = ""; await F.parentSet(S, input, t); show(); if (S.redraw) S.redraw(); return; }
        say.textContent = `No molecule called ${t}: pick one from the list, or type a formula, an m/z or a SMILES.`;
      };
      const pick = async (/** @type {any} */ row0) => {
        const m = (await a.prendi(row0.id)) || row0;
        chosen = m.nome || ""; row.hidden = true;
        await store({ name: chosen, ...(m.formula ? { formula: m.formula } : {}), smiles: m.smiles || "", inchikey: m.inchikey || "", mass: m.massa, typed: { text: chosen, adduct: "neutral" } });
      };
      const f = c.campo(host, {
        testi: { label: "Parent compound", placeholder: "name, formula, InChIKey or m/z", none: "No molecule found", count: (/** @type {number} */ n) => `${n} found` },
        valore: text || chosen, ampia: true, gestisci: null,
        // under an ion reading the list is searched with the neutral formula or mass, the database holds neutral molecules
        interroga: async (/** @type {string} */ t) => { const r = sel.value === "neutral" ? null : await ionOf(t, sel.value); return r && !("error" in r) ? (r.formula || String(r.massa)) : t; },
        onScegli: pick,
      });
      f.input.addEventListener("input", async () => { const r = await ionOf(f.input.value.trim(), sel.value); row.hidden = !r; });
      f.input.addEventListener("change", () => { commit(f.input.value); });
      f.input.addEventListener("keydown", (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter" && f.input.getAttribute("aria-expanded") !== "true") commit(f.input.value); });
      sel.addEventListener("change", () => { commit(f.input.value); });
      row.hidden = !(old && old.typed && old.typed.adduct && /^[\d.]+$|\d/.test(String(old.typed.text)) && !chosen);
    }).catch(() => { host.replaceChildren(plainBox(S, input, say)); });
    show();
    return box;
  };
})();
