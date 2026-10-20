// Runs a WASI module (wasm32-wasip1) with two folders: node tools/wasi_run.mjs MODULE.wasm IN_DIR OUT_DIR [ARG...]
// The module sees them as /in and /out and gets `flow /in/ARG... /out`-style arguments from the caller; its exit code is ours.
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

const [module, inDir, outDir, ...args] = process.argv.slice(2);
const wasi = new WASI({
  version: "preview1",
  args: ["flow", ...args],
  preopens: { "/in": inDir, "/out": outDir },
  returnOnExit: true,
});
const { instance } = await WebAssembly.instantiate(await readFile(module), wasi.getImportObject());
process.exitCode = wasi.start(instance);
