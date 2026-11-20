// Prints check(request) of the molkit .wasm (wasm-bindgen --target nodejs), substructure(request) / cip(request) / hilite(request) / draw(request) (draw_png(request) for "draw_png") when the third
// argument is "sub" / "cip" / "hilite" / "draw", flow_moiety(params, table) when it is "moiety" (the table is the file named by the fourth), open_file(bytes)
// when it is "open" (the second argument is then the path of the structure file), or open_size(bytes) when it is "size"; see tests/test_molkit_wasm.py.
// usage: node molkit_wasm_node.js <pkg dir> <request JSON text> [sub | cip | hilite | draw | draw_png | moiety <table file> | open | size]
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_mol_wasm.js"));
try {
  const mode = process.argv[4];
  if (mode === "size") { process.stdout.write(w.open_size(Number(process.argv[3])) + "\n"); process.exit(0); }   // the second argument is a number of bytes
  if (mode === "open") { process.stdout.write(w.open_file(fs.readFileSync(process.argv[3])) + "\n"); process.exit(0); }
  if (mode === "draw_png") { process.stdout.write(Buffer.from(w.draw_png(process.argv[3]))); process.exit(0); }   // raw bytes, no newline
  const out = mode === "moiety" ? w.flow_moiety(process.argv[3], fs.readFileSync(process.argv[5])) : ({ sub: w.substructure, cip: w.cip, hilite: w.hilite, draw: w.draw }[mode] || w.check)(process.argv[3]);
  process.stdout.write(out + (mode === "moiety" ? "" : "\n"));
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
