#!/usr/bin/env python3
"""Measures the Rust engine of the built site in Chromium, on the same files.

Time to open the files (upload + first TIC drawn), time for the TIC and five XIC of every file (what the page asks while a student
works) and the resident memory of the whole browser (Linux: sum of /proc RSS of the Chromium processes). It does not interpret the
numbers: AGENTS.md decides what they mean. Needs the site (python3 tools/build_site.py) and site/static/wasm (see .github/workflows/pages.yml).
Usage: python3 tools/bench.py [--files N] [--json]
--librerie: the spectral libraries (docs/agenti/librerie.md): synthetic libraries of 1e5 and 1e6 spectra (and the MassBank of the data repository when it is found
by tools/dati_percorsi.py), imported and searched through LIB in Chromium with the old JavaScript path (?libjs=1, only up to 1e5) and the WebAssembly engine
(site/static/wasm-lib), and the native builder (cargo example `build`); --gb adds an MGF of about 1 GB. Measure only, no interpretation.
Files: MZLAB_MZML or the usual data folders (tests_e2e/lib.py); Chromium: MZLAB_CHROMIUM = path of an executable, if needed."""
import argparse, json, os, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
from lib import MZ  # noqa: E402
from dati_percorsi import percorso  # noqa: E402

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
            pg.goto(f"http://127.0.0.1:{port}/")
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


def table(keys: list, rows: list) -> list:
    return ["| " + " | ".join(keys) + " |", "|" + "---|" * len(keys)] + ["| " + " | ".join(str(r.get(k, "")) for k in keys) + " |" for r in rows]


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
    mb = percorso("dati/librerie/massbank_eu_qft_pos.msp")
    if mb:
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
    ap.add_argument("--site", default=str(ROOT / "site"), help="folder of the built site to measure (default site/)")
    ap.add_argument("--scrivi", action="store_true", help="with --librerie: overwrite its section of docs/agenti/misure.md (date and commit)")
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
    names = sorted(p for p in Path(MZ).glob("*FullMass*.mzML"))[:a.files]
    if not names:
        sys.exit("no Full Scan files found (MZLAB_MZML)")
    rows = [run("rust", [str(x) for x in names], 8840)]
    if a.json:
        print(json.dumps(rows))
        return
    mb = sum(x.stat().st_size for x in names) / 2**20
    print("\n".join([f"## Tempi ({len(names)} file Full Scan, {mb:.1f} MB)"] + table(["engine", "start_s", "open_s", "tic_xic_s", "requests", "browser_rss_mb"], rows)))


if __name__ == "__main__":
    main()
