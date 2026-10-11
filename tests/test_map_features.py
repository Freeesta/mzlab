"""Features of a high-resolution result for the map of the page (TP_Mine/js/31_map_features.js): which rows are shown for a file, the window in ppm, the edges recorded by the
search (only for the inclusion list), and the quality filters with the thresholds the search used. Only reads: nothing is computed again."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
JS = ROOT / "TP_Mine" / "js" / "31_map_features.js"
pytestmark = pytest.mark.skipif(shutil.which("node") is None, reason="node is not installed")

RESULT = {"mode": "hr", "files": [{"name": "blank_1.mzML", "label": "blank_1", "type": "blank"}, {"name": "t0.mzML", "label": "t0", "type": "sample"}, {"name": "t30.mzML", "label": "t30", "type": "sample"}],
          "settings": {"min_height": 2e5, "min_treated": 2, "fold": 5.0, "rt_min": 0.7},
          "rows": [{"id": 1, "mz": 305.0702, "rt": 9.0, "series": [0, 4e5, 9e5]}, {"id": 2, "mz": 400.1, "rt": 5.0, "series": [3e4, 5e5, 6e5]}, {"id": 3, "mz": 410.2, "rt": 6.0, "series": [0, 0, 7e5]}],
          "inclusion": [{"mz": 305.0702, "rt": 9.0, "rt_from": 8.8, "rt_to": 9.3}]}


def run(f):
    code = ("const vm=require('vm'),fs=require('fs');const g={console,window:undefined};g.globalThis=g;vm.createContext(g);"
            f"vm.runInContext(fs.readFileSync({json.dumps(str(JS))},'utf8'),g);"
            f"const r={json.dumps(RESULT)};g.MAPFEAT._set(r);console.log(JSON.stringify(g.MAPFEAT.features({json.dumps(f)})));")
    return json.loads(subprocess.run(["node", "-e", code], capture_output=True, text=True, check=True, encoding="utf-8").stdout)


def test_rows_of_the_file_only():
    fs = run({"file": "/x/t30.mzML", "label": "t30"})
    assert [x["id"] for x in fs] == [1, 2, 3]
    fs = run({"file": "t0.mzML", "label": "t0"})
    assert [x["id"] for x in fs] == [1, 2]                      # id 3 has no area in t0
    assert run({"file": "other.mzML", "label": "other"}) is None


def test_window_edges_and_filters():
    f1 = run({"file": "t30.mzML", "label": "t30"})[0]
    assert f1["ppm"] == 5 and f1["rt0"] == 8.8 and f1["rt1"] == 9.3                                # edges recorded for the inclusion list
    f2 = run({"file": "t30.mzML", "label": "t30"})[1]
    assert f2["rt0"] is None and f2["rt1"] is None
    by = {x["key"]: x for x in f2["filters"]}
    assert by["min_height"]["thr"] == 2e5 and by["min_height"]["from"] == "height"                  # the height is read from the XIC by the page
    assert by["min_treated"]["value"] == 2 and by["min_treated"]["thr"] == 2
    assert abs(by["fold"]["value"] - 6e5 / 3e4) < 1e-9 and by["fold"]["thr"] == 5.0
    f3 = run({"file": "t30.mzML", "label": "t30"})[2]
    assert {x["key"]: x for x in f3["filters"]}["fold"]["value"] is None                            # no signal in the blank: no ratio, a note says why
    assert {x["key"]: x for x in f3["filters"]}["min_treated"]["value"] == 1
