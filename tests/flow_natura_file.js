// Flow: a node made (or a plan run) with files open in Data starts with what the files say: unit resolution preset, negative polarity, nominal masses; the user's choice wins. Synthetic file records. Exit code 1 on the first failure.
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const root = path.resolve(__dirname, "..");
const ctx = vm.createContext({});
ctx.window = ctx;
for (const f of ["registry", "check", "plan", "inputs", "names", "fix", "compare", "samples"]) vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-" + f + ".js"), "utf8"), ctx);
const F = ctx.FLOW;
const reg = F.registry(["steps-1", "steps-attributio-1", "steps-tabui-1", "steps-cli-1"].map(n => JSON.parse(fs.readFileSync(path.join(root, "docs/schemi", n + ".json"), "utf8"))));
const files = (pol, hr) => vm.runInContext(`globalThis.E = { files: [${[1, 2].map(k => `{ file: "f${k}.mzML", k: ${k}, polarity: "${pol}", prof1: { hr: ${hr} } }`).join(",")}] }; 0`, ctx);
const made = id => { const plan = F.newPlan(); return F.addStep(plan, reg, id); };

// no file open: the descriptor defaults
files("negative", false); vm.runInContext("E.files = []", ctx);
eq(made("features.detect").preset, undefined);
// QTRAP-like files: negative, unit resolution
files("negative", false);
eq(F.fileNature(), { polarity: -1, resolution: "lr" });
{
  const det = made("features.detect"), gen = made("tp.generate"), link = made("tp.link"), st = i => F.stepOf(reg, i);
  eq(det.preset, "lr");
  // the form shows what the files say as the default; nothing is written
  eq([F.defaults(det, st(det)).polarity, F.defaults(gen, st(gen)).polarity, F.defaults(gen, st(gen)).nominal, F.defaults(link, st(link)).polarity], [-1, -1, true, -1]);
  eq([det.params, gen.params, link.params], [undefined, undefined, undefined]);
  // the user changes them by hand: positive is a difference now and stays written; negative is the default and is not
  F.setParam(det, st(det), "polarity", 1); eq(det.params, { polarity: 1 });
  F.setParam(det, st(det), "polarity", -1); eq(det.params, undefined);
  F.setPreset(det, "hr"); eq(det.preset, "hr");
}
// positive HR files give the defaults: nothing is written
files("positive", true);
{
  const det = made("features.detect"), gen = made("tp.generate");
  eq([det.preset, det.params, gen.params, F.defaults(gen, F.stepOf(reg, gen)).polarity], [undefined, undefined, undefined, 1]);
}
// a mix of polarities or resolutions says nothing
vm.runInContext(`E.files[1].polarity = "negative"; E.files[1].prof1.hr = false`, ctx);
eq(F.fileNature(), { polarity: null, resolution: null });
// a plan made before the files were open takes them at run time, and what the plan wrote stays
files("negative", false);
{
  const plan = F.newPlan(); F.addStep(plan, reg, "features.detect"); const t = F.addStep(plan, reg, "tp.generate");
  delete plan.steps[0].preset; t.params = { polarity: 1 };
  const run = F.runPlanFromData({ plan, reg }).plan;
  eq(Object.keys(run.steps[0].params || {}).sort(), ["polarity"]);
  eq([run.steps[0].preset, run.steps[0].params.polarity, run.steps[1].params.polarity, run.steps[1].params.nominal], ["lr", -1, 1, true]);
  eq(plan.steps[0].preset, undefined);
}
console.log("ok");
