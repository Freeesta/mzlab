"""Turns off Ketcher's macromolecules mode in the vendored bundle and drops the monomer library nobody uses.

    python3 tools/ketcher_senza_monomeri.py [--dry-run]

Disegno never shows the macromolecules editor (peptides, RNA, DNA: draw.js hides its switch). The bundle still parses and converts its 3.5 MB default
monomer library (JSON.parse + one structure per monomer) every time the editor is created: seconds of main-thread time in the Disegno iframe.
Two changes to assets/index-*.js, both idempotent:
  - the render call gets `disableMacromoleculesEditor:!0` (a prop of Ketcher's Editor component),
  - the default monomer library template literal becomes the empty library `{root:{nodes:[],connections:[],templates:[]}}`.
Run after tools/ketcher_estrai_worker.py; nothing is printed beyond a few short lines (the bundle is a megabyte-long line).
"""
import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "mzlab" / "web" / "vendor" / "ketcher" / "assets"
RENDER = re.compile(r'(\.render\([\w$]+\.jsx\([\w$]+,\{staticResourcesUrl:"\./",structServiceProvider:[\w$]+,)(errorHandler)')
PROP = "disableMacromoleculesEditor:!0,"
# `const G$U = `{ "root": { "nodes": [], "connections": [], "templates": [ ... ] ... }``: the one template literal that starts with this header
LIBRARY = re.compile(r'(var [\w$]+)=`\{\n  "root": \{\n    "nodes": \[\],\n    "connections": \[\],\n    "templates": \[\n      \{\n        "\$ref": "monomerTemplate-[^`]*`')
EMPTY = '`{"root":{"nodes":[],"connections":[],"templates":[]}}`'


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()
    cands = [p for p in ASSETS.glob("index-*.js") if p.stat().st_size > 1_000_000]
    if len(cands) != 1:
        sys.exit(f"expected one page bundle, found {len(cands)}")
    path = cands[0]
    s = path.read_text("utf8")
    before = len(s)
    if PROP not in s:
        if len(RENDER.findall(s)) != 1:
            print("render call not found; 'staticResourcesUrl:' occurrences:", s.count("staticResourcesUrl:"))
            return 1
        s = RENDER.sub(lambda m: m.group(1) + PROP + m.group(2), s)
    lib = LIBRARY.findall(s)
    if len(lib) == 1:
        s = LIBRARY.sub(lambda m: m.group(1) + "=" + EMPTY, s)
    elif len(lib) > 1:
        print("several monomer libraries found:", len(lib))
        return 1
    elif EMPTY not in s:
        print("monomer library not found")
        return 1
    print(f"page bundle {before} -> {len(s)} bytes")
    if not a.dry_run:
        path.write_text(s, "utf8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
