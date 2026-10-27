// Globals that libreria-worker.js gets from importScripts("libreria-cartella.js", "libreria-scarica.js").
declare var CARTELLA: any;
declare var SCARICA: any;
declare var NOMI: any;

// Web APIs the bundled WebWorker lib does not have yet.
interface WorkerNavigator { maxTouchPoints: number }
interface StorageManager { persist(): Promise<boolean> }

// libreria-worker.js: peaks, indexes and results. A message of the page and a result of the wasm engine (parsed JSON) have no fixed shape here, hence the index signatures.
type LibSpec = { mz: Float32Array, it: Float32Array };
type LibQ = LibSpec & { prec: number, pol: number };
type LibIdx = { n: number, prec: Float64Array, ord: Uint32Array, pol: Int8Array, off: Uint32Array, totalPeaks?: number };
type LibSpecOut = { mz: number[], it: number[] };
type LibParsed = { prec: number, pol: number, mz: Float32Array, it: Float32Array, meta: string[] };
type LibHit = { [k: string]: any };
type LibMsg = { [k: string]: any };
type LibLoaded = { info: LibMsg, idx: LibIdx, meta: string[][], file: File | null, peaks: ArrayBuffer | null, pall?: ArrayBuffer };
type LibSync = { read(off: number, len: number): Uint8Array, write(off: number, data: Uint8Array): void, size(): number, close(remove?: boolean): Promise<void> };
type LibMem = LibSync & { blob(): Blob };
