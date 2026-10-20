// Runs the Flow checker (flow/lazy/flow-*.js) on the shared cases and prints {name, got} per case, as JSON; tests/test_flow_grafico.py compares with the file.
const fs = require("fs"), vm = require("vm"), path = require("path");
const root = path.resolve(__dirname, "..");
const ctx = vm.createContext({});
for (const f of ["registry", "check", "inputs", "compare"]) vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-" + f + ".js"), "utf8"), ctx);
const F = ctx.FLOW;
const reg = F.registry(["steps-1", "steps-attributio-1", "steps-tabui-1"].map(n => JSON.parse(fs.readFileSync(path.join(root, "docs/schemi", n + ".json"), "utf8"))));
const cases = JSON.parse(fs.readFileSync(path.join(root, "crates/mzlab-feat/tests/data/flow_check_cases.json"), "utf8"));
const compareCases = JSON.parse(fs.readFileSync(path.join(root, "crates/mzlab-cli/tests/data/flow_compare_cases.json"), "utf8"));
const out = cases.map(c => {
  try { return { name: c.name, got: F.check(F.normalize(c.plan), reg, "native").map(F.problemLine) }; }
  catch (e) { return { name: c.name, parse_error: e.key || String(e) }; }
});
// the plans with meta.compare, as the page validates them (flow-page.js F.validate): the checker on the plan without the comparisons, then F.validateExtra
const withCompare = compareCases.map(c => {
  try { return { name: c.name, got: [...F.check(F.normalize(F.cleanRefs(F.withoutCompare(c.plan))), reg, "native"), ...F.validateExtra(c.plan, reg)].map(F.problemLine) }; }
  catch (e) { return { name: c.name, parse_error: e.key || String(e) }; }
});
process.stdout.write(JSON.stringify({ cases: out, compare: withCompare }));
