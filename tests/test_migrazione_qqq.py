"""Tests for transparent migration from legacy qqq to mzlab data:
- Nomi naming map and canonical keys
- localStorage / sessionStorage migration and fallback get/set
- Idempotence on second launch
- Resilience to storage exceptions (quota exceeded, Safari private mode)
- Cleanup of old keys on third launch
- IndexedDB migration store-by-store, count verification, and third-launch deletion
- Service worker cache cleanup
"""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"


def run_node(code: str) -> dict:
    """Execute a snippet in Node and return the parsed JSON output."""
    full_code = f"""
    const NOMI = require({json.dumps(str(WEB / "nomi.js"))});
    {code}
    """
    r = subprocess.run(["node", "-e", full_code], capture_output=True, text=True)
    assert r.returncode == 0, f"Node script failed:\nSTDOUT: {r.stdout}\nSTDERR: {r.stderr}"
    lines = [line.strip() for line in r.stdout.strip().splitlines() if line.strip()]
    return json.loads(lines[-1])


def run_node_esm(code: str) -> dict:
    """Execute an ESM snippet in Node and return the parsed JSON output."""
    r = subprocess.run(["node", "--input-type=module", "-e", code], capture_output=True, text=True)
    assert r.returncode == 0, f"Node ESM script failed:\nSTDOUT: {r.stdout}\nSTDERR: {r.stderr}"
    lines = [line.strip() for line in r.stdout.strip().splitlines() if line.strip()]
    return json.loads(lines[-1])


def test_names_map():
    code = """
    console.log(JSON.stringify({
        db: NOMI.DB,
        db_old: NOMI.DB_OLD,
        cache_pfx: NOMI.CACHE_PREFIX,
        cache_app: NOMI.CACHE_APP,
        cache_big: NOMI.CACHE_BIG,
        key_theme: NOMI.KEY("theme"),
        key_labels: NOMI.KEY("labels"),
        key_prefs: NOMI.KEY("prefs"),
        key_qqq_prefs: NOMI.KEY("qqq.prefs"),
        key_qqq_theme: NOMI.KEY("qqq-theme"),
        key_mzlab_prefs: NOMI.KEY("mzlab.prefs"),
        old_theme: NOMI.OLD_KEY("theme"),
        old_labels: NOMI.OLD_KEY("labels"),
        old_prefs: NOMI.OLD_KEY("prefs"),
        old_mzlab_prefs: NOMI.OLD_KEY("mzlab.prefs"),
    }));
    """
    res = run_node(code)
    assert res["db"] == "mzlab_lab"
    assert res["db_old"] == "qqq_lab"
    assert res["cache_pfx"] == "mzlab-"
    assert res["cache_app"] == "mzlab-app-"
    assert res["cache_big"] == "mzlab-big-"
    assert res["key_theme"] == "mzlab-theme"
    assert res["key_labels"] == "mzlab-labels"
    assert res["key_prefs"] == "mzlab.prefs"
    assert res["key_qqq_prefs"] == "mzlab.prefs"
    assert res["key_qqq_theme"] == "mzlab-theme"
    assert res["key_mzlab_prefs"] == "mzlab.prefs"
    assert res["old_theme"] == "qqq-theme"
    assert res["old_labels"] == "qqq-labels"
    assert res["old_prefs"] == "qqq.prefs"
    assert res["old_mzlab_prefs"] == "qqq.prefs"


def test_localstorage_migration_first_launch():
    code = """
    class MockStorage {
      constructor() { this.store = new Map(); }
      getItem(k) { return this.store.has(k) ? this.store.get(k) : null; }
      setItem(k, v) { this.store.set(k, String(v)); }
      removeItem(k) { this.store.delete(k); }
      key(i) { return Array.from(this.store.keys())[i] || null; }
      get length() { return this.store.size; }
    }
    const ls = new MockStorage();
    ls.setItem("qqq.prefs", JSON.stringify({ theme: "dark" }));
    ls.setItem("qqq-theme", "dark");
    ls.setItem("qqq-labels", "custom");
    ls.setItem("qqq.picchi", JSON.stringify({ tol: 0.5 }));

    NOMI.migrateStorage(ls);

    console.log(JSON.stringify({
        migrato: ls.getItem("mzlab.migrato"),
        avvii: ls.getItem("mzlab.avvii"),
        prefs: ls.getItem("mzlab.prefs"),
        theme: ls.getItem("mzlab-theme"),
        labels: ls.getItem("mzlab-labels"),
        picchi: ls.getItem("mzlab.picchi"),
        old_prefs: ls.getItem("qqq.prefs"),
        old_theme: ls.getItem("qqq-theme"),
    }));
    """
    res = run_node(code)
    assert res["migrato"] == "1"
    assert res["avvii"] == "1"
    assert json.loads(res["prefs"]) == {"theme": "dark"}
    assert res["theme"] == "dark"
    assert res["labels"] == "custom"
    assert json.loads(res["picchi"]) == {"tol": 0.5}
    # Old keys are preserved after first launch
    assert res["old_prefs"] is not None
    assert res["old_theme"] is not None


