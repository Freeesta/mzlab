#!/usr/bin/env python3
"""Permanent bench of the mzlab-core jobs compiled to wasm in several build variants (docs/agenti/wasm-potenziale.md §9).

    python3 tools/banco_wasm.py --varianti base,o3 [--chromium] [--firefox] [--webkit] [--giri 5] [--raw] [--lavoro CARTELLA]

Builds the crate `tools/banco_wasm/` once per variant (own CARGO_TARGET_DIR, wasm-bindgen --target web, wasm-opt through
tools/wasm_opt.sh, so the binary and the flags are the site's), runs `js/suite.js` in Node and with --chromium/--firefox/--webkit in that browser,
prints one JSON line per variant and the table base -> variant. Every job returns a check value: the run fails when two
variants (or Node and Chromium) differ. Python only orchestrates; the numbers come from the wasm. All the work files live in
a temporary folder (or --lavoro), never in the repository.
"""
import argparse
import gzip
import json
import os
import shutil
import subprocess
import sys
import tempfile
import signal
import socket
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BANCO = ROOT / "tools" / "banco_wasm"
sys.path.insert(0, str(ROOT / "tools"))
import dati_percorsi  # noqa: E402

MEM = "-C link-arg=--initial-memory=33554432 -C link-arg=--max-memory=4294967296"
SIMD = "-C target-feature=+simd128"
# name -> (cargo profile, extra rustflags, features, wasm-opt level)
VARIANTI = {
    "base": ("release", "", "", "-Oz"),
    "o3": ("o3", "", "", "-Oz"),
    "o3simd": ("o3", SIMD, "", "-Oz"),
    "o3simd_zrs": ("o3", SIMD, "zlibrs", "-Oz"),
    "mix": ("mix", "", "", "-Oz"),
    "mix_zrs": ("mix", "", "zlibrs", "-Oz"),
    "sol_core": ("sol_core", "", "", "-Oz"),
    "sol_miniz": ("sol_miniz", "", "", "-Oz"),
    "sol_xml": ("sol_xml", "", "", "-Oz"),
    "sol_b64": ("sol_b64", "", "", "-Oz"),
    "sol_deps": ("sol_deps", "", "", "-Oz"),
    "sol_zrs": ("sol_zrs", "", "zlibrs", "-Oz"),
    # the site as built (z + mzlab-core at 3; picchi is not in the bench crate) with -Oz, and the same rustc output with -O3 (tools/wasm_opt.sh)
    "site": ("sol_core", "", "", "-Oz"),
    "site_o3": ("sol_core", "", "", "-O3"),
    # the site with zlib-rs (as shipped) before and after the parsing crates at 3, both with the site's -O3
    "site_z": ("sol_core", "", "zlibrs", "-O3"),
    "site_d": ("site_d", "", "zlibrs", "-O3"),
    # the site (site_z) plus ONE parsing crate at 3: the per-crate choice of the profile
    "d_xml": ("d_xml", "", "zlibrs", "-O3"),
    "d_b64": ("d_base64", "", "zlibrs", "-O3"),
    "d_zrs": ("d_zrs", "", "zlibrs", "-O3"),
    "d_miniz": ("d_miniz", "", "zlibrs", "-O3"),
}
HR_SCANS, HR_PEAKS = 1200, 400   # the 87 MB profile mzML of the study
# jobs in the order of the table; the label is the row of §3 of the study
LAVORI = [
    ("parse_hr", "mzML HR read"), ("index_hr", "mzidx HR"), ("xic200_hr", "200 XIC from the HR index"),
    ("centroid_hr", "centroids 100 HR scans"), ("pick50_hr", "50 XIC + peaks HR"), ("entropy300_hr", "entropy 300x300 MS2"),
    ("parse_lr", "mzML LR read"), ("index_lr", "mzidx LR"), ("xic200_lr", "200 XIC from the LR index"),
    ("pick50_lr", "50 XIC + peaks (AsLS) LR"), ("compose200", "200 compositions"),
    ("parse_raw", ".raw decode"), ("index_raw", ".raw mzidx (build_lettore)"), ("inflate64MB", "inflate 64 MB"),
]


def run(cmd, **kw):
    """Run a command, stop with its output when it fails."""
    r = subprocess.run(cmd, capture_output=True, text=True, **kw)
    if r.returncode:
        sys.exit(f"FAILED: {' '.join(map(str, cmd))}\n{r.stdout[-2000:]}\n{r.stderr[-3000:]}")
    return r.stdout


def lock_version(name: str) -> str:
    lines = (ROOT / "Cargo.lock").read_text().splitlines()
    i = lines.index(f'name = "{name}"')
    return lines[i + 1].split('"')[1]


