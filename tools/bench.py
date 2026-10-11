#!/usr/bin/env python3
"""Measures the two engines of the built site in Chromium: Python (Pyodide) and Rust (?motore=rust), on the same files.

For each engine: time to open the files (upload + first TIC drawn), time for the TIC and five XIC of every file (what the
page asks while a student works) and the resident memory of the whole browser (Linux: sum of /proc RSS of the Chromium processes).
It does not interpret the numbers: AGENTS.md / ARCHITETTURA.md decide what they mean. Needs the site (python3 tools/build_site.py)
and, for Rust, site/static/wasm (see .github/workflows/pages.yml). Usage: python3 tools/bench.py [--files N] [--json] [--memoria]
--memoria: memory breakdown per engine and per file set (6 Full Scan LR, the 4 anonymous HRMS examples): WASM heaps (Pyodide, Rust),
IndexedDB contents, JS heaps and browser RSS. Measure only, no interpretation.
--librerie: the spectral libraries (docs/agenti/librerie.md): synthetic libraries of 1e5 and 1e6 spectra (and the MassBank of the data repository when MZLAB_DATI
finds one), imported and searched through LIB in Chromium with the old JavaScript path (?libjs=1, only up to 1e5) and the WebAssembly engine
(site/static/wasm-lib), and the native builder (cargo example `build`); --gb adds an MGF of about 1 GB. Measure only, no interpretation.
Files: MZLAB_MZML or the usual data folders (tests_e2e/lib.py); Chromium: MZLAB_CHROMIUM = path of an executable, if needed."""
import argparse, json, os, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
from lib import MZ  # noqa: E402

MZS = [200, 250, 300, 364, 400]
GETB = """async u => { const t = performance.now(); const r = await fetch(u); if (!r.ok) throw new Error(u + " " + r.status); await r.arrayBuffer(); return performance.now() - t; }"""
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
w.addEventListener('message',e=>{const d=e.data;if(d&&(d.type==='step'||d.type==='ready'||d.type==='fatal'))window.__steps.push([d.type==='step'?(d.key||d.text):d.type,Math.round(performance.now())])});return w};
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


# ---- scenarios of the .mzidx work: response times, a map with every file, reopening, memory (python engine, the site given by --site) ----
def inflate(src: Path, copies: int, out: Path) -> Path:
    """A big file made of `copies` copies of the spectra of an mzML (ids, indexes, spectrumRef and retention times shifted): a file of the size of a real high-resolution run."""
    import re
    t = src.read_text(encoding="utf-8")
    a, b = t.index("<spectrum "), t.index("</spectrumList>")
    head, body, tail = t[:a], t[a:b], t[b:]
    blocks = re.findall(r"<spectrum .*?</spectrum>", body, re.S)
    n, last = len(blocks), max(int(m) for m in re.findall(r"scan=(\d+)", body))
    rtmax = max(float(m) for m in re.findall(r'accession="MS:1000016"[^>]*value="([0-9.]+)"', body))
    parts = []
    for c in range(copies):
        for bl in blocks:
            bl = re.sub(r'index="(\d+)"', lambda m: f'index="{int(m.group(1)) + c * n}"', bl, count=1)
            bl = re.sub(r"scan=(\d+)", lambda m: f"scan={int(m.group(1)) + c * last}", bl)
            if c:
                bl = re.sub(r'(accession="MS:1000016"[^>]*value=")([0-9.]+)', lambda m: f"{m.group(1)}{float(m.group(2)) + c * (rtmax + 0.1):.6f}", bl)
            parts.append(bl)
    head = re.sub(r'<spectrumList count="\d+"', f'<spectrumList count="{n * copies}"', head)
    out.write_text(head + "".join(parts) + tail, encoding="utf-8")
    return out


def pct(v: list, q: float) -> float:
    v = sorted(v)
    return round(v[min(len(v) - 1, int(q * len(v)))], 1) if v else 0.0


