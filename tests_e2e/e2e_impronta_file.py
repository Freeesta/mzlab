"""The hash workers read a member where it is (impronta.js `file`, impronta-worker.js `end` with a file): the path of a maker's member read in place, driven
here without the quota override of CDP so that it runs in every browser (WebKit = Safari). Two synthetic members (one of several 8 MiB pieces, one of
less than a piece), with two workers (four cores) and with one: [sha256, sha1, probe] must be the engine's own `Passata` over the same bytes, SHA-256 and
SHA-1 those of crypto.subtle, and the bytes told with `onRead` the size of the member. Needs the site with the engine .wasm (tools/prepara_sito.py)."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_impronta_file.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
# cores: navigator.hardwareConcurrency as createImpronte reads it (4 = two workers, 2 = one); each member a File of bytes from a fixed LCG
HASH = """async ([cores, sizes]) => {
  Object.defineProperty(navigator, 'hardwareConcurrency', { value: cores, configurable: true });
  const w = await import('./static/wasm/mzlab_wasm.js'); await w.default({ module_or_path: './static/wasm/mzlab_wasm_bg.wasm' });
  const module = await WebAssembly.compile(await (await fetch('./static/wasm/mzlab_wasm_bg.wasm')).arrayBuffer());
  const { createImpronte } = await import('./static/impronta.js');
  const imp = createImpronte({ Passata: w.Passata, workerUrl: new URL('./static/impronta-worker.js', location.href), module: () => module });
  const hex = async (alg, u) => [...new Uint8Array(await crypto.subtle.digest(alg, u))].map(x => x.toString(16).padStart(2, '0')).join('');
  const out = [];
  for (const [i, size] of sizes.entries()) {
    const u = new Uint8Array(size); let s = 12345 + i; for (let k = 0; k < size; k++) { s = (s * 1103515245 + 12345) >>> 0; u[k] = s >>> 24; }
    const file = new File([u], 'membro' + i + '.raw');
    let r = null, n = 0; const t0 = performance.now();
    while (!(r = imp.file(file, b => { n += b; }))) { if (performance.now() - t0 > 30000) return 'no hash worker after 30 s'; await new Promise(f => setTimeout(f, 100)); }
    const sha1 = await r.sha1, all = await r.all;
    const p = new w.Passata(size); p.aggiorna(u); const ref = Array.from(p.fine());
    out.push({ size, sha1, all, ref, read: n, sub: [await hex('SHA-256', u), await hex('SHA-1', u)] });
  }
  return out; }"""
SIZES = [(8 << 20) * 2 + 4099, 5 << 20]
r = Run(port=8906, wd="/tmp/wd_impronta_file")
try:
    with sync_playwright() as p:
        a = r.page(p); wait_for_js(a, "window.qqReady === true", 60000)
        def check(cores):
            got = a.evaluate(HASH, [cores, SIZES])
            assert isinstance(got, list), got
            for m in got:
                assert m["all"] == m["ref"], f"{m['size']} B: workers {m['all']} != engine {m['ref']}"
                assert m["sha1"] == m["ref"][1], f"{m['size']} B: sha1 {m['sha1']} != {m['ref'][1]}"
                assert m["ref"][:2] == m["sub"], f"{m['size']} B: engine {m['ref'][:2]} != crypto.subtle {m['sub']}"
                assert m["read"] == m["size"], f"{m['size']} B: onRead told {m['read']} B"
        step("two hash workers read each member from its File: the engine's fingerprints, SHA-1 first, every byte told", lambda: check(4))
        step("one hash worker reads each member from its File: the same fingerprints", lambda: check(2))
        assert a.evaluate("document.getElementById('err').textContent") == ""
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
