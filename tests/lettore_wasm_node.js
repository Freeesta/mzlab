// Opens a dataset with the registry of readers of the .wasm engine (nodejs target of wasm-bindgen): node lettore_wasm_node.js <pkg> <folder> [anon]
// The members are every file of the folder (logical names relative to it), read by offset as the page does. Prints the answers as one JSON.
const fs = require("fs"), path = require("path");
const [pkg, dir] = process.argv.slice(2);
const { LettoreWasm } = require(path.join(pkg, "mzlab_wasm.js"));
const walk = d => fs.readdirSync(d, { withFileTypes: true }).flatMap(e => e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]);
const files = walk(dir), names = files.map(f => path.relative(dir, f).split(path.sep).join("/")), sizes = files.map(f => fs.statSync(f).size);
const fds = files.map(f => fs.openSync(f, "r"));
const read = (i, o, l) => { const b = Buffer.alloc(l); fs.readSync(fds[i], b, 0, l, o); return new Uint8Array(b); };
const l = LettoreWasm.open(names, Float64Array.from(sizes), read, process.argv[4] === "anon");
const camp = JSON.parse(l.campioni()), idx = JSON.parse(l.indice(0));
const [h, mz, it] = l.scansione(0, 2);
console.log(JSON.stringify({ formato: l.formato(), campioni: camp, indice: idx, scan2: { h: JSON.parse(h), mz: Array.from(mz), it: Array.from(it) }, metodo: JSON.parse(l.metodo(0)) }));
