// @ts-strict
// What a bug report or a suggestion reads from the page: ONE function, `istantanea()`, returns the raw snapshot that bug-report-testo.js turns into fields (and cleans of names).
// Only what is neither personal nor data: language, window sizes, device class, engine state, the kind of the open tools (never their titles), counts and formats of the loaded files
// (never names), the names of the settings that differ from the default, recent errors and clicks (ERRORI), sizes of the stored state (never its keys). Nothing is sent.
/** @type {any} */
const W = window;
// E (files, panels), S (state), UIP and UIP_DEFAULT are top-level `const` of classic scripts: global names, not properties of window
const glob = (/** @type {() => any} */ f) => { try { return f(); } catch (_) { return undefined; } };

async function swState() {
  if (!("serviceWorker" in navigator)) return "unsupported";
  try {
    const r = await navigator.serviceWorker.getRegistration();
    const s = r ? (r.active ? "active" : r.waiting ? "waiting" : r.installing ? "installing" : "registered") : "none";
    return s + (navigator.serviceWorker.controller ? ", controlling" : "");
  } catch (_) { return "unknown"; }
}

/** the engine: its state, the version of its crate (api/provenance, answered by the engine itself) and the cache of the site build it came from */
async function motore() {
  const bits = [W.MZLAB_RUST ? "wasm ready" : W.MZLAB_RUST_ERR ? "wasm unavailable" : W.MZLAB_PHONE ? "no worker (phone)" : W.MZLAB_DESKTOP ? "native" : "wasm not started"];
  if (W.MZLAB_RUST || W.MZLAB_DESKTOP) {
    try { const r = await Promise.race([fetch("api/provenance"), new Promise((_, no) => setTimeout(no, 1500))]); const m = (await /** @type {Response} */ (r).text()).match(/# mzLab (\S+)/); if (m) bits.push("engine " + m[1]); } catch (_) { /* the engine does not answer: the state above says so */ }
  }
  try { const c = (await caches.keys()).filter(n => n.startsWith("mzlab-app-")); if (c.length) bits.push("build " + c[c.length - 1].slice(10, 30)); } catch (_) { /* no Cache API */ }
  return bits.join(", ");
}

async function archivio() {
  let n = 0, chars = 0;
  try { n = localStorage.length; for (let i = 0; i < n; i++) { const k = localStorage.key(i) || ""; chars += k.length + (localStorage.getItem(k) || "").length; } } catch (_) { return undefined; }
  let est = "";
  try { const e = await navigator.storage.estimate(); est = `, site data ${Math.round((e.usage || 0) / 1e6)} of ${Math.round((e.quota || 0) / 1e6)} MB`; } catch (_) { /* no estimate */ }
  return `${n} settings entries (${Math.round(chars / 1024)} KB)${est}`;
}

/** the kinds of the open cards, as «spec ×2, chrom ×1» */
function carte() {
  const by = new Map(), ps = glob(() => E.panels) || [];
  for (const p of ps) by.set(String(p.type || "?"), (by.get(String(p.type || "?")) || 0) + 1);
  return [...by].map(([k, n]) => `${k} ×${n}`).join(", ") || "none";
}

/** the raw snapshot; `suggestion` asks for the short one: the other fields are not even read */
export async function istantanea(suggestion = false) {
  const hash = (location.hash.match(/^#\/[A-Za-z0-9/_-]*/) || ["#/"])[0], view = `${glob(() => S.view) || "?"} (${hash})`;
  const lang = W.I18N && W.I18N.lang ? W.I18N.lang : document.documentElement.lang || "en";
  const base = { version: W.MZLAB_VERSION, commit: W.MZLAB_COMMIT, view, lang };
  if (suggestion) return base;
  const nav = /** @type {any} */ (navigator), act = glob(() => E.active), def = glob(() => UIP_DEFAULT) || {}, ui = glob(() => UIP) || {};
  return {
    ...base, date: new Date().toISOString(), ua: navigator.userAgent, mode: W.MZLAB_DESKTOP ? "desktop" : "web",
    screen: { sw: screen.width, sh: screen.height, iw: innerWidth, ih: innerHeight, dpr: devicePixelRatio },
    memory: nav.deviceMemory, cores: nav.hardwareConcurrency, isolated: !!W.crossOriginIsolated,
    opfs: !!(navigator.storage && typeof navigator.storage.getDirectory === "function"),
    engine: await motore(), tool: act && act.type ? String(act.type) : "none", cards: carte(), files: glob(() => E.files) || [],
    settings: Object.keys(def).filter(k => JSON.stringify(ui[k]) !== JSON.stringify(def[k])), sw: await swState(), storage: await archivio(),
    errors: W.ERRORI && W.ERRORI.ultimi ? W.ERRORI.ultimi(20) : [], actions: W.ERRORI && W.ERRORI.azioni ? W.ERRORI.azioni(15) : [],
  };
}

/** names of everything the user may have called something: files and labels of the open session (to be hidden from every text) */
export function nomi() {
  const out = /** @type {string[]} */ ([]);
  for (const f of glob(() => E.files) || []) for (const k of ["file", "label", "name", "sample", "nome"]) if (typeof f[k] === "string") out.push(f[k]);
  return out;
}
