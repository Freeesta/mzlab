// The pool of the page as feat-worker.js really behaves: `ident` and the ops flow_series_csv / flow_companions are cut out of the worker's source and run against a mock of the
// .wasm module, so a runner that reads the wrong shape of the answer (`ident` gives `r.text` unless `full`) fails here. Used by flow_web_*_node.js.
const fs = require("fs");
const path = require("path");
module.exports = (root, flowtab) => {
  const src = fs.readFileSync(path.join(root, "TP_Mine/files/feat-worker.js"), "utf8");
  const cut = (re, what) => { const m = re.exec(src); if (!m) throw new Error("feat-worker.js: " + what + " not found"); return m[1]; };
  const identSrc = cut(/ident: (\(mod, op, m, full\) => submit\([\s\S]*?return full \? r : r\.text; \}\)),/, "ident");
  const opSeries = cut(/OPS\.flow_series_csv = (async m => \(\{[^\n]*?\}\));/, "OPS.flow_series_csv");
  const opComp = cut(/OPS\.flow_companions = (async m => \{[\s\S]*?\});\s+OPS\.dig_lr/, "OPS.flow_companions");
  const OPS = {}, mods = { flowtab };
  OPS.flow_series_csv = new Function("mods", "return " + opSeries)(mods);
  OPS.flow_companions = new Function("mods", "JSON", "return " + opComp)(mods, JSON);
  const submit = fn => fn({}, async () => {});
  const call = async (slot, msg) => OPS[msg.op](msg);
  return { ident: new Function("submit", "call", "return " + identSrc)(submit, call) };
};