def controlla_strumenti():
    for t in ("cargo", "node", "wasm-bindgen"):
        if not shutil.which(t):
            sys.exit(f"missing tool: {t} (wasm-bindgen: cargo install wasm-bindgen-cli --version {lock_version('wasm-bindgen')} --locked)")
    v = lock_version("wasm-bindgen")
    if v not in run(["wasm-bindgen", "--version"]):
        sys.exit(f"wasm-bindgen must be {v} (Cargo.lock): cargo install wasm-bindgen-cli --version {v} --locked --force")


def trova_raw() -> Path | None:
    """A Thermo .raw of the private data repository, None without it."""
    cartella = dati_percorsi.percorso("dati/HRMS")
    if cartella is None:
        return None
    return next(iter(sorted(cartella.rglob("*.raw"))), None)


def prepara_dati(lav: Path, raw: Path | None) -> Path:
    """The two mzML of the study (seeded, so the same bytes every time) and the .raw if any, in lav/data."""
    d = lav / "data"
    d.mkdir(exist_ok=True)
    if not (d / "hr_prof.mzML").exists():
        env = {**os.environ, "CARGO_TARGET_DIR": str(lav / "tgt" / "host")}
        run(["cargo", "run", "-q", "--release", "--example", "gen", "--", str(d / "hr_prof.mzML"), str(HR_SCANS), str(HR_PEAKS)], cwd=BANCO, env=env)
    if not (d / "B_FullMass-t15.mzML").exists():
        sint = lav / "sint"
        run([sys.executable, str(ROOT / "tools" / "dati_sintetici.py"), str(sint)])
        shutil.copy(sint / "B_FullMass-t15.mzML", d / "B_FullMass-t15.mzML")
        shutil.rmtree(sint)
    if raw is not None:
        shutil.copy(raw, d / "x.raw")
    return d


def costruisci(nome: str, lav: Path, thermo: bool) -> dict:
    profilo, rf, feat, livello = VARIANTI[nome]
    feat = ",".join(f for f in (feat, "thermo" if thermo else "") if f)
    env = {**os.environ, "CARGO_TARGET_DIR": str(lav / "tgt" / nome), "RUSTFLAGS": f"{MEM} {rf}".strip()}
    cmd = ["cargo", "build", "-q", "--profile", profilo, "--target", "wasm32-unknown-unknown"] + (["--features", feat] if feat else [])
    run(cmd, cwd=BANCO, env=env)
    out = lav / "v" / nome
    shutil.rmtree(out, ignore_errors=True)
    out.mkdir(parents=True)
    wasm = lav / "tgt" / nome / "wasm32-unknown-unknown" / ("release" if profilo == "release" else profilo) / "wbench.wasm"
    run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(out), str(wasm)])
    bg = out / "wbench_bg.wasm"
    crudo = bg.stat().st_size
    # the site's own wasm-opt (npm wasm-opt, flags of tools/wasm_opt.sh); the size gate is set out of reach
    run(["bash", str(ROOT / "tools" / "wasm_opt.sh"), str(bg), str(2**40), nome], env={**os.environ, "RUNNER_TEMP": str(lav / "tmp"), "MZLAB_WASM_OPT_LEVEL": livello})
    dati = bg.read_bytes()
    return {"variant": nome, "raw_bytes": crudo, "opt_bytes": len(dati), "gzip_bytes": len(gzip.compress(dati, 9))}


def in_node(nome: str, lav: Path, giri: int) -> dict:
    for f in ("suite.js", "node_bench.mjs"):
        shutil.copy(BANCO / "js" / f, lav / f)
    return json.loads(run(["node", str(lav / "node_bench.mjs"), str(lav / "v" / nome), str(giri), str(lav / "data")]).strip().splitlines()[-1])


