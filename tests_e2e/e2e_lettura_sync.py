"""Source files of the index builder read from OPFS with a read-only SyncAccessHandle (lettura-sync.js): the same bytes as the Blob, and the Blob fallback
when the file is taken (a writer holds it), has another size or is missing. Run in a worker, the only place where handles exist."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_lettura_sync.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
WORKER = """
import { openSource } from %r;
const eq = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);
onmessage = async () => {
  const out = {};
  try {
    const root = await navigator.storage.getDirectory(), d = await (await root.getDirectoryHandle("mzlab", { create: true })).getDirectoryHandle("files", { create: true });
    const name = "ls-test.bin", data = new Uint8Array(3000000).map((_, i) => (i * 7 + 3) & 255);
    const w0 = await (await d.getFileHandle(name, { create: true })).createSyncAccessHandle(); w0.truncate(0); w0.write(data, { at: 0 }); w0.flush(); w0.close();
    const f = await (await d.getFileHandle(name)).getFile();
    const blocks = [[0, 1000], [5, 1048576], [1000000, 1500000], [2999000, 1000]];
    const same = (s, ref) => blocks.every(([o, n]) => o + n > ref.length || eq(s.read(o, n), ref.subarray(o, o + n)));
    let s = await openSource("files", name, f); out.direct = s.hand; out.directSame = same(s, data); s.close();
    const w = await (await d.getFileHandle(name)).createSyncAccessHandle();
    s = await openSource("files", name, f); out.lockedHand = s.hand; out.lockedSame = same(s, data); s.close(); w.close();
    s = await openSource("files", name, new Blob([data.subarray(0, 1000)])); out.sizeHand = s.hand; out.sizeSame = eq(s.read(0, 1000), data.subarray(0, 1000)); s.close();
    s = await openSource("files", "ls-missing.bin", f); out.missHand = s.hand; out.missSame = same(s, data); s.close();
    await d.removeEntry(name);
  } catch (e) { out.error = String(e && e.message || e); }
  postMessage(out);
};
"""
r = Run(port=8999, wd="/tmp/wdls")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        res = pg.evaluate("""async src => {
          const url = new URL("static/lettura-sync.js", document.baseURI).href;
          const w = new Worker(URL.createObjectURL(new Blob([src.replace("__U__", url)], { type: "text/javascript" })), { type: "module" });
          return await new Promise((ok, no) => { w.onmessage = e => ok(e.data); w.onerror = e => no(new Error(e.message || "worker")); w.postMessage(0); });
        }""", WORKER % "__U__")
        def direct(): assert not res.get("error"), res; assert res["direct"] and res["directSame"], res
        step("read-only handle: same bytes as the Blob, blocks of any size", direct)
        def locked(): assert res["lockedHand"] is False and res["lockedSame"], res
        step("file held by a writer: falls back to the Blob", locked)
        def size(): assert res["sizeHand"] is False and res["sizeSame"], res
        step("size different from the Blob: falls back", size)
        def miss(): assert res["missHand"] is False and res["missSame"], res
        step("file not in OPFS: falls back", miss)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
