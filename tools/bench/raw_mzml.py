"""Launcher of the `.raw` -> mzML bench (`crates/mzlab-core/examples/raw_mzml_banco`): builds it, measures the disk with `dd`, runs it on one input and
leaves the JSON of MISURE §4 in `.verifica/raw-mzml/`. No calculation here: every number comes from the Rust bench.
  python3 tools/bench/raw_mzml.py --file X.raw            a real file
  python3 tools/bench/raw_mzml.py --denso 2               the dense synthetic case, 2 GB
Options after `--` go to the bench as they are (`-- --solo scala --thread 1,4`). `--zlib-rs` also runs the build on the zlib-rs backend."""
import argparse
import json
import platform
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / ".verifica" / "raw-mzml"
EXE = ROOT / "target" / "cli" / "examples" / "raw_mzml_banco"


def sh(cmd, **kw):
    return subprocess.run(cmd, text=True, capture_output=True, cwd=ROOT, **kw)


def build(features):
    cmd = ["cargo", "build", "--profile", "cli", "-p", "mzlab-core", "--example", "raw_mzml_banco"]
    if features:
        cmd += ["--features", features]
    r = subprocess.run(cmd, cwd=ROOT)
    if r.returncode:
        sys.exit("build failed")


def disco(tmp, mb=1024):
    """Sequential write then read of `mb` MB with dd (page cache dropped before the read when allowed); MB/s or None."""
    f = Path(tmp) / "raw_mzml_dd.tmp"
    out = {}
    for nome, cmd in (("scrittura", f"dd if=/dev/zero of={f} bs=1M count={mb} conv=fdatasync"), ("lettura", f"dd if={f} of=/dev/null bs=1M")):
        if nome == "lettura":
            sh(["sh", "-c", "sync; echo 3 > /proc/sys/vm/drop_caches"])
        t = time.perf_counter()
        r = sh(["sh", "-c", cmd])
        s = time.perf_counter() - t
        out[nome] = round(mb / s, 1) if r.returncode == 0 and s > 0 else None
    f.unlink(missing_ok=True)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--file")
    ap.add_argument("--denso", type=float, help="GB of the dense synthetic case")
    ap.add_argument("--etichetta", default="")
    ap.add_argument("--giri", type=int, default=5)
    ap.add_argument("--tmp", default="/tmp")
    ap.add_argument("--zlib-rs", action="store_true")
    ap.add_argument("--senza-disco", action="store_true", help="skip the dd measurement")
    ap.add_argument("resto", nargs="*", help="options for the bench (after --)")
    a = ap.parse_args()
    if bool(a.file) == bool(a.denso):
        ap.error("exactly one of --file / --denso")
    OUT.mkdir(parents=True, exist_ok=True)
    bw = {} if a.senza_disco else disco(a.tmp)
    meta = {
        "git": sh(["git", "rev-parse", "HEAD"]).stdout.strip(),
        "rustc": sh(["rustc", "--version"]).stdout.strip(),
        "kernel": platform.release(),
        "profilo": "cli",
        "data": time.strftime("%Y-%m-%d"),
    }
    for backend, feat in (("miniz_oxide", ""),) + ((("zlib-rs", "banco-zlib-rs"),) if a.zlib_rs else ()):
        build(feat)
        etichetta = a.etichetta or (Path(a.file).stem if a.file else f"denso-{a.denso}gb")
        dest = OUT / f"{etichetta}.{backend}.json"
        cmd = [str(EXE), "--giri", str(a.giri), "--etichetta", etichetta, "--json", str(dest), "--tmp", a.tmp, "--meta", f"backend={backend}"]
        cmd += ["--file", a.file] if a.file else ["--denso", str(Path(a.tmp) / "raw_mzml_denso"), "--gb", str(a.denso), "--cancella"]
        for k, v in {**meta, "bw_lettura_mb_s": bw.get("lettura"), "bw_scrittura_mb_s": bw.get("scrittura")}.items():
            cmd += ["--meta", f"{k}={v}"]
        if bw.get("lettura"):
            cmd += ["--bw-lettura", str(bw["lettura"])]
        if bw.get("scrittura"):
            cmd += ["--bw-scrittura", str(bw["scrittura"])]
        cmd += a.resto
        subprocess.run(cmd, cwd=ROOT, check=True)
        print(json.dumps({"scritto": str(dest.relative_to(ROOT))}))


if __name__ == "__main__":
    main()
