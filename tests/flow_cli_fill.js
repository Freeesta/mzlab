// The command of the Run on CLI box: the folder of the user goes in once, in single quotes (flow-run-ui.js, F.cliFill).
const fs = require("fs"), vm = require("vm"), assert = require("assert");
const F = {}; const ctx = { globalThis: { FLOW: F }, window: {} }; ctx.globalThis.globalThis = ctx.globalThis;
vm.runInNewContext(fs.readFileSync(__dirname + "/../flow/lazy/flow-run-ui.js", "utf8"), ctx);
const base = "# x\nMZLAB_IN='/path/to/the/folder/with/your/files'\nRUN=\"r\"\n";
assert.strictEqual(F.cliFill(base, ""), base);
assert.strictEqual(F.cliFill(base, "/Users/me/My Data"), "# x\nMZLAB_IN='/Users/me/My Data'\nRUN=\"r\"\n");
assert.strictEqual(F.cliFill(base, "'/Users/me/My Data'"), "# x\nMZLAB_IN='/Users/me/My Data'\nRUN=\"r\"\n");
assert.strictEqual(F.cliFill(base, "\"/a b\""), "# x\nMZLAB_IN='/a b'\nRUN=\"r\"\n");
assert.strictEqual(F.cliFill(base, "/it's"), "# x\nMZLAB_IN='/it'\\''s'\nRUN=\"r\"\n");
assert.strictEqual(F.cliFill(base, "/a$b`c"), "# x\nMZLAB_IN='/a$b`c'\nRUN=\"r\"\n");
console.log("ok");
