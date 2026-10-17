// @ts-strict
// IndexedDB for mzLab ("mzlab_lab", migrated from legacy "qqq_lab"):
// manages stores (files, kv, liste_utente, manifest), connection lifecycle, and transparent migration.
export const DB_NAME = typeof NOMI !== "undefined" ? NOMI.DB : "mzlab_lab";
export const DB_OLD = typeof NOMI !== "undefined" && NOMI.DB_OLD ? NOMI.DB_OLD : "qqq_lab";
export const DB_VER = 3;
export const SCHEMA = 3;

/**
 * Upgrade schema to ensure all required stores exist.
 * @param {IDBDatabase} db
 */
export function upgrade(db) {
  if (!db.objectStoreNames.contains("files")) db.createObjectStore("files");
  if (!db.objectStoreNames.contains("kv")) db.createObjectStore("kv");
  if (!db.objectStoreNames.contains("liste_utente")) db.createObjectStore("liste_utente", { keyPath: "id" });
  if (!db.objectStoreNames.contains("manifest")) db.createObjectStore("manifest", { keyPath: "path" });
}

export function openOldDatabase(name) {
  return new Promise(resolve => {
    let createdEmpty = false;
    let req;
    try {
      req = indexedDB.open(name);
    } catch (_) {
      return resolve(null);
    }
    req.onupgradeneeded = e => {
      if (e.oldVersion === 0) createdEmpty = true;
    };
    req.onsuccess = () => {
      const db = req.result;
      if (createdEmpty) {
        db.close();
        try { indexedDB.deleteDatabase(name); } catch (_) {}
        resolve(null);
      } else {
        resolve(db);
      }
    };
    req.onerror = () => resolve(null);
    req.onblocked = () => resolve(null);
  });
}

export function countStore(db, storeName) {
  return new Promise(resolve => {
    if (!db.objectStoreNames.contains(storeName)) return resolve(0);
    try {
      const tx = db.transaction(storeName, "readonly");
      const req = tx.objectStore(storeName).count();
      req.onsuccess = () => resolve(req.result || 0);
      req.onerror = () => resolve(0);
    } catch (_) {
      resolve(0);
    }
  });
}

export function copyStore(oldDb, newDb, storeName) {
  return new Promise((resolve, reject) => {
    try {
      const oldTx = oldDb.transaction(storeName, "readonly");
      const oldStore = oldTx.objectStore(storeName);
      const isKeyPath = !!oldStore.keyPath;
      /** @type {any[]} */
      const entries = [];

      const req = oldStore.openCursor();
      req.onsuccess = () => {
        try {
          const cursor = req.result;
          if (cursor) {
            entries.push(isKeyPath ? cursor.value : [cursor.value, cursor.key]);
            cursor.continue();
          } else {
            if (entries.length === 0) return resolve(true);
            const newTx = newDb.transaction(storeName, "readwrite");
            const newStore = newTx.objectStore(storeName);
            for (const item of entries) {
              if (isKeyPath) newStore.put(item);
              else newStore.put(item[0], item[1]);
            }
            newTx.oncomplete = () => resolve(true);
            newTx.onerror = () => reject(newTx.error);
            newTx.onabort = () => reject(newTx.error);
          }
        } catch (err) {
          reject(err);
        }
      };
      req.onerror = () => reject(req.error);
      oldTx.onerror = () => reject(oldTx.error);
      oldTx.onabort = () => reject(oldTx.error);
    } catch (e) {
      reject(e);
    }
  });
}

export function getKv(db, key) {
  return new Promise(resolve => {
    if (!db.objectStoreNames.contains("kv")) return resolve(undefined);
    try {
      const tx = db.transaction("kv", "readonly");
      const req = tx.objectStore("kv").get(key);
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => resolve(undefined);
    } catch (_) {
      resolve(undefined);
    }
  });
}

