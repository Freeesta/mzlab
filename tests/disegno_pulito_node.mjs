// Runs canon() and isDirty() of mzlab/web/disegno-pulito.js and prints the results as JSON. See tests/test_disegno_pulito.py.
import { canon, isDirty } from "../mzlab/web/disegno-pulito.js";
const a = '{"root":{"nodes":[{"$ref":"mol0"}]},"mol0":{"type":"molecule","atoms":[{"label":"C","location":[0,0,0]}]}}';
const b = '{\n "mol0": {"atoms":[{"location":[0,0,0],"label":"C"}],"type":"molecule"},\n "root":{"nodes":[{"$ref":"mol0"}]}}';
const c = a.replace('"C"', '"O"');
const s = canon(a);
console.log(JSON.stringify([
  canon(a) === canon(b),            // key order and white space do not matter
  canon(a) === canon(c),            // a different atom does
  canon("not json"),                // never throws
  isDirty(a, null, true),           // empty canvas: nothing to lose
  isDirty(a, null, false),          // drawn, never saved
  isDirty(b, s, false),             // same canvas as the saved one (undo back): clean
  isDirty(c, s, false),             // edited after the save
  isDirty(c, s, true),              // cleared after the save: nothing to lose
]));
