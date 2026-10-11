"""RT x m/z map: the true zoom (a region on new bins) and, in mappa_calc, the comparison of two maps and the marked points (mzlab/explore.py)."""
import pytest

from test_pipeline import _mzml, _scan_xml


def _session(tmp_path, scans, name="full.mzML"):
    from mzlab.explore import Session
    f = tmp_path / name
    f.write_text(_mzml(f'<spectrumList count="{len(scans)}">{"".join(scans)}</spectrumList>'), encoding="utf-8")
    return Session([{"file": name}], tmp_path)


def test_region_is_mean_per_scan_on_its_own_bins(tmp_path):
    """With agg="mean" the zoomed region has its own finer bins; values are the mean per scan, peaks outside the region are left out."""
    scans = [_scan_xml(0, 1.00, [150.2, 150.7, 400.0], [100.0, 40.0, 9.0]), _scan_xml(1, 1.01, [150.4], [300.0]), _scan_xml(2, 3.00, [250.7], [50.0])]
    it = _session(tmp_path, scans).items[0]
    m = it.ionmap_region(1, 0.95, 1.15, 2, 150.0, 151.0, 4, "mean")             # RT bins of 0.1 min, m/z bins of 0.25
    assert m.shape == (2, 4)
    assert m[0, 0] == pytest.approx(50.0) and m[0, 1] == pytest.approx(150.0) and m[0, 2] == pytest.approx(20.0)   # two scans in the bin: means
    assert m.sum() == pytest.approx(220.0)                                  # 400.0 and the scan at 3.00 min are outside
    assert it.ionmap_region(1, 2.0, 1.0, 5, 150, 151, 5).sum() == 0          # empty or reversed region: zeros, no error


def test_region_endpoint(tmp_path):
    from mzlab.api import dispatch
    from mzlab.app import App
    import base64
    import numpy as np
    app = App(tmp_path)
    app.session = _session(tmp_path, [_scan_xml(0, 1.00, [150.2], [100.0]), _scan_xml(1, 2.00, [150.6], [60.0])])
    code, _, body = dispatch(app, "GET", "/api/map", {"k": "0", "rt0": "0.5", "rt1": "2.5", "mz0": "150", "mz1": "151", "nrt": "4", "nmz": "2000"})[:3]
    assert code == 200
    import json
    j = json.loads(body)
    assert j["region"] and j["nmz"] == 1024 and j["nrt"] == 4                # at most 1024 bins per axis
    m = np.frombuffer(base64.b64decode(j["data"]), "<f4").reshape(4, 1024)
    assert m.sum() == pytest.approx(160.0)


def test_difference_with_itself_is_zero_and_tolerates_drift():
    """A - A = 0; a peak that moved by one RT bin in the reference cancels with the tolerance (local maximum of B) and not without it."""
    import numpy as np
    from mzlab.explore import diff_tolerant
    rng = np.random.default_rng(0)
    a = rng.random((40, 30)).astype(np.float32)
    assert not diff_tolerant(a, a, 0).any() and (diff_tolerant(a, a, 3) <= 0).all()      # with the tolerance B only grows: A - B <= 0
    a, b = np.zeros((20, 5), np.float32), np.zeros((20, 5), np.float32)
    a[10, 2], b[11, 2] = 100.0, 100.0                                                     # same peak, one bin later in the reference
    assert diff_tolerant(a, b, 0)[10, 2] == 100.0 and diff_tolerant(a, b, 2)[10, 2] == 0.0
    a[5, 1] = 50.0                                                                        # a real increase stays
    assert diff_tolerant(a, b, 2)[5, 1] == 50.0


def test_difference_tolerates_ppm_drift_in_mz_only_when_asked():
    """High resolution: B is also widened along the m/z axis (cells of equal ppm), so a peak that moved by one cell in the reference cancels; wm = 0 (low resolution) leaves it."""
    import numpy as np
    from mzlab.explore import diff_tolerant
    a, b = np.zeros((10, 12), np.float32), np.zeros((10, 12), np.float32)
    a[4, 5], b[4, 6] = 100.0, 100.0                                                       # same ion, one m/z cell higher in the reference
    assert diff_tolerant(a, b, 0, 0)[4, 5] == 100.0 and diff_tolerant(a, b, 0, 1)[4, 5] == 0.0
    a[4, 9] = 40.0                                                                        # a real increase far away stays
    assert diff_tolerant(a, b, 0, 1)[4, 9] == 40.0
    assert (diff_tolerant(a, b, 2, 0) == diff_tolerant(a, b, 2)).all()                     # default: low resolution unchanged


