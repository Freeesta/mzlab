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
declare var TPMINE: any;
