// Globals that libreria-worker.js gets from importScripts("libreria-cartella.js", "libreria-scarica.js").
declare var CARTELLA: any;
declare var SCARICA: any;
declare var NOMI: any;

// Web APIs the bundled WebWorker lib does not have yet.
interface WorkerNavigator { maxTouchPoints: number }
interface StorageManager { persist(): Promise<boolean> }
interface FileSystemFileHandle { move(name: string): Promise<void> }
interface FileSystemDirectoryHandle { entries(): AsyncIterableIterator<[string, FileSystemHandle]> }
