// SCIEX .wiff in the worker (rust-worker.js): the samples of a .wiff and one sample as mzML pieces. `readerFor(blob)` gives `(offset, length) => Uint8Array`; both files are read by offset (the .wiff is small, the .wiff.scan is never held).

const samples = (engine, m, id, readerFor) => postMessage({ id, op: m.op, n: 0, info: engine.wiff_samples(m.name, m.blob.size, readerFor(m.blob)), buf: null, len: 0 });         // info: JSON array of the sample numbers

// One sample of a .wiff + .wiff.scan -> the mzML pieces (the .wiff.scan is read in windows; m.scan null = missing). `onProgress({ total, read, written, scans, scansTotal })` is
// called a few times a second: scans = spectra written, scansTotal = spectra to write, read = how far in the .wiff.scan the reading has got (bytes).
export function convert(engine, m, readerFor, onProgress) {
  const parts = [], total = m.scan ? m.scan.size : 0, rd = m.scan ? readerFor(m.scan) : () => new Uint8Array(0);
  let read = 0, written = 0, last = 0, scans = 0, scansTotal = 0;
  const tick = force => { const t = performance.now(); if (force || t - last > 150) { last = t; onProgress({ total, read, written, scans, scansTotal }); } };
  engine.wiff_to_mzml(m.name, m.blob.size, readerFor(m.blob), m.scan ? total : -1, (o, l) => { const b = rd(o, l); read = Math.max(read, o + b.length); return b; }, m.sample, m.centroid === true,
    piece => { parts.push(piece); written += piece.length; tick(false); }, (done, all) => { scans = done; scansTotal = all; tick(done === 0 || done === all); });
  return parts;
}

// "wiffsamples" in the engine's own thread; "wiff2mzml" in a pool of short-lived workers (wiff-worker.js), one sample each, several at once. If the pool cannot start
// (`unavailable`) the sample is converted here, as before. "wiffcancel" stops a queued or running sample by its key.
export function wiffOp(engine, m, id, readerFor, pool) {
  if (m.op === "wiffsamples") return samples(engine, m, id, readerFor);
  if (m.op === "wiffcancel") { pool().cancel(m.key); return postMessage({ id, op: m.op, n: 0, info: "" }); }
  const progress = p => postMessage({ op: "progress", id, ...p });
  const send = parts => postMessage({ id, op: m.op, n: parts.length, parts }, parts.map(a => a.buffer));
  const fail = text => postMessage({ id, op: m.op, error: /^\{/.test(text) ? text : JSON.stringify({ error_key: "err.file.wiff", params: { detail: "worker" } }), fatal: false });
  return pool().build({ name: m.key, msg: { name: m.name, blob: m.blob, scan: m.scan, sample: m.sample, centroid: m.centroid === true }, onProgress: progress }).then(r => {
    if (r.ok) send(r.parts);
    else if (r.reason === "cancelled") postMessage({ id, op: m.op, cancelled: true });
    else if (r.reason === "unavailable") { try { send(convert(engine, m, readerFor, progress)); } catch (e) { fail(typeof e === "string" ? e : String(e)); } }
    else fail(r.text || "");
  });
}
