// A JS caller that passes fewer sizes than files must get {"error_key":"error.badIndex"}, not a trap (the module is built with panic = "abort").
// usage: node feat_wasm_sizes_node.js <pkg dir> <regole_tp_2.json> <file.mzidx>... ; prints one line per check, "ok <name>" or "FAIL <name>: <why>".
const fs = require("fs");
const path = require("path");
const [pkg, rules, ...files] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_feat_wasm.js"));
const fds = files.map(f => fs.openSync(f, "r"));
const sizes = Float64Array.from(files, f => fs.statSync(f).size);
const read = (k, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
const sink = () => { const parts = []; return { parts, write: (off, b) => { parts.push([off, b.slice()]); } }; };
const bytes = s => { const n = s.parts.reduce((m, [o, b]) => Math.max(m, o + b.length), 0); const out = new Uint8Array(n); s.parts.forEach(([o, b]) => out.set(b, o)); return out; };

// what a call throws, as a string; null when it returns
const thrown = fn => { try { fn(); return null; } catch (e) { return String(e && e.message ? e.message : e); } };
const check = (name, err) => {
  if (err !== null && err.includes("error.badIndex")) console.log(`ok ${name}`);
  else console.log(`FAIL ${name}: ${err === null ? "returned without an error" : err}`);
};

const recipe = '{"mode":"ion"}';
const stored = files.map((_, k) => w.detect_file((_, off, len) => read(k, off, len), sizes[k], `f${k}`, recipe));
const short = sizes.slice(0, files.length - 1);

// align_untargeted_to: the `.mzfeatf` of the untargeted pipeline, then the join with one size fewer
const unt = '{"engine":"untargeted","polarity":1}';
const ustored = files.map((_, k) => { const s = sink(); w.detect_untargeted_to((_, off, len) => read(k, off, len), sizes[k], `f${k}`, unt, s.write); return bytes(s); });
check("align_untargeted_to", thrown(() => w.align_untargeted_to(ustored, read, short, unt, sink().write, 0)));
const good = sink();
const ok = thrown(() => w.align_untargeted_to(ustored, read, sizes, unt, good.write, 0));
console.log(ok === null ? "ok align_untargeted_to afterwards" : `FAIL align_untargeted_to afterwards: ${ok}`);

// build_evidence_to: the table of the series, the hypotheses and the proofs that measure the rows with one `.mzidx` size fewer
const table = sink();
w.align_series_to(stored, read, sizes, recipe, table.write, 0);
const tb = bytes(table);
const h = w.run_hypotheses(stored, read, sizes, fs.readFileSync(rules), "", "C21H32N2O2", 1.0, recipe);
const evid = sz => w.build_evidence_to((_, off, len) => tb.slice(off, off + len), tb.length, w.load_hypotheses(h.state()), Float64Array.from(files, () => NaN),
  345.253655, 1.0, recipe, sink().write, 0, undefined, stored, read, sz);
check("build_evidence_to", thrown(() => evid(short)));
const again = thrown(() => evid(sizes));
console.log(again === null ? "ok build_evidence_to afterwards" : `FAIL build_evidence_to afterwards: ${again}`);
