// Runs the real `tableOf` of flow/lazy/flow-live-jobs.js on a mock of the page and prints the series name it gives for a table and for a `table.filter` view of it
// (the name the native run gives the same table); see tests/test_flow_web_contract_js.py. usage: node flow_table_name_node.js <repo root> <view text>
const fs = require("fs");
const path = require("path");
const root = process.argv[2], view = process.argv[3];
globalThis.window = globalThis;
const insts = { al: { id: "al", step: "features.align@1" }, fl: { id: "fl", step: "table.filter@1" } };   // `fl` is a filter view of `al`
globalThis.FLOW = { refs: (inst, port) => (inst.id === "v" ? ["fl.table"] : ["al.table"]), runners: { web: { kit: {} } } };
new Function(fs.readFileSync(path.join(root, "flow/lazy/flow-live-jobs.js"), "utf8")).call(globalThis);
const x = { inst: id => insts[id], tableKey: () => "key", files: new Map([["fl", { text: view }]]) };
const p = { fs: { exists: async () => true, read: async () => new Uint8Array([1]) } };
(async () => {
  const t = globalThis.FLOW.liveJobs.tableOf;
  const direct = await t(x, p, { id: "u", step: "features.union@1" }, "table", 0);
  const viaView = await t(x, p, { id: "v", step: "features.union@1" }, "table", 0);
  process.stdout.write(JSON.stringify({ direct: direct && direct.name, view: viaView && viaView.name }));
})();
