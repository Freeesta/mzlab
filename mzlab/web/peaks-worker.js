// Worker of the peak measures (qualita.js): the Rust functions of mzlab-wasm that need no file, only a trace of the page. Separate from the
// worker of the Rust engine (rust-worker.js), so the quality columns and the methods of the limits work with or without ?motore=rust.
import init, { detect_peaks, peak_quality } from "./wasm/mzlab_wasm.js";

const FIELDS = 10;                                      // PEAK_FIELDS of mzlab-wasm
const ready = init();

onmessage = async ev => {
  const m = ev.data;
  try {
    await ready;
    if (m.op === "quality") {
      const out = new Float64Array(3);
      const ok = peak_quality(m.rt, m.y, m.lo, m.hi, out);
      postMessage({ id: m.id, v: ok ? Array.from(out) : null });
    } else if (m.op === "detect") {
      const out = new Float64Array(FIELDS * 64);
      const n = detect_peaks(m.rt, m.y, m.mode, out);
      const peaks = [];
      for (let i = 0; i < n; i++) {
        const r = out.subarray(i * FIELDS, (i + 1) * FIELDS);
        peaks.push({ apex: r[0], lo: r[1], hi: r[2], area: r[3], height: r[4], snr: r[5], n: r[6], gaussian: r[7], asymmetry: Number.isNaN(r[8]) ? null : r[8], noise: r[9] });
      }
      postMessage({ id: m.id, v: peaks });
    } else postMessage({ id: m.id, error: "unknown operation" });
  } catch (e) {
    postMessage({ id: m.id, error: String((e && e.message) || e) });
  }
};