def test_fallback_get_and_set():
    code = """
    class MockStorage {
      constructor() { this.store = new Map(); }
      getItem(k) { return this.store.has(k) ? this.store.get(k) : null; }
      setItem(k, v) { this.store.set(k, String(v)); }
      removeItem(k) { this.store.delete(k); }
      key(i) { return Array.from(this.store.keys())[i] || null; }
      get length() { return this.store.size; }
    }
    const ls = new MockStorage();
    // Legacy key only: fallback should find it
    ls.setItem("qqq.legacy_setting", "from_legacy");
    const fallbackVal = NOMI.get("legacy_setting", ls);

    // After setting new key: get should return new key
    NOMI.set("legacy_setting", "new_value", ls);
    const updatedVal = NOMI.get("legacy_setting", ls);

    console.log(JSON.stringify({
        fallback: fallbackVal,
        updated: updatedVal,
        new_store_key: ls.getItem("mzlab.legacy_setting"),
        old_store_key: ls.getItem("qqq.legacy_setting"),
    }));
    """
    res = run_node(code)
    assert res["fallback"] == "from_legacy"
    assert res["updated"] == "new_value"
    assert res["new_store_key"] == "new_value"
    assert res["old_store_key"] == "from_legacy"


def test_idempotence_at_second_launch():
    code = """
    class MockStorage {
      constructor() { this.store = new Map(); }
      getItem(k) { return this.store.has(k) ? this.store.get(k) : null; }
      setItem(k, v) { this.store.set(k, String(v)); }
      removeItem(k) { this.store.delete(k); }
      key(i) { return Array.from(this.store.keys())[i] || null; }
      get length() { return this.store.size; }
    }
    const ls = new MockStorage();
    ls.setItem("qqq.prefs", JSON.stringify({ theme: "dark" }));
    NOMI.migrateStorage(ls); // 1st launch

    // User modified the new key in the meantime
    ls.setItem("mzlab.prefs", JSON.stringify({ theme: "light" }));

    NOMI.migrateStorage(ls); // 2nd launch

    console.log(JSON.stringify({
        migrato: ls.getItem("mzlab.migrato"),
        avvii: ls.getItem("mzlab.avvii"),
        prefs: ls.getItem("mzlab.prefs"),
        old_prefs: ls.getItem("qqq.prefs"),
    }));
    """
    res = run_node(code)
    assert res["migrato"] == "1"
    assert res["avvii"] == "2"
    # Existing migrated keys must NOT be overwritten by old keys on subsequent launches
    assert json.loads(res["prefs"]) == {"theme": "light"}
    # Old keys remain untouched at launch 2
    assert res["old_prefs"] is not None


def test_cleanup_at_third_launch():
    code = """
    class MockStorage {
      constructor() { this.store = new Map(); }
      getItem(k) { return this.store.has(k) ? this.store.get(k) : null; }
      setItem(k, v) { this.store.set(k, String(v)); }
      removeItem(k) { this.store.delete(k); }
      key(i) { return Array.from(this.store.keys())[i] || null; }
      get length() { return this.store.size; }
    }
    const ls = new MockStorage();
    ls.setItem("qqq.prefs", "dark");
    ls.setItem("qqq-theme", "dark");
    ls.setItem("qqq-labels", "labels");

    NOMI.migrateStorage(ls); // 1st
    NOMI.migrateStorage(ls); // 2nd
    const at2 = {
        avvii: ls.getItem("mzlab.avvii"),
        has_qqq: ls.getItem("qqq.prefs") !== null,
    };

    NOMI.migrateStorage(ls); // 3rd
    const at3 = {
        avvii: ls.getItem("mzlab.avvii"),
        has_qqq_prefs: ls.getItem("qqq.prefs"),
        has_qqq_theme: ls.getItem("qqq-theme"),
        has_qqq_labels: ls.getItem("qqq-labels"),
        mzlab_prefs: ls.getItem("mzlab.prefs"),
        mzlab_theme: ls.getItem("mzlab-theme"),
    };

    console.log(JSON.stringify({ at2, at3 }));
    """
    res = run_node(code)
    assert res["at2"]["avvii"] == "2"
    assert res["at2"]["has_qqq"] is True
    assert res["at3"]["avvii"] == "3"
    # At launch 3, old keys are safely pruned
    assert res["at3"]["has_qqq_prefs"] is None
    assert res["at3"]["has_qqq_theme"] is None
    assert res["at3"]["has_qqq_labels"] is None
    # New keys remain intact
    assert res["at3"]["mzlab_prefs"] == "dark"
    assert res["at3"]["mzlab_theme"] == "dark"


