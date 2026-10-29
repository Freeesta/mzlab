// Shared bench suite (Node and browser). m = wasm-bindgen module (initialised), files = {hr, lr, raw?}; returns {job: {ms, min, v}}: v is the check value, identical in every variant.
async function suite(m, files, reps, log) {
  const now = () => performance.now();
  const res = {};
  const T = (name, f, r = reps) => { const t = []; let v; for (let i = 0; i < r; i++) { const a = now(); v = f(); t.push(now() - a); } t.sort((a, b) => a - b); res[name] = { ms: +t[Math.floor(t.length / 2)].toFixed(1), min: +t[0].toFixed(1), v }; log(name, res[name]); };
  m.load(files.hr);
  T("parse_hr", () => m.parse());
  T("index_hr", () => m.index());
  T("xic200_hr", () => m.xic(200, 120, 980, 0.005));
  T("centroid_hr", () => m.centroid(0.0015, 0.0036, 100));
  T("pick50_hr", () => m.pick(50, 120, 980, 0.005));
  T("entropy300_hr", () => m.entropy(300));
  m.load(files.lr);
  T("parse_lr", () => m.parse());
  T("index_lr", () => m.index());
  T("xic200_lr", () => m.xic(200, 130, 470, 0.5));
  T("pick50_lr", () => m.pick(50, 130, 470, 0.5));
  T("compose200", () => m.compose(200, 10));
  if (files.raw && m.raw_parse) {
    m.load(files.raw);
    T("parse_raw", () => m.raw_parse());
    T("index_raw", () => m.raw_index());
  }
  m.deflate_setup(64);
  T("inflate64MB", () => m.inflate());
  return res;
}
if (typeof module !== "undefined") module.exports = { suite };
