// @ts-check
// Flow (administrator mode): the form of one step, generated from its descriptor (step and algorithm parameters, with title, unit, limits, default and description),
// plus the choice of algorithm and preset. The file keeps only what differs from the default (F.setParam). A field is committed on `change`, never on every key.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  // the values of an enum are codes in the descriptor; these are the words shown
  /** @type {Record<string, Record<string, string>>} */
  const WORDS = { polarity: { "1": "positive", "-1": "negative", "0": "all scans" } };

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
    const num = p.type === "number" || p.type === "integer";
    return el("input", {
      type: num ? "number" : "text", id, "data-fk": id, value: cur ?? "", step: p.type === "integer" ? 1 : "any", min: p.minimum, max: p.maximum,
      placeholder: p["x-estimate"] ? "estimated from the data" : "", onchange: (/** @type {any} */ e) => { const t = e.target.value; set(t === "" ? null : num ? +t : t); },
    });
  };

  /** the fields of a list of [name, schema] @param {any} inst @param {any} step @param {[string, any][]} entries @param {() => void} done @param {string} prefix */
  const fields = (inst, step, entries, done, prefix) => {
    const { el } = F, defaults = F.defaults(inst, step), rows = [];
    for (const [name, p] of entries) {
      const id = `fl-${prefix}-${inst.id}-${name}`, explicit = inst.params && name in inst.params, cur = explicit ? inst.params[name] : defaults[name];
      const set = (/** @type {any} */ v) => { F.setParam(inst, step, name, v); done(); };
      const label = el("label", { for: p.type === "array" ? undefined : id, text: p.title + (p["x-unit"] ? ` (${p["x-unit"]})` : "") });
      const reset = explicit ? F.btn("reset", `Back to the default: ${p.title}`, () => set(null)) : el("span");
      rows.push(label, el("span", { class: "fl-meta" }, control(p, name, cur, set, id), reset));
      if (p.description) rows.push(el("span", { class: "fl-ds", text: p.description }));
    }
    return rows;
  };

  /** the form of an instance: algorithm, preset, parameters (the advanced ones under «More») @param {any} inst @param {any} step @param {() => void} done called after each change */
  F.form = (inst, step, done) => {
    const { el } = F, e = F.effective(inst, step), box = el("div", { class: "fl-form" });
    if (step.algorithms && step.algorithms.length) {
      const id = `fl-alg-${inst.id}`;
      box.append(el("label", { for: id, text: "Algorithm" }), el("select", { id, "data-fk": id, onchange: (/** @type {any} */ ev) => { F.setAlgorithm(inst, step, ev.target.value || null); done(); } },
        ...step.algorithms.map((/** @type {any} */ a) => el("option", { value: a.id, selected: !!e.algorithm && a.id === e.algorithm.id, text: a.title + (a.status === "planned" ? " (planned)" : "") }))));
      if (e.algorithm && e.algorithm.doc) box.append(el("span"), el("span", { class: "fl-ds", text: e.algorithm.doc }));
    }
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
