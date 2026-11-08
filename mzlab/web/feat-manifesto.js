// @ts-strict
// One shared fetch of wasm-feat/manifest.json per JS context (page or worker): every optional module asks here instead of fetching it again.
// A missing manifest (the e2e site builds none of the modules) is answered once, quietly, as an empty table.

/** @type {Promise<Record<string, any>> | null} */ let manifest = null;

/** the parsed manifest, `{}` when it cannot be had @param {URL | string} base the wasm-feat/ folder */
export function featManifest(base) {
  if (!manifest) manifest = fetch(new URL("manifest.json", base)).then(r => (r.ok ? r.json() : {}), () => ({})).catch(() => ({}));
  return manifest;
}
