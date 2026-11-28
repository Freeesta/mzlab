// Runs flow/lazy/flow-web-pair.js (the page runner of features.pair) against a mock of the page and prints what it asked of the worker as JSON;
// see tests/test_flow_web_pair_js.py. usage: node flow_web_pair_node.js <repo root> <flow_pair answer json>
const fs = require("fs");
const path = require("path");
const root = process.argv[2], answer = process.argv[3];
const realPool = require("./real_pool.js");
const log = { ident: [], files: {}, calls: [], remembered: [] };
const tabs = { pos: { name: "p.mzfeat", key: "kp", read: async () => new Uint8Array([1, 2]) }, neg: { name: "view-0123456789abcdef.mzfeat", key: "kn", read: async () => new Uint8Array([3]) } };
const flowtab = { flow_pair: (params, posName, pos, negName, neg) => { log.ident.push({ mod: "flowtab", op: "flow_pair", params: JSON.parse(params), posName, negName, pos: Array.from(pos), neg: Array.from(neg) }); if (answer.startsWith("ERR")) throw answer.slice(3); return answer; } };
const pool = realPool(root, flowtab);   // the real `ident` and the real op of feat-worker.js over a mock of the .wasm
globalThis.FLOW = {
  defaults: () => ({ rt_tol: 0.1, mass_unit: "ppm" }), stepOf: () => ({}),
  webMsn: { keep: async () => {} },
  runners: { web: { kit: { getPool: async () => pool } } },
  liveJobs: {
    job: () => "job", sha: async s => "sha:" + s, recall: async () => null, remember: async (p, key) => { log.remembered.push(key); },
    tableOf: async (x, p, inst, port) => tabs[port], detached: async (x, spec, inst, fn) => fn(),
  },
};
new Function(fs.readFileSync(path.join(root, "flow/lazy/flow-web-pair.js"), "utf8")).call(globalThis);
const x = { files: new Map(), S: { reg: {} }, call: async (n) => { log.calls.push(n); } };
(async () => {
  let error = null;
  try { await globalThis.FLOW.webPair.run(x, { key: "spec" }, { id: "pr", params: { rt_tol: 0.3 } }); } catch (e) { error = { key: e.key || null, message: String(e.message || e), params: e.params || null }; }
  log.files = Object.fromEntries([...x.files].map(([k, v]) => [k, v]));
  process.stdout.write(JSON.stringify({ ...log, error }));
})();
