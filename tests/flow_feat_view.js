// The feature table view of Flow (flow/lazy/flow-views-features.js without the DOM): the names and the polarity read from the meta of a .mzfeat. Exit code 1 on the first failure
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const ctx = vm.createContext({ window: {}, document: {} });
ctx.FLOW = { registerView() {} };
ctx.globalThis = ctx;
vm.runInContext(fs.readFileSync(path.resolve(__dirname, "../flow/lazy/flow-views-features.js"), "utf8"), ctx);
const F = ctx.FLOW;
// the stems of the inputs, without .mzidx / .mzML
eq(F.featMeta({ inputs: [{ name: "a.mzidx" }, { name: "b.mzML" }, { name: "c" }], polarity: -1 }), { inputs: [{ name: "a.mzidx" }, { name: "b.mzML" }, { name: "c" }], names: ["a", "b", "c"], pol: -1 });
// no inputs or no polarity: empty list, positive
eq(F.featMeta({}), { inputs: [], names: [], pol: 1 });
eq(F.featMeta(null).pol, 1);
console.log("ok");
