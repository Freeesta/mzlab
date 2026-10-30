"""OPFS promote(): a file written under a temporary name takes its final name (the step that ends every index build). WebKit rejected move(name) with
"Not enough arguments": the file must arrive whole under the new name, the temporary one gone, on every engine (decoy: the bytes are those written)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
WORKER = """
const A = await import(%r);
const out = [];
const data = new Uint8Array(70000).map((_, i) => (i * 31 + 7) & 255);
await A.put("idx", "t.mzidx.part", data);
const f = await A.promote("idx", "t.mzidx.part", "t.mzidx");
const back = new Uint8Array(await (await A.get("idx", "t.mzidx")).arrayBuffer());
const names = (await A.list("idx")).map(e => e.name);
await A.remove("idx", "t.mzidx");
postMessage({ size: f.size, same: back.length === data.length && back.every((v, i) => v === data[i]), names });
"""
r = Run(port=8975, wd="/tmp/wd_promote")
res = None
try:
    with sync_playwright() as p:
        pg = r.page(p)
        res = pg.evaluate("""async src => {
          const url = location.origin + '/static/archivio.js';
          const blob = new Blob([`self.onmessage = async () => { try { ${src.replace('%r', JSON.stringify('__URL__'))} } catch (e) { postMessage({ error: String(e && e.message || e) }); } };`.replace('__URL__', url)], { type: 'text/javascript' });
          const w = new Worker(URL.createObjectURL(blob), { type: 'module' });
          return await new Promise((ok, no) => { w.onmessage = e => ok(e.data); w.onerror = e => no(new Error(e.message)); setTimeout(() => no(new Error('timeout')), 20000); w.postMessage(0); });
        }""", WORKER)
        print("promote:", res)
        assert "error" not in res, res
        assert res["size"] == 70000 and res["same"], res
        assert "t.mzidx" in res["names"] and "t.mzidx.part" not in res["names"], res
        print("OK promote")
finally:
    r.close()
