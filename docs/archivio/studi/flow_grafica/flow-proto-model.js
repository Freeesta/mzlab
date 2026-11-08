// Flow prototype, model half: a tiny registry of fake steps (a subset of passo/1 from docs/studi/workflow/contratto.schema.json),
// the link rule (type, accepts, cardinality, cycle), the shortest registered conversion, Kahn order, cache keys and mzflow/2 save/load.
// No DOM here: the canvas (flow-proto.js) only draws what this file decides, as COMUNICAZIONE.md §5 asks of the real engine.
"use strict";
const FLOW = (() => {
  // Data types: colour (Okabe-Ito via CSS variables) + shape + letter, so no information rides on colour alone.
  const TYPES = {
    raw: { shape: "square", letter: "R", title: "Raw file" },
    mzml: { shape: "square", letter: "M", title: "mzML" },
    mzidx: { shape: "circle", letter: "I", title: "Scan index" },
    peaks: { shape: "diamond", letter: "P", title: "Peaks per file" },
    table: { shape: "triangle", letter: "T", title: "Feature table" },
    structure: { shape: "hexagon", letter: "S", title: "Structure" },
    tp: { shape: "triangle", letter: "D", title: "TP view" },
    report: { shape: "circle", letter: "E", title: "Export" },
  };
  // Fake steps: ports, params (JSON-Schema-like), conversions and a fake cost in ms per item.
  const STEPS = {
    "import.raw": { title: "Open raw files", icon: "📂", out: [{ id: "raw", type: "raw", many: true }], params: {}, ms: 300 },
    "convert.mzml": { title: "Convert to mzML", icon: "⇄", in: [{ id: "raw", type: "raw", many: true }], out: [{ id: "mzml", type: "mzml", many: true }],
      converts: { from: "raw", to: "mzml" }, cost: 2, params: {}, ms: 900 },
    "index.mzidx": { title: "Build index", icon: "▦", in: [{ id: "raw", type: "raw", accepts: ["mzml"], many: true }], out: [{ id: "index", type: "mzidx", many: true }],
      converts: { from: "raw", to: "mzidx" }, cost: 1, params: {}, ms: 700 },
    "features.detect": { title: "Detect features", icon: "⛰", in: [{ id: "index", type: "mzidx", many: true }], out: [{ id: "peaks", type: "peaks", many: true }],
      params: { min_height: { type: "number", default: 2e5, minimum: 0, unit: "counts", estimate: true, title: "Minimum height" },
        ppm: { type: "number", default: 5, minimum: 0.5, maximum: 50, unit: "ppm", title: "m/z tolerance" } }, ms: 1200 },
    "features.align": { title: "Align samples", icon: "≣", in: [{ id: "peaks", type: "peaks", many: true }], out: [{ id: "table", type: "table" }],
      params: { rt_tol: { type: "number", default: 0.1, minimum: 0.01, maximum: 2, unit: "min", title: "RT tolerance" } }, ms: 800 },
    "structure.draw": { title: "Parent structure", icon: "⌬", out: [{ id: "mol", type: "structure" }], params: {}, ms: 50 },
    "tp.link": { title: "Link TPs", icon: "⤳", in: [{ id: "table", type: "table" }, { id: "parent", type: "structure", optional: true }],
      out: [{ id: "tp", type: "tp" }], params: {}, ms: 600 },
    "export.csv": { title: "Export CSV", icon: "⇩", in: [{ id: "table", type: "table", accepts: ["tp"] }], out: [{ id: "csv", type: "report" }], params: {}, ms: 150 },
  };

  const step = n => STEPS[n.step];
  const portOf = (n, dir, id) => (step(n)[dir] || []).find(p => p.id === id);

  /** Shortest chain of registered conversions from type a to type b (breadth first, ties: lower cost). [] = none needed, null = impossible. */
  function convPath(a, b) {
    if (a === b) return [];
    const conv = Object.entries(STEPS).filter(([, s]) => s.converts).sort((x, y) => x[1].cost - y[1].cost);
    const seen = new Set([a]);
    let front = [{ t: a, path: [] }];
    while (front.length) {
      const next = [];
      for (const f of front) for (const [name, s] of conv) {
        if (s.converts.from !== f.t || seen.has(s.converts.to)) continue;
        const path = [...f.path, name];
        if (s.converts.to === b) return path;
        seen.add(s.converts.to); next.push({ t: s.converts.to, path });
      }
      front = next;
    }
    return null;
  }

  function reaches(g, from, to) {
    const stack = [from], seen = new Set();
    while (stack.length) {
      const k = stack.pop();
      if (k === to) return true;
      if (seen.has(k)) continue;
      seen.add(k);
      for (const e of g.edges) if (e.from === k) stack.push(e.to);
    }
    return false;
  }

  /** Can out-port (src node, port) feed in-port (dst node, port)? Returns {ok:true} | {adapt:[steps]} | {error_key, params}. */
  function check(g, src, sp, dst, dp) {
    const o = portOf(src, "out", sp), i = portOf(dst, "in", dp);
    if (!o || !i) return { error_key: "err.flow.port" };
    if (src.id === dst.id || reaches(g, dst.id, src.id)) return { error_key: "err.flow.cycle" };
    if (o.many && !i.many) return { error_key: "err.flow.cardinality", params: { from: "many", to: "one" } };
    if (o.type === i.type || (i.accepts || []).includes(o.type)) return { ok: true };
    const path = convPath(o.type, i.type);
    if (path && path.length) return { adapt: path };
    return { error_key: "err.flow.type", params: { from: o.type, to: i.type } };
  }

  /** Plan problems before running: missing required inputs. */
  function problems(g) {
    const out = [];
    for (const n of g.nodes) for (const p of step(n).in || []) {
      if (!p.optional && !g.edges.some(e => e.to === n.id && e.tp === p.id)) out.push({ instance: n.id, port: p.id, error_key: "err.flow.missing" });
    }
    return out;
  }

  /** Kahn order; ties keep the order of g.nodes, so the same file always gives the same plan. */
  function order(g) {
    const indeg = new Map(g.nodes.map(n => [n.id, 0]));
    for (const e of g.edges) indeg.set(e.to, (indeg.get(e.to) || 0) + 1);
    const ready = g.nodes.filter(n => !indeg.get(n.id)).map(n => n.id), out = [];
    while (ready.length) {
      const k = ready.shift();
      out.push(k);
      for (const e of g.edges) if (e.from === k) { indeg.set(e.to, indeg.get(e.to) - 1); if (!indeg.get(e.to)) ready.push(e.to); }
    }
    return out;
  }

  // FNV-1a over text: enough for a fake cache key (the real one is SHA-256 of canonical params, WORKFLOW_MODULARI.md §4 rule 1-2).
  function fnv(s) {
    let h = 0x811c9dc5;
    for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 0x01000193) >>> 0; }
    return h.toString(16).padStart(8, "0");
  }
  function canon(n) {
    const sch = step(n).params, p = {};
    for (const k of Object.keys(sch).sort()) p[k] = n.params[k] ?? sch[k].default;
    return JSON.stringify(p);
  }
  /** Cache key of a node = its step, canonical params and the keys of what feeds it (so a change upstream invalidates downstream). */
  function key(g, id, memo = new Map()) {
    if (memo.has(id)) return memo.get(id);
    const n = g.nodes.find(x => x.id === id);
    const ins = g.edges.filter(e => e.to === id).map(e => `${e.tp}<${key(g, e.from, memo)}.${e.fp}`).sort();
    const k = fnv(`${n.step}|${canon(n)}|${ins.join(",")}`);
    memo.set(id, k);
    return k;
  }

  /** mzflow/2 passi + a separate "ui" block (positions never enter the plan or its hash). */
  function save(g, view) {
    const passi = order(g).map(id => {
      const n = g.nodes.find(x => x.id === id), inn = {};
      for (const e of g.edges.filter(x => x.to === id)) (inn[e.tp] = inn[e.tp] || []).push(`${e.from}.${e.fp}`);
      const params = {};
      for (const [k, v] of Object.entries(n.params)) if (v !== step(n).params[k]?.default) params[k] = v;
      return { id, step: `${n.step}@1`, in: inn, ...(Object.keys(params).length ? { params } : {}) };
    });
    const ui = { view, nodes: Object.fromEntries(g.nodes.map(n => [n.id, { x: Math.round(n.x), y: Math.round(n.y) }])) };
    return { formato: "mzflow/2", ingressi: [], passi, ui };
  }
  function load(doc) {
    if (!doc || doc.formato !== "mzflow/2" || !Array.isArray(doc.passi)) throw new Error("err.flow.format");
    const g = { nodes: [], edges: [] };
    doc.passi.forEach((p, i) => {
      const name = String(p.step).split("@")[0];
      if (!STEPS[name]) throw new Error("err.flow.unavailable");
      const at = doc.ui?.nodes?.[p.id] || { x: 40 + 240 * i, y: 80 };
      g.nodes.push({ id: p.id, step: name, params: { ...(p.params || {}) }, x: at.x, y: at.y });
    });
    for (const p of doc.passi) for (const [tp, refs] of Object.entries(p.in || {})) for (const r of [].concat(refs)) {
      const [from, fp] = String(r).split(".");
      g.edges.push({ from, fp, to: p.id, tp });
    }
    return { g, view: doc.ui?.view };
  }

  return { TYPES, STEPS, step, portOf, convPath, check, problems, order, key, save, load };
})();
