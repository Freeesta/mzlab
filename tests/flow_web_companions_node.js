// Runs flow/lazy/flow-web-companions.js against a mock of the page and the real `ident` of feat-worker.js (tests/real_pool.js); prints what the step kept as JSON.
// usage: node flow_web_companions_node.js <repo root>
const fs = require("fs");
const path = require("path");
const root = process.argv[2];
const realPool = require("./real_pool.js");
const log = { files: {}, calls: [], remembered: [], notes: [] };
const flowtab = { flow_companions: (params, bytes) => ({ take_text: () => "companions/1 of " + bytes.length + " bytes with " + params, warnings: () => JSON.stringify(["w.one"]), free: () => {} }) };
const pool = realPool(root, flowtab);
globalThis.FLOW = {
  message: m => m.key, defaults: () => ({ k: 1 }), stepOf: () => ({}),
  runners: { web: { kit: { getPool: async () => pool } } },
  liveJobs: {
    job: () => "job", sha: async s => "sha:" + s, recall: async () => null, remember: async (p, key, v) => { log.remembered.push({ key, v }); },
    tableOf: async () => ({ key: "kt", read: async () => new Uint8Array([1, 2, 3]) }), detached: async (x, spec, inst, fn) => fn(),
  },
};
new Function(fs.readFileSync(path.join(root, "flow/lazy/flow-web-companions.js"), "utf8")).call(globalThis);
const x = { files: new Map(), S: { reg: {} }, errors: new Map(), notes: new Map(), inst: () => ({}), call: async (n, a) => { log.calls.push([n, a && a.result ? Object.keys(a.result) : null]); } };
(async () => {
  let error = null;
  try { await globalThis.FLOW.webCompanions.run(x, { key: "spec" }, { id: "c", params: { k: 2 } }); } catch (e) { error = String(e && e.message || e); }
  log.files = Object.fromEntries([...x.files].map(([k, v]) => [k, v]));
  process.stdout.write(JSON.stringify({ ...log, notes: Object.fromEntries(x.notes), error }));
})();
