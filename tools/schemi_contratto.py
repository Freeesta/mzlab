#!/usr/bin/env python3
"""Writes docs/schemi/mzflow-2.json and prov-1.json from the workflow contract (docs/studies/workflow/contract.schema.json): each file is the contract
with its `$id` and a root `$ref` to one definition, so `mzlab-cli schema mzflow/2|prov/1` serves a schema that validates by itself.

    python3 tools/schemi_contratto.py           # rewrite the two files
    python3 tools/schemi_contratto.py --check   # exit 1 if they are stale (tests/test_cli_catalog.py runs this)
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / "docs/studies/workflow/contract.schema.json"
OUT = {
    "mzflow-2.json": ("mzflow/2", "mzflow2", "mzflow/2: a Flow plan, the file `flow run` and `flow check` read"),
    "prov-1.json": ("prov/1", "prov1", "prov/1: the record of a run, the `prov.json` `flow run` writes"),
}


def build() -> dict[str, str]:
    c = json.loads(CONTRACT.read_text(encoding="utf-8"))
    out = {}
    for name, (sid, key, title) in OUT.items():
        defs = c["$defs"]
        if key == "mzflow2":
            # the reader takes what the engine takes: `program` and the input fingerprints are optional (the BASE plan of for-agents.md has neither)
            defs = copy.deepcopy(defs)
            plan = defs["mzflow2"]
            plan["required"] = ["steps"]
            plan["properties"]["inputs"]["items"]["required"] = ["name"]
        doc = {"$schema": c["$schema"], "$id": sid, "title": title, "description": c["description"], "$ref": f"#/$defs/{key}", "$defs": defs}
        out[name] = json.dumps(doc, indent=2, ensure_ascii=False) + "\n"
    return out


def main() -> int:
    stale = []
    for name, text in build().items():
        p = ROOT / "docs/schemi" / name
        if "--check" in sys.argv:
            if not p.exists() or p.read_text(encoding="utf-8") != text:
                stale.append(name)
        else:
            p.write_text(text, encoding="utf-8")
    if stale:
        print("stale: " + ", ".join(stale) + " (run python3 tools/schemi_contratto.py)", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
