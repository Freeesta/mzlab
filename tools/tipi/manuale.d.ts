// Hand-written declarations for tsc (job «tipi»): typed globals that no classic script declares as a top-level name, and worker-only API.
// Everything else (window.X assignments, names of the files that tsc reads as CommonJS, names for the module and worker projects) is generated
// into globali.d.ts and globali-nomi.d.ts by genera_globali.py.
declare var APP_NAME: string;
declare var MZLAB_VERSION: string;
declare var MZLAB_COMMIT: string;
declare var MZLAB_PHONE: boolean;
declare var QQQ_PHONE: boolean;
declare var MZLAB_DESKTOP: boolean;
declare var SITO_PUBBLICO: { readonly scaricaApp: boolean; readonly teoria: boolean };
declare var MZLAB_TOFILE: ((f: { id: number; name: string; size: number }) => File) | undefined;
declare var __TAURI__: any;
declare class FileReaderSync { readAsArrayBuffer(blob: Blob): ArrayBuffer; }
// Browser API not yet in lib.dom (File System Observer) and globals of the TP_Mine scripts, which load lazily after the page.
declare var FileSystemObserver: any;
declare var MAPFEAT: any;
declare var IMPCOMP: any;
// generated index of the Teoria search (teoria/indice-ricerca.js), loaded on demand by ricerca.js
declare var INDICE_RICERCA: string[][];
declare var TPMINE: any;

/** An entry of the loading table (ST.files) as ruoli-*.js reads it: `role`/`sub` are campioni/2 `tipo` and `tipo_bianco`, `time` is in minutes, `gT` = proposed from the name. */
type Voce = { name: string; kind?: string; use: boolean; type: string; role?: string; sub?: string | null; time: number | null; gT?: boolean; conc?: number | null; prop?: number; ed?: number };

/** What a `catch` of the page receives: an Error, an `api/` answer `{error, error_key, params}` or `{json}` of a failed fetch; `message` is typed as present (as on an Error), the other fields are optional and the code checks what it reads. */
type Lanciato = { message: string; name?: string; key?: string; fatal?: boolean; stack?: string; error?: string; error_key?: string; params?: any; json?: any; [k: string]: any };

/** An element found by a selector on `[data-x]` attributes: the `dataset` keys are there because the selector asked for them. */
type ElementoDati = HTMLElement & { dataset: Record<string, string> };
type CampoDati = HTMLInputElement & { dataset: Record<string, string> };

// Shapes shared by libreria-cartella.js, libreria-scarica.js and libreria-worker.js (they are modules for tsc, so their own typedefs would not be global; the page loads them too).
type LibBlob = { size: number, arrayBuffer: () => Promise<ArrayBuffer>, text: () => Promise<string> };
type LibFile = { name: string, size: number, lastModified: number, slice: (a: number, b: number) => LibBlob, text: () => Promise<string> };  // a File, or the wrapper of a native file
type LibNative = LibFile & { readSync: (off: number, len: number) => Uint8Array };
type LibEnt = { name: string, size: number, mtime: number };
type LibInfo = { n: number, pos: number, neg: number, dropped: number, peaks: number, date?: number };
type LibVoce = { nome: string, tipo: string, id: string, size: number, mtime: number, stato: string, info?: any, versione?: string, nuova?: string };
type LibQuery = Record<string, string | number>;
type LibDir = { kind: string, name: string, list(sub?: string): Promise<LibEnt[]>, file(rel: string): Promise<LibFile>, write(rel: string, data: string | Blob | Uint8Array): Promise<void>, remove(rel: string): Promise<void>, replace(tmp: string, rel: string): Promise<void>, reader(file: LibFile): (off: number, len: number) => Uint8Array };  // same interface for handles and for the native folder
type LibPiece = { file: string, dimensione: number, sha256: string };
type LibPkg = { id: string, versione: string, sha256: string, dimensione: number, pezzi: LibPiece[], lettore_min?: number, spettri?: number, picchi?: number, polarita?: number };  // an entry of the catalog
type LibCatalog = { formato: string, librerie: LibPkg[], esterne?: { id: string, url: string }[] };
type LibErr = Error & { key: string, params: Record<string, unknown> };
type LibFetch = (url: string, opts?: RequestInit) => Promise<Response>;
// File System Access API members the bundled libs do not have (libreria-cartella.js reads and writes the folders with them).
interface FileSystemFileHandle { move(name: string): Promise<void>; move(dir: FileSystemDirectoryHandle, name: string): Promise<void>; createSyncAccessHandle(): Promise<FileSystemSyncAccessHandle> }
interface FileSystemDirectoryHandle { entries(): AsyncIterableIterator<[string, FileSystemHandle]> }
interface FileSystemSyncAccessHandle { close(): void; flush(): void; getSize(): number; truncate(size: number): void; read(buffer: ArrayBufferView | ArrayBuffer, options?: { at?: number }): number; write(buffer: ArrayBufferView | ArrayBuffer, options?: { at?: number }): number }
