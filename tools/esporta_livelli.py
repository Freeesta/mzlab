#!/usr/bin/env python3
"""Writes crates/mzlab-feat/data/livelli-1.json from mzlab/web/livelli-1.js (the only source of the thresholds and of the points of the proofs).

The JavaScript data file is evaluated by node (it is plain JSON inside an assignment); the output is the same dictionary, written in a fixed way.
`--controlla` writes nothing: it fails (exit code 1) when the JSON committed in the crate is not what the data file gives now.
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "mzlab" / "web" / "livelli-1.js"
TARGET = ROOT / "crates" / "mzlab-feat" / "data" / "livelli-1.json"


def generated() -> str:
    code = f"globalThis.window = globalThis; require({str(SOURCE)!r}); console.log(JSON.stringify(window.LIVELLI_DATI));"
    out = subprocess.run(["node", "-e", code], check=True, capture_output=True, encoding="utf-8").stdout
    return json.dumps(json.loads(out), ensure_ascii=False, indent=1) + "\n"


def main(argv: list[str]) -> int:
    text = generated()
    if "--controlla" in argv:
        if not TARGET.exists() or TARGET.read_text(encoding="utf-8") != text:
            print(f"{TARGET.relative_to(ROOT)} does not match {SOURCE.relative_to(ROOT)}: run python3 tools/esporta_livelli.py", file=sys.stderr)
            return 1
        print("livelli-1.json: ok")
        return 0
    TARGET.parent.mkdir(parents=True, exist_ok=True)
    TARGET.write_text(text, encoding="utf-8")
    print(f"written {TARGET.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
