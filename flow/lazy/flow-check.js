// @ts-check
// Flow (administrator mode): reads an mzflow/2 object and checks it against the descriptors, as flow::parse and flow::check of mzlab-core do (check.rs). The Rust
// checker is the reference: tests/test_flow_grafico.py runs both on the cases of crates/mzlab-feat/tests/data/flow_check_cases.json. A problem is
// {instance, port, key, params}; keys are the err.flow.* of the Rust side, the texts are in flow-page.js.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** @param {string} key @param {Record<string, string>} params */
  const fail = (key, params) => Object.assign(new Error(key), { key, params });
  /** `file:NAME` or `STEP.OUTPUT` @param {any} s @param {string} instance */
  const reference = (s, instance) => {
    const bad = () => fail("err.flow.ref", { instance, ref: String(s) });
    if (typeof s !== "string") throw bad();
    if (s.startsWith("file:")) { if (s.length === 5) throw bad(); return { file: s.slice(5) }; }
    const k = s.indexOf(".");
    if (k <= 0 || k === s.length - 1 || s.indexOf(".", k + 1) >= 0) throw bad();
    return { instance: s.slice(0, k), port: s.slice(k + 1) };
  };
  /** @param {any} v @param {string} key */
  const text = (v, key) => v && typeof v[key] === "string" ? v[key] : null;

  /** one instance of the file as the checker reads it @param {any} v */
  const instance = v => {
    const id = text(v, "id");
    if (id == null) throw fail("err.flow.instance", { field: "id" });
    const stepText = text(v, "step");
    if (stepText == null) throw fail("err.flow.instance", { instance: id, field: "step" });
    const at = stepText.lastIndexOf("@"), version = at < 0 ? NaN : /^\d+$/.test(stepText.slice(at + 1)) ? +stepText.slice(at + 1) : NaN;
    if (at <= 0 || Number.isNaN(version)) throw fail("err.flow.step", { instance: id, step: stepText });
    /** @type {[string, any[]][]} */ const ins = [];
    for (const [port, val] of Object.entries(v.in && typeof v.in === "object" ? v.in : {})) {
      if (typeof val === "string") ins.push([port, [reference(val, id)]]);
      else if (Array.isArray(val)) ins.push([port, val.map(x => reference(x, id))]);
      else throw fail("err.flow.ref", { instance: id, port });
    }
    const params = v.params && typeof v.params === "object" && !Array.isArray(v.params) ? Object.entries(v.params) : [];
    return { id, step: stepText.slice(0, at), version, algorithm: text(v, "algorithm"), preset: text(v, "preset"), label: text(v, "label"), ins, params };
  };
  /** the facets a file input declares @param {any} v */
  const observed = v => ({ resolution: (v && v.resolution || []).filter((/** @type {any} */ x) => typeof x === "string"), polarity: (v && v.polarity || []).filter(Number.isFinite), ms_levels: (v && v.ms_levels || []).filter(Number.isFinite) });

  /** flow::parse: the plan as the checker reads it; throws an Error with {key, params} for a file that is not an mzflow/2 object
   * @param {any} v the parsed JSON */
  F.normalize = v => {
    if (!v || typeof v !== "object" || Array.isArray(v)) throw fail("err.flow.json", {});
    const format = text(v, "format");
    if (format != null && format !== "mzflow/2") throw fail("err.flow.format", { format });
    const list = (/** @type {string} */ k) => Array.isArray(v[k]) ? v[k] : [];
    const inputs = list("inputs").map(f => {
      const name = text(f, "name");
      if (name == null) throw fail("err.flow.input", { field: "name" });
      return { name, type: text(f, "type"), observed: observed(f.facets) };
    });
    return { inputs, steps: list("steps").map(instance) };
  };
  /** the ids of the instances an instance reads from @param {any} i */
  const sources = i => [...new Set(i.ins.flatMap((/** @type {any} */ x) => x[1]).filter((/** @type {any} */ r) => r.instance).map((/** @type {any} */ r) => r.instance))];
  /** @param {any} plan @returns {number[][]} the positions each instance depends on */
  const dependencies = plan => plan.steps.map((/** @type {any} */ s) => sources(s).map(id => plan.steps.findIndex((/** @type {any} */ x) => x.id === id)).filter(k => k >= 0).sort((a, b) => a - b));
  /** flow::plan_order: Kahn's sort, ties in file order; null when a cycle is left @param {any} plan */
  F.planOrder = plan => {
    const deps = dependencies(plan), done = deps.map(() => false), order = [];
    while (order.length < deps.length) {
      const next = deps.findIndex((d, i) => !done[i] && d.every(x => done[x]));
      if (next < 0) return null;
      done[next] = true; order.push(next);
    }
    return order;
  };
  /** positions on a cycle (or between two): what stays after peeling what nothing waits for and what waits for nothing @param {any} plan */
  const cyclic = plan => {
    const deps = dependencies(plan), alive = deps.map(() => true);
    for (let changed = true; changed;) {
      changed = false;
      deps.forEach((d, i) => {
        if (alive[i] && (!d.some(x => alive[x]) || !deps.some((e, j) => alive[j] && e.includes(i)))) { alive[i] = false; changed = true; }
      });
    }
    return alive.flatMap((a, i) => a ? [i] : []);
  };

  /** the reason a value does not fit a parameter schema, or null (check.rs param_error) @param {any} p @param {any} v */
  const paramError = (p, v) => {
    const num = (/** @type {any} */ x) => typeof x === "number" && Number.isFinite(x) ? x : null;
    const out = (/** @type {number} */ n) => (p.minimum != null && n < p.minimum) || (p.maximum != null && n > p.maximum);
    if (p.enum) return p.enum.some((/** @type {any} */ x) => x === v) ? null : "err.flow.param.range";
    if (p.type === "number" || p.type === "integer") {
      const n = num(v);
      if (n == null || (p.type === "integer" && !Number.isInteger(n))) return "err.flow.param.type";
      return out(n) ? "err.flow.param.range" : null;
    }
    if (p.type === "boolean") return typeof v === "boolean" ? null : "err.flow.param.type";
    if (p.type === "string") return typeof v === "string" ? null : "err.flow.param.type";
    if (p.type === "array") {
      const a = Array.isArray(v) ? v : [], x = num(a[0]), y = num(a[1]), min = p.items && p.items.minimum != null ? p.items.minimum : -Infinity;
      if (a.length !== 2 || x == null || y == null) return "err.flow.param.type";
      return x >= min && y >= x ? null : "err.flow.param.range";
    }
    return null;
  };

  /** the check of a plan for a platform; returns every problem in file order, cycles last
   * @param {any} plan the result of F.normalize @param {{steps: any[], byId: Map<string, any>}} reg @param {"native" | "wasm"} platform */
  F.check = (plan, reg, platform) => {
    /** @type {any[]} */ let problems = [];
    const add = (/** @type {any} */ i, /** @type {string} */ port, /** @type {string} */ key, /** @type {Record<string, string>} */ params = {}) => {
      const p = { instance: i.id, port, key, params };
      if (!problems.some(x => x.instance === p.instance && x.port === p.port && x.key === p.key && JSON.stringify(x.params) === JSON.stringify(p.params))) problems.push(p);
    };
    const stepOf = (/** @type {any} */ i) => { const s = reg.byId.get(i.step); return s && s.version === i.version ? s : null; };
    const stepLevel = (/** @type {any} */ i) => {
      const known = reg.byId.get(i.step);
      if (!known) return add(i, "", "err.flow.step.unknown", { step: i.step }), null;
      if (known.version !== i.version) return add(i, "", "err.flow.step.version", { step: i.step, version: String(i.version), available: String(known.version) }), null;
      if (platform === "native" ? !known.runs_in.native : !known.runs_in.wasm) add(i, "", "err.flow.unavailable", { step: i.step, platform });
      const e = F.effective(i, known);
      if (e.error) add(i, "", e.error.key, { [e.error.name]: e.error.value });
      else if (e.algorithm && e.algorithm.status === "planned") add(i, "", "err.flow.algorithm.unavailable", { step: i.step, algorithm: e.algorithm.id });
      return known;
    };
    const paramsOf = (/** @type {any} */ i, /** @type {any} */ step) => {
      const known = F.declared(step, F.effective(i, step).algorithm);
      for (const [name, v] of i.params) {
        const p = known.find((/** @type {any} */ x) => x[0] === name);
        if (!p) { add(i, "", "err.flow.param.unknown", { param: name }); continue; }
        const key = paramError(p[1], v);
        if (key) add(i, "", key, { param: name });
      }
    };
    /** the type a reference gives: of an input file or of an output of another instance; null when it cannot be known */
    const sourceType = (/** @type {any} */ i, /** @type {FlowPort} */ port, /** @type {any} */ r) => {
      if (r.file != null) {
        const f = plan.inputs.find((/** @type {any} */ x) => x.name === r.file);
        if (!f) return add(i, port.name, "err.flow.input.unknown", { ref: r.file }), null;
        return f.type && F.TYPES[f.type] ? { ty: f.type, list: false } : null;
      }
      const text = r.instance + "." + r.port, src = plan.steps.find((/** @type {any} */ x) => x.id === r.instance);
      if (!src) return add(i, port.name, "err.flow.ref.unknown", { ref: text }), null;
      const s = stepOf(src);
      if (!s) return null;
      const out = s.out && s.out[r.port];
      if (!out) return add(i, port.name, "err.flow.port.unknown", { ref: text }), null;
      return { ty: out.type, list: !!out.many };
    };
    const portsOf = (/** @type {any} */ i, /** @type {any} */ step, /** @type {any[]} */ seen) => {
      const declared = F.ports(step.in);
      for (const [name, refs] of i.ins) {
        const port = declared.find((/** @type {FlowPort} */ p) => p.name === name);
        if (!port) { add(i, name, "err.flow.port.unknown", { port: name }); continue; }
        if (!port.many && refs.length > 1) add(i, name, "err.flow.cardinality", { port: name, count: String(refs.length) });
        for (const r of refs) {
          const s = sourceType(i, port, r);
          if (!s) continue;
          if (s.list && !port.many) add(i, name, "err.flow.cardinality", { port: name, count: "many" });
          if (F.fit(reg, s.ty, port) === "no") add(i, name, "err.flow.type", { from: s.ty, to: port.type });
        }
      }
      for (const port of declared.filter((/** @type {FlowPort} */ p) => !p.optional)) {
        if (!i.ins.some((/** @type {any} */ x) => x[0] === port.name && x[1].length)) add(i, port.name, "err.flow.port.missing", { port: port.name });
      }
      facets(i, declared, seen);
    };
    const sees = (/** @type {any} */ i, /** @type {any[]} */ seen) => {
      const o = { resolution: /** @type {any[]} */ ([]), polarity: /** @type {any[]} */ ([]), ms_levels: /** @type {any[]} */ ([]) };
      for (const r of i.ins.flatMap((/** @type {any} */ x) => x[1])) {
        const k = r.file != null ? -1 : plan.steps.findIndex((/** @type {any} */ x) => x.id === r.instance);
        const s = r.file != null ? (plan.inputs.find((/** @type {any} */ x) => x.name === r.file) || {}).observed : k >= 0 ? seen[k] : null;
        if (s) { o.ms_levels.push(...s.ms_levels); o.polarity.push(...s.polarity); o.resolution.push(...s.resolution); }
      }
      return o;
    };
    const facets = (/** @type {any} */ i, /** @type {FlowPort[]} */ declared, /** @type {any[]} */ seen) => {
      const have = sees(i, seen);
      /** known and not all of what is required is in it (unknown = checked when the step starts) */
      const misses = (/** @type {any[]} */ want, /** @type {any[]} */ h, /** @type {(w: any) => boolean} */ has) => h.length > 0 && !want.every(has);
      for (const port of declared) {
        const w = port.requires;
        if (misses(w.ms_levels || [], have.ms_levels, x => have.ms_levels.includes(x))) add(i, port.name, "err.flow.facet", { facet: "ms_levels" });
        if (misses(w.polarity || [], have.polarity, x => x === 0 || have.polarity.includes(x))) add(i, port.name, "err.flow.facet", { facet: "polarity" });
        if (misses(w.resolution || [], have.resolution, x => have.resolution.includes(x))) add(i, port.name, "err.flow.facet", { facet: "resolution" });
      }
    };

    plan.steps.forEach((/** @type {any} */ i, /** @type {number} */ k) => { if (plan.steps.slice(0, k).some((/** @type {any} */ x) => x.id === i.id)) add(i, "", "err.flow.instance.duplicate"); });
    const duplicates = problems; problems = [];
    const order = F.planOrder(plan) || plan.steps.map((/** @type {any} */ _, /** @type {number} */ k) => k);
    const seen = plan.steps.map(() => ({ resolution: [], polarity: [], ms_levels: [] }));
    /** @type {any[][]} */ const byInstance = plan.steps.map(() => []);
    for (const k of order) {
      const i = plan.steps[k], before = problems.length, step = stepLevel(i);
      if (step) {
        paramsOf(i, step); portsOf(i, step, seen);
        // the facets of the data pass through an instance unchanged, to the outputs that still are the scans
        if (Object.values(step.out || {}).some((/** @type {any} */ o) => ["raw", "mzml", "mzidx"].includes(o.type))) seen[k] = sees(i, seen);
      }
      byInstance[k] = problems.splice(before);
    }
    const all = [...duplicates, ...byInstance.flat()];
    const at = (/** @type {any} */ p) => plan.steps.findIndex((/** @type {any} */ x) => x.id === p.instance);
    problems = all.map((p, n) => [p, n]).sort((a, b) => at(a[0]) - at(b[0]) || a[1] - b[1]).map(x => x[0]);
    for (const k of cyclic(plan)) add(plan.steps[k], "", "err.flow.cycle");
    return problems;
  };
  /** the line the Rust test helper and the shared cases use: instance|port|key|k=v,k=v @param {any} p */
  F.problemLine = p => `${p.instance}|${p.port}|${p.key}|${Object.entries(p.params).map(([k, v]) => k + "=" + v).join(",")}`;
})();
