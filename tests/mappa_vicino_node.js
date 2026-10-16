// Node check of MAPVICINO (mzlab/web/mappa-vicino.js): previous / next file of the map, jumping over the reference. Exits 1 on the first failure.
const fs = require("fs"), assert = require("assert");
const M = new Function(fs.readFileSync(process.argv[2], "utf8").replace("const MAPVICINO", "var MAPVICINO") + ";return MAPVICINO")();
const k = [3, 5, 8, 9];                                                  // t0 t5 t10 t15 (keys are not 0..n)
assert.strictEqual(M.vicino(k, 5, 1, 3), 8); assert.strictEqual(M.vicino(k, 5, -1, null), 3);
assert.strictEqual(M.vicino(k, 3, -1, null), null); assert.strictEqual(M.vicino(k, 9, 1, 3), null);   // no file before / after: the arrow is disabled
assert.strictEqual(M.vicino(k, 5, -1, 3), null);                         // the only file before is the reference: nothing to show
assert.strictEqual(M.vicino(k, 3, 1, 5), 8); assert.strictEqual(M.vicino(k, 8, -1, 5), 3);   // the reference in the middle is jumped
assert.strictEqual(M.vicino(k, 8, 1, 9), null);                          // the next one is the reference and the last: none
assert.strictEqual(M.vicino(k, 7, 1, null), null); assert.strictEqual(M.vicino([], 1, 1, null), null);   // file not in the menu
console.log("ok");
