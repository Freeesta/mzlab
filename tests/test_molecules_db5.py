"""IndexedDB version 5 (mzlab/web/qqq-db.js): the store `molecules` of the molecules of the user is added to a database that has the four stores of version 3 (or 4), nothing else changes,
and nomi.js names the same version. The real-browser migration with rows in every store is a step of tests_e2e/e2e_molecules_ui.py."""
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
    const made = [];
    const rows = {{ liste_utente: [{{ id: "l1", name: "My list" }}] }};
    const have = new Set(["files", "kv", "liste_utente", "manifest", "molecules"]);
    const db = {{ objectStoreNames: {{ contains: n => have.has(n) }}, createObjectStore: (n, o) => {{ made.push([n, o && o.keyPath]); have.add(n); }} }};
    upgrade(db);
    const once = made.length;
    upgrade(db);
    console.log(JSON.stringify({{ ver: DB_VER, made, idempotent: made.length === once, old: [...have].includes("liste_utente"), rows: rows.liste_utente }}));
    """
    res = run_node_esm(code)
    assert res["ver"] == 6 and res["made"] == [["user_lists", "id"]] and res["idempotent"], res
    assert res["old"] and res["rows"] == [{"id": "l1", "name": "My list"}], res


def test_names_and_the_schema_name_the_same_version():
    assert re.search(r"const DB_VER = 6;", (WEB / "nomi.js").read_text(encoding="utf-8"))
    assert re.search(r"export const DB_VER = 6;", (WEB / "qqq-db.js").read_text(encoding="utf-8"))