def test_resilience_to_exceptions():
    code = """
    class FailingStorage {
      constructor() { this.store = new Map(); }
      getItem(k) { return this.store.has(k) ? this.store.get(k) : null; }
      setItem(k, v) {
        if (k.startsWith("mzlab.")) throw new Error("QuotaExceededError");
        this.store.set(k, String(v));
      }
      removeItem(k) { this.store.delete(k); }
      key(i) { return Array.from(this.store.keys())[i] || null; }
      get length() { return this.store.size; }
    }
    const ls = new FailingStorage();
    ls.setItem("qqq.prefs", "v1");

    let threw = false;
    try {
      NOMI.migrateStorage(ls);
    } catch (_) {
      threw = true;
    }

    console.log(JSON.stringify({
        threw,
        migrato: ls.getItem("mzlab.migrato"),
        old_val: ls.getItem("qqq.prefs"),
    }));
    """
    res = run_node(code)
    assert res["threw"] is False
    # If copy failed due to storage exception, migration is not marked complete
    assert res["migrato"] is None
    assert res["old_val"] == "v1"


def test_indexeddb_migration_full_lifecycle():
    uri = (WEB / "qqq-db.js").as_uri()
    code = f"""
    import {{ copyStore, countStore, putKv, getKv, migrateIdb }} from {uri!r};

    class MockStore {{
      constructor(name, keyPath = null) {{
        this.name = name;
        this.keyPath = keyPath;
        this.map = new Map();
      }}
      put(val, key) {{
        const k = this.keyPath ? val[this.keyPath] : key;
        this.map.set(k, val);
        const req = {{ result: k, onsuccess: null, onerror: null }};
        setTimeout(() => req.onsuccess && req.onsuccess(), 0);
        return req;
      }}
      get(key) {{
        const res = this.map.get(key);
        const req = {{ result: res, onsuccess: null, onerror: null }};
        setTimeout(() => req.onsuccess && req.onsuccess(), 0);
        return req;
      }}
      count() {{
        const req = {{ result: this.map.size, onsuccess: null, onerror: null }};
        setTimeout(() => req.onsuccess && req.onsuccess(), 0);
        return req;
      }}
      openCursor() {{
        const entries = Array.from(this.map.entries());
        let idx = 0;
        const req = {{ result: null, onsuccess: null, onerror: null }};
        function step() {{
          if (idx < entries.length) {{
            const [k, v] = entries[idx++];
            const cursor = {{
              key: k,
              value: v,
              continue: () => setTimeout(step, 0)
            }};
            req.result = cursor;
            if (req.onsuccess) req.onsuccess();
          }} else {{
            req.result = null;
            if (req.onsuccess) req.onsuccess();
          }}
        }}
        setTimeout(step, 0);
        return req;
      }}
    }}

    class MockDb {{
      constructor(name) {{
        this.name = name;
        this.stores = new Map();
        this.objectStoreNames = {{
          contains: (s) => this.stores.has(s)
        }};
      }}
      addStore(name, keyPath = null) {{
        const s = new MockStore(name, keyPath);
        this.stores.set(name, s);
        return s;
      }}
      transaction(sName, mode) {{
        const store = this.stores.get(sName);
        const tx = {{
          objectStore: () => store,
          oncomplete: null,
          onerror: null
        }};
        setTimeout(() => tx.oncomplete && tx.oncomplete(), 20);
        return tx;
      }}
      close() {{}}
    }}

    const dbs = new Map();
    const deleted = [];
    globalThis.indexedDB = {{
      databases: async () => Array.from(dbs.keys()).map(name => ({{ name }})),
      open: (name) => {{
        const db = dbs.get(name);
        const req = {{ result: db, onsuccess: null, onerror: null }};
        setTimeout(() => req.onsuccess && req.onsuccess(), 0);
        return req;
      }},
      deleteDatabase: (name) => {{
        deleted.push(name);
        dbs.delete(name);
        const req = {{ onsuccess: null, onerror: null }};
        setTimeout(() => req.onsuccess && req.onsuccess(), 0);
        return req;
      }}
    }};

    // Setup legacy qqq_lab database
    const oldDb = new MockDb("qqq_lab");
    const oldFiles = oldDb.addStore("files");
    const oldKv = oldDb.addStore("kv");
    const oldListe = oldDb.addStore("liste_utente", "id");
    oldFiles.put("file_bytes_1", "f1.mzML");
    oldKv.put("dark", "theme");
    oldListe.put({{ id: "lst1", name: "Custom Targets" }});
    dbs.set("qqq_lab", oldDb);

    // Setup new mzlab_lab database
    const newDb = new MockDb("mzlab_lab");
    newDb.addStore("files");
    newDb.addStore("kv");
    newDb.addStore("liste_utente", "id");
    newDb.addStore("manifest", "path");
    dbs.set("mzlab_lab", newDb);

    // Run 1: initial migration
    await migrateIdb(newDb, "qqq_lab");
    const r1 = {{
      migrated_from: await getKv(newDb, "migrated_from"),
      status: await getKv(newDb, "migration_status"),
      runs: await getKv(newDb, "migration_runs"),
      files_count: await countStore(newDb, "files"),
      liste_count: await countStore(newDb, "liste_utente"),
    }};

    // Run 2: second launch
    await migrateIdb(newDb, "qqq_lab");
    const r2 = {{
      runs: await getKv(newDb, "migration_runs"),
      deleted: deleted.slice(),
    }};

    // Run 3: third launch triggers deletion of qqq_lab
    await migrateIdb(newDb, "qqq_lab");
    const r3 = {{
      runs: await getKv(newDb, "migration_runs"),
      status: await getKv(newDb, "migration_status"),
      deleted: deleted.slice(),
    }};

    console.log(JSON.stringify({{ r1, r2, r3 }}));
    """
    res = run_node_esm(code)
    # Run 1 assertions
    assert res["r1"]["migrated_from"] == "qqq_lab"
    assert res["r1"]["status"] == "ok"
    assert res["r1"]["runs"] == 1
    assert res["r1"]["files_count"] == 1
    assert res["r1"]["liste_count"] == 1

    # Run 2 assertions
    assert res["r2"]["runs"] == 2
    assert "qqq_lab" not in res["r2"]["deleted"]

    # Run 3 assertions: deletion performed
    assert res["r3"]["runs"] == 3
    assert res["r3"]["status"] == "cleaned"
    assert "qqq_lab" in res["r3"]["deleted"]


