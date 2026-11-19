// The metadata table of the Import data card (flow/lazy/flow-import.js without the DOM): reading a CSV/TSV table and laying it over the sample list. Exit code 1 on the first failure
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const ctx = vm.createContext({ window: {}, document: {} });
ctx.FLOW = { sampleBlank: i => ({ file: "", injection_order: i + 1, kind: "sample", time_min: null }) };
ctx.globalThis = ctx;
vm.runInContext(fs.readFileSync(path.resolve(__dirname, "../flow/lazy/flow-import.js"), "utf8"), ctx);
const F = ctx.FLOW, kinds = ["sample", "blank", "qc"];
// comma, semicolon and tab; quotes; BOM; header names of a lab sheet
let r = F.parseMetadata('﻿File,Role,Time (min),Series,Replicate\n"a, 1.mzML",Blank,,S1,1\nb.mzML,sample,30,"S ""x""",2\n', kinds);
eq(r.rows, [{ file: "a, 1.mzML", kind: "blank", series: "S1", replicate: 1 }, { file: "b.mzML", kind: "sample", time_min: 30, series: 'S "x"', replicate: 2 }]);
r = F.parseMetadata("Name;Type\nc.mzML;weird\nd.mzML;qc", kinds);
eq(r.rows, [{ file: "c.mzML" }, { file: "d.mzML", kind: "qc" }]); eq(r.unknown, ["weird"]);
eq(F.parseMetadata("a\tb\n1\t2", kinds).error, "The table needs a column named File (or Name).");
eq(F.parseMetadata("File\n", kinds).error, "The table has no rows.");
// patches go over the rows with the same file; a file Data does not have is added; a time is marked as confirmed
const base = { schema: "samples/1", samples: [{ file: "a.mzML", kind: "sample", time_min: null, series: "" }] };
const d = F.applyMetadata(base, [{ file: "a.mzML", kind: "blank", time_min: 5 }, { file: "z.mzML", series: "S" }]);
eq(d.samples.map(x => [x.file, x.kind, x.time_min, x.time_confirmed, x.series]), [["a.mzML", "blank", 5, true, ""], ["z.mzML", "sample", null, undefined, "S"]]);
eq(base.samples.length, 1);
console.log("ok");
