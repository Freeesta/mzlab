// Writes tests/data/molecole_tabui_oro.json: every name that the name box of Tabui (csvRows/getList/suggest of TP_Mine/js/20_tpmine.js, run as they are) finds for
// each query, in its order, as indices of the rows of TP_Mine/data/composti.csv. Only numbers are saved: the queries are rebuilt by the Rust test from the catalogue with
// the same rule (prefixes of 2, 3 and 5 characters of every name, then 100 inner pieces), so no name is written in the file.
// Run once, while the old list and the old code exist:  node tools/molecole_oro_tabui.mjs
import { readFileSync, writeFileSync } from "node:fs";

const root = new URL("..", import.meta.url).pathname;
const src = readFileSync(root + "TP_Mine/js/20_tpmine.js", "utf8");
const from = src.indexOf("const csvRows = txt => {");
const to = src.indexOf("/** @type {any[]} */ let sugItems");
if (from < 0 || to < 0) throw new Error("csvRows/getList/suggest not found in 20_tpmine.js");
const composti = readFileSync(root + "TP_Mine/data/composti.csv", "utf8");
// the box shows 8 proposals: the lists here are whole (the slice is lifted) so that the test can tell a lost hit from one that went below the 8th place
const whole = src.slice(from, to).replace(".slice(0, 8)", "");
if (whole.length === to - from) throw new Error("the slice of suggest was not found");
const { getList, suggest } = new Function("ctx", `let LIST = null; ${whole}; return { getList, suggest };`)(() => ({ files: { "composti.csv": composti } }));
const list = getList();

// the catalogue, read with the same rule as the Rust test (`;` delimiter, quotes)
const rows = [];
{
  const t = readFileSync(root + "mzlab/web/molecole/catalogo.csv", "utf8");
  let row = [], f = "", q = false;
  for (let i = 0; i < t.length; i++) {
    const c = t[i];
    if (q) { if (c === '"') { if (t[i + 1] === '"') { f += '"'; i++; } else q = false; } else f += c; }
    else if (c === '"') q = true; else if (c === ";") { row.push(f); f = ""; }
    else if (c === "\n") { row.push(f); rows.push(row); row = []; f = ""; } else f += c;
  }
}
const head = rows.shift();
const col = n => head.indexOf(n);
const names = [];
for (const r of rows) {
  const cats = r[col("catalog")].split("|");
  if (!cats.includes("tabui")) continue;
  names.push(r[col("name")]);
  if (cats.length === 1) names.push(...r[col("synonyms")].split("|").filter(Boolean));
}

const queries = [], seen = new Set();
for (const n of names) for (const k of [2, 3, 5]) {
  const c = Array.from(n);
  if (c.length < k) continue;
  const p = c.slice(0, k).join("");
  if (!seen.has(p)) { seen.add(p); queries.push(p); }
}
for (let i = 0; i < 100; i++) {
  let j = (i * 37 + 11) % names.length;
  while (Array.from(names[j]).length < 8) j = (j + 1) % names.length;
  const c = Array.from(names[j]), len = 3 + (i % 3), start = 1 + ((i * 7) % (c.length - len - 1));
  queries.push(c.slice(start, start + len).join(""));
}
const index = new Map(list.map((c, i) => [c, i]));
const results = queries.map(q => suggest(q).map(c => index.get(c)));
writeFileSync(root + "tests/data/molecole_tabui_oro.json", JSON.stringify({ righe: list.length, risultati: results }) + "\n");
console.log(queries.length, "queries,", results.filter(r => r.length).length, "with results");
