#!/usr/bin/env python3
"""Measures the two engines of the built site in Chromium: Python (Pyodide) and Rust (?motore=rust), on the same files.

For each engine: time to open the files (upload + first TIC drawn), time for the TIC and five XIC of every file (what the
page asks while a student works) and the resident memory of the whole browser (Linux: sum of /proc RSS of the Chromium processes).
It does not interpret the numbers: AGENTS.md / ARCHITETTURA.md decide what they mean. Needs the site (python3 tools/build_site.py)
and, for Rust, site/static/wasm (see .github/workflows/pages.yml). Usage: python3 tools/bench.py [--files N] [--json] [--memoria]
--memoria: memory breakdown per engine and per file set (6 Full Scan LR, the 4 anonymous HRMS examples): WASM heaps (Pyodide, Rust),
IndexedDB contents, JS heaps and browser RSS. Measure only, no interpretation.
Files: MZLAB_MZML or the usual data folders (tests_e2e/lib.py); Chromium: MZLAB_CHROMIUM = path of an executable, if needed."""
import argparse, json, os, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
from lib import MZ  # noqa: E402

MZS = [200, 250, 300, 364, 400]
GET = """async u => { const t = performance.now(); const r = await fetch(u); await r.json(); return performance.now() - t; }"""


def rss_mb() -> float:
    """Resident memory (MB) of every Chromium process (Linux: /proc; 0 elsewhere). Only the benchmark's browser is running."""
    total = 0
    try:
        for d in Path("/proc").iterdir():
            if d.name.isdigit():
                try:
                    if b"chrom" not in (d / "cmdline").read_bytes()[:400].lower():
                        continue
                    total += int((d / "statm").read_text().split()[1]) * 4096
                except OSError:
                    pass
    except OSError:
        return 0.0
    return round(total / 2**20, 1)


def rss_by_type() -> dict:
    """RSS (MB) of the Chromium processes grouped by --type (renderer, gpu-process, utility, zygote, browser)."""
    out: dict = {}
    try:
        for d in Path("/proc").iterdir():
            if d.name.isdigit():
                try:
                    cmd = (d / "cmdline").read_bytes().split(b"\0")
                    if b"chrom" not in cmd[0].lower():
                        continue
                    t = next((c[7:].decode() for c in cmd if c.startswith(b"--type=")), "browser")
                    out[t] = out.get(t, 0) + int((d / "statm").read_text().split()[1]) * 4096
                except OSError:
                    pass
    except OSError:
        pass
    return {k: round(v / 2**20, 1) for k, v in sorted(out.items())}


def run(engine: str, files: list[str], port: int) -> dict:
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1", "-d", str(ROOT / "site")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    try:
        with sync_playwright() as p:
            b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
            pg = b.new_page()
            t0 = time.perf_counter()
            pg.goto(f"http://127.0.0.1:{port}/?motore={engine}")
            pg.wait_for_selector("#drop", timeout=180000)
            out = {"engine": engine, "start_s": round(time.perf_counter() - t0, 2)}
            t1 = time.perf_counter()
            pg.set_input_files("#pick", files)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(files)}", timeout=300000)
            pg.click("#opbtn")
            pg.wait_for_selector(".pnl.chrom canvas", timeout=300000)
            if engine == "rust":
                pg.wait_for_function("window.MZLAB_RUST.stats.rust > 0", timeout=120000)
            out["open_s"] = round(time.perf_counter() - t1, 2)
            n = pg.evaluate("E.files.length")
            t2 = time.perf_counter()
            for k in range(n):
                pg.evaluate(GET, f"api/chrom?k={k}&kind=tic&level=1")
                for mz in MZS:
                    pg.evaluate(GET, f"api/xic?k={k}&mz={mz}&tol=0.35&level=1")
            out["tic_xic_s"] = round(time.perf_counter() - t2, 2)
            out["requests"] = n * (1 + len(MZS))
            if engine == "rust":
                out["served_by_rust"] = pg.evaluate("window.MZLAB_RUST.stats.rust")
            out["browser_rss_mb"] = rss_mb()
            b.close()
            return out
    finally:
        srv.terminate()


