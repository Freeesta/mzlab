// @ts-strict
// Single scans for rust-bridge.js: those of a PROFILE file come from the index; those of a centroid file, and `api/scan` / `api/scanavg`, from slices
// of the mzML read by offset in the worker (the table of the scans of the index gives where each <spectrum> starts and ends). Returns the JSON
// text, or null when Pyodide must answer.
async function scanRoute({ slots, rpc, stats }, k, path, query) {
  const s = slots[k];
  await s.opening;
  if (!s.idx || s.bad) return null;
  let r = path === "/api/spectra" ? await rpc({ op: "route", slot: k, path, query }) : {};
  if (!r.dead && !r.error && !r.info && s.blob) r = await rpc({ op: "mzmlroute", slot: k, path, query, blob: s.blob });
  if (r.dead || r.error || !r.info) return null;
  stats.rust++;
  return r.info;
}

// `api/spectrum` with a background (`bgk`): the window minus the window of another file (or of the same one), both from their indexes.
async function bgRoute({ slots, rpc, stats }, k, bgk, query) {
  const ks = [k, bgk];
  if (!ks.every(x => Number.isInteger(x) && slots[x])) return null;
  await Promise.all(ks.map(x => slots[x].opening));
  if (!ks.every(x => slots[x].idx && !slots[x].bad)) return null;
  const r = await rpc({ op: "specbg", slot: k, query });
  if (r.dead || r.error || !r.info) return null;
  stats.rust++;
  return r.info;
}

// `api/scanbin` of a centroid file: the binary block of the scans, read from the mzML in the worker. Returns the bytes, or null when Pyodide must answer.
async function scanBinRoute({ slots, rpc, stats }, k, query) {
  const s = slots[k];
  await s.opening;
  if (!s.idx || s.bad || !s.blob) return null;
  const r = await rpc({ op: "scanbin", slot: k, query, blob: s.blob });
  if (r.dead || r.error || !r.buf) return null;
  stats.rust++;
  return r.buf;
}

const SCAN_ROUTES = /^\/api\/(spectra|scan|scanavg|scanbin|msntree)$/;
// true for the routes of single scans (and of a spectrum with a background) that do not point to a reference file
export const hasScan = (path, q) => !q.get("ref") && (SCAN_ROUTES.test(path) || (path === "/api/spectrum" && !!q.get("bgk")));

// The answer { text } or { buf } of those routes, or null when Pyodide must answer.
export async function scanRoutes(ctx, k, path, q, search) {
  if (path === "/api/scanbin") {
    const buf = await scanBinRoute(ctx, k, search);
    return buf && { buf };
  }
  const text = path === "/api/spectrum" ? await bgRoute(ctx, k, Number(q.get("bgk")), search) : await scanRoute(ctx, k, path, search);
  return text && { text };
}