export function putKv(db, key, val) {
  return new Promise((resolve, reject) => {
    try {
      const tx = db.transaction("kv", "readwrite");
      tx.objectStore("kv").put(val, key);
      tx.oncomplete = () => resolve(true);
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    } catch (e) {
      reject(e);
    }
  });
}

async function doMigration(newDb, oldName) {
  if (typeof indexedDB === "undefined") return;
  try {
    if (typeof indexedDB.databases === "function") {
      const dbs = await indexedDB.databases();
      if (!dbs.some(d => d.name === oldName)) return;
    }
  } catch (_) {}

  const oldDb = await openOldDatabase(oldName);
  if (!oldDb) return;

  try {
    const STORES = ["files", "kv", "liste_utente"];
    const migratedFrom = await getKv(newDb, "migrated_from");
    const cFiles = await countStore(newDb, "files");
    const cListe = await countStore(newDb, "liste_utente");
    const cKv = await countStore(newDb, "kv");
    const schema = await getKv(newDb, "schema");
    const isEmpty = cFiles === 0 && cListe === 0 && (cKv === 0 || (schema !== undefined && cKv <= 1));

    if (!migratedFrom && isEmpty) {
      for (const s of STORES) {
        if (oldDb.objectStoreNames.contains(s) && newDb.objectStoreNames.contains(s)) {
          await copyStore(oldDb, newDb, s);
        }
      }
      await putKv(newDb, "migrated_from", oldName);
      await putKv(newDb, "migration_runs", 1);
      const matchFiles = (await countStore(newDb, "files")) === (await countStore(oldDb, "files"));
      const matchListe = (await countStore(newDb, "liste_utente")) === (await countStore(oldDb, "liste_utente"));
      const matchKv = (await countStore(newDb, "kv")) >= (await countStore(oldDb, "kv"));
      if (matchFiles && matchListe && matchKv) {
        await putKv(newDb, "migration_status", "ok");
      }
    } else if (migratedFrom === oldName) {
      const matchFiles = (await countStore(newDb, "files")) >= (await countStore(oldDb, "files"));
      const matchListe = (await countStore(newDb, "liste_utente")) >= (await countStore(oldDb, "liste_utente"));
      const matchKv = (await countStore(newDb, "kv")) >= (await countStore(oldDb, "kv"));
      if (matchFiles && matchListe && matchKv) {
        const runs = Number((await getKv(newDb, "migration_runs")) || 1) + 1;
        await putKv(newDb, "migration_runs", runs);
        if (runs >= 3) {
          oldDb.close();
          await new Promise(r => {
            const req = indexedDB.deleteDatabase(oldName);
            req.onsuccess = req.onerror = req.onblocked = () => r(true);
          });
          await putKv(newDb, "migration_status", "cleaned");
          return;
        }
      }
    }
  } finally {
    try { oldDb.close(); } catch (_) {}
  }
}

export function migrateIdb(newDb, oldName = DB_OLD) {
  const L = typeof navigator !== "undefined" && navigator.locks;
  return L ? L.request("mzlab-idb-migra", { mode: "exclusive" }, () => doMigration(newDb, oldName)) : doMigration(newDb, oldName);
}

// A function that gives one connection for the whole life of the worker; it closes itself when another tab needs to upgrade the database.
export function lab(onblocked) {
  let dbP = null;
  return () => dbP || (dbP = new Promise((res, rej) => {
    const r = indexedDB.open(DB_NAME, DB_VER);
    r.onupgradeneeded = () => upgrade(r.result);
    if (onblocked) r.onblocked = onblocked;
    r.onsuccess = async () => {
      const db = r.result;
      db.onversionchange = () => { db.close(); dbP = null; };
      try {
        await migrateIdb(db, DB_OLD);
      } catch (e) {
        console.warn("[mzLab] idb migration:", e);
      }
      res(db);
    };
    r.onerror = () => rej(r.error);
  }).catch(e => { dbP = null; throw e; }));
}
