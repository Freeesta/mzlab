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


# ---- a fake OPFS (in memory) for the lease, copy and rawSink tests: files are Uint8Arrays, sync handles obey a byte quota and an optional opening delay
FAKE_OPFS = """
const files = new Map(), log = [];
let quotaLeft = Infinity, openDelay = {};
class FileSystemFileHandle {
  constructor(name) { this.kind = "file"; this.name = name; if (!files.has(name)) files.set(name, new Uint8Array(0)); }
  async getFile() { const u = files.get(this.name); return Object.assign(new Blob([u]), { lastModified: Date.now() }); }
  async createSyncAccessHandle() {
    if (openDelay[this.name]) await new Promise(r => setTimeout(r, openDelay[this.name]));
    const name = this.name;
    return { closed: false,
      read(buf, { at }) { const u = files.get(name); const n = Math.max(0, Math.min(buf.length, u.length - at)); buf.set(u.subarray(at, at + n)); return n; },
      write(data, { at }) { const n = Math.min(data.length, quotaLeft); quotaLeft -= n; const u = files.get(name); const v = new Uint8Array(Math.max(u.length, at + n)); v.set(u); v.set(data.subarray(0, n), at); files.set(name, v); return n; },
      truncate(n) { files.set(name, files.get(name).slice(0, n)); }, getSize() { return files.get(name).length; }, flush() {}, close() { this.closed = true; } };
  }
}
class Dir {
  async getDirectoryHandle() { return new Dir(); }
  async getFileHandle(n, o) { if (!files.has(n) && !(o && o.create)) throw new Error("NotFound"); return new FileSystemFileHandle(n); }
  async removeEntry(n) { files.delete(n); }
  async *entries() { for (const n of files.keys()) yield [n, new FileSystemFileHandle(n)]; }
}
globalThis.FileSystemFileHandle = FileSystemFileHandle;
nav({ storage: { getDirectory: async () => new Dir(), estimate: async () => ({ usage: 0, quota: 100 << 20 }) } });
"""


def test_a_wake_during_parkall_keeps_the_handles_the_request_will_read():
    got = run(FAKE_OPFS + """
  await out(async () => {
    files.set("a", new Uint8Array([1, 2, 3])); files.set("b", new Uint8Array([4, 5, 6]));
    await A.lease("idx", "a");
    openDelay.b = 30; const opening = A.lease("idx", "b");          // b is still opening when the request ends
    const parking = A.parkAll();                                      // a is closed, then it waits for b
    await new Promise(r => setTimeout(r, 5));
    const waking = A.wake();                                          // the next request arrives meanwhile
    await Promise.all([opening, parking, waking]);
    const buf = new Uint8Array(3);
    return [A.leaseRead("idx", "a", buf, 0), A.leaseRead("idx", "b", buf, 0)];
  });""")
    assert got == [3, 3]


def test_parkall_with_no_wake_still_closes_every_handle():
    got = run(FAKE_OPFS + """
  await out(async () => {
    files.set("a", new Uint8Array([1, 2, 3]));
    await A.lease("idx", "a"); await A.parkAll();
    return A.leaseRead("idx", "a", new Uint8Array(3), 0);
  });""")
    assert got == -1


def test_the_quota_error_of_the_raw_sink_counts_the_block_that_did_not_fit():
    got = run(FAKE_OPFS + """
  const { rawSink } = await import(%r);
  await out(async () => {
    quotaLeft = 2 << 20;
    const s = await rawSink("c.mzML");
    s.write(new Uint8Array(1 << 20));                                  // fits
    let threw = false;
    try { s.write(new Uint8Array(3 << 20)); } catch (_) { threw = true; }   // 1 MiB of it fits, 2 MiB do not
    return [threw, JSON.parse(await s.error())];
  });""" % (WEB / "rust-opfs.js").as_uri())
    assert got[0] is True
    assert got[1]["error_key"] == "error.quota" and got[1]["params"]["serve_mb"] == 4


def test_the_copy_reads_the_next_block_while_the_previous_one_is_written_and_hashed_in_file_order():
    got = run(FAKE_OPFS + """
  await out(async () => {
    const step = 8 << 20, size = 3 * step + 5, ev = [], seen = [];
    const chunk = async (o, n) => { ev.push("read" + o / step); await new Promise(r => setTimeout(r, 10)); ev.push("got" + o / step); return new Uint8Array(n).fill(o / step + 1); };
    const f = await A.putChunks("files", "big", size, chunk, async (b, o) => { ev.push("after" + o / step); await new Promise(r => setTimeout(r, 30)); seen.push(o); });
    return [f.size, ev.indexOf("read1") < ev.indexOf("after0"), seen, files.get("big")[step], files.get("big")[size - 1]];
  });""")
    assert got == [3 * (8 << 20) + 5, True, [0, 8 << 20, 16 << 20, 24 << 20], 2, 4]


def test_a_failed_read_of_the_prefetched_block_still_removes_the_partial_file():
    got = run(FAKE_OPFS + """
  await out(async () => {
    const step = 8 << 20;
    let err = null;
    try { await A.putChunks("files", "big", 2 * step, async (o, n) => { if (o > 0) throw new Error("read failed"); return new Uint8Array(n); }); } catch (e) { err = e.message; }
    return [err, files.has("big")];
  });""")
    assert got == ["read failed", False]
