// Agilent MassHunter .D folder -> mzML, done in the engine's worker (rust-worker.js, case "agilent2mzml") like the Thermo .raw (rust-raw.js): the mzML goes to a file of OPFS
// (rust-opfs.js), MSPeak.bin is read by offset (Blob.slice, synchronously) and never held, the small files of the folder are read whole. `files` = { scan, xsd, peak, ts, devices, contents,
// tree, acqmeth, datams }: Blobs (File) of AcqData/MSScan.bin, AcqData/MSScan.xsd, AcqData/MSPeak.bin, AcqData/MSTS.xml, AcqData/Devices.xml, AcqData/Contents.xml, AcqData/MSTree2.bin, acqmeth.txt and data.ms; scan and peak are required unless
// there is a data.ms (an older run), the others may be missing (null).

// worker side: `readerFor(blob)` gives (offset, length) => Uint8Array; `rawSink` is rust-opfs.js::rawSink
export async function agilentOp(engine, m, id, readerFor, rawSink) {
  const f = m.files, small = b => (b && b.size ? readerFor(b)(0, b.size) : new Uint8Array(0)), total = f.peak ? f.peak.size : 0, rd = f.peak ? readerFor(f.peak) : () => new Uint8Array(0), sink = await rawSink(m.dest);
  let read = 0, written = 0, last = 0, scans = 0, scansTotal = null;
  const tick = force => { const t = performance.now(); if (force || t - last > 150) { last = t; postMessage({ op: "progress", id, total, read, written, scans, scansTotal }); } };
  try {
    engine.agilent_to_mzml(m.name, small(f.scan), small(f.xsd), small(f.ts), small(f.devices), small(f.contents), small(f.tree), small(f.acqmeth), small(f.datams), total,
      (o, l) => { const b = rd(o, l); read = Math.max(read, o + b.length); tick(false); return b; }, !!m.anonymous, piece => { sink.write(piece); written += piece.length; tick(false); },
      (done, all) => { scans = done; scansTotal = all; tick(done === 0 || done === all); });
  } catch (e) { const q = await sink.error(); await sink.abort(); throw q || e; }          // q: the disk is full (error.quota)
  const r = await sink.finish();
  postMessage({ id, op: m.op, n: r.parts ? r.parts.length : 1, parts: r.parts || null, file: r.file || null }, r.parts ? r.parts.map(a => a.buffer) : []);
}

// page side, for rust-bridge.js: Blob of mzML (a File of OPFS when there is OPFS). `opts.anonymous`: leave the user, the computer, the serial number and the paths out. `signal` (AbortSignal) cancels it: the worker is busy in one long call, so cancelling asks the bridge to
// stop it and start a new one (h.abort). Rejects with an Error whose `json` is { error, error_key, params } like the API errors, or with { cancelled: true }.
export function makeConvertAgilent(h) {
  return async (name, files, onProgress, signal, opts = {}) => {
    const w = h.current(), cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });
    if (signal && signal.aborted) throw cancelled();
    const onAbort = () => { if (w && w === h.current()) h.abort(w); };
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    const dest = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}.mzML`;      // in OPFS mzlab/conv/
    const drop = () => navigator.storage.getDirectory().then(d => d.getDirectoryHandle("mzlab")).then(d => d.getDirectoryHandle("conv")).then(d => d.removeEntry(dest)).catch(() => {});
    const r = await h.rpc({ op: "agilent2mzml", name: h.base(name), files, dest, anonymous: !!opts.anonymous }, onProgress);
    if (signal) signal.removeEventListener("abort", onAbort);
    if (r.cancelled || signal && signal.aborted) { drop(); throw cancelled(); }
    if (r.dead || r.error) {
      drop();
      let e = { error_key: "err.file.agilent", params: { detail: "" } };
      try { const j = JSON.parse(r.error); if (j.error_key) e = j; } catch (_) { /* the worker was lost */ }
      throw Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
    }
    return Object.assign(r.file || new Blob(r.parts, { type: "application/xml" }), { mzlabId: null });
  };
}
