// @ts-strict
// Single scans for rust-bridge.js: those of a PROFILE file come from the index; those of a centroid file, and `api/scan` / `api/scanavg` / `api/dda_window`, from slices
// of the mzML read by offset in the worker (the table of the scans of the index gives where each <spectrum> starts and ends). A refusal of the engine
// (a scan outside the file: `{"error_key",..}`) is the answer, with status 400 as the desktop's errors; null = the route is not answered here.
export const refusal = r => !r.dead && typeof r.error === "string" && r.error.startsWith("{") ? { status: 400, text: r.error } : null;

// The JSON text of a single scan, or a refusal.
async function scanRoute({ slots, rpc, stats }, k, path, query) {
  const s = slots[k];
  await s.opening;
  if (!s.idx || s.bad) return null;
  let r = path === "/api/spectra" ? await rpc({ op: "route", slot: k, path, query }) : {};
  if (!r.dead && !r.error && !r.info && s.blob) r = await rpc({ op: "mzmlroute", slot: k, path, query, blob: s.blob });
  if (r.error) return refusal(r);
  if (r.dead || !r.info) return null;
  stats.rust++;
  return { text: r.info };
}

// `api/spectrum` with a background (`bgk`): the window minus the window of another file (or of the same one), both from their indexes.
async function bgRoute({ slots, rpc, stats }, k, bgk, query) {
  const ks = [k, bgk];
  if (!ks.every(x => Number.isInteger(x) && slots[x])) return null;
  await Promise.all(ks.map(x => slots[x].opening));
  if (!ks.every(x => slots[x].idx && !slots[x].bad)) return null;
  const r = await rpc({ op: "specbg", slot: k, query });
  if (r.error) return refusal(r);
  if (r.dead || !r.info) return null;
  stats.rust++;
  return { text: r.info };
}

// `api/scanbin` of a centroid file: the binary block of the scans, read from the mzML in the worker, or a refusal.
async function scanBinRoute({ slots, rpc, stats }, k, query) {
  const s = slots[k];
  await s.opening;
  if (!s.idx || s.bad || !s.blob) return null;
  const r = await rpc({ op: "scanbin", slot: k, query, blob: s.blob });
  if (r.error) return refusal(r);
  if (r.dead || !r.buf) return null;
  stats.rust++;
  return { buf: r.buf };
}

const SCAN_ROUTES = /^\/api\/(spectra|scan|scanavg|scanbin|msntree|dda_window)$/;                // dda_window reads the parent survey scans for the precursor intensities a reader lacks
// true for the routes of single scans (and of a spectrum with a background) that do not point to a reference file
export const hasScan = (path, q) => !q.get("ref") && (SCAN_ROUTES.test(path) || (path === "/api/spectrum" && !!q.get("bgk")));

// The answer { text } or { buf } of those routes (with { status } for a refusal), or null when they are not answered here.
export async function scanRoutes(ctx, k, path, q, search) {
  if (path === "/api/scanbin") return scanBinRoute(ctx, k, search);
  return path === "/api/spectrum" ? bgRoute(ctx, k, Number(q.get("bgk")), search) : scanRoute(ctx, k, path, search);
}
