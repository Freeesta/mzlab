// The settings digest of Flow (flow/lazy/flow-digest.js without the DOM): read from the summary of the evidence export. Exit code 1 on the first failure
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const ctx = vm.createContext({ window: {}, document: {} });
ctx.FLOW = {};
ctx.globalThis = ctx;
vm.runInContext(fs.readFileSync(path.resolve(__dirname, "../flow/lazy/flow-digest.js"), "utf8"), ctx);
const F = ctx.FLOW;
const summary = JSON.stringify({ app_version: "1", settings_hash: "0123456789ab", settings: "tol=0.5;min=3", features: 4, missing_reference: [] });
eq(F.settingsDigest({ files: [{ name: "x_POS1_a.svg", text: "<svg/>" }, { name: "evidence_camp.json", text: summary }] }), { hash: "0123456789ab", settings: "tol=0.5;min=3", features: 4 });
// no summary, a spreadsheet answer or broken JSON: no digest
eq(F.settingsDigest({ files: [{ name: "evidence_camp.csv", text: "a" }] }), null);
eq(F.settingsDigest({ files: [{ name: "evidence_camp.json", rows: [] }] }), null);
eq(F.settingsDigest({ files: [{ name: "evidence_camp.json", text: "{" }] }), null);
eq(F.settingsDigest({}), null);
console.log("ok");
