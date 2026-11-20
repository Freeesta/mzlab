"""IndexedDB upgrade (mzlab/web/qqq-db.js, version 6): the store `user_lists` is added to a real version 5 database (files, kv, liste_utente, manifest, molecules) and nothing else changes:
the old `liste_utente` store and its rows are not touched, and nomi.js names the same version. The real-browser migration with rows in every store is a step of tests_e2e/e2e_molecules_ui.py."""
import json
import re
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def run_node_esm(code: str) -> dict:
    r = subprocess.run(["node", "--input-type=module", "-e", code], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, f"Node ESM script failed:\nSTDOUT: {r.stdout}\nSTDERR: {r.stderr}"
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_database_version_6_adds_the_user_lists_store_and_leaves_the_old_one():
    # a real version 5 database: files, kv, liste_utente, manifest, molecules (no user_lists); the upgrade creates user_lists only
    uri = (WEB / "qqq-db.js").as_uri()
    code = f"""
    import {{ upgrade, DB_VER }} from {uri!r};
    const made = [], touched = [];
    // every store is a real Map of rows; any call that could change a row or drop a store is recorded
    const stores = new Map(["files", "kv", "liste_utente", "manifest", "molecules"].map(n => [n, new Map()]));
    stores.get("liste_utente").set("l1", {{ id: "l1", name: "My list" }});
    const before = JSON.stringify([...stores].map(([n, m]) => [n, [...m]]));
    const db = {{
      objectStoreNames: {{ contains: n => stores.has(n) }},
      createObjectStore: (n, o) => {{ made.push([n, o && o.keyPath]); stores.set(n, new Map()); }},
      deleteObjectStore: n => touched.push(["delete", n]),
      transaction: n => {{ touched.push(["transaction", n]); return {{}}; }},
    }};
    upgrade(db);
    const once = made.length;
    upgrade(db);
    const rows = JSON.stringify([...stores].filter(([n]) => n !== "user_lists").map(([n, m]) => [n, [...m]]));
    console.log(JSON.stringify({{ ver: DB_VER, made, idempotent: made.length === once, touched, same: rows === before, old: stores.get("liste_utente").get("l1") }}));
    """
    res = run_node_esm(code)
    assert res["ver"] == 6 and res["made"] == [["user_lists", "id"]] and res["idempotent"], res
    assert res["touched"] == [] and res["same"] and res["old"] == {"id": "l1", "name": "My list"}, res


def test_names_and_the_schema_name_the_same_version():
    assert re.search(r"const DB_VER = 6;", (WEB / "nomi.js").read_text(encoding="utf-8"))
    assert re.search(r"export const DB_VER = 6;", (WEB / "qqq-db.js").read_text(encoding="utf-8"))
