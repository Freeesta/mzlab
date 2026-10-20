"""The form of the Pipeline panel is generated from docs/schemi/steps-1.json (TP_Mine/js/32_pipeline_form.js): the site ships the file and every MAP entry names a real step parameter."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STEPS = json.loads((ROOT / "docs" / "schemi" / "steps-1.json").read_text(encoding="utf-8"))
SRC = (ROOT / "TP_Mine" / "js" / "32_pipeline_form.js").read_text(encoding="utf-8")


def test_map_names_real_parameters():
    by = {s["id"]: s["params"]["properties"] for s in STEPS}
    rows = re.findall(r'\{ id: "(\w+)", step: "([\w.]+)", key: (null|"\w+")', SRC)
    assert len(rows) == 8
    for _id, step, key in rows:
        assert step in by, step
        if key != "null":
            assert key.strip('"') in by[step], (step, key)


def test_site_ships_the_descriptors():
    assert 'dst / "steps-1.json"' in (ROOT / "tools" / "build_site.py").read_text(encoding="utf-8")
