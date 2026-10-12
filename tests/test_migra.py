"""Migrations of the archive (mzlab/web/migra.js, run by node with fake storage): numbered steps, idempotent and repeatable, inside an exclusive lock."""
import json
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"

PRELUDE = f"""
import {{ migra, runSteps, rowOf, STEPS }} from {(WEB / "migra.js").as_uri()!r};
const mk = (files, start) => {{
  const st = {{ schema: start, manifest: new Map(), calls: [] }};
  return {{ st, io: {{ getSchema: async () => st.schema, setSchema: async n => {{ st.schema = n; }}, list: async k => files[k] || [],
    putManifest: async r => {{ st.calls.push(r.path); st.manifest.set(r.path, r); }} }} }};
}};
const FILES = {{ files: [{{ name: "a.mzML", size: 10, lastModified: 5 }}], idx: [{{ name: "a.mzML.mzidx", size: 3, lastModified: 6 }}], feat: [{{ name: "s.mzfeat", size: 2, lastModified: 7 }}] }};
const out = async f => console.log(JSON.stringify(await f()));
"""


def run(body):
    r = subprocess.run(["node", "--input-type=module", "-e", PRELUDE + body], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_data_without_a_schema_are_migrated_to_the_manifest_and_the_version_is_written():
    got = run("""await out(async () => { const { st, io } = mk(FILES, undefined); const n = await runSteps(io);
      return [n, st.schema, [...st.manifest.keys()].sort(), st.manifest.get("idx/a.mzML.mzidx").tipo, st.manifest.get("files/a.mzML").formato]; });""")
    assert got == [3, 3, ["feat/s.mzfeat", "files/a.mzML", "idx/a.mzML.mzidx"], "indice", "mzML"]


def test_a_step_that_was_interrupted_is_done_again_without_damage_and_a_current_schema_does_nothing():
    got = run("""await out(async () => {
      const { st, io } = mk(FILES, 2); let boom = true;
      const flaky = { 3: async x => { await STEPS[3](x); if (boom) { boom = false; throw new Error("cut"); } } };
      let first = "";
      try { await runSteps(io, flaky, 3); } catch (e) { first = e.message; }
      const after = st.schema, rows = st.manifest.size;
      await runSteps(io, flaky, 3);
      const again = mk(FILES, 3); await runSteps(again.io);
      return [first, after, rows, st.schema, st.manifest.size, again.st.calls.length]; });""")
    assert got == ["cut", 2, 3, 3, 3, 0]


def test_the_migration_runs_inside_the_exclusive_lock_when_there_are_web_locks():
    got = run("""Object.defineProperty(globalThis, "navigator", { value: { locks: { request: async (name, opt, fn) => { globalThis.LOCK = [name, opt.mode]; return fn(); } } }, configurable: true });
      await out(async () => { const { st, io } = mk(FILES, 2); await migra(io); return [globalThis.LOCK, st.schema]; });""")
    assert got == [["mzlab-migra", "exclusive"], 3]
