// Writes golden.json: the output of the JS readers (TP_Mine/js/25_untarget.js) on the fixtures of ../, the reference of crates/mzlab-feat/tests/import_parity.rs.
// node genera.js   (from this directory). Cases: {name, files:[path], pol, max, variant, expect}. variant "crlf" = CRLF line ends.
globalThis.window = globalThis; const store = {};
globalThis.localStorage = { getItem: k => (k in store ? store[k] : null), setItem: (k, v) => { store[k] = String(v); } };
const fs = require("fs"), path = require("path"), W = path.join(__dirname, "../../../../mzlab/web"), J = path.join(__dirname, "../../../../TP_Mine/js");
require(path.join(W, "livelli-1.js")); require(path.join(W, "livelli.js")); const U = require(path.join(J, "25_untarget.js"));
const rd = p => fs.readFileSync(path.join(__dirname, "..", p), "utf8");
const variant = (text, v) => (v === "crlf" ? text.replace(/\n/g, "\r\n") : text);
const strip = t => { const o = JSON.parse(JSON.stringify(t, (k, v) => (v instanceof Map ? undefined : v))); delete o.byId; return o; };
const ALL = ["mzmine/tabella_pos.csv", "mzmine/annotazioni_pos.csv", "mzmine/fbmn.mgf", "gnps/biblioteca.tsv", "gnps/rete.tsv"];
const cases = [
  { name: "all_pos", files: ALL, pol: 1 }, { name: "all_nopol", files: ALL, pol: 0 }, { name: "all_neg", files: ALL, pol: -1 }, { name: "all_max3", files: ALL, pol: 1, max: 3 },
  { name: "all_crlf", files: ALL, pol: 1, variant: "crlf" }, { name: "full_only", files: ["mzmine/tabella_pos.csv"], pol: 0 },
  { name: "full_annotations", files: ["mzmine/tabella_pos.csv", "mzmine/annotazioni_pos.csv"], pol: 1 },
  { name: "quant_edges", files: ["mzmine/fbmn_quant.csv", "mzmine/fbmn_edges.csv", "mzmine/fbmn.mgf"], pol: 1 },
  { name: "quant_crlf", files: ["mzmine/fbmn_quant.csv", "mzmine/fbmn_edges.csv"], pol: 1, variant: "crlf" },
  { name: "msdial", files: ["msdial/allineamento.txt"], pol: 1 }, { name: "msdial_crlf", files: ["msdial/allineamento.txt"], pol: 1, variant: "crlf" },
  { name: "mgf_only", files: ["mzmine/fbmn.mgf"], pol: 1 }, { name: "gnps_only", files: ["gnps/biblioteca.tsv", "gnps/rete.tsv"], pol: 1 },
  { name: "full_gnps_first", files: ["gnps/rete.tsv", "gnps/biblioteca.tsv", "mzmine/tabella_pos.csv"], pol: 1 },
];
for (const c of cases) {
  const files = c.files.map(p => ({ name: p.split("/").pop(), text: variant(rd(p), c.variant), sha256: "abc" }));
  try { c.expect = strip(U.build(files, { pol: c.pol, maxFeatures: c.max })); } catch (e) { c.expect = { error: e.key || "" }; }
}
const num = ["1", " 2.5 ", "1,5", "1,234.5", "na", "NaN", "-", "", "x", "1e3", ".5", "5.", "+7", "0x10", "1 000", "Infinity", "-3", "  "].map(s => [s, U.num(s)]);
const inline = [
  ["x,y\n", ",", "a,b,c\n1,\"x,y\",3\n2,\"line1\nline2\",4\n"], ["x", ",", "a,b\n1,2,\n,\n\n\"q\"\"r\",\"s\"t,u\n"], ["x", "\t", "\n\na\tb\n1\t2"],
].map(([, d, text]) => { const rows = []; const h = U.eachRow(text, d, h => null, r => rows.push(r)); return { d, text, header: h, rows }; });
const adducts = ["M+H", "M-H", "[M+Na]+", "M+HCOO", "M-H2O+H", "M-H2O", " [M + H]+ ", "", "M+Cl", "M+2H", "M+H-", "M+CH3COO"].map(s => [s, U.normAdduct(s)]);
const checks = [["C10H17NO2", 184.1332, "M+H", 1], ["C10H17NO2", 206.1151, "[M+H]+", 1], ["C10H17NO2", 301.1547, "M+H", 1], ["", 184.1, "M+H", 1], ["C10H17NO2", 182.1187, "[M-H]-", -1],
  ["C10H17NO2", 0, "M+H", 1], ["C10H17NO2", null, "M+H", 1], ["Xx5", 184.1, "M+H", 1], ["C6H12O6", 203.0526, "", 1], ["C6H12O6", 179.0561, "[M-H]-", -1], ["C10H17NO2", 184.1332, "M+Na", 0],
  ["B2H6", 28.0, "M+H", 1], ["FeCl3", 162.8, "[M+Cl]-", -1]].map(a => [a, U.checkStructure(a[0], a[1], a[2], a[3])]);
const forms = ["C10H17NO2", "C6H12O6", "", "c6", "C10H17NO2 ", "H2O", "CH3OH", "(CH3)2", "Fe2O3", "BH3", "C0"].map(s => [s, U.formulaMass(s)]);
const names = ["Massbank:Caffeine", "MassbankEU:Foo|HCD 30", "Spectral Match to Bar from NIST14", "MoNA:Baz - 30.0 eV", "MB000123 Qux", "Quux_CE35.5", "  Plain  ", "ReSpect:Name|x|y", "", "AB12345 Name", "ABC123456  Name2",
  "ABCD12345 Name3", "12345 Name4", "1234 Name5", "Name_CE30", "Name_CE30.5x", "Name_CE", "massbank:lower", "Foo - 40.0 eV|x", "X_CE1_CE2"].map(s => [s, U.cleanName(s)]);
const prop = ["Candidate X", "Y (known structural isomers)", "Z (delta mass: 14)", "Candidate", "candidate_x", "Candidates", " candidate x", "Plain"].map(s => [s, U.isPropagated(s)]);
const spec = ["10:5 20:6", "10:5 x:6 30", "10: :4", "  ", "5:1:2 7:8"].map(s => [s, U.parseSpectrumText(s)]);
const heads = [];
for (const p of ["mzmine/tabella_pos.csv", "mzmine/annotazioni_pos.csv", "mzmine/fbmn_quant.csv", "mzmine/fbmn_edges.csv", "mzmine/fbmn.mgf", "msdial/allineamento.txt", "gnps/biblioteca.tsv", "gnps/rete.tsv"]) heads.push([p, U.detect("x", rd(p))]);
heads.push(["a.mgf", U.detect("a.mgf", "id,mz,rt\n")], ["none", U.detect("x", "a,b\n1,2\n")], ["quoted", U.detect("x", '"id","mz","rt"\n1,2,3\n')], ["empty-first", U.detect("x", "\n\nid,mz,rt\n")]);
fs.writeFileSync(path.join(__dirname, "golden.json"), JSON.stringify({ cases, num, inline, adducts, checks, forms, names, prop, spec, heads }, null, 1) + "\n");
