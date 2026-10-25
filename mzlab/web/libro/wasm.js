// @ts-strict
// Loads the Rust Libro converter (crates/mzlab-libro-wasm) built by tools/build_feat_wasm.sh: the manifest of wasm-feat/ gives its file and SHA-256, the bytes are checked before they run.

/** @typedef {{version: string, convert: (text: string, name: string, rawOk: boolean) => string, preview: (text: string, name: string, rawOk: boolean) => string, edit: (text: string, name: string, rawOk: boolean, src: string, html: string) => string, retitled: (oldText: string, newText: string, name: string) => boolean, reindex: (indexJs: string, file: string, html: string) => string}} Converter */

/** @param {ArrayBuffer} bytes */
async function sha256(bytes) {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return Array.from(d, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** @param {string} [dir] folder of wasm-feat/ relative to this page @returns {Promise<Converter>} */
export async function loadConverter(dir = "../wasm-feat/") {
  const root = new URL(dir, location.href);
  const res = await fetch(new URL("manifest.json", root), { cache: "no-store" });
  if (!res.ok) throw new Error("The converter is not in this build (no wasm-feat/manifest.json).");
  const entry = (await res.json()).libro;
  if (!entry) throw new Error("The converter is not in this build (no libro entry in the manifest).");
  const bytes = await (await fetch(new URL(entry.wasm, root))).arrayBuffer();
  if ((await sha256(bytes)) !== entry.sha256) throw new Error("The converter does not match its manifest (SHA-256). Reload the page.");
  const mod = await import(/* @vite-ignore */ new URL(entry.js, root).href);
  await mod.default({ module_or_path: bytes });
  return { version: mod.version(), convert: (text, name, rawOk) => mod.convert_chapter(text, name, rawOk),
    preview: (text, name, rawOk) => mod.preview_chapter(text, name, rawOk),
    edit: (text, name, rawOk, src, html) => mod.edit_preview_block(text, name, rawOk, src, html),
    retitled: (oldText, newText, name) => mod.headings_retitled(oldText, newText, name),
    reindex: (indexJs, file, html) => mod.reindex_chapter(indexJs, file, html),
  };
}