def test_snap_to_the_local_maximum(tmp_path):
    """A click near a peak goes to its apex (scan with the highest intensity within +-0.2 min and +-1 bin), with the m/z of the file."""
    scans = [_scan_xml(i, 1.0 + 0.05 * i, [150.25, 160.4], [h, 10.0]) for i, h in enumerate([10.0, 40.0, 90.0, 60.0, 20.0])]
    it = _session(tmp_path, scans).items[0]
    s = it.map_snap(1, 1.02, 151.1)                                                       # one bin above, before the apex
    assert s["rt"] == pytest.approx(1.10) and s["mz"] == pytest.approx(150.25)
    assert it.map_snap(1, 3.0, 150.2) is None                                             # nothing within 0.2 min


def test_rt_groups_and_delta_m():
    """Two points at the same RT form one group, an isolated one its own; delta m is from the most intense point of the group."""
    from mzlab.explore import rt_groups
    rows = rt_groups([{"rt": 5.00, "mz": 300.2, "ia": 1000}, {"rt": 5.03, "mz": 282.2, "ia": 300}, {"rt": 8.0, "mz": 250.1, "ia": 50}], 0.05)
    assert [r["g"] for r in rows] == [1, 1, 2]
    assert rows[0]["dm"] == 0 and rows[1]["dm"] == pytest.approx(-18.0) and rows[2]["dm"] == 0
    assert [r["g"] for r in rt_groups([{"rt": 5.0, "mz": 1, "ia": 1}, {"rt": 5.04, "mz": 2, "ia": 1}, {"rt": 5.08, "mz": 3, "ia": 1}], 0.05)] == [1, 1, 1]   # chained


def test_points_endpoint(tmp_path):
    """/api/mappunti: intensity in A at the apex, local maximum in B within 0.1 min, A - B, A/B and the groups."""
    import json
    from mzlab.api import dispatch
    from mzlab.app import App
    from mzlab.explore import Session
    def write(name, scans):
        (tmp_path / name).write_text(_mzml(f'<spectrumList count="{len(scans)}">{"".join(scans)}</spectrumList>'), encoding="utf-8")
    write("a.mzML", [_scan_xml(i, 1.0 + 0.05 * i, [150.25, 132.3], [h, h / 2]) for i, h in enumerate([0.0, 40.0, 90.0, 60.0, 0.0])])
    write("b.mzML", [_scan_xml(i, 1.0 + 0.05 * i, [150.25], [h]) for i, h in enumerate([0.0, 0.0, 10.0, 30.0, 0.0])])
    app = App(tmp_path)
    app.session = Session([{"file": "a.mzML"}, {"file": "b.mzML"}], tmp_path)
    code, _, body = dispatch(app, "GET", "/api/mappunti", {"k": "0", "ref": "1", "pts": "1.10,150.25;1.10,132.3"})[:3]
    assert code == 200
    rows = json.loads(body)["rows"]
    assert rows[0]["ia"] == pytest.approx(90.0) and rows[0]["ib"] == pytest.approx(30.0) and rows[0]["d"] == pytest.approx(60.0) and rows[0]["ratio"] == pytest.approx(3.0)
    assert rows[0]["g"] == rows[1]["g"] == 1 and rows[1]["dm"] == pytest.approx(132.3 - 150.25)


def _mapfile(tmp_path):
    """Peaks 150.2 and 150.4 (100 and 300) in one RT bin, 250.7 later, a constant ion 400.0 in all of 20 scans, a peak 401.0 in one."""
    scans = [_scan_xml(i, 1.0 + 0.1 * i, [400.0] + ([150.2, 150.4] if i == 3 else []) + ([401.0] if i == 7 else []),
                       [50.0] + ([100.0, 300.0] if i == 3 else []) + ([900.0] if i == 7 else [])) for i in range(20)]
    return _session(tmp_path, scans).items[0]


def test_agg_max_does_not_dilute_and_mean_does(tmp_path):
    it = _mapfile(tmp_path)
    mx = it.ionmap_region(1, 0.9, 3.1, 1, 150.0, 152.0, 2, "max")
    mn = it.ionmap_region(1, 0.9, 3.1, 1, 150.0, 152.0, 2, "mean")
    assert mx[0, 0] == pytest.approx(300.0)                       # the most intense peak of the cell
    assert mn[0, 0] == pytest.approx(400.0 / 20)                  # 400 spread over 20 scans of the RT bin
    with pytest.raises(ValueError):
        it.ionmap_region(1, 0.9, 3.1, 1, 150.0, 152.0, 2, "median")


def test_low_resolution_grid_is_nominal_masses(tmp_path):
    """Bin n = [n - 0.2; n + 0.8]: 150.2 and 150.7 are mass 150, 150.9 is 151."""
    from mzlab.explore import Session
    sess = _session(tmp_path, [_scan_xml(0, 1.0, [150.2, 150.7, 150.9], [10.0, 20.0, 5.0])])
    g = sess.grid(1)
    assert g["mz0"] == pytest.approx(149.8) and g["dmz"] == 1.0
    m = sess.items[0].ionmap(1, g)
    assert m[0, 0] == 20.0 and m[0, 1] == 5.0 and m.sum() == 25.0        # max of 150.2 and 150.7 in bin 150; 150.9 in bin 151
    from mzlab.explore import map_cells
    lo, hi, n, _ = map_cells(False, 150.4, 160.3, 11)
    assert lo == pytest.approx(149.8) and hi == pytest.approx(160.8) and n == 11
    assert map_cells(False, 150.0, 151.0, 50) == (150.0, 151.0, 50, False)     # fine zoom: unchanged


