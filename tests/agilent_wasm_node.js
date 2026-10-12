// Converts a synthetic Agilent .D folder to mzML with the .wasm engine (nodejs target of wasm-bindgen): node agilent_wasm_node.js <pkg> <folder.D> <out.mzML>.
// The files are given as the page gives them: the small ones whole, MSPeak.bin by offset; a missing file is an empty array.
const fs = require("fs"), path = require("path");
const [pkg, dir, out] = process.argv.slice(2);
const { Engine } = require(path.join(pkg, "mzlab_wasm.js"));
const small = p => { try { return new Uint8Array(fs.readFileSync(path.join(dir, p))); } catch (_) { return new Uint8Array(0); } };
const peak = fs.existsSync(path.join(dir, "AcqData/MSPeak.bin")) ? fs.readFileSync(path.join(dir, "AcqData/MSPeak.bin")) : Buffer.alloc(0), parts = [], progress = [];
new Engine().agilent_to_mzml(path.basename(dir), small("AcqData/MSScan.bin"), small("AcqData/MSScan.xsd"), small("AcqData/MSTS.xml"), small("AcqData/Devices.xml"),
  small("AcqData/Contents.xml"), small("AcqData/MSTree2.bin"), small("acqmeth.txt"), small("data.ms"), peak.length, (o, l) => new Uint8Array(peak.subarray(o, o + l)),
  p => parts.push(Buffer.from(p)), (d, t) => progress.push([d, t]));
fs.writeFileSync(out, Buffer.concat(parts));
console.log(JSON.stringify({ pieces: parts.length, last: progress[progress.length - 1] }));
