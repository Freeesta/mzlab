// Loads mzlab/web/gradiente.js with stubs and prints the right-axis box (margin, title position) of the gradient / pressure overlays as JSON; tests/test_gradiente_margine.py checks it.
const fs = require("fs"), vm = require("vm"), path = require("path");
const win = {};
const ctx = vm.createContext({ window: win, I18N: { t: k => k }, document: { addEventListener() {} } });
vm.runInContext(fs.readFileSync(path.join(__dirname, "..", "mzlab/web/gradiente.js"), "utf8"), ctx);
const G = win.GRADIENTE;
const ASC = 0.8, DESC = 0.2;                                  // title glyph box around its baseline (rotated: the ascent points to the right)
const labelSets = { grad: ["0", "25", "50", "75", "100"], bar: ["0", "100", "200", "300", "400"], bar4: ["0", "500", "1000", "1500", "2000"] };
const rows = [], bad = [];
for (const z of [0.8, 1, 1.25, 1.5, 2]) for (const [name, labels] of Object.entries(labelSets)) {
  const fp = 12 * z, L = G.rightLayout(labels, fp), tickEnd = 7 + labels.reduce((m, s) => Math.max(m, Math.ceil(s.length * 0.6 * fp)), 0);

  const left = L.titleX - DESC * fp, right = L.titleX + ASC * fp;
  rows.push({ z, name, r: L.r, titleX: L.titleX, left: +left.toFixed(1), right: +right.toFixed(1) });
  if (right > L.r) bad.push(["clipped", z, name, right, L.r]);
  if (left < tickEnd) bad.push(["overlaps ticks", z, name, left, tickEnd]);
}
// the margin the code used before: fixed 46 * z with the baseline at W - 4 (right edge = 4 px inside the panel + ascent outside)
const before = [0.8, 1, 1.25, 1.5, 2].map(z => ({ z, r: Math.round(46 * z), overhang: +(ASC * 12 * z - 4).toFixed(1) }));
const fits = ["%B", "bar", "Pressure (bar)", "A very long axis title that cannot fit"].map(s => [s, G.fitTitle(s, 60, 12)]);
process.stdout.write(JSON.stringify({ rows, bad, before, fits }));
