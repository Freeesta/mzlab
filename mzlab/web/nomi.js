// @ts-strict
// Central naming module for browser storage, caches and database names.
// Preserves compatibility with existing user data while providing a unified API.
const NOMI = (function () {
  "use strict";

  const DB = "qqq_lab";
  const DB_VERSION = 2;
  const CACHE_PREFIX = "qqq-";
  const CACHE_APP = "qqq-app-";
  const CACHE_BIG = "qqq-big-";

  /**
   * Return canonical storage key for a given identifier.
   * @param {string} k
   * @returns {string}
   */
  function KEY(k) {
    if (k === "theme") return "qqq-theme";
    if (k === "labels") return "qqq-labels";
    if (typeof k === "string" && k.startsWith("qqq")) return k;
    return "qqq." + k;
  }

  const res = {
    DB,
    DB_NAME: DB,
    DB_VERSION,
    DB_VER: 3,
    CACHE_PREFIX,
    CACHE_APP,
    CACHE_BIG,
    KEY,
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