# Prepended to the workers' source (Playwright route): remembers every WebAssembly.Memory the worker creates or exports.
HOOK = """;(()=>{const W=globalThis.WebAssembly;if(!W||globalThis.__mems)return;const m=globalThis.__mems=[];
const M=W.Memory;W.Memory=function(...a){const x=new M(...a);m.push(x);return x};W.Memory.prototype=M.prototype;
const ex=i=>{try{for(const v of Object.values(i.exports))if(v instanceof M&&!m.includes(v))m.push(v)}catch(_){}};
for(const f of ["instantiate","instantiateStreaming"]){const o=W[f];if(o)W[f]=async(...a)=>{const r=await o.apply(W,a);ex(r.instance||r);return r}}})();
"""
WASM_MEM = "(globalThis.__mems||[]).reduce((s,x)=>s+x.buffer.byteLength,0)"
IDB_SIZE = """async () => { const db = await new Promise((s, j) => { const r = indexedDB.open('qqq_lab'); r.onsuccess = () => s(r.result); r.onerror = () => j(r.error); });
  const all = await new Promise((s, j) => { const r = db.transaction('files').objectStore('files').getAll(); r.onsuccess = () => s(r.result); r.onerror = () => j(r.error); });
  db.close(); return all.reduce((n, b) => n + (b.byteLength ?? b.size ?? 0), 0); }"""
JS_HEAP = "performance.memory ? performance.memory.usedJSHeapSize : 0"


def memory(engine: str, files: list[str], port: int) -> dict:
    """One engine, one file set: every memory figure after opening the files and asking TIC + XIC of each."""
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1", "-d", str(ROOT / "site")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    try:
        with sync_playwright() as p:
            b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None, args=["--enable-precise-memory-info"])
            ctx = b.new_context(service_workers="block")

            def inject(route):
                r = route.fetch()
                route.fulfill(response=r, body=HOOK + r.text(), headers={**r.headers, "content-type": "text/javascript"})
            ctx.route("**/browser-worker.js", inject)
            ctx.route("**/rust-worker.js", inject)
            pg = ctx.new_page()
            pg.goto(f"http://127.0.0.1:{port}/?motore={engine}")
            pg.wait_for_selector("#drop", timeout=180000)
            pg.set_input_files("#pick", files)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(files)}", timeout=300000)
            pg.click("#opbtn")
            pg.wait_for_selector(".pnl.chrom canvas", timeout=300000)
            if engine == "rust":
                pg.wait_for_function("window.MZLAB_RUST.stats.rust > 0", timeout=120000)
            n = pg.evaluate("E.files.length")
            for k in range(n):
                pg.evaluate(GET, f"api/chrom?k={k}&kind=tic&level=1")
                for mz in MZS:
                    pg.evaluate(GET, f"api/xic?k={k}&mz={mz}&tol=0.35&level=1")
            out = {"engine": engine, "files": n, "file_mb": round(sum(os.path.getsize(f) for f in files) / 2**20, 1)}
            mb = lambda v: round(v / 2**20, 1)
            for w in pg.workers:
                kind = "pyodide" if "browser-worker" in w.url else "rust" if "rust-worker" in w.url else None
                if kind:
                    out[f"{kind}_wasm_mb"] = mb(w.evaluate(WASM_MEM))
                    out[f"{kind}_worker_js_heap_mb"] = mb(w.evaluate(JS_HEAP))
            out["page_js_heap_mb"] = mb(pg.evaluate(JS_HEAP))
            out["indexeddb_mb"] = mb(pg.evaluate(IDB_SIZE))
            out["browser_rss_mb"] = rss_mb()
            out["rss_by_type"] = rss_by_type()
            b.close()
            return out
    finally:
        srv.terminate()


