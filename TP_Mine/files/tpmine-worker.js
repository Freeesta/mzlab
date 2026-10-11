// TPMINE-PRIVATE  Web Worker of TP Mine: its own Pyodide (Python + numpy) running the private engine on the mzML bytes sent by the page.
let py = null, api = null, bigN = 0;
const say = text => postMessage({ type: "step", text });
onmessage = async ev => {
  const m = ev.data;
  try {
    if (m.type === "init") {
      say("Carico Python...");
      const { loadPyodide } = await import(m.pyodideMjs);
      py = await loadPyodide({ indexURL: m.indexURL });
      say("Carico numpy...");
      await py.loadPackage("numpy");
      say("Preparo il motore...");
      py.FS.mkdirTree("/qqq"); py.unpackArchive(m.qqqZip, "zip", { extractDir: "/qqq" });
      py.FS.mkdirTree("/tpm"); py.unpackArchive(m.tpZip, "zip", { extractDir: "/tpm" });
      py.FS.mkdirTree("/tp_data");
      py.runPython("import sys\nsys.path[:0] = ['/qqq', '/tpm']\nimport tpmine.api");
      api = py.pyimport("tpmine.api");
      api._progress = (text, frac, phase, payload) => postMessage({ type: "progress", text, frac, phase, payload: payload ? JSON.parse(payload) : null });
      postMessage({ type: "ready" });
    } else if (m.type === "put") {
      const dst = "/tp_data/" + m.name;
      try { py.FS.unlink(dst); } catch (_) { /* not there yet */ }
      if (m.blob) {      // a big file is not copied into the heap of WebAssembly: the Blob is mounted read-only (WORKERFS) and linked, like the main program does
        const dir = "/big/" + (++bigN);
        py.FS.mkdirTree(dir);
        py.FS.mount(py.FS.filesystems.WORKERFS, { blobs: [{ name: "data", data: m.blob }] }, dir);
        py.FS.symlink(dir + "/data", dst);
      } else py.FS.writeFile(dst, new Uint8Array(m.buf));
      postMessage({ id: m.id, result: true });
    } else if (m.type === "mem") {          // size of the WebAssembly memory: it only grows, so at the end it is the peak of the run (used by the tests)
      postMessage({ id: m.id, result: { wasm: py._module.HEAPU8.length } });
    } else if (m.type === "call") {
      const r = api[m.fn](...(m.args || []));
      postMessage({ id: m.id, result: typeof r === "string" ? r : String(r) });
    }
  } catch (e) {
    postMessage({ id: m.id, error: String(e && e.message || e).split("\n").filter(Boolean).slice(-3).join(" | "), fatal: m.type === "init" });
  }
};
