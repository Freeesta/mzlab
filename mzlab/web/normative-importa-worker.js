// @ts-check
// Worker of the import of an official regulatory file (docs/agenti/normative.md): loads the .wasm «normative» of wasm-feat/manifest.json (SHA-256 checked), runs `import_official`
// on the CSV and the CAS sidecar the page hands over, writes the private .mzsus and its lists to OPFS (`normative-imported/`) and answers the summary. Started by normative-importa.js on a click, never at start-up.

const hex = (/** @type {ArrayBuffer} */ b) => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
/** @type {any} */ let normative = null;

async function load(/** @type {string} */ base) {
  if (normative) return normative;
  const m = (await (await fetch(base + "manifest.json")).json()).normative;
  const bytes = await (await fetch(base + m.wasm)).arrayBuffer();
  if (hex(await crypto.subtle.digest("SHA-256", bytes)) !== m.sha256) throw Object.assign(new Error("error.featWasmHash"), { key: "error.featWasmHash" });
  const x = await import(base + m.js);
  await x.default({ module_or_path: bytes });
  return (normative = x);
}

async function put(/** @type {string} */ name, /** @type {Uint8Array | string} */ body) {
  const d = await (await navigator.storage.getDirectory()).getDirectoryHandle("normative-imported", { create: true });
  const h = await (await d.getFileHandle(name, { create: true })).createSyncAccessHandle();
  try { const u = typeof body === "string" ? new TextEncoder().encode(body) : body; h.truncate(0); h.write(u, { at: 0 }); h.flush(); } finally { h.close(); }
}

self.onmessage = async (/** @type {MessageEvent<any>} */ ev) => {
  const { id, base, reader, csv, sidecar, label } = ev.data;
  try {
    const x = await load(base);
    const b = x.import_official(reader, new Uint8Array(csv), new TextDecoder().decode(sidecar), label);
    const n = new DataView(b.buffer, b.byteOffset, 4).getUint32(0, true);
    const summary = new TextDecoder().decode(b.subarray(4, 4 + n)), raw = b.slice(4 + n);
    const id_ = JSON.parse(summary).id;
    await put(`${id_}.mzsus`, raw);
    await put(`${id_}.json`, x.lists_of(raw));
    self.postMessage({ id, ok: true, summary });
  } catch (e_) {
    const e = /** @type {any} */ (e_);
    self.postMessage({ id, ok: false, error: typeof e === "string" ? e : String(e && (e.key || e.message) || e) });
  }
};
