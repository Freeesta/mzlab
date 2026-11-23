// Runs file pack / check-pack / unpack on the .wasm `pack` (crates/mzlab-pack-wasm, wasm-bindgen --target nodejs) with plain files standing in for OPFS, and prints the answer.
// See tests/test_parita_pack.py. usage: node pack_wasm_node.js <pkg dir> <job.json>
// jobs: {op: "pack", files: [[rel, abs]...], dir, name, part_size} | {op: "plan", ...same, no reads} | {op: "verify", manifest, dir} | {op: "unpack", manifest, dir, out, force, dry_run}
const fs = require("fs");
const path = require("path");
const mod = require(path.join(process.argv[2], "mzlab_pack_wasm.js"));
const job = JSON.parse(fs.readFileSync(process.argv[3], "utf8"));
const readAt = (fd, off, len) => { const b = Buffer.alloc(len); const n = fs.readSync(fd, b, 0, len, off); return new Uint8Array(b.subarray(0, n)); };
const out = {};
try {
  if (job.op === "plan" || job.op === "pack") {
    const list = JSON.stringify(job.files.map(([rel, abs]) => [rel, fs.statSync(abs).size]));
    if (job.op === "plan") out.doc = mod.pack_plan(list, job.name, job.part_size);
    else {
      const fds = job.files.map(([, abs]) => fs.openSync(abs, "r")), parts = new Map();
      const manifest = mod.pack(list, job.name, job.part_size, (i, off, len) => readAt(fds[i], off, len),
        (p, bytes) => { if (!parts.has(p)) parts.set(p, fs.openSync(path.join(job.dir, `${job.name}.part${String(p + 1).padStart(4, "0")}`), "w")); fs.writeSync(parts.get(p), bytes); });
      for (const fd of [...fds, ...parts.values()]) fs.closeSync(fd);
      fs.writeFileSync(path.join(job.dir, `${job.name}.pack.json`), manifest);
      out.doc = mod.pack_doc(manifest);
    }
  } else {
    const manifest = fs.readFileSync(job.manifest, "utf8"), names = JSON.parse(manifest).parts.map(p => p.name);
    const file = n => path.join(job.dir, n);
    const size = i => fs.existsSync(file(names[i])) ? fs.statSync(file(names[i])).size : -1;
    const read = (i, off, len) => { const fd = fs.openSync(file(names[i]), "r"); try { return readAt(fd, off, len); } finally { fs.closeSync(fd); } };
    if (job.op === "verify") out.doc = mod.verify_pack(manifest, size, read, null);
    else {
      let cur = null;
      const sink = {
        exists: p => fs.existsSync(path.join(job.out, p)),
        begin: p => { const to = path.join(job.out, p); fs.mkdirSync(path.dirname(to), { recursive: true }); cur = { to, fd: fs.openSync(to + ".mzpack-tmp", "w") }; },
        write: b => fs.writeSync(cur.fd, b),
        end: keep => { fs.closeSync(cur.fd); if (keep) fs.renameSync(cur.to + ".mzpack-tmp", cur.to); else fs.unlinkSync(cur.to + ".mzpack-tmp"); cur = null; },
      };
      out.doc = mod.unpack(manifest, size, read, sink, !!job.force, !!job.dry_run, null);
    }
  }
} catch (e) { out.error = typeof e === "string" ? e : String(e && e.message || e); }
process.stdout.write(JSON.stringify(out));
