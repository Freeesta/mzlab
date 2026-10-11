"""/api/map says where the file has data (`extent`): RT of the first and the last scan with peaks, lowest and highest m/z; the page shows that part as the whole view."""
import json
from pathlib import Path

import numpy as np

from mzlab import demo
from mzlab.app import App
from mzlab.api import dispatch


def write(path, times, empty_edges=()):
    rng = np.random.default_rng(2)
    specs = []
    for i, rt in enumerate(times):
        if i in empty_edges:
            mz, it = np.zeros(0), np.zeros(0)
        else:
            mz = np.sort(rng.uniform(150, 380, 12)); it = rng.lognormal(8, .5, 12)
        specs.append(demo._spectrum(i, 1, float(rt), mz, it))
    head = '<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="r"><spectrumList count="%d">' % len(specs)
    Path(path).write_text(head + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")


def test_extent_is_first_to_last_scan_with_peaks(tmp_path):
    times = np.arange(0.5, 20.01, 0.1)
    write(tmp_path / "a-t0.mzML", times, empty_edges=(0, len(times) - 1))                 # the first and the last scan are empty
    app = App(tmp_path); app.open_session({"samples": [{"file": "a-t0.mzML"}]})
    st, ct, body, _ = dispatch(app, "GET", "/api/map", {"k": "0", "level": "1"})
    j = json.loads(body); e = j["extent"]
    assert abs(e["rt"][0] - 0.6) < 1e-6 and abs(e["rt"][1] - 19.9) < 1e-6, e
    assert 150 <= e["mz"][0] < e["mz"][1] <= 380, e
    assert j["rt0"] == 0.0 and j["rt1"] == 20.0                                           # the grid (shared bins) is unchanged


def test_region_answer_has_no_extent(tmp_path):
    write(tmp_path / "a-t0.mzML", np.arange(0.5, 5, 0.1))
    app = App(tmp_path); app.open_session({"samples": [{"file": "a-t0.mzML"}]})
    st, ct, body, _ = dispatch(app, "GET", "/api/map", {"k": "0", "level": "1", "rt0": "1", "rt1": "3", "mz0": "150", "mz1": "300", "nrt": "20", "nmz": "20"})
    assert "extent" not in json.loads(body)
