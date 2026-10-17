// @ts-strict
// Central naming module for browser storage, caches and database names.
// Handles transparent migration from legacy qqq keys/databases to mzlab.
const NOMI = (function () {
  "use strict";

  const DB = "mzlab_lab";
  const DB_OLD = "qqq_lab";
  const DB_VERSION = 2;
  const DB_VER = 3;
  const CACHE_PREFIX = "mzlab-";
  const CACHE_APP = "mzlab-app-";
  const CACHE_BIG = "mzlab-big-";

  /**
   * Return canonical storage key for a given identifier.
   * @param {string} k
   * @returns {string}
   */
  function KEY(k) {
    if (k === "theme" || k === "qqq-theme") return "mzlab-theme";
    if (k === "labels" || k === "qqq-labels") return "mzlab-labels";
    if (typeof k === "string") {
      if (k.startsWith("mzlab")) return k;
      if (k.startsWith("qqq.")) return "mzlab." + k.slice(4);
    }
    return "mzlab." + k;
  }

  /**
   * Return legacy storage key for fallback.
   * @param {string} k
   * @returns {string}
   */
  function OLD_KEY(k) {
    if (k === "theme" || k === "mzlab-theme" || k === "qqq-theme") return "qqq-theme";
    if (k === "labels" || k === "mzlab-labels" || k === "qqq-labels") return "qqq-labels";
    if (typeof k === "string") {
      if (k.startsWith("mzlab.")) return "qqq." + k.slice(6);
      if (k.startsWith("qqq.")) return k;
    }
    return "qqq." + k;
  }

  function resolveStore(s, kind) {
    if (s) return s;
    if (typeof window !== "undefined") {
      try { return (kind === "session" ? window.sessionStorage : window.localStorage) || null; } catch (_) { return null; }
    }
    return null;
  }

  /**
   * Get item trying new key first, then fallback to old key.
   * @param {string} k
   * @param {Storage} [storage]
   * @returns {string | null}
   */
  function get(k, storage) {
    try {
      const s = resolveStore(storage, "local");
      if (!s) return null;
      const v = s.getItem(KEY(k));
      if (v !== null) return v;
      return s.getItem(OLD_KEY(k));
    } catch (_) {
      return null;
    }
  }

  /**
   * Set item using new key.
   * @param {string} k
   * @param {string} v
   * @param {Storage} [storage]
   * @returns {boolean}
   */
  function set(k, v, storage) {
    try {
      const s = resolveStore(storage, "local");
      if (!s) return false;
      s.setItem(KEY(k), v);
      return true;
    } catch (_) {
      return false;
    }
  }

  /**
   * Synchronous storage migration from legacy qqq keys to mzlab keys.
   * @param {Storage} [ls]
   * @param {Storage} [ss]
   */
  function migrateStorage(ls, ss) {
    try {
      const lStore = resolveStore(ls, "local");
      if (lStore) {
        migrateLs(lStore);
      }
      const sStore = resolveStore(ss, "session");
      if (sStore) {
        migrateSs(sStore);
      }
    } catch (_) {
      // Storage unavailable or quota exceeded; ignore safely
    }
  }

  function migrateLs(ls) {
    const migrato = ls.getItem("mzlab.migrato");
    if (migrato !== "1") {
      const toCopy = [];
      for (let i = 0; i < ls.length; i++) {
        const k = ls.key(i);
        if (k && (k.startsWith("qqq.") || k === "qqq-theme" || k === "qqq-labels")) {
          toCopy.push(k);
        }
      }
      for (const oldK of toCopy) {
        const val = ls.getItem(oldK);
        let newK;
        if (oldK === "qqq-theme") newK = "mzlab-theme";
        else if (oldK === "qqq-labels") newK = "mzlab-labels";
        else newK = "mzlab." + oldK.slice(4);
        if (val !== null && ls.getItem(newK) === null) {
          ls.setItem(newK, val);
        }
      }
      ls.setItem("mzlab.migrato", "1");
      ls.setItem("mzlab.avvii", "1");
    } else {
      let avvii = parseInt(ls.getItem("mzlab.avvii") || "1", 10);
      if (isNaN(avvii)) avvii = 1;
      avvii++;
      ls.setItem("mzlab.avvii", String(avvii));
      if (avvii >= 3) {
        ls.setItem("mzlab.purged", "1");
        const toRemove = [];
        for (let i = 0; i < ls.length; i++) {
          const k = ls.key(i);
          if (k && (k.startsWith("qqq.") || k === "qqq-theme" || k === "qqq-labels")) {
            toRemove.push(k);
          }
        }
        for (const k of toRemove) {
          ls.removeItem(k);
        }
      }
    }
  }

  function migrateSs(ss) {
    const toCopy = [];
    for (let i = 0; i < ss.length; i++) {
      const k = ss.key(i);
      if (k && (k.startsWith("qqq.") || k === "qqq-theme" || k === "qqq-labels")) {
        toCopy.push(k);
      }
    }
    for (const oldK of toCopy) {
      const val = ss.getItem(oldK);
      let newK;
      if (oldK === "qqq-theme") newK = "mzlab-theme";
      else if (oldK === "qqq-labels") newK = "mzlab-labels";
      else newK = "mzlab." + oldK.slice(4);
      if (val !== null && ss.getItem(newK) === null) {
        ss.setItem(newK, val);
      }
    }
  }

  function hookSetItem(proto) {
    const orig = proto.setItem;
    proto.setItem = function (k, v) {
      orig.call(this, k, v);
      try {
        if (typeof k !== "string" || this.getItem("mzlab.purged") === "1") return;
        const avvii = parseInt(this.getItem("mzlab.avvii") || "1", 10);
        if (avvii >= 3) return;
        if (k.startsWith("mzlab.")) orig.call(this, "qqq." + k.slice(6), v);
        else if (k === "mzlab-theme") orig.call(this, "qqq-theme", v);
        else if (k === "mzlab-labels") orig.call(this, "qqq-labels", v);
        else if (k.startsWith("qqq.")) orig.call(this, "mzlab." + k.slice(4), v);
        else if (k === "qqq-theme") orig.call(this, "mzlab-theme", v);
        else if (k === "qqq-labels") orig.call(this, "mzlab-labels", v);
      } catch (_) {}
    };
  }

  function hookGetItem(proto) {
    const orig = proto.getItem;
    proto.getItem = function (k) {
      const val = orig.call(this, k);
      if (val !== null || typeof k !== "string") return val;
      try {
        if (k.startsWith("qqq.")) return orig.call(this, "mzlab." + k.slice(4));
        if (k === "qqq-theme") return orig.call(this, "mzlab-theme");
        if (k === "qqq-labels") return orig.call(this, "mzlab-labels");
        if (k.startsWith("mzlab.")) return orig.call(this, "qqq." + k.slice(6));
        if (k === "mzlab-theme") return orig.call(this, "qqq-theme");
        if (k === "mzlab-labels") return orig.call(this, "qqq-labels");
      } catch (_) {}
      return null;
    };
  }

  function hookRemoveItem(proto) {
    const orig = proto.removeItem;
    proto.removeItem = function (k) {
      orig.call(this, k);
      try {
        if (typeof k !== "string" || this.getItem("mzlab.purged") === "1") return;
        if (k.startsWith("qqq.")) orig.call(this, "mzlab." + k.slice(4));
        else if (k === "qqq-theme") orig.call(this, "mzlab-theme");
        else if (k === "qqq-labels") orig.call(this, "mzlab-labels");
        else if (k.startsWith("mzlab.")) orig.call(this, "qqq." + k.slice(6));
        else if (k === "mzlab-theme") orig.call(this, "qqq-theme");
        else if (k === "mzlab-labels") orig.call(this, "qqq-labels");
      } catch (_) {}
    };
  }

  function installStorageHooks() {
    if (typeof window === "undefined" || typeof Storage === "undefined" || !Storage.prototype) return;
    try {
      hookSetItem(Storage.prototype);
      hookGetItem(Storage.prototype);
      hookRemoveItem(Storage.prototype);
    } catch (_) {}
  }

  installStorageHooks();

  // Synchronous migration on load
  migrateStorage();

  const res = {
    DB,
    DB_NAME: DB,
    DB_OLD,
    DB_VERSION,
    DB_VER,
    CACHE_PREFIX,
    CACHE_APP,
    CACHE_BIG,
    KEY,
    OLD_KEY,
    get,
    set,
    migrateStorage,
  };

  if (typeof window !== "undefined") {
    /** @type {any} */ (window).NOMI = res;
  }
  if (typeof self !== "undefined") {
    /** @type {any} */ (self).NOMI = res;
  }
  if (typeof module !== "undefined" && module.exports) {
    module.exports = res;
  }
  return res;
})();
