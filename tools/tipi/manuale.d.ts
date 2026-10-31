// Hand-written declarations for tsc (job «tipi»): typed globals that no classic script declares as a top-level name, and worker-only API.
// Everything else (window.X assignments, names of the files that tsc reads as CommonJS, names for the module and worker projects) is generated
// into globali.d.ts and globali-nomi.d.ts by genera_globali.py.
declare var APP_NAME: string;
declare var MZLAB_VERSION: string;
declare var MZLAB_COMMIT: string;
declare var MZLAB_PHONE: boolean;
declare var QQQ_PHONE: boolean;
declare var MZLAB_DESKTOP: boolean;
declare var MZLAB_LAB: any;
declare var SITO_PUBBLICO: { readonly scaricaApp: boolean };
declare var MZLAB_TOFILE: ((f: { id: number; name: string; size: number }) => File) | undefined;
declare var __TAURI__: any;
declare class FileReaderSync { readAsArrayBuffer(blob: Blob): ArrayBuffer; }
// Browser API not yet in lib.dom (File System Observer) and globals of the TP_Mine scripts, which load lazily after the page.
declare var FileSystemObserver: any;
declare var MAPFEAT: any;
declare var IMPCOMP: any;
declare var TPMINE: any;

/** An entry of the loading table (ST.files) as ruoli-*.js reads it: `role`/`sub` are campioni/2 `tipo` and `tipo_bianco`, `time` is in minutes, `gR`, `gT` = role and time proposed from the name (until the student edits them). */
type Voce = { name: string; kind?: string; use: boolean; type: string; role?: string; sub?: string | null; time: number | null; gR?: boolean; gT?: boolean; conc?: number | null; prop?: number; ed?: number };

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
type LibCatalog = { formato: string, librerie: LibPkg[], esterne?: { id: string, url: string }[], fonti?: { id: string, modo: string }[] };
type LibErr = Error & { key: string, params: Record<string, unknown> };
type LibFetch = (url: string, opts?: RequestInit) => Promise<Response>;
// File System Access API members the bundled libs do not have (libreria-cartella.js reads and writes the folders with them).
interface FileSystemFileHandle { move(name: string): Promise<void>; move(dir: FileSystemDirectoryHandle, name: string): Promise<void>; createSyncAccessHandle(): Promise<FileSystemSyncAccessHandle> }
interface FileSystemDirectoryHandle { entries(): AsyncIterableIterator<[string, FileSystemHandle]> }
interface FileSystemSyncAccessHandle { close(): void; flush(): void; getSize(): number; truncate(size: number): void; read(buffer: ArrayBufferView | ArrayBuffer, options?: { at?: number }): number; write(buffer: ArrayBufferView | ArrayBuffer, options?: { at?: number }): number }
type DrawKet = any;  // Ketcher editor object or one of its parts (vendor bundle, untyped)
type DrawLine = [string | number, string][];  // one line of a drawing label: pieces [text, "sub" | "sup" | "it" | ""]
type DrawStruct = { formula: string, mass: number, n: Record<string, number>, q: number, cx: number, y: number, x0: number, x1: number, y0: number, y1: number };  // a connected structure of the drawing
/** A file of the loading table / of ST.files as the engine reports it (shell.js, explore.js): the fields differ by experiment, so only `name` is fixed. */
type FileRiga = { name: string; [k: string]: any };
/** A file open in the session (E.files): `file` is its name in the session, `gone` marks one taken away. */
type FileAperto = { file: string; label?: string; gone?: boolean; color?: string; [k: string]: any };
/** A file still to be opened one by one (E.pending). */
type Pendiente = { file: string; label: string; state: string; msg?: string };
/** A file as the loading code handles it: the File of the picker or of a converter, plus the records that makers (Rust engine, readers module) attach. */
type FileCarica = File & { mzlabNat?: { members: { name: string; blob: Blob }[]; probed?: { motivi: string[] }; c?: number; [k: string]: any }; mzlabModulo?: { fam: string; members?: { name: string; blob: Blob }[]; c?: number; [k: string]: any } };
/** What one loading collects for the notice after it (shell.js reportLoad). */
type CaricaRep = { unopen: { name: string; ext: string; label: string; st: string }[]; other: string[]; errs: Record<string, string[]>; moved: { dam: string[]; data: string[] }; notes: string[]; diag?: Record<string, any>; [k: string]: any };

/** Loose record from the untyped engine: a panel, a session entry or a JSON reply (explore.js). */
type Bag = Record<string, any>;
/** One colour palette of explore.js PALS. */
type Palette = { seq?: string[]; seqDark?: string[]; a?: number; b?: number; aDark?: number; bDark?: number; cat: string[]; catDark: string[]; blank: string; blankDark: string; std: string; stdDark: string; dash?: boolean; lw?: number; plain?: boolean; hc?: boolean; hue?: (u: number) => string };
/** A (real or synthetic) event that opens the context menu of explore.js menu(). */
type CtxEv = { preventDefault: () => void, clientX: number, clientY: number, kbd?: boolean, lp?: boolean };

/** A target of the HR screening list (liste-target-core.js): a mass the list does not have is NaN/missing at run time and the core falls back from the adduct table to the monoisotopic mass. */
type TgtEntry = { id: string; name_en: string; neutral_formula: string; mz?: Record<string, number>; mz_mh: number; mz_mna: number; mz_mnh4: number; monoisotopic_mass: number; [k: string]: any };
/** A spectrum as the target core reads it: `intensity` or `y`, `precursor` or `prec` depending on the maker. */
type TgtSpec = { rt: number; level?: number; mz?: number[]; intensity?: number[]; y?: number[]; precursor?: number | null; prec?: number | null; [k: string]: any };
/** One chromatographic peak found by the target core. */
type TgtPeak = { apex_idx: number; rt_apex: number; rt_start: number; rt_end: number; height: number; area: number; sn_ratio: number; right: number };
/** The options of a target search, with the defaults applied. */
type TgtOpts = { tolerance_ppm: number; min_sn: number; min_area: number; adducts: string[] };
/** One hit of a target search (evaluatePeak). */
type TgtMatch = { id: string; name_en: string; neutral_formula: string; adduct: string; target_mz: number; observed_mz: number; ppm_error: number; rt_apex: number; rt_start: number; rt_end: number; height: number; area: number; sn_ratio: number; isotope_score: number | null; ms2_matched: boolean; proposal: string; motif: string };

/** One point of a gradient program (gradiente.js): time in min, %B and the optional flow in mL/min. */
type GradPt = { t: number; b: number; flow?: number | null };
/** A gradient model as the engine and the notebook keep it (api/gradient). */
type GradModel = { source: string; points: GradPt[]; run_time?: number; nonlinear?: boolean; [k: string]: any };
/** A row of the gradient table being edited: the cells are still text (or the numbers of the loaded model). */
type GradRow = Record<string, number | string | null>;

/** A row of the files being read, converted and indexed (CONVROWS in conversione.js). */
type ConvRow = { name: string; state: string; pc: number | null; floor?: number; mono?: boolean; det?: { done: number; total: number } | null; sub?: { done: number; total: number } | null; t0?: number; seen?: number; tr?: string[]; last?: string; scans?: number; kind?: string; missing?: number; cached?: boolean; secs?: number; msg?: string; diag?: any; mem?: string };