def scenario_mzidx(name: str, files: list[str], port: int, site: Path) -> dict:
    """Open the files, TIC + 5 XIC of each, the map of every file, response times of the three things a student does most, then reopen the page and ask them again."""
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1", "-d", str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    n = len(files)
    out: dict = {"scenario": name, "files": n, "file_mb": round(sum(os.path.getsize(f) for f in files) / 2**20, 1)}
    ALL = ",".join(str(k) for k in range(n))
    def timed(pg, urls, reps=1, fn=GET):
        v = []
        for _ in range(reps):
            for u in urls:
                v.append(pg.evaluate(fn, u))
        return v
    def latencies(pg, tag):
        spec = timed(pg, [f"api/scanbin?k={k}&i0={i}&i1={i + 1}&level=1" for i in (5, 40, 90) for k in range(n)], 1, GETB)
        xic = timed(pg, [f"api/xic?k={ALL}&mz={mz}&tol=0.35&level=1" for mz in MZS], 2)
        sw = timed(pg, [f"api/chrom?k={k}&kind=tic&level=1" for k in list(range(n)) * 2])
        out[f"{tag}spectrum_ms_p50"], out[f"{tag}spectrum_ms_max"] = pct(spec, .5), round(max(spec), 1)
        out[f"{tag}xic_all_files_ms_p50"], out[f"{tag}xic_all_files_ms_max"] = pct(xic, .5), round(max(xic), 1)
        out[f"{tag}file_switch_ms_p50"], out[f"{tag}file_switch_ms_max"] = pct(sw, .5), round(max(sw), 1)
    try:
        with sync_playwright() as p, Peak() as pk:
            # a persistent profile, like a student's browser: a private window has a quota of a few hundred MB, far less than 15 files of 80 MB
            ctx = p.chromium.launch_persistent_context(tempfile.mkdtemp(prefix="mzlab-profile-"), executable_path=os.environ.get("MZLAB_CHROMIUM") or None, args=["--enable-precise-memory-info"], service_workers="block")
            ctx.add_init_script(STEPS)
            ctx.route("**/browser-worker.js", lambda route: (lambda r: route.fulfill(response=r, body=HOOK + r.text(), headers={**r.headers, "content-type": "text/javascript"}))(route.fetch()))
            pg = ctx.new_page()
            pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
            pg.goto(f"http://127.0.0.1:{port}/?motore=python")
            pg.wait_for_selector("#drop", timeout=300000)
            t1 = time.perf_counter()
            pg.set_input_files("#pick", files)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=1200000)
            out["open_files_s"] = round(time.perf_counter() - t1, 2)
            t2 = time.perf_counter()
            pg.click("#opbtn")
            pg.wait_for_selector(".pnl.chrom canvas", timeout=1200000)
            out["first_draw_s"] = round(time.perf_counter() - t2, 2)
            t3 = time.perf_counter()
            for k in range(n):
                pg.evaluate(GET, f"api/chrom?k={k}&kind=tic&level=1")
                for mz in MZS:
                    pg.evaluate(GET, f"api/xic?k={k}&mz={mz}&tol=0.35&level=1")
            out["tic_xic_s"] = round(time.perf_counter() - t3, 2)
            t4 = time.perf_counter()
            for k in range(n):
                pg.evaluate(GET, f"api/map?k={k}&level=1")
            out["map_all_files_s"] = round(time.perf_counter() - t4, 2)
            latencies(pg, "")
            for w in pg.workers:
                if "browser-worker" in w.url:
                    out["wasm_mb_after_work"] = round(w.evaluate(WASM_MEM) / 2**20, 1)
            out["rss_peak_mb_work"] = pk.max
            pg.close()
            pg = ctx.new_page()
            t5 = time.perf_counter()
            pg.goto(f"http://127.0.0.1:{port}/?motore=python")
            pg.wait_for_function(f"!!document.querySelector('.pnl.chrom canvas') || document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=1200000)
            if pg.query_selector(".pnl.chrom canvas") is None:
                pg.click("#opbtn")
                pg.wait_for_selector(".pnl.chrom canvas", timeout=1200000)
            out["reopen_first_draw_s"] = round(time.perf_counter() - t5, 2)
            t6 = time.perf_counter()
            for k in range(n):
                pg.evaluate(GET, f"api/map?k={k}&level=1")
            out["reopen_map_all_files_s"] = round(time.perf_counter() - t6, 2)
            for w in pg.workers:
                if "browser-worker" in w.url:
                    out["wasm_mb_reopened"] = round(w.evaluate(WASM_MEM) / 2**20, 1)
            out["opfs_mb"] = round(pg.evaluate("navigator.storage.estimate().then(e => e.usage)") / 2**20, 1)
            out["rss_peak_mb"] = pk.max
            ctx.close()
    finally:
        srv.terminate()
    return out


def main_mzidx(tmp: Path, site: Path, hr_copies: int, hr_files: int, only: str = "") -> list:
    """The scenarios of the .mzidx work: 20 LR files, N high-resolution files of the size of a real run ("15 files of 80 MB": lab data when MZLAB_DATI has them, otherwise
    the synthetic Orbitrap DDA file repeated up to ~80 MB), the map of every file, reopening, and memory."""
    syn = tmp / "sintetici"
    if not list(Path(MZ).glob("*FullMass*.mzML")):
        subprocess.run([sys.executable, str(ROOT / "tools" / "dati_sintetici.py"), str(syn)], check=True)
        lr_src = sorted(syn.glob("B_FullMass-t*.mzML"))
        hr_base = syn / "HR_DDA-Exploris-t30.mzML"
        origin = "synthetic data"
    else:
        lr_src = sorted(p for p in Path(MZ).glob("*FullMass*.mzML") if "neg" not in p.name)
        hr_dir = Path(os.environ.get("MZLAB_HRDDA") or Path(MZ).parent / "HRMS")
        hr_real = sorted(hr_dir.glob("*DDA*.mzML"))
        hr_base = hr_real[0] if hr_real else ROOT / "mzlab" / "web" / "esempi" / "HRMS_t000.mzML"
        origin = "lab data (LR) " + ("(HR: lab data)" if hr_real else "(HR: synthetic repetition)")
    hr_dir_out = tmp / "hr"
    hr_dir_out.mkdir(parents=True, exist_ok=True)
    hr_src = []
    for i in range(hr_files):
        f = hr_dir_out / f"HR_DDA_big_{i:02d}.mzML"
        if not f.exists():
            inflate(hr_base, hr_copies, f)
        hr_src.append(f)
    sets = {f"20 Full Scan LR": replicate(lr_src, 20, tmp / "lr"), f"{hr_files} HR DDA ({round(hr_src[0].stat().st_size / 2**20)} MB each)": [str(x) for x in hr_src]}
    rows, port = [], 8900
    for name, fl in sets.items():
        port += 1
        if only and only not in name.lower().split()[1:3] and only not in name.lower():
            continue
        rows.append(scenario_mzidx(name, fl, port, site))
        print(rows[-1], flush=True)
    k1 = ["scenario", "files", "file_mb", "open_files_s", "first_draw_s", "tic_xic_s", "map_all_files_s", "reopen_first_draw_s", "reopen_map_all_files_s"]
    k2 = ["scenario", "spectrum_ms_p50", "spectrum_ms_max", "xic_all_files_ms_p50", "xic_all_files_ms_max", "file_switch_ms_p50", "file_switch_ms_max"]
    k3 = ["scenario", "wasm_mb_after_work", "wasm_mb_reopened", "opfs_mb", "rss_peak_mb_work", "rss_peak_mb"]
    return [f"Files: {origin}.", ""] + table(k1, rows) + [""] + table(k2, rows) + [""] + table(k3, rows)


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


# ---- spectral libraries ----
def synth_library(path: Path, n: int, fmt: str = "msp", seed: int = 7, n_q: int = 5000) -> tuple[list, list]:
    """Writes n SYNTHETIC spectra (8-40 peaks, 30% negative, no compound names) and returns (identity queries, analogue queries) cut from them."""
    import numpy as np
    rng = np.random.default_rng(seed)
    want = set(rng.choice(n, size=min(n, n_q), replace=False).tolist())
    ident, analog = [], []
    with open(path, "w", encoding="utf-8", buffering=1 << 20) as f:
        for b0 in range(0, n, 20000):
            b1 = min(n, b0 + 20000)
            m = b1 - b0
            prec = rng.uniform(150, 900, m).round(4)
            npk = rng.integers(8, 41, m)
            neg = rng.random(m) < 0.3
            out = []
            for k in range(m):
                i = b0 + k
                top = prec[k] - 3.0
                mz = np.sort(rng.uniform(50, top, npk[k])).round(4)
                it = (rng.pareto(1.5, npk[k]) * 1000 + 50).round(1)
                pol = -1 if neg[k] else 1
                if fmt == "msp":
                    head = f"Name: Synthetic {i}\nPrecursorMZ: {prec[k]}\nPrecursor_type: {'[M-H]-' if neg[k] else '[M+H]+'}\nIon_mode: {'Negative' if neg[k] else 'Positive'}\nCollision_energy: 30\nInstrument_type: Orbitrap\nNum Peaks: {npk[k]}\n"
                    out.append(head + "\n".join(f"{a} {c}" for a, c in zip(mz, it)) + "\n\n")
                else:
                    head = f"BEGIN IONS\nNAME=Synthetic {i}\nPEPMASS={prec[k]}\nCHARGE={'1-' if neg[k] else '1+'}\nIONMODE={'Negative' if neg[k] else 'Positive'}\n"
                    out.append(head + "\n".join(f"{a} {c}" for a, c in zip(mz, it)) + "\nEND IONS\n\n")
                if i in want:
                    q = {"prec": float(prec[k]) + float(rng.normal(0, 0.0005)), "pol": pol,
                         "peaks": [[float(a) + float(rng.normal(0, 0.002)), float(c) * float(rng.uniform(0.7, 1.3))] for a, c in zip(mz, it)]}
                    ident.append({**q, "key": i, "rt": 0})
                    d = 14.01565
                    sel = rng.random(len(mz)) < 0.5
                    analog.append({"prec": float(prec[k]) + d, "pol": pol, "key": i, "rt": 0,
                                   "peaks": [[float(a) + (d if s else 0.0), float(c)] for a, c, s in zip(mz, it, sel)]})
            f.write("".join(out))
    return ident, analog


LIB_ADD = """async () => { const f = document.getElementById('benchfile').files[0]; const t0 = performance.now(); const r = await LIB.call('add', { file: f }); return { ms: performance.now() - t0, info: r.info }; }"""
LIB_IDENT = """async ([qs, reps]) => { const t = []; for (let k = 0; k < reps; k++) { const q = qs[k % qs.length], t0 = performance.now(); await LIB.call('search', { peaks: q.peaks, prec: q.prec, pol: q.pol, tol: { v: 10, unit: 'ppm' }, frag: 0.01 }); t.push(performance.now() - t0); } return t; }"""
LIB_AN = """async ([qs, reps]) => { const t = []; for (let k = 0; k < reps; k++) { const q = qs[k % qs.length], t0 = performance.now(); await LIB.call('analogs', { peaks: q.peaks, prec: q.prec, pol: q.pol, frag: 0.01, maxDelta: 200, max: 10 }); t.push(performance.now() - t0); } return t; }"""
LIB_ALL = """async qs => { const t0 = performance.now(); const r = await LIB.call('searchAll', { queries: qs, tol: { v: 10, unit: 'ppm' }, frag: 0.01 }); return { ms: performance.now() - t0, named: r.rows.filter(x => x.name).length }; }"""


def _med(v: list, q: float = 0.5) -> float:
    v = sorted(v)
    return round(v[min(len(v) - 1, int(len(v) * q))], 2) if v else 0.0


def lib_scenario(engine: str, path: Path, ident: list, analog: list, port: int, label: str, n_all: int = 5000) -> dict:
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    out: dict = {"library": label, "engine": engine, "file_mb": round(path.stat().st_size / 2**20, 1)}
    try:
        with sync_playwright() as p, Peak() as pk:
            # a persistent profile: the storage quota of a private context is too small for libraries of 1e6 spectra
            import tempfile
            b = ctx = p.chromium.launch_persistent_context(tempfile.mkdtemp(prefix="mzlab-bench-prof-"), executable_path=os.environ.get("MZLAB_CHROMIUM") or None, args=["--enable-precise-memory-info"], service_workers="block")
            pg = ctx.new_page()
            pg.goto(f"http://127.0.0.1:{port}/" + ("?libjs=1" if engine == "js" else ""))
            pg.wait_for_function("window.LIB !== undefined && LIB.engine !== undefined", timeout=120000)
            has = pg.evaluate("LIB.engine.info().then(e => e.wasm)")
            if (engine == "wasm") != bool(has):
                out["error"] = "engine not available"; b.close(); return out
            pg.evaluate("(() => { const i = document.createElement('input'); i.type = 'file'; i.id = 'benchfile'; document.body.appendChild(i); })()")
            pg.set_input_files("#benchfile", str(path))
            time.sleep(1)
            base = rss_mb()
            r = pg.evaluate(LIB_ADD)
            out["import_s"] = round(r["ms"] / 1000, 1)
            out["spectra"] = r["info"]["n"]
            out["rss_peak_import_delta_mb"] = round(max(pk.max - base, 0), 0)
            est = pg.evaluate("navigator.storage.estimate().then(e => e.usage)")
            out["disk_mb"] = round((r["info"].get("bytes") or est) / 2**20, 1)
            out["disk_mb_origin"] = round(est / 2**20, 1)
            t = pg.evaluate(LIB_IDENT, [ident, 1 + 30])
            out["ident_cold_ms"], out["ident_ms"] = round(t[0], 1), _med(t[1:])
            t = pg.evaluate(LIB_AN, [analog, 1 + 10])
            out["analogs_cold_ms"], out["analogs_ms"] = round(t[0], 1), _med(t[1:])
            if n_all:
                a = pg.evaluate(LIB_ALL, ident[:n_all])
                out["identify_all_s"], out["identify_all_n"], out["identify_all_named"] = round(a["ms"] / 1000, 1), min(len(ident), n_all), a["named"]
            out["rss_peak_mb"] = pk.max
            b.close()
    finally:
        srv.terminate()
    return out


def native_build(path: Path) -> dict:
    r = subprocess.run(["cargo", "run", "--release", "--quiet", "-p", "mzlab-lib", "--example", "build", "--", str(path), str(path) + ".mzlib"], cwd=ROOT, capture_output=True, text=True)
    import re
    m = re.search(r"(\d+) spectra .*?([\d.]+) MB -> ([\d.]+) MB.*?total ([\d.]+)s; peak RSS (\d+) MB", r.stdout)
    try: os.remove(str(path) + ".mzlib")
    except OSError: pass
    return {"library": path.name, "engine": "native", "file_mb": float(m.group(2)) if m else None, "spectra": int(m.group(1)) if m else None, "disk_mb": float(m.group(3)) if m else None,
            "import_s": float(m.group(4)) if m else None, "rss_peak_mb": int(m.group(5)) if m else None, "error": None if m else r.stderr[-300:]}


def main_librerie(gb: bool) -> list:
    import tempfile
    tmp = Path(tempfile.mkdtemp(prefix="mzlab-bench-lib-"))
    rows, nat = [], []
    if not (ROOT / "site" / "static" / "wasm-lib" / "mzlab_lib_wasm.js").exists():
        print("site/static/wasm-lib missing: build it as in .github/workflows/pages.yml (the WebAssembly rows will be empty)", file=sys.stderr)
    jobs = []
    for n, label in ((100_000, "synthetic 1e5"), (1_000_000, "synthetic 1e6")):
        path = tmp / f"sint_{n}.msp"
        print(f"writing {label} ...", file=sys.stderr)
        ident, analog = synth_library(path, n)
        jobs.append((label, path, ident, analog, n <= 100_000))
    dati = next((Path(c) for c in (os.environ.get("MZLAB_DATI"), str(ROOT.parent / "mzlab-dati")) if c and (Path(c) / "librerie" / "massbank_eu_qft_pos.msp").is_file()), None)
    if dati:
        mb = dati / "librerie" / "massbank_eu_qft_pos.msp"
        ident = []
        jobs.append(("MassBank (data repository)", mb, None, None, True))
    port = 8860
    for label, path, ident, analog, with_js in jobs:
        if ident is None:                       # real library: queries cut from its own first spectra
            ident, analog = _queries_from_msp(path)
        for eng in (["js"] if with_js else []) + ["wasm"]:
            print(f"{label}: {eng} ...", file=sys.stderr)
            rows.append(lib_scenario(eng, path, ident, analog, port, label)); port += 1
        nat.append(native_build(path))
    if gb:
        path = tmp / "sint_1gb.mgf"
        print("writing the 1 GB MGF ...", file=sys.stderr)
        n = 2_100_000
        ident, analog = synth_library(path, n, fmt="mgf")
        while path.stat().st_size < 1.0e9:
            n = int(n * 1.1); ident, analog = synth_library(path, n, fmt="mgf")
        rows.append(lib_scenario("wasm", path, ident, analog, port, f"synthetic MGF {path.stat().st_size / 1e9:.2f} GB", n_all=0)); port += 1
        nat.append(native_build(path))
    for f in tmp.iterdir():
        try: f.unlink()
        except OSError: pass
    keys = ["library", "engine", "spectra", "file_mb", "disk_mb", "import_s", "ident_cold_ms", "ident_ms", "analogs_cold_ms", "analogs_ms", "identify_all_s", "identify_all_named", "rss_peak_import_delta_mb", "rss_peak_mb"]
    return table(keys, rows) + ["", "Native builder (cargo example build; the memory is the peak RSS of the process):"] + table(["library", "engine", "spectra", "file_mb", "disk_mb", "import_s", "rss_peak_mb"], nat)


def _queries_from_msp(path: Path, n: int = 400) -> tuple[list, list]:
    """Queries from the first spectra of a real MSP library: the spectrum itself (identity) and a copy shifted by 14.01565 on half of the peaks (analogue)."""
    ident, analog, cur = [], [], None
    for line in open(path, encoding="utf-8", errors="replace"):
        t = line.strip()
        if not t:
            if cur and cur["peaks"] and cur["prec"]:
                ident.append({**cur, "key": len(ident), "rt": 0})
                d = 14.01565
                analog.append({"prec": cur["prec"] + d, "pol": cur["pol"], "key": len(analog), "rt": 0, "peaks": [[m + (d if k % 2 else 0.0), i] for k, (m, i) in enumerate(cur["peaks"])]})
                if len(ident) >= n:
                    break
            cur = None
            continue
        cur = cur or {"prec": 0.0, "pol": 1, "peaks": []}
        if t.lower().startswith("precursormz:"):
            cur["prec"] = float(t.split(":", 1)[1])
        elif t.lower().startswith("ion_mode:"):
            cur["pol"] = -1 if t.split(":", 1)[1].strip().lower().startswith("n") else 1
        elif t[0].isdigit() and ":" not in t:
            a = t.replace(";", " ").split()
            if len(a) >= 2:
                cur["peaks"].append([float(a[0]), float(a[1])])
    return ident, analog


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--librerie", action="store_true", help="the spectral libraries: import, disk, identity, analogues, identify all (see the module doc)")
    ap.add_argument("--gb", action="store_true", help="with --librerie: also an MGF of about 1 GB")
    ap.add_argument("--files", type=int, default=4, help="how many Full Scan files to open")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--memoria", action="store_true", help="also the memory breakdown (WASM heaps, IndexedDB, JS heaps, RSS)")
    ap.add_argument("--velocita", action="store_true", help="only the many-files scenarios (20 Full Scan LR, 9 HR DDA): time breakdown, reopening, peak memory")
    ap.add_argument("--mzidx", action="store_true", help="the .mzidx scenarios (20 LR, N big HR files): response times, map of every file, reopening, memory; prints a markdown table")
    ap.add_argument("--site", default=str(ROOT / "site"), help="folder of the built site to measure (default site/)")
    ap.add_argument("--solo", default="", help="--mzidx: only the scenario whose name contains this word (lr or hr)")
    ap.add_argument("--hr-files", type=int, default=15, help="--mzidx: how many big high-resolution files")
    ap.add_argument("--hr-copies", type=int, default=20, help="--mzidx: how many times the base HR file is repeated inside each (20 x 4 MB ~ 80 MB)")
    ap.add_argument("--scrivi", action="store_true", help="overwrite docs/agenti/misure.md with the tables (date and commit)")
    a = ap.parse_args()
    if not (Path(a.site) / "index.html").exists():
        sys.exit("site/ missing: python3 tools/build_site.py")
    if a.librerie:
        lines = main_librerie(a.gb)
        print("\n".join(lines))
        if a.scrivi:
            md = ROOT / "docs" / "agenti" / "misure.md"
            commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
            sec = [f"## Librerie spettrali (`python3 tools/bench.py --librerie{' --gb' if a.gb else ''} --scrivi`, Chromium Linux, {time.strftime('%Y-%m-%d')}, commit {commit})"] + lines
            txt = md.read_text(encoding="utf-8")
            txt = txt.split("\n## Librerie spettrali")[0].rstrip("\n") + "\n\n" + "\n".join(sec) + "\n"
            md.write_text(txt, encoding="utf-8")
        return
    if a.mzidx:
        import tempfile
        print("\n".join(main_mzidx(Path(tempfile.mkdtemp(prefix="mzlab-bench-")), Path(a.site), a.hr_copies, a.hr_files, a.solo.lower())))
        return
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
