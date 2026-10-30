// @ts-check
// Flow (administrator mode): the form of one step, generated from its descriptor (step and algorithm parameters, with label, unit, limits and default; the sentence that
// says what a field does is behind its «ⓘ», F.tip), plus the choice of algorithm and preset. The file keeps only what differs from the default (F.setParam). A field is committed on `change`, never on every key.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  // the values of an enum are codes in the descriptor; these are the words shown
  /** @type {Record<string, Record<string, string>>} */
  const WORDS = { polarity: { "1": "positive", "-1": "negative", "0": "all scans" } };

  /** the «ⓘ» of a field and its note: a click (or Enter) opens the note under the field, Esc, a second click or the focus elsewhere close it; the field points to the note (aria-describedby)
   * @param {any} field the control the note describes @param {string} label the name of the field @param {string} text one sentence @param {string} id unique id of the note
   * @returns {{ btn: any, note: any }} */
  F.tip = (field, label, text, id) => {
    const { el } = F, note = el("span", { class: "fl-tipnote", id, role: "note", hidden: true, text });
    const btn = el("button", { type: "button", class: "fl-b fl-tip", "aria-label": `About ${label}`, "aria-expanded": "false", "aria-controls": id }, "ⓘ");
    if (field) field.setAttribute("aria-describedby", id);
    const shut = () => {
      note.hidden = true; btn.setAttribute("aria-expanded", "false");
      document.removeEventListener("keydown", onKey, true); document.removeEventListener("pointerdown", onAway, true); document.removeEventListener("focusin", onAway, true);
    };
    /** @param {KeyboardEvent} e */
    function onKey(e) { if (e.key === "Escape") { shut(); btn.focus(); } }
    /** @param {Event} e */
    function onAway(e) { if (!btn.contains(/** @type {Node} */ (e.target)) && !note.contains(/** @type {Node} */ (e.target))) shut(); }
    btn.addEventListener("click", () => {
      if (!note.hidden) return shut();
      note.hidden = false; btn.setAttribute("aria-expanded", "true");
      document.addEventListener("keydown", onKey, true); document.addEventListener("pointerdown", onAway, true); document.addEventListener("focusin", onAway, true);
    });
    return { btn, note };
  };

  /** @param {any} p @param {string} name @param {any} cur @param {(v: any) => void} set @param {string} id */
  const control = (p, name, cur, set, id) => {
    const { el } = F;
    if (p.enum) {
      return el("select", { id, "data-fk": id, onchange: (/** @type {any} */ e) => set(p.enum[+e.target.value]) },
        ...p.enum.map((/** @type {any} */ v, /** @type {number} */ k) => el("option", { value: k, selected: v === cur, text: (WORDS[name] || {})[String(v)] || String(v) })));
    }
    if (p.type === "boolean") return el("input", { type: "checkbox", id, "data-fk": id, checked: !!cur, onchange: (/** @type {any} */ e) => set(e.target.checked) });
    if (p.type === "array") {
      const pair = (/** @type {any} */ v) => Array.isArray(v) ? v : ["", ""];
      const box = el("span", { class: "fl-meta", id }), n = pair(cur).map((/** @type {any} */ x, /** @type {number} */ k) => el("input", { type: "number", step: "any", value: x, "aria-label": `${p.title} ${k ? "to" : "from"}`, "data-fk": id + k }));
      for (const i of n) i.addEventListener("change", () => { const v = n.map((/** @type {any} */ x) => x.value === "" ? NaN : +x.value); set(v.some(Number.isNaN) ? null : v); });
      box.append(...n); return box;
    }
    if (p["x-liste"] && F.listeEditor) { const ed = F.listeEditor(p, cur, set, id); if (ed) return ed; }
    const num = p.type === "number" || p.type === "integer";
    return el("input", {
      type: num ? "number" : "text", id, "data-fk": id, value: cur ?? "", step: p.type === "integer" ? 1 : "any", min: p.minimum, max: p.maximum,
      placeholder: p["x-estimate"] ? "estimated" : "", onchange: (/** @type {any} */ e) => { const t = e.target.value; set(t === "" ? null : num ? +t : t); },
    });
  };

  /** the fields of a list of [name, schema] @param {any} inst @param {any} step @param {[string, any][]} entries @param {() => void} done @param {string} prefix */
  const fields = (inst, step, entries, done, prefix) => {
    const { el } = F, defaults = F.defaults(inst, step), rows = [];
    for (const [name, p] of entries) {
      const id = `fl-${prefix}-${inst.id}-${name}`, explicit = inst.params && name in inst.params, cur = explicit ? inst.params[name] : defaults[name];
      const set = (/** @type {any} */ v) => { F.setParam(inst, step, name, v); done(); };
      const shown = p["x-label"] || p.title, text = shown + (p["x-unit"] ? ` (${p["x-unit"]})` : "");
      const label = el("label", { for: p.type === "array" ? undefined : id, text });
      const reset = explicit ? F.btn("reset", `Back to the default: ${shown}`, () => set(null)) : el("span");
      const ctl = control(p, name, cur, set, id), tipText = p["x-tip"] || p.description;
      const tip = tipText ? F.tip(ctl, shown, tipText, `${id}-tip`) : null;
      // a value the program estimates from the data when the plan runs: say so and offer to fix it at the default
      const estTip = p["x-estimate"] && !explicit && defaults[name] !== undefined ? F.tip(null, `${shown} estimate`, "Left empty, it is estimated from the scan index of your files when the plan runs.", `${id}-est`) : null;
      const est = estTip ? el("span", { class: "flx-est" }, estTip.btn,
        F.btn("ok", `Use a fixed value for ${shown}`, () => { inst.params = inst.params || {}; inst.params[name] = defaults[name]; done(); }, "Use fixed")) : null;
      rows.push(label, el("span", { class: "fl-meta" }, ctl, tip && tip.btn, est, reset));
      if (tip) rows.push(tip.note);
      if (estTip) rows.push(estTip.note);
    }
    return rows;
  };

  /** the form of an instance: algorithm, preset, parameters (the advanced ones under «More») @param {any} inst @param {any} step @param {() => void} done called after each change */
  F.form = (inst, step, done) => {
    const { el } = F, e = F.effective(inst, step), box = el("div", { class: "fl-form" });
    if (step.algorithms && step.algorithms.length) {
      const id = `fl-alg-${inst.id}`;
      const sel = el("select", { id, "data-fk": id, onchange: (/** @type {any} */ ev) => { F.setAlgorithm(inst, step, ev.target.value || null); done(); } },
        ...step.algorithms.map((/** @type {any} */ a) => el("option", { value: a.id, selected: !!e.algorithm && a.id === e.algorithm.id, text: a.title + (a.status === "planned" ? " (planned)" : "") })));
      const tip = e.algorithm && e.algorithm.doc ? F.tip(sel, "Algorithm", e.algorithm.doc, `${id}-tip`) : null;
      box.append(el("label", { for: id, text: "Algorithm" }), el("span", { class: "fl-meta" }, sel, tip && tip.btn));
      if (tip) box.append(el("span"), tip.note);
    }
    if (F.msnMolecule && inst.step.startsWith("spectra.msn_graph@")) box.append(F.msnMolecule(inst, step, done));
    // the regulatory lists ticked in the Library window (libreria-normative.js keeps them as `lists` and `include`) become this step's own parameters
    if (/^(screen\.suspects|annot\.regulatory)@/.test(inst.step)) box.append(el("span"), F.btn("ok", "Use the regulatory lists ticked in the Library window", () => {
      let sel = null; try { sel = JSON.parse(localStorage.getItem("mzlab.normative.sel") || "null"); } catch (_) { /* no storage */ }
      if (!sel || !sel.lists) return;
      F.setParam(inst, step, "lists", sel.lists); F.setParam(inst, step, "include", sel.include || null); done();
    }, "Use Library lists"));
    const presets = F.presetList(step);
    if (presets.length) {
      const id = `fl-pre-${inst.id}`;
      box.append(el("label", { for: id, text: "Preset" }), el("select", { id, "data-fk": id, onchange: (/** @type {any} */ ev) => { F.setPreset(inst, ev.target.value || null); done(); } },
        el("option", { value: "", text: "none" }), ...presets.map((/** @type {any} */ p) => el("option", { value: p.id, selected: !!e.preset && p.id === e.preset.id, text: p.title }))));
    }
    const all = F.declared(step, e.algorithm), main = all.filter((/** @type {any} */ x) => !x[1]["x-advanced"]), more = all.filter((/** @type {any} */ x) => x[1]["x-advanced"]);
    box.append(...fields(inst, step, main, done, "p"));
    if (more.length) {
      const adv = el("div", { class: "fl-form" });
      adv.append(...fields(inst, step, more, done, "m"));
      const open = more.some((/** @type {any} */ x) => inst.params && x[0] in inst.params);
      box.append(el("details", { style: "grid-column:1/-1", open }, el("summary", { text: `More (${more.length})` }), adv));
    }
    return box;
  };
})();
