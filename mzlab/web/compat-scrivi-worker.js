// @ts-check
// Writer of compat.js (the createWritable fallback for WebKit before Safari 26): writes one Blob to a file of the private storage (OPFS) with a
// synchronous handle, which WebKit gives only in a worker. Message {id, path: [dir, …, name], blob} -> {id, ok} or {id, ok: false, name, message}.
// The handle methods are awaited: WebKit before 16.4 answers them with promises.
self.onmessage = async (/** @type {MessageEvent} */ e) => {
  const { id, path, blob } = e.data;
  try {
    let d = await navigator.storage.getDirectory();
    for (const s of path.slice(0, -1)) d = await d.getDirectoryHandle(s);
    const fh = await d.getFileHandle(path[path.length - 1], { create: true });
    const a = await /** @type {any} */ (fh).createSyncAccessHandle();
    try {
      await a.truncate(0);
      const CH = 8 << 20;
      for (let o = 0; o < blob.size; o += CH) await a.write(new Uint8Array(await blob.slice(o, o + CH).arrayBuffer()), { at: o });
      await a.flush();
    } finally { await a.close(); }
    postMessage({ id, ok: true });
  } catch (err_) {
    const err = /** @type {any} */ (err_);
    postMessage({ id, ok: false, name: (err && err.name) || "UnknownError", message: String((err && err.message) || err) });
  }
};
