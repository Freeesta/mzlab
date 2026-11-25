// Runs flow/lazy/flow-web-union.js (the page runner of features.union) against a mock of the page and prints what it asked of the workers as JSON;
// see tests/test_flow_web_union_js.py. usage: node flow_web_union_node.js <repo root> <rpc answer json>
const fs = require("fs");
const path = require("path");
const root = process.argv[2], answer = JSON.parse(process.argv[3]);
const log = { ident: [], rpc: [], files: {}, calls: [], remembered: [] };
const tabs = [{ name: "a.mzfeat", key: "ka", read: async () => new Uint8Array([1, 2]) }, { name: "view-0123456789abcdef.mzfeat", key: "kb", read: async () => new Uint8Array([3]) }];
globalThis.FLOW = {
  refs: () => ["file:a", "file:b"], defaults: () => ({ da: 0.5, polarity: "any" }), stepOf: () => ({}),
  webMsn: { keep: async () => {} },
  runners: { web: { kit: { getPool: async () => ({ ident: async (mod, op, m) => { log.ident.push({ mod, op, table: Array.from(m.table) }); return "id,mz,rt\ncsv-of-" + m.table.length + "\n"; } }) } } },
  liveJobs: {
    job: () => "job", sha: async s => "sha:" + s, recall: async () => null, remember: async (p, key) => { log.remembered.push(key); },
    tableOf: async (x, p, inst, port, k) => tabs[k], detached: async (x, spec, inst, fn) => fn(),
  },
};
globalThis.MZLAB_RUST_READY = Promise.resolve({ rpc: async m => { log.rpc.push(m); return answer; } });
new Function(fs.readFileSync(path.join(root, "flow/lazy/flow-web-union.js"), "utf8")).call(globalThis);
const x = { files: new Map(), S: { reg: {} }, call: async (n) => { log.calls.push(n); } };
(async () => {
  let error = null;
  try { await globalThis.FLOW.webUnion.run(x, { key: "spec" }, { id: "u", params: { da: 0.3 } }); } catch (e) { error = { key: e.key || null, message: String(e.message || e), params: e.params || null }; }
  log.files = Object.fromEntries([...x.files].map(([k, v]) => [k, v]));
  process.stdout.write(JSON.stringify({ ...log, error }));
})();