def baseline() -> dict:
    """Chromium with an empty page: what the browser costs before mzLab."""
    from playwright.sync_api import sync_playwright
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        b.new_page().goto("about:blank")
        time.sleep(1)
        out = {"rss_mb": rss_mb(), "by_type": rss_by_type()}
        b.close()
        return out


def table(keys: list, rows: list) -> list:
    return ["| " + " | ".join(keys) + " |", "|" + "---|" * len(keys)] + ["| " + " | ".join(str(r.get(k, "")) for k in keys) + " |" for r in rows]


def main_memoria() -> list:
    """Memory breakdown for the two file sets and the two engines, plus the empty-page baseline; returns the markdown lines."""
    sets = {"6 Full Scan LR": sorted(str(x) for x in Path(MZ).glob("*FullMass*.mzML"))[:6],
            "4 HRMS": sorted(str(x) for x in (ROOT / "mzlab" / "web" / "esempi").glob("HRMS_*.mzML"))}
    keys = ["set", "engine", "file_mb", "pyodide_wasm_mb", "rust_wasm_mb", "page_js_heap_mb", "indexeddb_mb", "browser_rss_mb"]
    rows, port = [], 8860
    for name, fl in sets.items():
        for e in ("python", "rust"):
            if fl:
                port += 1
                rows.append({**memory(e, fl, port), "set": name})
    return table(keys, rows) + ["", f"Empty page (about:blank), same browser: RSS {baseline()['rss_mb']} MB."]


# ---- "velocità" scenarios: many files, with the breakdown of the time and the peak memory (python engine only) ----
STEPS = """(()=>{const W=window.Worker;window.__steps=[];window.Worker=function(...a){const w=new W(...a);
w.addEventListener('message',e=>{const d=e.data;if(d&&(d.type==='step'||d.type==='ready'||d.type==='fatal'))window.__steps.push([d.type==='step'?d.text:d.type,Math.round(performance.now())])});return w};
window.Worker.prototype=W.prototype})();"""


class Peak:
    """Samples the browser RSS in a thread and keeps the maximum (MB)."""
    def __init__(self): self.max, self._stop = 0.0, False
    def _loop(self):
        while not self._stop:
            self.max = max(self.max, rss_mb()); time.sleep(0.25)
    def __enter__(self):
        import threading
        self._t = threading.Thread(target=self._loop, daemon=True); self._t.start(); return self
    def __exit__(self, *a): self._stop = True; self._t.join()


def replicate(src: list[Path], n: int, out: Path) -> list[str]:
    """n files for the benchmark: the sources cycled, each copy with its own name (hard link when possible)."""
    out.mkdir(parents=True, exist_ok=True)
    res = []
    for i in range(n):
        f = src[i % len(src)]
        d = out / (f"{f.stem}_c{i // len(src)}{f.suffix}" if i >= len(src) else f.name)
        if not d.exists():
            try: os.link(f, d)
            except OSError: import shutil; shutil.copyfile(f, d)
        res.append(str(d))
    return res


