"""Archive of the browser (mzlab/web/archivio.js, pulizia.js) run by node with fake Web Locks and storage: one writer per file with a wait of 2 s that ends in an error with a key,
room checked before a big write (error.quota), persist() asked once, the other tabs told through BroadcastChannel, the automatic cleaning."""
import json
import subprocess
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"

PRELUDE = f"""
globalThis.self = globalThis;
const nav = o => Object.defineProperty(globalThis, "navigator", {{ value: o, configurable: true }});
import * as A from {(WEB / "archivio.js").as_uri()!r};
import {{ limpia, HOUR, WEEK }} from {(WEB / "pulizia.js").as_uri()!r};
const out = async f => {{ console.log(JSON.stringify(await f())); process.exit(0); }};
"""


def run(body):
    r = subprocess.run(["node", "--input-type=module", "-e", PRELUDE + body], capture_output=True, encoding="utf-8")
    assert r.returncode == 0, r.stderr
    return json.loads(r.stdout.strip().splitlines()[-1])


def test_a_file_held_by_another_tab_for_more_than_two_seconds_is_an_error_with_a_key():
    got = run("""
  let held = null;
  nav({ locks: { request: (key, opt, fn) => new Promise((ok, no) => {
    if (key.endsWith("busy")) { opt.signal.addEventListener("abort", () => no(Object.assign(new Error("a"), { name: "AbortError" }))); return; }
    Promise.resolve(fn()).then(ok, no); }) } });
  await out(async () => {
    const ok = await A.locked("files", "free", async () => 7);
    let err = null; const t0 = Date.now();
    try { await A.locked("files", "busy", async () => 1); } catch (e) { err = JSON.parse(e); }
    return [ok, err, Date.now() - t0 >= 1900];
  });""")
    assert got == [7, {"error_key": "error.fileInUse", "params": {"file": "busy"}}, True]


def test_room_is_checked_before_a_big_write_and_persist_is_asked_once():
    got = run("""
  let persist = 0;
  nav({ storage: { estimate: async () => ({ usage: 900 << 20, quota: 1000 << 20 }), persisted: async () => false, persist: async () => { persist++; return true; } } });
  await out(async () => {
    await A.ensureRoom(10 << 20);                                  // small: not checked
    let err = null;
    try { await A.ensureRoom(100 << 20); } catch (e) { err = JSON.parse(e); }
    await A.persistOnce(); await A.persistOnce(); await A.persistOnce();
    return [err, persist];
  });""")
    assert got == [{"error_key": "error.quota", "params": {"serve_mb": 180, "libero_mb": 100}}, 1]


def test_the_other_tabs_hear_of_the_changes():
    got = run("""
  const heard = [];
  A.listen(m => heard.push(m));
  const other = new BroadcastChannel("mzlab");
  other.postMessage({ t: "file", ev: "put", kind: "files", name: "a.mzML", size: 5 });
  await out(async () => { await new Promise(r => setTimeout(r, 100)); return heard; });""")
    assert got == [{"t": "file", "ev": "put", "kind": "files", "name": "a.mzML", "size": 5}]


def test_cleaning_removes_old_part_files_and_week_old_orphan_indexes_never_a_source():
    got = run("""
  const now = 100 * 24 * 3600e3, gone = [];
  const lists = {
    files: [{ name: "keep.mzML", size: 1, lastModified: 0 }, { name: "x.mzML.part", size: 1, lastModified: now - 2 * HOUR }, { name: "y.mzML.part", size: 1, lastModified: now - 60e3 }],
    idx: [{ name: "keep.mzML.mzidx", size: 1, lastModified: 0 }, { name: "old.mzML.mzidx", size: 1, lastModified: now - 8 * 24 * 3600e3 }, { name: "new.mzML.mzidx", size: 1, lastModified: now - 3600e3 }],
    feat: [{ name: "s.mzfeat.part", size: 1, lastModified: now - 3 * HOUR }, { name: "s.mzfeat", size: 1, lastModified: 0 }] };
  await out(async () => { const g = await limpia({ list: async k => lists[k], remove: async (k, n) => { gone.push(k + "/" + n); }, manifest: async () => [] }, now); return [g.sort(), gone.sort()]; });""")
    assert got[0] == ["feat/s.mzfeat.part", "files/x.mzML.part", "idx/old.mzML.mzidx"] and got[0] == got[1]
