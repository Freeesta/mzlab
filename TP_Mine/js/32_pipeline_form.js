// @ts-strict
// TPMINE-PRIVATE  Form of the Pipeline panel (window.PIPEFORM) generated from the step/1 descriptors (docs/schemi/steps-1.json, shipped as static/mzfinder/steps-1.json): title, unit, limits, default and
// description of every field come from the descriptor; MAP is the ONE place that says which parameter of which step feeds which input of the panel. Used by 32_pipeline.js, which keeps the ids (#pipe-ppm...).
(() => {
  "use strict";
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
  // panel input -> step and parameter. The panel keys are not the engine names (run.rs apply_params): `ppm` and `rt` feed align and gap filling at once, here the descriptor that names the number;
  // key null = the step itself is the switch (a checkbox); more = under «More»; min = limit of the panel when stricter than the schema; skip = enum values the panel does not offer
  /** @type {{id: string, step: string, key: string | null, more?: boolean, min?: number, skip?: any[]}[]} */
  const MAP = [
    { id: "ppm", step: "features.align", key: "ppm", min: 0.5 },
    { id: "floor", step: "features.detect", key: "floor", min: 1 },
    { id: "hmin", step: "features.detect", key: "min_height", min: 1 },
    { id: "npts", step: "features.gapfill", key: "min_points" },
    { id: "rt", step: "features.gapfill", key: "rt_tol", min: 0.005 },
    { id: "fill", step: "features.gapfill", key: null },
    { id: "pol", step: "features.detect", key: "polarity", more: true, skip: [0] },
    { id: "mode", step: "features.align", key: "mode", more: true },
  ];
  // the descriptors name the values of an enum by their code; the words shown are these
  /** @type {Record<string, Record<string, string>>} */
  const WORDS = { polarity: { "1": "positive", "-1": "negative" }, mode: { family: "one per family of ions", ion: "one per ion" } };
  const CSS = `#pipe .pf{display:flex;flex-direction:column;gap:2px;font-size:12px}#pipe .pf .pin{display:flex;gap:4px;align-items:center}#pipe .pf .pin input,#pipe .pf .pin select{flex:1;min-width:0}
#pipe .pf label.ck{flex-direction:row;align-items:center;gap:6px;min-height:24px}#pipe .pf label.ck input{width:24px;height:24px;min-height:24px;flex:none}
#pipe .rs{display:inline-flex;align-items:center;justify-content:center;gap:4px;padding:0 6px;border:1px solid var(--line);border-radius:6px;background:var(--panel);color:var(--ink);cursor:pointer;transition:opacity var(--t-fast),background-color var(--t-med)}
#pipe .rs:hover:not(:disabled){background:var(--sel)}#pipe .rs:disabled{opacity:.4;cursor:default}#pipe .rs:focus-visible{outline:2px solid var(--accent);outline-offset:1px}
@media (prefers-reduced-motion:reduce){#pipe .rs{transition:none}}`;

  /** the list of step/1 descriptors from the files given to the scripts (QTOOLS.ctx.files), null when absent or unreadable */
  function load(/** @type {any} */ files) {
    const b = files && files["steps-1.json"];
    if (!b) return null;
    try { const l = JSON.parse(typeof b === "string" ? b : new TextDecoder().decode(b)); return Array.isArray(l) ? l : null; } catch (_) { return null; }
  }
  /** MAP joined with its descriptors; null if a descriptor named by MAP is missing */
  function resolve(/** @type {any[]} */ steps) {
    const out = MAP.map(m => {
      const s = steps.find(x => x.id === m.step), p = s && m.key ? s.params.properties[m.key] : null;
      return s && (p || !m.key) ? { m, s, p } : null;
    });
    return out.every(Boolean) ? /** @type {any[]} */ (out) : null;
  }
  const icon = () => { const q = /** @type {any} */ (window).QICON; return q && q.reset ? q.get("reset", 16) : ""; };
  function field(/** @type {any} */ r) {
    const { m, s, p } = r, id = "pipe-" + m.id, why = `<span class="why" id="${id}-why">${esc(p && p.description || "")}</span>`;
    if (!p) return `<div class="pf"><label class="ck" for="${id}"><input id="${id}" type="checkbox" checked>${esc(s.title)}</label></div>`;
    const title = esc(p.title) + (p["x-unit"] ? ` (${esc(p["x-unit"])})` : ""), est = !!p["x-estimate"];
    const ctl = p.enum
      ? `<select id="${id}" aria-describedby="${id}-why">${p.enum.filter((/** @type {any} */ v) => !(m.skip || []).includes(v)).map((/** @type {any} */ v) => `<option value="${esc(v)}">${esc((WORDS[m.key] || {})[v] || v)}</option>`).join("")}</select>`
      : `<input id="${id}" type="number" value="${esc(p.default)}" step="${p.type === "integer" ? 1 : "any"}" min="${m.min ?? p.minimum ?? ""}" aria-describedby="${id}-why">`;
    const name = est ? "Auto" : "Reset";
    return `<div class="pf"><label for="${id}">${title}</label><div class="pin">${ctl}<button type="button" class="rs" data-rs="${m.id}" aria-label="${name}: ${esc(p.title)} ${est ? "estimated from the file" : "back to " + esc(WORDS[m.key]?.[p.default] ?? p.default)}" title="${est ? "Estimate it from the file (the proof is below)" : "Back to the default"}">${icon()}${est ? "<span>Auto</span>" : ""}</button></div>${why}</div>`;
  }
  /** markup of the panel parameters from the descriptors: {main, more} (the ids are the ones of the panel) */
  function html(/** @type {any[]} */ steps) {
    const rs = resolve(steps);
    if (!rs) throw new Error("steps-1.json does not describe " + MAP.map(m => m.step + "." + m.key).join(", "));
    const f = (/** @type {boolean} */ more) => rs.filter(r => !!r.m.more === more).map(field).join("");
    return { main: f(false), more: f(true), rs };
  }
  /** wires the reset buttons of `root`; host.onReset(id) runs after the default is put back, host.atDefault(id) may answer for the inputs the host tracks (an estimated value) */
  function mount(/** @type {ParentNode} */ root, /** @type {any[]} */ rs, /** @type {{onReset: (id: string) => void, atDefault: (id: string) => boolean | undefined}} */ host) {
    const input = (/** @type {string} */ id) => /** @type {HTMLInputElement | null} */ (root.querySelector("#pipe-" + id));
    const def = (/** @type {any} */ p) => String(p.default);
    function sync() {
      for (const { m, p } of rs) {
        const b = /** @type {HTMLButtonElement | null} */ (root.querySelector(`[data-rs="${m.id}"]`)), i = input(m.id);
        if (!b || !i || !p) continue;
        const own = host.atDefault(m.id);
        b.disabled = own !== undefined ? own : p.type === "number" || p.type === "integer" ? +i.value === +p.default : i.value === def(p);
      }
    }
    root.addEventListener("click", e => {
      const b = /** @type {HTMLElement} */ (e.target).closest("[data-rs]");
      const r = b && rs.find(x => x.m.id === b.getAttribute("data-rs")), i = r && input(r.m.id);
      if (!r || !i) return;
      i.value = def(r.p); host.onReset(r.m.id); sync(); i.focus();
    });
    root.addEventListener("input", sync);
    sync();
    return { sync };
  }
  /** the values of the descriptors that the panel defaults must equal (the e2e compares them with PIPE.DEF) */
  function defaults(/** @type {any[]} */ steps) { const o = /** @type {Record<string, any>} */ ({}); for (const r of resolve(steps) || []) o[r.m.id] = r.p ? r.p.default : true; return o; }

  /** @type {any} */ (window).PIPEFORM = { MAP, css: CSS, load, html, mount, defaults };
})();
