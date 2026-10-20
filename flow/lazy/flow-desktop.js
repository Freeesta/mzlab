// @ts-check
// Flow (administrator mode): the «This computer» runner, only in the desktop app (window.MZLAB_DESKTOP). It sends the plan to the native engine (route `flow/run` of mzlab://,
// the engine of mzlab-cli) and lists what each step did from the `prov/1` it answers. The files the plan names must be open in the Data tab. The web exports the plan instead.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
  const available = () => !!(/** @type {any} */ (window)).MZLAB_DESKTOP;

  /** runs the plan of S in the app; resolves with {dir, steps: [{id, ms, error}]}, rejects with {key, params} @param {any} S */
  const run = async S => {
    const r = await fetch(BASE + "rpc", { method: "POST", body: JSON.stringify({ op: "flow/run", plan: F.exportText(S.plan) }) });
    const text = await r.text();
    if (r.status !== 200) { let e = /** @type {any} */ ({}); try { e = JSON.parse(text); } catch (_) { /* a body that is not JSON */ } throw { key: e.error_key || "err.flow.run", params: e.params || {} }; }
    const out = JSON.parse(text), prov = JSON.parse(out.prov);
    const steps = Object.entries(prov.activity || {}).map(([id, a]) => { const x = /** @type {any} */ (a); return { id, ms: x.ms, error: x.error ? { key: x.error.error_key, params: x.error.params } : null }; });
    return { dir: out.dir, steps };
  };
  F.runners.desktop = { name: "This computer", available, run };

  /** the run box of the desktop: a button and one slot per step (waiting, done with its time, or the reason it failed) @param {any} S */
  F.desktopRunUi = S => {
    const { el, btn } = F;
    const list = el("ul", { class: "fl-slots" }), note = el("p", { class: "fl-note", role: "status" });
    const title = (/** @type {any} */ inst) => F.nameOf(String(inst.step).split("@")[0])[0];
    const slot = (/** @type {any} */ inst, /** @type {string} */ text) => el("li", {}, el("strong", { text: title(inst) }), el("span", { text: ` — ${text}` }));
    const waiting = () => { list.textContent = ""; for (const inst of S.plan.steps) list.append(slot(inst, "waiting")); };
    const go = btn("run", "Run the plan on this computer", async () => {
      go.disabled = true; note.textContent = "Running…"; waiting();
      try {
        const res = await run(S), by = new Map(res.steps.map((/** @type {any} */ s) => [s.id, s]));
        list.textContent = "";
        for (const inst of S.plan.steps) { const s = /** @type {any} */ (by.get(inst.id)); list.append(slot(inst, !s ? "skipped" : s.error ? F.message({ key: s.error.key, params: s.error.params }) : `done in ${(s.ms / 1000).toFixed(1)} s`)); }
        note.textContent = `Finished. The results are in ${res.dir}.`;
      } catch (e) { list.textContent = ""; note.textContent = F.message(/** @type {any} */ (e)); }
      go.disabled = false;
    }, "Run on this computer");
    waiting();
    return el("div", { class: "fl-meta" }, go, note, list);
  };
})();
