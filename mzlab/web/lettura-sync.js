// @ts-check
// Reads a source file of the index builder (a plain mzML, a member of a maker's set) straight from its copy in OPFS with a read-only SyncAccessHandle, in blocks of any size:
// no File object, no copy through the browser process. Without a handle (not supported, the file is taken, the size is not the Blob's) every read goes through
// the FileReaderSync of the Blob, as before. Used by idx-worker.js (a worker: handles exist only there).

/** @typedef {{ read: (offset: number, length: number) => Uint8Array, close: () => void, hand: boolean }} Fonte */

/**
 * @param {string} kind folder of the copy under `mzlab/` ("files")
 * @param {string} name name of the copy
 * @param {Blob} blob the same file as a Blob: the fallback, and the size the handle must have
 * @returns {Promise<Fonte>}
 */
export async function openSource(kind, name, blob) {
  const fr = new FileReaderSync();
  const viaBlob = (/** @type {number} */ o, /** @type {number} */ n) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(o, o + n)));
  /** @type {any} */ let ah = null;
  try {
    const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle(kind);
    const fh = /** @type {any} */ (await d.getFileHandle(name));
    // read-only handles share the file with the other readers; a browser that ignores the mode gives an exclusive one, which is closed with the build
    ah = await fh.createSyncAccessHandle({ mode: "read-only" });
    if (ah.getSize() !== blob.size) { ah.close(); ah = null; }
  } catch (_) { ah = null; }
  if (!ah) return { read: viaBlob, close() {}, hand: false };
  const h = ah;
  let buf = new Uint8Array(1 << 20);
  return {
    hand: true,
    read(o, n) {
      if (buf.length < n) buf = new Uint8Array(n);
      const out = buf.subarray(0, n);
      return h.read(out, { at: o }) === n ? out : viaBlob(o, n);
    },
    close() { try { h.close(); } catch (_) { /* already closed */ } },
  };
}
