"""IndexedDB version 4 (mzlab/web/qqq-db.js): the store `molecole` of the molecules of the user is added to a database that has the four stores of version 3, nothing else changes,
and nomi.js names the same version. The real-browser migration with rows in every store is a step of tests_e2e/e2e_molecole_ui.py."""
import json
import re
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"


def run_node_esm(code: str) -> dict:
    r = subprocess.run(["node", "--input-type=module", "-e", code], capture_output=True, text=True, encoding="utf-8")
    assert r.returncode == 0, f"Node ESM script failed:\nSTDOUT: {r.stdout}\nSTDERR: {r.stderr}"
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_database_version_4_adds_only_the_molecules_store():
    uri = (WEB / "qqq-db.js").as_uri()
    code = f"""
    import {{ upgrade, DB_VER }} from {uri!r};
    const made = [];
    const have = new Set(["files", "kv", "liste_utente", "manifest"]);
    const db = {{ objectStoreNames: {{ contains: n => have.has(n) }}, createObjectStore: (n, o) => {{ made.push([n, o && o.keyPath]); have.add(n); }} }};
    upgrade(db);
    const once = made.length;
    upgrade(db);
    console.log(JSON.stringify({{ ver: DB_VER, made, idempotent: made.length === once }}));
    """
    res = run_node_esm(code)
    assert res["ver"] == 4 and res["made"] == [["molecole", "id"]] and res["idempotent"], res


def test_nomi_and_the_schema_name_the_same_version():
    assert re.search(r"const DB_VER = 4;", (WEB / "nomi.js").read_text(encoding="utf-8"))
    assert re.search(r"export const DB_VER = 4;", (WEB / "qqq-db.js").read_text(encoding="utf-8"))
