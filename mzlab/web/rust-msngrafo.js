// @ts-strict
// The fragmentation graph of an MSn infusion in the worker of the engine (rust-worker.js, op "msngrafo"): the .wasm `msngrafo` of
// wasm-feat/manifest.json (crates/mzlab-msngrafo-wasm) is fetched on the first request, checked against its SHA-256, and reads the .mzidx of
// OPFS and the mzML of the file through `readerFor`. The engine itself is not touched. docs/agenti/msn-grafo.md.

/** @type {Record<string, Promise<any>>} */ const loading = {};

/** the module `key` of wasm-feat/manifest.json, fetched on first use and checked against its SHA-256 @param {string} key */
function load(key) {
  if (!loading[key]) loading[key] = (async () => {
    const base = new URL("./wasm-feat/", import.meta.url);
    const man = (await (await fetch(new URL("manifest.json", base))).json())[key];
    if (!man) throw JSON.stringify({ error_key: "err.route.unavailable", params: {} });
    const bytes = await (await fetch(new URL(man.wasm, base))).arrayBuffer();
    const hex = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(x => x.toString(16).padStart(2, "0")).join("");
    if (hex !== man.sha256) throw JSON.stringify({ error_key: "error.featWasmHash", params: {} });
    const m = await import(new URL(man.js, base).href);
    await m.default({ module_or_path: bytes });
    return m;
  })().catch(e => { delete loading[key]; throw e; });
  return loading[key];
}

/**
 * The query with the expectations of the structure rules (`attese`) when it asks for the rules with a SMILES: the .wasm `msnregole` computes them,
 * so `msngrafo` does not carry the chemistry. A SMILES that does not parse is left to the graph, which reports it.
 * @param {string} query
 */
async function conAttese(query) {
  const q = new URLSearchParams(query);
  if (q.get("regole") !== "1" || !q.get("smiles") || q.get("attese")) return query;
  try { q.set("attese", (await load("msnregole")).attese(q.get("smiles"))); } catch { return query; }
  return q.toString();
}

/** the .mzidx of a file, where the index builder put it in OPFS @param {string} name */
async function indexOf(name) {
  const root = await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab");
  return (await (await root.getDirectoryHandle("idx")).getFileHandle(name + ".mzidx")).getFile();
}

/**
 * The `msngrafo/1` of the file `name` (a JSON string); a failure is thrown as the JSON string `{error_key, params}` of the module.
 * @param {string} name @param {Blob} blob the mzML @param {string} query the query string of the request @param {boolean} esporta the spectra as text
 * @param {(blob: Blob) => (offset: number, length: number) => Uint8Array} readerFor
 * @param {File} [idxFile] the index to read instead of the one of `name` (the derived mzML of a sample of a maker's file, rust-msngrafo-nativo.js)
 */
export async function msnGrafo(name, blob, query, readerFor, esporta = false, idxFile = undefined) {
  const [m, idx] = await Promise.all([load("msngrafo"), idxFile || indexOf(name)]);
  const f = esporta ? m.msn_grafo_export : m.msn_grafo;
  return f(readerFor(idx), idx.size, readerFor(blob), name, await conAttese(query));
}

/**
 * The text of an `msngrafo/1` with the key `vista` added (a graph that has it comes back as it is); a failure is thrown as the JSON string `{error_key, params}`.
 * @param {string} json
 */
export async function msnCompleta(json) {
  return (await load("msngrafo")).completa(json);
}

/**
 * The library text with the entry of `molecola` and the graph of the query added: the JSON string `{libreria, doppione}` of the module.
 * @param {string} name @param {Blob} blob the mzML @param {string} query the parameters of the graph as a query string
 * @param {string} libreria the library file of the user ("" = none) @param {string} molecola
 * @param {(blob: Blob) => (offset: number, length: number) => Uint8Array} readerFor
 * @param {File} [idxFile] the index to read instead of the one of `name` (the derived mzML of a sample of a maker's file, rust-msngrafo-nativo.js)
 */
export async function msnLibAggiungi(name, blob, query, libreria, molecola, readerFor, idxFile = undefined) {
  const [m, idx] = await Promise.all([load("msngrafo"), idxFile || indexOf(name)]);
  return m.msnlib_aggiungi(readerFor(idx), idx.size, readerFor(blob), name, await conAttese(query), libreria, molecola);
}

/**
 * The candidates of the library text for the graph of the query (`msncerca/1`, a JSON string); the query carries the parameters of the graph and the options of the search.
 * @param {string} name @param {Blob} blob the mzML @param {string} query @param {string} libreria the library file of the user
 * @param {(blob: Blob) => (offset: number, length: number) => Uint8Array} readerFor
 * @param {File} [idxFile] the index to read instead of the one of `name` (the derived mzML of a sample of a maker's file, rust-msngrafo-nativo.js)
 */
export async function msnLibCerca(name, blob, query, libreria, readerFor, idxFile = undefined) {
  const [m, idx] = await Promise.all([load("msngrafo"), idxFile || indexOf(name)]);
  return m.msnlib_cerca(readerFor(idx), idx.size, readerFor(blob), name, await conAttese(query), libreria);
}
