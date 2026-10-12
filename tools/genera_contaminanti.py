"""Write mzlab/web/contaminanti.json: the answer of /api/contaminants (built-in list of known contaminants expanded into ions), served as a static file
by the flow without Pyodide (web/lr.js). tests/test_contaminanti.py keeps it equal to mzlab.chem.contaminants.builtin().

    python3 tools/genera_contaminanti.py
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from mzlab.chem.contaminants import builtin  # noqa: E402

OUT = ROOT / "mzlab" / "web" / "contaminanti.json"


def text() -> str:
    return json.dumps({"lists": [builtin()]})


if __name__ == "__main__":
    OUT.write_text(text(), encoding="utf-8")
    print(f"{OUT} ({OUT.stat().st_size} bytes)")
