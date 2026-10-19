// Hand-written declarations for tsc (job «tipi»): typed globals that no classic script declares as a top-level name, and worker-only API.
// Everything else (window.X assignments, names of the files that tsc reads as CommonJS, names for the module and worker projects) is generated
// into globali.d.ts and globali-nomi.d.ts by genera_globali.py.
declare var APP_NAME: string;
declare var MZLAB_VERSION: string;
declare var MZLAB_COMMIT: string;
declare var MZLAB_PHONE: boolean;
declare var QQQ_PHONE: boolean;
declare var MZLAB_DESKTOP: boolean;
declare var __TAURI__: any;
declare class FileReaderSync { readAsArrayBuffer(blob: Blob): ArrayBuffer; }
// Browser API not yet in lib.dom (File System Observer) and globals of the TP_Mine scripts, which load lazily after the page.
declare var FileSystemObserver: any;
declare var MAPFEAT: any;
// generated index of the Teoria search (teoria/indice-ricerca.js), loaded on demand by ricerca.js
declare var INDICE_RICERCA: string[][];
declare var TPMINE: any;

/** An entry of the loading table (ST.files) as ruoli-*.js reads it: `role`/`sub` are campioni/2 `tipo` and `tipo_bianco`, `time` is in minutes, `gT` = proposed from the name. */
type Voce = { name: string; kind?: string; use: boolean; type: string; role?: string; sub?: string | null; time: number | null; gT?: boolean; conc?: number | null; prop?: number; ed?: number };