def test_high_resolution_cells_stop_at_the_instrument(tmp_path):
    from mzlab.explore import map_cells
    mz0, mz1, n, log = map_cells(True, 500.0, 500.01, 1000, True)      # 20 ppm: 3 ppm cells, not 1000
    assert log and 6 <= n <= 8
    assert map_cells(True, 100.0, 1000.0, 1000, True)[2] == 1000       # wide view: the cap
    assert map_cells(True, 500.0, 500.01, 1000, False)[3] is False


def test_log_cells_are_equal_ppm(tmp_path):
    it = _mapfile(tmp_path)
    m = it.ionmap_region(1, 0.9, 3.1, 1, 100.0, 1000.0, 4, "max", True)       # edges 100, 177.8, 316.2, 562.3, 1000
    assert m[0, 0] == 300.0 and m[0, 1] == 0.0 and m[0, 2] == 900.0 and m[0, 3] == 0.0


def test_constant_background_goes_and_the_peak_stays(tmp_path):
    from mzlab.explore import subtract_constant
    import numpy as np
    m = np.zeros((20, 3), np.float32)
    m[:, 0] = 50.0                  # a line: present all along the run
    m[7, 1] = 900.0                 # a peak
    m[:, 2] = 10.0
    m[5, 2] = 80.0
    out = subtract_constant(m, np.median(m, axis=0))
    assert not out[:, 0].any() and out[7, 1] == 900.0 and out[5, 2] == 70.0 and out[0, 2] == 0.0


def test_background_endpoint_removes_the_constant_ion(tmp_path):
    import base64, json
    import numpy as np
    from mzlab.api import dispatch
    from mzlab.app import App
    app = App(tmp_path)
    scans = [_scan_xml(i, 1.0 + 0.1 * i, [400.0] + ([401.0] if i == 7 else []), [50.0] + ([900.0] if i == 7 else [])) for i in range(20)]
    app.session = _session(tmp_path, scans)
    q = {"k": "0", "rt0": "0.9", "rt1": "3.1", "mz0": "399", "mz1": "403", "nrt": "22", "nmz": "10"}
    get = lambda **e: json.loads(dispatch(app, "GET", "/api/map", {**q, **e})[2])
    raw, no = get(), get(bg="1")
    m0 = np.frombuffer(base64.b64decode(raw["data"]), "<f4").reshape(22, raw["nmz"])
    m1 = np.frombuffer(base64.b64decode(no["data"]), "<f4").reshape(22, no["nmz"])
    assert m0.max() == 900.0 and (m0 == 50.0).sum() >= 15 and raw["agg"] == "max" and raw["bg"] is False
    assert m1.max() == 900.0 and not (m1 == 50.0).any() and (m1 > 0).sum() == 1 and no["bg"] is True
    assert dispatch(app, "GET", "/api/map", {**q, "agg": "x"})[0] == 400


def test_sticks_endpoint_gives_the_strongest_points_up_to_the_cap(tmp_path):
    import base64, json
    import numpy as np
    from mzlab.api import dispatch
    from mzlab.app import App
    app = App(tmp_path)
    app.session = _session(tmp_path, [_scan_xml(i, 1.0 + 0.1 * i, [150.0 + i, 300.0 + i], [10.0 * (i + 1), 5.0]) for i in range(10)])
    q = {"k": "0", "rt0": "0", "rt1": "5", "mz0": "100", "mz1": "400", "max": "3"}
    code, _, body = dispatch(app, "GET", "/api/mappunti3d", q)[:3]
    j = json.loads(body)
    assert code == 200 and j["n"] == 3 and j["total"] == 20
    y = np.frombuffer(base64.b64decode(j["inten"]), "<f4")
    mz = np.frombuffer(base64.b64decode(j["mz"]), "<f8")
    rt = np.frombuffer(base64.b64decode(j["rt"]), "<f4")
    assert list(y) == [100.0, 90.0, 80.0] and list(mz) == [159.0, 158.0, 157.0] and rt[0] == pytest.approx(1.9)
    assert json.loads(dispatch(app, "GET", "/api/mappunti3d", {**q, "max": "1000000"})[2])["n"] == 20      # the cap never grows past 200000
    assert dispatch(app, "GET", "/api/mappunti3d", {**q, "rt1": "-1"})[0] == 400                              # empty region: error
