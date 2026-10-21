// @ts-check
// Flow (administrator mode): the tools the assistant may call on the plan (catalog `strumenti/1`, docs/agenti/llm.md). Each call is the same function the user's click uses (F.addStep, F.setParam,
// F.removeStep, F.validate, the runners), so an edit of the assistant is a change of the plan like any other: one entry of S.undo. Whether a call may happen at all is decided by Rust (policy),
// not here. Loaded with flow-llm.js only when the assistant is on. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const bad = (/** @type {string} */ key, /** @type {any} */ params) => ({ ok: false, content: { error_key: key, params: params || {} } });
  const good = (/** @type {any} */ content) => ({ ok: true, content });

  /** what changes between two plans, for the card of «Apply»: steps added and removed, parameters and wiring changed, inputs added and removed @param {any} a the plan now @param {any} b the proposed one */
  const llmDiff = (a, b) => {
    const byId = (/** @type {any} */ p) => new Map((p.steps || []).map((/** @type {any} */ s) => [s.id, s]));
    const A = byId(a), B = byId(b), out = /** @type {{kind: string, text: string}[]} */ ([]);
    for (const [id, s] of B) {
      if (!A.has(id)) { out.push({ kind: "add", text: `Add ${id}: ${s.step}` }); continue; }
      const o = /** @type {any} */ (A.get(id));
      if (JSON.stringify(o.params || {}) !== JSON.stringify(s.params || {})) out.push({ kind: "change", text: `${id}: parameters ${JSON.stringify(o.params || {})} → ${JSON.stringify(s.params || {})}` });
      if (JSON.stringify(o.in || {}) !== JSON.stringify(s.in || {})) out.push({ kind: "change", text: `${id}: inputs rewired` });
      if (o.algorithm !== s.algorithm || o.preset !== s.preset) out.push({ kind: "change", text: `${id}: algorithm or preset changed` });
    }
    for (const [id, s] of A) if (!B.has(id)) out.push({ kind: "remove", text: `Remove ${id}: ${s.step}` });
    const names = (/** @type {any} */ p) => new Set((p.inputs || []).map((/** @type {any} */ f) => f.name));
    for (const n of names(b)) if (!names(a).has(n)) out.push({ kind: "add", text: `Add input ${n}` });
    for (const n of names(a)) if (!names(b).has(n)) out.push({ kind: "remove", text: `Remove input ${n}` });
    return out;
  };
  F.llmDiff = llmDiff;

  /** runs the plan where the call says (web or this computer) and reports the state of each step @param {any} ctx @param {any} a @param {(plan: any) => any[]} problems */
  const runTool = async (ctx, a, problems) => {
    const S = ctx.S;
    const desk = F.runners.desktop, web = F.runners.web;
    const where = a.executor === "web" ? "web" : a.executor === "native" ? "desktop" : desk && desk.available() ? "desktop" : "web";
    const runner = where === "desktop" ? desk : web;
    if (!runner || !runner.available()) return bad("err.flow.unavailable", { executor: where });
    const found = problems(S.plan);
    if (found.length) return bad("err.flow.check", { count: found.length, first: found[0].error_key, instance: found[0].instance });
    ctx.runs.running = true; ctx.runs.states = {};
    S.llmRef = await ctx.llmRef();
    try {
      const r = await runner.run(S, { onEvent: (/** @type {any} */ e) => { ctx.runs.states[e.passo] = e.stato; } });
      if (where === "desktop") for (const s of r.steps) ctx.runs.states[s.id] = s.error ? "error" : s.cached ? "reused" : "done";
      else for (const id of r.steps) ctx.runs.states[id] = ctx.runs.states[id] || "done";
      return good({ executor: where, steps: ctx.runs.states });
    } catch (e) {
      const x = /** @type {any} */ (e);
      return bad(x && x.key || "err.flow.run", x && x.params);
    } finally { delete S.llmRef; ctx.runs.running = false; }
  };

  /** shows something to the user: selects a node or runs a UI action @param {any} ctx @param {any} a */
  const showTool = (ctx, a) => {
    if (a.show === "select_node") {
      const inst = F.instanceOf(S.plan, a.args.instance);
      if (!inst) return bad("err.llm.tool.args", { field: "instance" });
      ctx.select(inst.id);
      return good({ shown: "select_node", instance: inst.id });
    }
    const AZ = /** @type {any} */ (globalThis).AZIONI, id = { show_xic: "spec.xic", show_spectrum_at_rt: "chrom.specAtRt", show_row: "table.showRow" }[/** @type {"show_xic"} */ (a.show)];
    if (!AZ || !id || typeof AZ.esegui !== "function") return bad("err.llm.tool.unavailable", { tool: a.show });
    try { AZ.esegui(id, a.args); return good({ shown: a.show }); } catch (_) { return bad("err.llm.tool.unavailable", { tool: a.show }); }
  };

  /** the tools bound to one Flow page; `ctx`: {S, select(instance), propose(plan, note), edited(label)}, `ctx.runs` the states of the steps of the last run @param {any} ctx */
  const llmTools = ctx => {
    const S = ctx.S;
    const problems = (/** @type {any} */ plan) => F.validate(plan, S.reg).map((/** @type {any} */ p) => ({ error_key: p.key, instance: p.instance || "", port: p.port || "", params: p.params || {} }));
    const stepOf = (/** @type {string} */ ref) => S.reg.byId.get(String(ref).split("@")[0]);
    /** an edit of the plan: runs `fn`, draws, and tells the panel when the plan really changed (one entry of S.undo) @param {string} label @param {() => any} fn */
    const edit = (label, fn) => {
      const before = S.history.length, r = fn();
      S.refresh(null);
      if (S.history.length > before) ctx.edited(label);
      return r;
    };
    const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];

    /** @type {Record<string, (a: any) => any>} */
    const T = {
      flow_list_steps: () => good({ steps: S.reg.steps.map((/** @type {any} */ s) => ({ id: s.id, version: s.version, title: s.title })) }),
      flow_describe_step: a => {
        const s = stepOf(a.step);
        if (!s) return bad("err.llm.tool.args", { field: "step" });
        return good(JSON.parse(JSON.stringify(s, (k, v) => (k === "why" || k === "notes" ? undefined : v))));
      },
      flow_get_plan: () => good({ plan: JSON.parse(F.exportText(S.plan)), problems: problems(S.plan) }),
      flow_check: a => good({ problems: problems(a.plan) }),
      flow_add_step: a => {
        const s = stepOf(a.step);
        if (!s) return bad("err.llm.tool.args", { field: "step" });
        const inst = edit("Added " + s.title, () => { const i = F.addStep(S.plan, S.reg, s.id); if (a.label) i.label = String(a.label).slice(0, 80); return i; });
        return good({ instance: inst.id, step: inst.step, in: inst.in || {}, problems: problems(S.plan) });
      },
      flow_set_param: a => {
        const inst = F.instanceOf(S.plan, a.instance), s = inst && stepOf(inst.step);
        if (!inst || !s) return bad("err.llm.tool.args", { field: "instance" });
        edit(`Set ${a.name} of ${inst.id}`, () => {
          if (a.name === "algorithm") F.setAlgorithm(inst, s, a.value);
          else if (a.name === "preset") F.setPreset(inst, a.value);
          else F.setParam(inst, s, a.name, a.value);
        });
        return good({ instance: inst.id, name: a.name, params: inst.params || {}, problems: problems(S.plan).filter((/** @type {any} */ p) => p.instance === inst.id) });
      },
      flow_remove_step: a => {
        const inst = F.instanceOf(S.plan, a.instance);
        if (!inst) return bad("err.llm.tool.args", { field: "instance" });
        edit(`Removed ${inst.id}`, () => F.removeStep(S.plan, inst.id));
        return good({ removed: inst.id, problems: problems(S.plan) });
      },
      flow_status: () => good({ running: !!ctx.runs.running, steps: ctx.runs.states }),
      flow_read_result: async a => {
        const inst = F.instanceOf(S.plan, a.instance);
        if (!inst) return bad("err.llm.tool.args", { field: "instance" });
        const st = ctx.runs.states[inst.id];
        if (!st) return bad("err.llm.result.none", { instance: inst.id });
        return good({ instance: inst.id, port: a.port, context: JSON.parse(await ctx.context()) });
      },
    };

    T.flow_propose_plan = async a => {
      const found = problems(a.plan);
      if (found.length) return { ok: false, content: { problems: found } };
      const r = F.importText(JSON.stringify(a.plan));
      if (r.error) return bad(r.error.key, r.error.params);
      return good({ applied: await ctx.propose(r.plan, String(a.note || "")) });
    };

    T.flow_run = a => runTool(ctx, a, problems);

    T.ui_show = a => showTool(ctx, a);
    for (const n of ["data_summary", "chart_make", "code_export"]) T[n] = () => bad("err.llm.tool.unavailable", { tool: n });

    return {
      /** one call of the model: {ok, content}; a tool that throws is a failed call, never a broken page @param {string} name @param {any} args */
      async call(name, args) {
        const f = T[name];
        if (!f) return bad("err.llm.tool.unknown", { tool: name });
        try { return await f(args || {}); } catch (e) { const x = /** @type {any} */ (e); return bad(x && x.key || "err.llm.tool.failed", x && x.params); }
      },
      base,
    };
  };
  F.llmTools = llmTools;
})();
