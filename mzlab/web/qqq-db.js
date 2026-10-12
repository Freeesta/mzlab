// The IndexedDB of the page ("qqq_lab": the name is kept, renaming it would lose the users' data): its schema and one way to open it.
// Used by rust-worker.js and (with a dynamic import) liste.js.
// SCHEMA is the version of the whole archive (this database and the folders of OPFS together); `kv.schema` holds the one the data are at and migra.js brings it
// up to this value. DB_VER is the version of IndexedDB itself: it follows SCHEMA only when a store is added.
export const DB_NAME = "qqq_lab", DB_VER = 3, SCHEMA = 3;

// stores: the files kept without OPFS, the notebook and the settings (kv), the user's lists of the Liste panel, the manifest of the files of OPFS
// (one row per file: { path, tipo, bytes, creato, usato, formato, versione_formato, crate, sorgente_sha256 })
export function upgrade(db) {
  if (!db.objectStoreNames.contains("files")) db.createObjectStore("files");
  if (!db.objectStoreNames.contains("kv")) db.createObjectStore("kv");
  if (!db.objectStoreNames.contains("liste_utente")) db.createObjectStore("liste_utente", { keyPath: "id" });
  if (!db.objectStoreNames.contains("manifest")) db.createObjectStore("manifest", { keyPath: "path" });
}

// a function that gives one connection for the whole life of the worker; it closes itself when another tab needs to upgrade the database
export function lab(onblocked) {
  let dbP = null;
  return () => dbP || (dbP = new Promise((res, rej) => {
    const r = indexedDB.open(DB_NAME, DB_VER);
    r.onupgradeneeded = () => upgrade(r.result);
    if (onblocked) r.onblocked = onblocked;        // another tab holds an older connection: the upgrade waits until it is closed
    r.onsuccess = () => { const db = r.result; db.onversionchange = () => { db.close(); dbP = null; }; res(db); };
    r.onerror = () => rej(r.error);
  }).catch(e => { dbP = null; throw e; }));
}