def scenario(name: str, files: list[str], port: int) -> dict:
    """Open the files, ask TIC + 5 XIC of each, reload the page in the same browser profile (IndexedDB kept) and time the reopening."""
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1", "-d", str(ROOT / "site")],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    n = len(files)
    out: dict = {"scenario": name, "files": n, "file_mb": round(sum(os.path.getsize(f) for f in files) / 2**20, 1)}
    try:
        with sync_playwright() as p, Peak() as pk:
            b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None, args=["--enable-precise-memory-info"])
            ctx = b.new_context(service_workers="block")
            ctx.add_init_script(STEPS)
            ctx.route("**/browser-worker.js", lambda route: (lambda r: route.fulfill(response=r, body=HOOK + r.text(), headers={**r.headers, "content-type": "text/javascript"}))(route.fetch()))
            pg = ctx.new_page()
            t0 = time.perf_counter()
            pg.goto(f"http://127.0.0.1:{port}/?motore=python")
            pg.wait_for_selector("#drop", timeout=300000)
            out["engine_start_s"] = round(time.perf_counter() - t0, 2)
            t1 = time.perf_counter()
            pg.set_input_files("#pick", files)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=600000)
            out["read_mzml_s"] = round(time.perf_counter() - t1, 2)
            t2 = time.perf_counter()
            pg.click("#opbtn")
            pg.wait_for_selector(".pnl.chrom canvas", timeout=600000)
            out["first_draw_s"] = round(time.perf_counter() - t2, 2)
            t3 = time.perf_counter()
            for k in range(n):
                pg.evaluate(GET, f"api/chrom?k={k}&kind=tic&level=1")
                for mz in MZS:
                    pg.evaluate(GET, f"api/xic?k={k}&mz={mz}&tol=0.35&level=1")
            out["tic_xic_s"] = round(time.perf_counter() - t3, 2)
            out["idb_mb"] = round(pg.evaluate(IDB_SIZE) / 2**20, 1)
            pg.close()
            # reopening: a new page of the same context (same IndexedDB)
            pg = ctx.new_page()
            t4 = time.perf_counter()
            pg.goto(f"http://127.0.0.1:{port}/?motore=python")
            # LR: a reload goes straight to the data view; HR: it lands on the file list and the student presses "open"
            pg.wait_for_function(f"!!document.querySelector('.pnl.chrom canvas') || document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=600000)
            out["reopen_files_listed_s"] = round(time.perf_counter() - t4, 2)
            if pg.query_selector(".pnl.chrom canvas") is None:
                pg.click("#opbtn")
                pg.wait_for_selector(".pnl.chrom canvas", timeout=600000)
            out["reopen_first_draw_s"] = round(time.perf_counter() - t4, 2)
            out["reopen_files"] = pg.evaluate("fetch('api/state').then(r => r.json()).then(d => (d.files || d.items || []).length)")
            steps = pg.evaluate("window.__steps") or []
            prev = 0
            for text, at in steps:
                out[f"reopen_step[{text}]_s"] = round((at - prev) / 1000, 2); prev = at
            for w in pg.workers:
                if "browser-worker" in w.url:
                    out["pyodide_wasm_mb"] = round(w.evaluate(WASM_MEM) / 2**20, 1)
            out["rss_peak_mb"] = pk.max
            b.close()
    finally:
        srv.terminate()
    return out


def main_velocita(tmp: Path) -> list:
    """The three "many files" scenarios; real data when MZLAB_DATI / the usual folders have them, synthetic otherwise."""
    syn = tmp / "sintetici"
    if not list(Path(MZ).glob("*FullMass*.mzML")):
        subprocess.run([sys.executable, str(ROOT / "tools" / "dati_sintetici.py"), str(syn)], check=True)
        lr_src = sorted(syn.glob("B_FullMass-t*.mzML"))
        hr_src = sorted(syn.glob("HR_DDA-*.mzML"))
        origin = "synthetic data"
    else:
        lr_src = sorted(p for p in Path(MZ).glob("*FullMass*.mzML") if "neg" not in p.name)
        hr_dir = Path(os.environ.get("MZLAB_HRDDA") or Path(MZ).parent / "HRMS")
        hr_src = sorted(hr_dir.glob("*DDA*.mzML")) or sorted((ROOT / "mzlab" / "web" / "esempi").glob("HRMS_*.mzML"))
        origin = "lab data (copies renamed when the folder has fewer files)"
    sets = {"20 Full Scan LR": replicate(lr_src, 20, tmp / "lr"), "9 HR DDA": replicate(hr_src, 9, tmp / "hr")}
    rows, port = [], 8880
    for name, fl in sets.items():
        port += 1
        rows.append(scenario(name, fl, port))
        print(rows[-1], flush=True)
    keys = ["scenario", "files", "file_mb", "engine_start_s", "read_mzml_s", "first_draw_s", "tic_xic_s", "idb_mb"]
    keys2 = ["scenario", "reopen_files_listed_s", "reopen_first_draw_s", "reopen_files", "pyodide_wasm_mb", "rss_peak_mb"]
    step_keys = sorted({k for r in rows for k in r if k.startswith("reopen_step[")})
    return [f"Files: {origin}.", ""] + table(keys, rows) + [""] + table(keys2, rows) + ["", "Reopening, seconds spent between worker messages:"] + table(["scenario"] + step_keys, rows)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--files", type=int, default=4, help="how many Full Scan files to open")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--memoria", action="store_true", help="also the memory breakdown (WASM heaps, IndexedDB, JS heaps, RSS)")
    ap.add_argument("--velocita", action="store_true", help="only the many-files scenarios (20 Full Scan LR, 9 HR DDA): time breakdown, reopening, peak memory")
    ap.add_argument("--scrivi", action="store_true", help="overwrite docs/agenti/misure.md with the tables (date and commit)")
    a = ap.parse_args()
    if not (ROOT / "site" / "index.html").exists():
        sys.exit("site/ missing: python3 tools/build_site.py")
    if a.velocita:
        import tempfile
        lines = main_velocita(Path(tempfile.mkdtemp(prefix="mzlab-bench-")))
        print("\n".join(lines))
        if a.scrivi:
            md = ROOT / "docs" / "agenti" / "misure.md"
            commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
            sec = [f"## Molti file (`python3 tools/bench.py --velocita --scrivi`, motore Python, {time.strftime('%Y-%m-%d')}, commit {commit})",
                   "Apertura = invio dei file, lettura mzML e primo disegno; riapertura = nuova pagina nello stesso profilo (IndexedDB), fino al primo disegno "
                   "(HR: fino alla lista dei file, poi «Apri»). I passi sono i secondi tra i messaggi del worker; «ready» include la rilettura da IndexedDB e l'avvio di `mzlab.browser`."] + lines
            txt = md.read_text(encoding="utf-8")
            txt = txt.split("\n## Molti file")[0].rstrip("\n") + "\n\n" + "\n".join(sec) + "\n"
            md.write_text(txt, encoding="utf-8")
        return
    names = sorted(p for p in Path(MZ).glob("*FullMass*.mzML"))[:a.files]
    if not names:
        sys.exit("no Full Scan files found (MZLAB_MZML)")
    rows = [run(e, [str(x) for x in names], 8840 + i) for i, e in enumerate(("python", "rust"))]
    if a.json:
        print(json.dumps(rows))
        return
    mb = sum(x.stat().st_size for x in names) / 2**20
    out = [f"## Tempi ({len(names)} file Full Scan, {mb:.1f} MB)"] + table(["engine", "start_s", "open_s", "tic_xic_s", "requests", "browser_rss_mb"], rows)
    if a.memoria:
        out += ["", "## Ripartizione della memoria (MB)", *main_memoria()]
    print("\n".join(out))
    if a.scrivi:
        commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
        head = (f"# Misure dei motori (`python3 tools/bench.py --memoria --scrivi`)\nChromium Linux, sito costruito con il motore Rust, "
                f"{time.strftime('%Y-%m-%d')}, commit {commit}, stessa macchina, un giro. «open» = caricamento + primo TIC disegnato; "
                "«TIC+XIC» = TIC e 5 XIC di ogni file. Con `?motore=rust` il motore Python resta caricato e legge comunque i file (Rust risponde solo "
                "a TIC, XIC e scansioni): la memoria è la somma dei due. Memoria dopo l'apertura dei file e le richieste di TIC e XIC: "
                "heap WASM = somma delle `WebAssembly.Memory` del worker; IndexedDB = byte dei file salvati; heap JS della pagina da "
                "`performance.memory` (nei worker non c'è); RSS = somma dei processi Chromium. Rifare la misura: `python3 tools/build_site.py`, "
                "costruire il Rust come in `pages.yml`, poi `python3 tools/bench.py --memoria --scrivi` (file LR: `MZLAB_MZML`; Chromium: `MZLAB_CHROMIUM`).\n\n")
        (ROOT / "docs" / "agenti" / "misure.md").write_text(head + "\n".join(out) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
