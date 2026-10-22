// The metadata table of the Import data card (flow/lazy/flow-import.js without the DOM): reading a CSV/TSV table and laying it over the sample list. Exit code 1 on the first failure
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const ctx = vm.createContext({ window: {}, document: {} });
ctx.FLOW = { sampleBlank: i => ({ file: "", ordine_iniezione: i + 1, tipo: "sample", tempo_min: null }) };
ctx.globalThis = ctx;
vm.runInContext(fs.readFileSync(path.resolve(__dirname, "../flow/lazy/flow-import.js"), "utf8"), ctx);
const F = ctx.FLOW, kinds = ["sample", "blank", "qc"];
// comma, semicolon and tab; quotes; BOM; header names of a lab sheet
let r = F.parseMetadata('﻿File,Role,Time (min),Series,Replicate\n"a, 1.mzML",Blank,,S1,1\nb.mzML,sample,30,"S ""x""",2\n', kinds);
eq(r.rows, [{ file: "a, 1.mzML", tipo: "blank", serie: "S1", replica: 1 }, { file: "b.mzML", tipo: "sample", tempo_min: 30, serie: 'S "x"', replica: 2 }]);
r = F.parseMetadata("Name;Type\nc.mzML;weird\nd.mzML;qc", kinds);
eq(r.rows, [{ file: "c.mzML" }, { file: "d.mzML", tipo: "qc" }]); eq(r.unknown, ["weird"]);
eq(F.parseMetadata("a\tb\n1\t2", kinds).error, "The table needs a column named File (or Name).");
eq(F.parseMetadata("File\n", kinds).error, "The table has no rows.");
// patches go over the rows with the same file; a file Data does not have is added; a time is marked as confirmed
const base = { schema: "campioni/2", campioni: [{ file: "a.mzML", tipo: "sample", tempo_min: null, serie: "" }] };
const d = F.applyMetadata(base, [{ file: "a.mzML", tipo: "blank", tempo_min: 5 }, { file: "z.mzML", serie: "S" }]);
eq(d.campioni.map(x => [x.file, x.tipo, x.tempo_min, x.tempo_confermato, x.serie]), [["a.mzML", "blank", 5, true, ""], ["z.mzML", "sample", null, undefined, "S"]]);
eq(base.campioni.length, 1);
console.log("ok");
