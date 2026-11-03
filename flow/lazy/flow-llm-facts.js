// @ts-check
// Flow (profile «ricerca»): the facts of the open files that need no model. It collects from the engine routes (the same in the site and in the app: `api/session`, `api/scan_stats`,
// `api/mrm`, `api/metodo`, `api/campioni/proponi`) the sources of `ispezione/1`, asks Rust (llm/ worker, no conversation, nothing leaves the site) for the inspection and the proposals
// of the parameters, and shows them as the «Files» card with «Apply»/«Discard» (one entry of S.undo each). It also gives the readers of the assistant their inputs (`readerInput`) and the
// charts their sources (`F.chartSources`). Loaded by flow-shell.js in the profile «ricerca», with or without the assistant. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const CSS = `
.flx .flx-facts{border:1px solid var(--line);border-radius:8px;background:var(--panel);color:var(--ink);padding:6px 8px;margin-top:6px}
.flx .flx-facts>summary{min-height:24px;cursor:pointer;font-weight:600;display:flex;align-items:center;gap:6px}
.flx .flx-facts button,.flx .flx-facts summary{min-height:24px;min-width:24px}
.flx .flx-facts button:focus-visible,.flx .flx-facts summary:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
@media (pointer:coarse){.flx .flx-facts button,.flx .flx-facts summary{min-height:40px;min-width:40px}}
.flx .flx-facts-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:6px;margin-top:6px}
.flx .flx-facts-file,.flx .flx-facts-prop{border:1px solid var(--line);border-left:4px solid var(--accent);border-radius:6px;padding:4px 8px;background:var(--panel)}
.flx .flx-facts-file.bad{border-left-color:var(--bad)}.flx .flx-facts dl{margin:0;display:grid;grid-template-columns:auto 1fr;gap:1px 8px;font-size:12px}
.flx .flx-facts dt{color:var(--muted)}.flx .flx-facts dd{margin:0;overflow-wrap:anywhere}
.flx .flx-facts h4{margin:0 0 2px;font-size:13px;overflow-wrap:anywhere}.flx .flx-facts .note{color:var(--muted);font-size:12px;margin:2px 0}
.flx .flx-facts .row{display:flex;flex-wrap:wrap;gap:6px;align-items:center}.flx .flx-facts .done{opacity:.7}
@media (prefers-reduced-motion:reduce){.flx .flx-facts,.flx .flx-facts *{transition:none!important;animation:none!important}}
`;
  /** the budget of the inspection, in tokens (`mzlab-cli llm inspect`'s default) */
  const BUDGET = 30000;

  /** @param {string} url @param {any} [body] */
  const J = async (url, body) => {
    const r = await fetch(url, body === undefined ? undefined : { method: "POST", body: JSON.stringify(body) }), v = JSON.parse(await r.text());
    if (v && v.error_key) throw v;
    return v;
  };
  const hideNames = () => { try { return !!(JSON.parse(localStorage.getItem("mzlab.llm") || "{}") || {}).hideNames; } catch (_) { return false; } };

  /** the open files of the engine, grouped by file (the parts of a file share its name before «#») and sorted by name, as `mzlab-cli llm inspect` lists a folder: [name, [{item, k}]] */
  const openFiles = async () => {
    const session = /** @type {any[]} */ ((await J("api/session")).session || []), by = /** @type {Map<string, {item: any, k: number}[]>} */ (new Map());
    session.forEach((item, k) => {
      const name = String(item.file || "").split("#")[0];
      if (!name) return;
      if (!by.has(name)) by.set(name, []);
      /** @type {any[]} */ (by.get(name)).push({ item, k });
    });
    return [...by].sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
  };

  /** the sources of `ispezione/1`, in the order and with the keys of `mzlab-cli llm inspect` (crates/mzlab-cli/src/llm_facts.rs) @param {boolean} hide */
  const collect = async hide => {
    const files = await openFiles();
    let samples = /** @type {any[]} */ ([]);
    try { samples = (await J("api/campioni/proponi", { files: files.map(([name]) => ({ name })) })).campioni || []; } catch (_) { /* no proposal: every file is a sample */ }
    const out = [];
    for (let i = 0; i < files.length; i++) {
      const [name, parts] = files[i], k = parts[0].k;
      try {
        /** @type {any} */ const o = { name, items: parts.map(p => p.item), scan_stats: await J("api/scan_stats?k=" + k) };
        const srm = parts.find(p => +p.item.srm > 0);
        if (srm) o.mrm = (await J("api/mrm?k=" + srm.k)).files[0].transitions;
        const doc = ((await J("api/metodo?k=" + k)) || {}).metodo;
        if (doc) o.method = doc;
        o.sample = { tipo: (samples[i] && samples[i].tipo) || "sample", ordine_iniezione: i + 1 };
        out.push(o);
      } catch (e) { out.push({ name, error: { error_key: (/** @type {any} */ (e) && /** @type {any} */ (e).error_key) || "err.file.unreadable" } }); }
    }
    return { profile: "ricerca", hide_names: hide, files: out };
  };

  /** the worker of the facts, opened on first use @type {any} */ let run = null;
  const worker = async () => {
    if (run) return run;
    // served from static/mzfinder/lazy/, the module from static/llm/ (a variable: the source tree has another layout)
    const where = "../../llm/llm-agente.js", mod = /** @type {any} */ (await import(where));
    run = mod.factsRun();
    return run;
  };

  /** @type {Map<string, Promise<{inspection: string, proposal: string}>>} the facts of a set of open files, by names and hide flag */
  const cache = new Map();
  /** the facts of the open files: the inspection and the proposals, as the text Rust wrote (the CLI writes the same bytes) @param {any} S @param {boolean} [hide] */
  const facts = async (S, hide = hideNames()) => {
    const names = (await openFiles()).map(([n, p]) => n + ":" + p.length).join("|"), key = (hide ? "h" : "v") + names;
    if (!cache.has(key)) {
      const p = (async () => {
        const r = await worker(), sources = await collect(hide);
        const inspection = await r.inspectText(JSON.stringify(sources), BUDGET);
        const proposal = await r.paramsProposeText(inspection, "", JSON.stringify(S.reg.steps));
        return { inspection, proposal };
      })();
      cache.clear(); cache.set(key, p);
      p.catch(() => cache.delete(key));
    }
    return /** @type {Promise<{inspection: string, proposal: string}>} */ (cache.get(key));
  };

  /** what a reader of the assistant may see, keyed by source; null when the page has none (the reader is skipped) @param {any} S @param {string} role */
  const readerInput = async (S, role) => {
    if (role === "interprete") return typeof F.tabcheckSource === "function" ? F.tabcheckSource() : null;
    if (role !== "metadati" && role !== "cromatografia" && role !== "parametri") return null;
    let f;
    try { f = await facts(S); } catch (_) { return null; }
    const insp = JSON.parse(f.inspection);
    if (!insp.files || !insp.files.length) return null;
    return role === "parametri" ? { "proposta-param/1": JSON.parse(f.proposal) } : { "ispezione/1": insp };
  };

  /** the sources of the charts: TIC and BPC of the open files, `{files: [{name, tic: {rt, i}, bpc: {rt, i}}]}` */
  const chartSources = async () => {
    const files = [];
    for (const [name, parts] of await openFiles()) {
      const k = parts[0].k, o = /** @type {any} */ ({ name });
      for (const kind of ["tic", "bpc"]) {
        try { const c = await J(`api/chrom?k=${k}&kind=${kind}&level=1`); if (c && c.rt) o[kind] = { rt: c.rt, i: c.y }; } catch (_) { /* no trace of this kind */ }
      }
      files.push(o);
    }
    return { files };
  };

  const fmt = (/** @type {any} */ v) => (Array.isArray(v) ? v.join(" – ") : v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v));
  /** the lines of one file of the inspection: [term, value] @param {any} f */
  const lines = f => {
    const L = /** @type {[string, string][]} */ ([]);
    const add = (/** @type {string} */ t, /** @type {any} */ v) => { const s = fmt(v); if (s) L.push([t, s]); };
    add("Type", [f.model, f.format].filter(Boolean).join(", "));
    add("Polarity", f.polarity);
    add("Resolution", f.class ? f.class + (f.class_basis && f.class_basis.length ? ` (${f.class_basis.join(", ")})` : "") : "");
    if (f.resolving_power) add("Resolving power", Object.entries(f.resolving_power).map(([k, v]) => `${k.toUpperCase()} ${v}`).join(", "));
    add("Acquisition", (f.acquisition || []).join(", ").replace(/_/g, " "));
    add("Levels", (f.levels || []).map((/** @type {number} */ l) => "MS" + l).join(", "));
    add("RT (min)", f.rt_range_min);
    add("m/z", f.mz_range);
    if (f.isolation) add("Isolation", `${f.isolation.windows} windows, width ${fmt(f.isolation.width_mz)} m/z`);
    if (Array.isArray(f.gradient) && f.gradient.length) add("Gradient", f.gradient.map((/** @type {any} */ g) => `${g.t_min} min ${g.B_pct}% B`).join(" → "));
    add("Role", f.role ? f.role + (f.role_source ? ` (${f.role_source})` : "") : "");
    for (const w of f.warnings || []) add("Warning", w);
    return L;
  };

  /** the card of one file of the inspection @param {any} f */
  const fileCard = f => {
    const { el } = F;
    const dl = el("dl", {}); for (const [t, v] of lines(f)) dl.append(el("dt", { text: t }), el("dd", { text: v }));
    const bad = f.status && f.status !== "ok";
    return el("div", { class: "flx-facts-file" + (bad ? " bad" : ""), role: "group", "aria-label": "File " + (f.name || f.alias) },
      el("h4", { text: `${f.alias}: ${f.name || f.alias}` }), bad ? el("p", { class: "note", text: "Not readable: " + (f.error_key || f.status) }) : dl);
  };

  /** the instances of the plan a proposal applies to @param {any} S @param {any} p */
  const targets = (S, p) => {
    const base = String(p.step).split("@")[0];
    return S.plan.steps.filter((/** @type {any} */ i) => String(i.step).split("@")[0] === base && (!p.algorithm || (i.algorithm || "") === p.algorithm));
  };

  /** a proposal of a parameter: «Apply» sets it on every instance of its step (one entry of S.undo), «Discard» leaves the plan as it is @param {any} ctx @param {any} p */
  const proposalCard = (ctx, p) => {
    const { el, btn } = F, S = ctx.S;
    const title = `${p.step} · ${p.name} = ${fmt(p.value)}`;
    const c = el("div", { class: "flx-facts-prop", role: "group", "aria-label": "Proposal " + title }, el("h4", { text: title }),
      el("p", { class: "note", text: `Rule ${p.rule}, confidence ${p.confidence}${p.range_ok === false ? ", out of the step's range" : ""}. Signals: ${(p.signals || []).join(", ")}` }));
    const row = el("div", { class: "row" }); c.append(row);
    const end = (/** @type {string} */ word) => { row.textContent = ""; c.classList.add("done"); row.append(el("em", { text: word })); };
    const on = targets(S, p);
    if (!on.length) { row.append(el("em", { text: "The step is not in the plan." })); return c; }
    row.append(btn("ok", `Apply: ${title}`, () => {
      const s = S.reg.byId.get(String(p.step).split("@")[0]), before = S.history.length;
      for (const inst of targets(S, p)) F.setParam(inst, s, p.name, p.value);
      S.refresh(null); ctx.redraw();
      end(S.history.length > before ? "Applied (Undo restores it)." : "Already set.");
    }, "Apply"), btn("x", `Discard: ${title}`, () => end("Discarded."), "Discard"));
    return c;
  };

  /** mounts the «Files» card in `ctx.host`; `ctx`: {S, host, root, redraw} */
  const mount = ctx => {
    const { el, btn } = F, S = ctx.S;
    if (!document.getElementById("flx-facts-css")) { const css = document.createElement("style"); css.id = "flx-facts-css"; css.textContent = CSS; document.head.append(css); }
    const status = el("p", { class: "note", id: "flx-facts-status", role: "status", text: "Reading the open files…" });
    const filesBox = el("div", { class: "flx-facts-grid", id: "flx-facts-files" }), propBox = el("div", { class: "flx-facts-grid", id: "flx-facts-props" });
    const propHead = el("h4", { text: "Proposed parameters" }); propHead.hidden = true;
    const draw = async () => {
      status.textContent = "Reading the open files…";
      let f;
      try { f = await facts(S); } catch (e) { status.textContent = "The facts of the files are not available: " + ((/** @type {any} */ (e) && /** @type {any} */ (e).key) || "err.llm.module"); return; }
      const insp = JSON.parse(f.inspection), prop = JSON.parse(f.proposal);
      filesBox.textContent = ""; propBox.textContent = "";
      if (!insp.files.length) { status.textContent = "No file is open."; propHead.hidden = true; return; }
      const sr = insp.series || {};
      status.textContent = `${sr.files} files, ${sr.readable} readable; polarity ${(sr.polarities || []).join(", ")}; resolution ${(sr.classes || []).join(", ")}.${insp.truncated ? " Some details were left out." : ""}`;
      for (const x of insp.files) filesBox.append(fileCard(x));
      propHead.hidden = !prop.proposals.length;
      for (const p of prop.proposals) propBox.append(proposalCard(ctx, p));
      if (prop.missing && prop.missing.length) propBox.append(el("p", { class: "note", text: "Not decided from the files: " + prop.missing.map((/** @type {any} */ m) => m.signal.replace(/_/g, " ")).join(", ") + "." }));
    };
    const again = btn("undo", "Read the open files again", () => { cache.clear(); draw(); }, "Refresh");
    const box = el("details", { class: "flx-facts", id: "flx-facts", open: true },
      el("summary", { text: "Files" }), el("div", { class: "row" }, status, again), filesBox, propHead, propBox);
    ctx.host.textContent = ""; ctx.host.append(box);
    draw();
    return { draw, box };
  };

  F.llmFacts = { collect, facts, mount, readerInput, chartSources };
  F.chartSources = chartSources;
})();
