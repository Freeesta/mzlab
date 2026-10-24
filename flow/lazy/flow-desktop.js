// @ts-check
// Flow (administrator mode): the «This computer» runner, only in the desktop app (window.MZLAB_DESKTOP). It sends the plan to the native engine (the live run of flow-live-desktop.js: routes `flow/live/*` of mzlab://,
// the engine of mzlab-cli; the sample lists the page holds go along as text) and lists what each step did from the `prov/1` it answers. The files open in the Data tab become the scan inputs of the plan (a file the app does not hold is left out and reported). The web exports the plan instead.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
  const available = () => !!(/** @type {any} */ (window)).MZLAB_DESKTOP;

  /** one call of the native engine: the text it answers; rejects with {key, params} */
  const post = async (/** @type {string} */ op, /** @type {any} */ p) => {
    const r = await fetch(BASE + "rpc", { method: "POST", body: JSON.stringify({ op, ...p }) }), text = await r.text();
    if (r.status !== 200) { let e = /** @type {any} */ ({}); try { e = JSON.parse(text); } catch (_) { /* a body that is not JSON */ } throw { key: e.error_key || "err.flow.run", params: e.params || {} }; }
    return text;
  };

  /** runs the plan of S in the app (the live run of flow-live-desktop.js, routes flow/live/*) with the files open in the Data tab as its scan inputs; a file the app does not hold is left out and the run goes on with the others;
   * resolves with {dir, steps: [{id, ms, cached, error, made}], excluded: [name], warns}, rejects with {key, params} @param {any} S */
  const run = async S => {
    const out = await F.desktopLive.run(S), prov = JSON.parse(out.prov), ent = prov.entity || {};
    const steps = Object.entries(prov.activity || {}).map(([id, a]) => { const x = /** @type {any} */ (a);
      const made = (x.generated || []).map((/** @type {string} */ k) => ({ type: ent[k] && ent[k].type, label: ent[k] && ent[k].label })).filter((/** @type {any} */ m) => m.type && m.label);
      return { id, ms: x.ms, cached: !!x.cached, made, error: x.error ? { key: x.error.error_key, params: x.error.params } : null }; });
    return { dir: out.dir, steps, excluded: out.excluded, warns: out.warns };
  };
  /** the text of a file the last run wrote @param {string} name */
  const read = name => post("flow/read", { name });
  F.desktopPost = post;
  /** the bytes of a binary file the last run wrote (a .mzfeat): the route answers {base64}; any other answer is an error, so the file stays a button of the Results @param {string} name @returns {Promise<File>} */
  const readFile = async name => {
    const a = JSON.parse(await post("flow/read", { name, encoding: "base64" }));
    if (!a || typeof a.base64 !== "string") throw new Error("not base64");
    return new File([Uint8Array.from(atob(a.base64), c => c.charCodeAt(0))], name);
  };
  F.runners.desktop = { name: "This computer", available, run, report: () => post("flow/report", {}) };

  /** the run box of the desktop: a button and one slot per step (waiting, done with its time, or the reason it failed) @param {any} S */
  F.desktopRunUi = S => {
    const { el, btn } = F;
    const list = el("ul", { class: "fl-slots" }), note = el("p", { class: "fl-note", role: "status" });
    const title = (/** @type {any} */ inst) => F.nameOf(String(inst.step).split("@")[0])[0];
    const slot = (/** @type {any} */ inst, /** @type {string} */ text) => el("li", {}, el("strong", { text: title(inst) }), el("span", { text: ` — ${text}` }));
    const waiting = () => { list.textContent = ""; for (const inst of S.plan.steps) list.append(slot(inst, "waiting")); };
    // while the run goes: every step says what its card says (the words of F.liveView)
    const live = () => {
      list.textContent = "";
      for (const inst of S.plan.steps) { const n = F.liveView.node(inst.id); list.append(slot(inst, n ? F.liveView.look(n).word : "waiting")); }
    };
    // before Run: how many steps the cache of the last run would give back (the native route flow/reuse looks at the keys, nothing runs); a later answer replaces an earlier one
    const reuse = el("p", { class: "fl-note", role: "status", "data-reuse": "1" });
    let asked = 0, due = 0;
    const preview = async () => {
      const n = ++asked;
      if (go.dataset.busy === "1" || S.problems.length) { reuse.textContent = ""; return; }
      try {
        const { plan } = F.runPlanFromData(S), a = JSON.parse(await post("flow/reuse", { plan: F.exportText(plan), files: S.samplesJson || {} }));
        if (n === asked && go.dataset.busy !== "1") reuse.textContent = `${a.reused} step${a.reused === 1 ? "" : "s"} reused, ${a.compute} to compute`;
      } catch (_) { if (n === asked) reuse.textContent = ""; }
    };
    const soon = () => { clearTimeout(due); due = window.setTimeout(preview, 600); };
    const rep = btn("file", "Open the report of the last run", async () => {
      try { F.openResult(await F.lastRunReport()); } catch (e) { note.textContent = F.message(/** @type {any} */ (e)); }
    }, "Report");
    rep.disabled = true;
    const go = btn("run", "Run the plan on this computer", async () => {
      go.dataset.busy = "1"; go.disabled = true; note.textContent = "Running…"; waiting();
      document.addEventListener("flow:live-view", live);
      try {
        const res = await run(S).finally(() => document.removeEventListener("flow:live-view", live)), by = new Map(res.steps.map((/** @type {any} */ s) => [s.id, s]));
        list.textContent = ""; S.results = {};
        for (const s of res.steps) S.results[s.id] = s.made.map((/** @type {any} */ m) => ({ ...m, text: () => read(m.label), load: /\.mzfeat$/i.test(m.label) ? () => readFile(m.label) : undefined }));
        if (S.redraw) S.redraw();
        for (const inst of S.plan.steps) { const s = /** @type {any} */ (by.get(inst.id)); list.append(slot(inst, !s ? "skipped" : s.error ? F.message({ key: s.error.key, params: s.error.params }) : s.cached ? "reused (nothing changed)" : `done in ${(s.ms / 1000).toFixed(1)} s`)); }
        const reused = res.steps.filter((/** @type {any} */ s) => s.cached).length, made = res.steps.filter((/** @type {any} */ s) => !s.error).length - reused;
        const cut = res.excluded.length ? ` ${res.excluded.length} file${res.excluded.length > 1 ? "s" : ""} excluded (not held by the app): ${res.excluded.join(", ")}.` : "";
        note.textContent = `Finished. ${reused} steps reused, ${made} computed. The results are in ${res.dir}.${cut}${res.warns.length ? " Warning: " + res.warns.join(" ") : ""}`;
        rep.disabled = false;
        await F.afterRun(S);
      } catch (e) { list.textContent = ""; note.textContent = F.message(/** @type {any} */ (e)); }
      go.dataset.busy = ""; go.disabled = S.problems.length > 0;
      soon();
    }, "Run on this computer");
    waiting();
    const box = el("div", { class: "fl-meta" }, go, rep, note, reuse, list);
    // a plan with problems does not run: the reason stays next to the button and follows the plan
    const gate = () => {
      if (!box.isConnected && box.dataset.shown) { document.removeEventListener("flow:changed", gate); clearTimeout(due); return; }
      box.dataset.shown = "1";
      soon();
      const bad = S.problems.length;
      go.disabled = go.dataset.busy === "1" || bad > 0;
      if (bad) note.textContent = `Cannot run yet: the plan has ${bad} problem${bad === 1 ? "" : "s"}. Open the checklist to see what to fix.`;
      else if (note.textContent.startsWith("Cannot run yet")) note.textContent = "";
    };
    document.addEventListener("flow:changed", gate); gate();
    return box;
  };
})();
