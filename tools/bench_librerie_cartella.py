#!/usr/bin/env python3
"""Measures «Load folder…» of the HR identification panel on a folder (tools/librerie_cartella_sintetica.py) in Chromium: wall time of every LIB add, of the
following list, and the long tasks (> 50 ms) of the main thread. Needs the site (tools/build_site.py + site/static/wasm-lib). Usage: python3 tools/bench_librerie_cartella.py DIR [--runs 5]"""
import argparse, json, os, re, statistics, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
READABLE = re.compile(r"\.(msp|mgf|json|jsonl|ndjson|mzlib)$", re.I)
JS = """async () => {
  const files = [...document.getElementById('benchdir').files].filter(f => /\\.(msp|mgf|json|jsonl|ndjson|mzlib)$/i.test(f.name));
  const lt = []; new PerformanceObserver(l => { for (const e of l.getEntries()) lt.push(Math.round(e.duration)); }).observe({ entryTypes: ['longtask'] });
  const rows = []; const t0 = performance.now();
  for (const f of files) { const t = performance.now(); let ok = true; try { await LIB.call('add', { file: f }); } catch (e) { ok = false; } rows.push({ name: f.name, size: f.size, ok, ms: Math.round(performance.now() - t) }); }
  const tl = performance.now(); const libs = (await LIB.call('list')).libs || []; const listMs = Math.round(performance.now() - tl);
  await new Promise(r => setTimeout(r, 300));
  return { total: Math.round(performance.now() - t0), rows, listMs, nlibs: libs.length, longtasks: lt };
}"""


def once(folder: Path, port: int) -> dict:
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    try:
        with sync_playwright() as p:
            ctx = p.chromium.launch_persistent_context(tempfile.mkdtemp(prefix="mzlab-bench-prof-"), executable_path=os.environ.get("MZLAB_CHROMIUM") or None, service_workers="block")
            pg = ctx.new_page(); pg.goto(f"http://127.0.0.1:{port}/")
            pg.wait_for_function("window.LIB !== undefined && LIB.engine !== undefined", timeout=120000)
            pg.evaluate("(() => { const i = document.createElement('input'); i.type = 'file'; i.id = 'benchdir'; i.multiple = true; i.setAttribute('webkitdirectory', ''); document.body.appendChild(i); })()")
            pg.set_input_files("#benchdir", str(folder)); time.sleep(1)
            pg.on("console", lambda m: print(m.text) if m.text.startswith("PH") else None); r = pg.evaluate(JS); ctx.close(); return r
    finally:
        srv.terminate()


def main() -> None:
    ap = argparse.ArgumentParser(); ap.add_argument("dir"); ap.add_argument("--runs", type=int, default=1); a = ap.parse_args()
    res = [once(Path(a.dir), 8870 + i) for i in range(a.runs)]
    for k, r in enumerate(res):
        print(f"run {k + 1}: total {r['total']} ms, list {r['listMs']} ms, libraries {r['nlibs']}, long tasks {sorted(r['longtasks'], reverse=True)[:5]}")
        for x in sorted(r["rows"], key=lambda x: -x["ms"])[:4]:
            print(f"   {x['name']:<18} {x['size'] >> 10:>7} KB  {x['ms']:>6} ms  ok={x['ok']}")
    if a.runs > 1:
        print("median total ms:", statistics.median(r["total"] for r in res))
    json.dump(res, open(Path(tempfile.gettempdir()) / "bench_cartella.json", "w"))


if __name__ == "__main__":
    main()