def in_browser(motore: str, nomi: list[str], lav: Path, giri: int) -> dict:
    """Run the suite of every variant in Chromium, Firefox or WebKit (compileStreaming) from a local server; its PID is saved and it is stopped by that PID."""
    from playwright.sync_api import sync_playwright
    for f in ("suite.js", "run.html"):
        shutil.copy(BANCO / "js" / f, lav / f)
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        porta = s.getsockname()[1]
    srv = subprocess.Popen([sys.executable, "-m", "http.server", str(porta), "--bind", "127.0.0.1", "--directory", str(lav)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    pidfile = lav / "server.pid"
    pidfile.write_text(str(srv.pid))
    risultati = {}
    try:
        time.sleep(1)
        with sync_playwright() as p:
            try:
                br = getattr(p, motore).launch()
            except Exception:
                if motore != "chromium":
                    raise   # Playwright of another build than the installed browsers: use the system Chromium if there is one
                br = p.chromium.launch(executable_path=os.environ.get("CHROMIUM_PATH", "/opt/pw-browsers/chromium"))
            for n in nomi:
                pg = br.new_page()
                pg.goto(f"http://127.0.0.1:{porta}/run.html?v={n}&reps={giri}")
                pg.wait_for_function("window.RESULT", timeout=3_600_000)
                risultati[n] = pg.evaluate("window.RESULT")
                pg.close()
            risultati["_versione"] = br.version
            br.close()
    finally:
        os.kill(int(pidfile.read_text()), signal.SIGTERM)
        srv.wait()
        pidfile.unlink(missing_ok=True)
    return risultati


def confronta(righe: list[dict], motore: str) -> list[str]:
    """Check values must be bit for bit the same in every variant: returns the differences found."""
    errori = []
    rif = righe[0]
    for r in righe[1:]:
        a, b = rif[motore].get("check", {}), r[motore].get("check", {})
        for lavoro in sorted(set(a) | set(b)):
            if a.get(lavoro) != b.get(lavoro):
                errori.append(f"{motore}: {lavoro} differs between {rif['variant']} ({a.get(lavoro)!r}) and {r['variant']} ({b.get(lavoro)!r})")
    return errori


def tabella(righe: list[dict], motore: str):
    base = righe[0]
    print(f"\n{motore} (ms, median; ratio = base / variant)")
    print("| job | " + " | ".join(r["variant"] for r in righe) + " |")
    print("|---|" + "---|" * len(righe))
    for k, label in LAVORI:
        if k not in base[motore].get("ms", {}):
            continue
        celle = [f"{base[motore]['ms'][k]:g}"] + [f"{r[motore]['ms'][k]:g} (x{base[motore]['ms'][k] / r[motore]['ms'][k]:.2f})" if r[motore]["ms"][k] else "-" for r in righe[1:]]
        print(f"| {label} | " + " | ".join(celle) + " |")
    print("| size (opt / gzip) | " + " | ".join(f"{r['opt_bytes']} / {r['gzip_bytes']}" for r in righe) + " |")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--varianti", default="base,o3", help="comma list among: " + ", ".join(VARIANTI) + " (the first is the base of the table)")
    ap.add_argument("--chromium", action="store_true", help="also run the suite in Chromium (Playwright)")
    ap.add_argument("--firefox", action="store_true", help="also run the suite in Firefox (Playwright)")
    ap.add_argument("--webkit", action="store_true", help="also run the suite in WebKit (Playwright)")
    ap.add_argument("--giri", type=int, default=5, help="repetitions per job, median kept")
    ap.add_argument("--raw", action="store_true", help="also the .raw jobs, with a file of the private data repository (skipped without it)")
    ap.add_argument("--lavoro", help="work folder kept between runs (default: a temporary one, removed at the end)")
    a = ap.parse_args()
    nomi = a.varianti.split(",")
    for n in nomi:
        if n not in VARIANTI:
            sys.exit(f"unknown variant {n}")
    controlla_strumenti()
    raw = trova_raw() if a.raw else None
    if a.raw and raw is None:
        print("no .raw in the private data repository: the .raw jobs are skipped", file=sys.stderr)
    lav = Path(a.lavoro) if a.lavoro else Path(tempfile.mkdtemp(prefix="banco_wasm_"))
    lav.mkdir(parents=True, exist_ok=True)
    try:
        prepara_dati(lav, raw)
        righe = []
        for n in nomi:
            r = costruisci(n, lav, raw is not None)
            r["node"] = in_node(n, lav, a.giri)
            righe.append(r)
        motori = [m for m in ("chromium", "firefox", "webkit") if getattr(a, m)]
        for m in motori:
            br = in_browser(m, nomi, lav, a.giri)
            for r in righe:
                r[m] = br[r["variant"]]
                if "error" in r[m]:
                    sys.exit(f"{m} {r['variant']}: {r[m]['error']}")
            print(f"{m} {br['_versione']}", file=sys.stderr)
        for r in righe:
            print(json.dumps(r))
        errori = confronta(righe, "node")
        for m in motori:
            errori += confronta(righe, m)
            for r in righe:
                for k, v in r["node"]["check"].items():
                    if r[m]["check"].get(k) != v:
                        errori.append(f"{r['variant']}: {k} differs between Node ({v!r}) and {m} ({r[m]['check'].get(k)!r})")
        for m in ["node"] + motori:
            tabella(righe, m)
        if errori:
            sys.exit("CHECK VALUES DIFFER:\n" + "\n".join(errori))
        print(f"\ncheck values identical in {len(righe)} variants")
    finally:
        if not a.lavoro:
            shutil.rmtree(lav, ignore_errors=True)


if __name__ == "__main__":
    main()
