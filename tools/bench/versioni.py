"""Exact versions of every tool of the benchmark (`versioni.json`, kept in the private repository next to the results).

    python3 tools/bench/versioni.py OUT.json [--mzmine DIR --java-home DIR]"""
from __future__ import annotations

import argparse
import json
import platform
import re
import shutil
import subprocess
import sys
from importlib import metadata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def _out(cmd: list[str], env=None) -> str:
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=300)
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable"
    return "\n".join(l for l in (r.stdout + r.stderr).splitlines() if not l.startswith("Picked up")).strip() or "unavailable"


def raccogli(mzmine: str | None = None, java_home: str | None = None) -> dict:
    v = {"python": platform.python_version(), "sistema": platform.platform()}
    for nome, pacchetto in (("pyopenms", "pyopenms"), ("asari", "asari-metabolomics"), ("numpy", "numpy")):
        try:
            v[nome] = metadata.version(pacchetto)
        except metadata.PackageNotFoundError:
            v[nome] = "unavailable"
    if shutil.which("Rscript"):
        v["R"] = _out(["Rscript", "-e", "cat(as.character(getRversion()))"])
        for pk in ("xcms", "CAMERA", "MSnbase", "mzR", "BiocManager"):
            v[pk] = _out(["Rscript", "-e", f"cat(tryCatch(as.character(packageVersion('{pk}')), error=function(e) 'unavailable'))"])
    if mzmine and java_home:
        env = {"JAVA_HOME": java_home, "PATH": f"{java_home}/bin:/usr/bin:/bin"}
        v["mzmine"] = (re.search(r"Starting MZmine ([\d.]+)", _out([str(Path(mzmine) / "bin" / "MZmine"), "-h"], env)) or [None, "unavailable"])[1]
        v["java"] = _out([f"{java_home}/bin/java", "-version"], env).splitlines()[0]
    trfp = shutil.which("ThermoRawFileParser")
    if trfp:
        v["ThermoRawFileParser"] = _out([trfp, "--version"]).splitlines()[-1]
    if shutil.which("mono"):
        v["mono"] = _out(["mono", "--version"]).splitlines()[0]
    toml = (ROOT / "Cargo.toml").read_text(encoding="utf-8") if (ROOT / "Cargo.toml").exists() else ""
    m = re.search(r'^version\s*=\s*"([^"]+)"', toml, re.M)
    v["mzlab"] = m.group(1) if m else "unavailable"
    v["rustc"] = _out(["rustc", "--version"]) if shutil.which("rustc") else "unavailable"
    return {"schema": "bench/versioni/1", **v}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("out")
    ap.add_argument("--mzmine")
    ap.add_argument("--java-home")
    a = ap.parse_args()
    Path(a.out).write_text(json.dumps(raccogli(a.mzmine, a.java_home), indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(a.out)


if __name__ == "__main__":
    sys.exit(main())