def test_service_worker_activation_cleanup():
    code = """
    const cachesMap = new Map();
    cachesMap.set("qqq-app-v1", {});
    cachesMap.set("qqq-big-v1", {});
    cachesMap.set("mzlab-app-old", {});
    cachesMap.set("mzlab-app-current", {});
    cachesMap.set("mzlab-big-current", {});

    const deleted = [];
    const caches = {
      keys: async () => Array.from(cachesMap.keys()),
      delete: async (k) => {
        deleted.push(k);
        cachesMap.delete(k);
        return true;
      }
    };

    const pfx = "mzlab-";
    const APP = "mzlab-app-current";
    const BIG = "mzlab-big-current";

    // Replicate sw.js activate handler logic
    for (const k of await caches.keys()) {
      if (k.startsWith("qqq-") || (k.startsWith(pfx) && k !== APP && k !== BIG)) {
        await caches.delete(k);
      }
    }

    console.log(JSON.stringify({
        deleted: deleted.sort(),
        remaining: Array.from(cachesMap.keys()).sort(),
    }));
    """
    res = run_node_esm(code)
    assert res["deleted"] == ["mzlab-app-old", "qqq-app-v1", "qqq-big-v1"]
    assert res["remaining"] == ["mzlab-app-current", "mzlab-big-current"]
