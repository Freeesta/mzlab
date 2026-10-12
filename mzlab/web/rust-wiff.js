// SCIEX .wiff in the worker (rust-worker.js): the samples of a .wiff and one sample as mzML pieces. `readerFor(blob)` gives `(offset, length) => Uint8Array`.
const bytes = blob => new Uint8Array(new FileReaderSync().readAsArrayBuffer(blob));

const samples = (engine, m, id) => postMessage({ id, op: m.op, n: 0, info: engine.wiff_samples(m.name, bytes(m.blob)), buf: null, len: 0 });         // info: JSON array of the sample numbers

// one sample of a .wiff + .wiff.scan -> mzML pieces (the .wiff.scan is read block by block; m.scan null = missing); progress messages as the .raw conversion
function toMzml(engine, m, id, readerFor) {
  const parts = [], total = m.scan ? m.scan.size : 0, rd = m.scan ? readerFor(m.scan) : () => new Uint8Array(0);
  let read = 0, written = 0, last = 0;
  const tick = () => { const t = performance.now(); if (t - last > 150) { last = t; postMessage({ op: "progress", id, total, read, written }); } };
  engine.wiff_to_mzml(m.name, bytes(m.blob), m.sample, m.scan ? total : -1, (o, l) => { const b = rd(o, l); read = Math.max(read, o + b.length); tick(); return b; }, piece => { parts.push(piece); written += piece.length; tick(); });
  postMessage({ id, op: m.op, n: parts.length, parts }, parts.map(a => a.buffer));
}

export const wiffOp = (engine, m, id, readerFor) => (m.op === "wiffsamples" ? samples : toMzml)(engine, m, id, readerFor);
