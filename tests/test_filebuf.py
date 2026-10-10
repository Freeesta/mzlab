"""Large files (B7): Run(mode="file") over _FileBuf gives the same answers as the memory map; float32 peak tables for high-resolution MS1."""
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab.explore import Session  # noqa: E402
from mzlab.reader import mzml  # noqa: E402
from mzlab.reader.mzml import Run, _FileBuf  # noqa: E402


@pytest.fixture(scope="module")
def d(tmp_path_factory):
    p = tmp_path_factory.mktemp("big")
    ds.full_scan(p / "full.mzML", 0, np.random.default_rng(1))
    ds.ms2(p / "ms2.mzML", np.random.default_rng(2))
    ds.mrm(p / "mrm.mzML", 1.0, np.random.default_rng(3), 1)
    ds.mixed_ida(p / "ida.mzML", np.random.default_rng(4))
    return p


def test_filebuf_matches_a_bytes_object(tmp_path):
    data = (b"abc<spectrum id=1>xxxxxxxxxx</spectrum>" * 50) + b"END"
    (tmp_path / "f.bin").write_bytes(data)
    fb = _FileBuf(tmp_path / "f.bin")
    for cache in (8, 17, 64, 1 << 20):       # tiny windows force patterns across window borders
        fb.CACHE = cache
        fb._off, fb._buf = 0, b""
        assert len(fb) == len(data)
        for pat, start, end in ((b"<spectrum ", 0, None), (b"</spectrum>", 100, None), (b"END", 0, None), (b"NOPE", 0, None),
                                (b"<spectrum ", 500, 530), (b"xxx", 3, 20), (b"id=1>", 1000, 1200)):
            assert fb.find(pat, start, end) == data.find(pat, start, end if end is not None else len(data)), (cache, pat, start, end)
        pos, got = 0, []
        while (a := fb.find(b"<spectrum ", pos)) >= 0:
            got.append(a)
            pos = a + 1
        assert got == [i for i in range(len(data)) if data.startswith(b"<spectrum ", i)]
        assert fb[:4096] == data[:4096] and fb[5:50] == data[5:50] and fb[-3:] == b"END" and fb[10:10] == b""
    fb.close()


@pytest.mark.parametrize("name", ["full", "ms2", "mrm", "ida"])
def test_file_mode_reads_like_mmap(d, name, monkeypatch):
    monkeypatch.setattr(_FileBuf, "CACHE", 50_000)          # many window changes, also on small files
    a, b = Run(d / f"{name}.mzML", mode="mmap"), Run(d / f"{name}.mzML", mode="file")
    assert (a.mode, b.mode) == ("mmap", "file")
    assert [(s.native, s.start, s.end, s.level, s.rt, s.precursor, s.filter) for s in a.scans] == [(s.native, s.start, s.end, s.level, s.rt, s.precursor, s.filter) for s in b.scans]
    assert a.n_chromatograms == b.n_chromatograms and a.metadata() == b.metadata() and a.scan_window() == b.scan_window()
    for i in range(0, len(a.scans), max(1, len(a.scans) // 25)):
        for x, y in zip(a.read(i), b.read(i)):
            assert np.array_equal(x, y)
    for c in a.chromatograms():
        assert np.array_equal(a.chromatogram(c["index"])[1], b.chromatogram(c["index"])[1])
    assert [c["id"] for c in a.chromatograms()] == [c["id"] for c in b.chromatograms()]
    if a.scans:
        lvl = a.scans[0].level
        ta, tb = a.table(lvl), b.table(lvl)
        assert np.array_equal(ta.mz, tb.mz) and np.array_equal(ta.inten, tb.inten) and np.array_equal(ta.pos, tb.pos)
    a.close(); b.close()


def test_auto_mode_follows_size_and_workerfs(d, monkeypatch, tmp_path):
    r = Run(d / "full.mzML")
    assert r.mode == "mmap"
    r.close()
    monkeypatch.setattr(mzml, "FILE_MODE_BYTES", 1000)
    r = Run(d / "full.mzML")
    assert r.mode == "file"
    r.close()
    monkeypatch.setattr(mzml, "FILE_MODE_BYTES", 300 << 20)
    link = tmp_path / "link.mzML"                            # the browser links a Blob mounted under /big into the work folder
    link.symlink_to(d / "full.mzML")
    monkeypatch.setattr(mzml, "WORKERFS_ROOT", str(d.resolve()))
    r = Run(link)
    assert r.mode == "file"
    r.close()


def test_session_opens_a_file_in_file_mode(d, monkeypatch):
    monkeypatch.setattr(mzml, "FILE_MODE_BYTES", 1000)
    s = Session([{"file": "full.mzML", "time": 0, "type": "sample"}], d)
    assert s.items[0].run.mode == "file"
    monkeypatch.setattr(mzml, "FILE_MODE_BYTES", 300 << 20)
    ref = Session([{"file": "full.mzML", "time": 0, "type": "sample"}], d)
    assert ref.items[0].run.mode == "mmap"
    xa, xb = s.items[0].xic(364.4, 0.5, 1), ref.items[0].xic(364.4, 0.5, 1)
    assert np.array_equal(xa[1], xb[1]) and s.items[0].info()["scans"] == ref.items[0].info()["scans"]


def test_high_resolution_table_is_float32_and_xic_agrees(d, tmp_path):
    a, b = Run(d / "full.mzML", index_path=tmp_path / "a.mzidx"), Run(d / "full.mzML", index_path=tmp_path / "b.mzidx")      # the width of the table is written in the index: one each
    b.hr1 = True
    ta, tb = a.table(1), b.table(1)
    assert ta.mz.dtype == np.float64 and tb.mz.dtype == np.float32 and tb.inten.dtype == np.float32 and tb.pos.dtype == np.int32
    assert tb.mz.nbytes + tb.inten.nbytes + tb.pos.nbytes < 0.65 * (ta.mz.nbytes + ta.inten.nbytes + ta.pos.nbytes)
    assert tb.mz.dtype == np.float32 and np.all(np.diff(tb.mz) >= 0)
    xa, xb = ta.xic(364.4, 0.5), tb.xic(364.4, 0.5)
    assert xa.shape == xb.shape and np.allclose(xa, xb, rtol=1e-5)
    assert np.allclose(ta.xic(194.2, 0.01), tb.xic(194.2, 0.01), rtol=1e-5)
    a.close(); b.close()
    ms2 = Run(d / "ms2.mzML", index_path=tmp_path / "c.mzidx")
    ms2.hr1 = True                                       # only level 1 is float32
    assert ms2.table(2).mz.dtype == np.float64


def test_browser_links_a_mounted_file_like_an_upload(d, tmp_path, monkeypatch):
    """browser.link_big: the worker mounted a Blob under /big (here: a plain folder), Python links it into the work folder."""
    from mzlab import browser
    from mzlab.app import App
    work = tmp_path / "sessione"
    mount = tmp_path / "big" / "1"
    mount.mkdir(parents=True)
    (mount / "data").write_bytes((d / "full.mzML").read_bytes())
    monkeypatch.setattr(browser, "WORK", work)
    monkeypatch.setattr(mzml, "WORKERFS_ROOT", str(tmp_path / "big"))
    browser.start()
    code, text = browser.link_big("C:\\dati\\Esempio_t0.mzML", str(mount / "data"))
    import json
    out = json.loads(code == 200 and text)
    assert [f["name"] for f in out["files"]] == ["Esempio_t0.mzML"] and out["files"][0]["kind"] == "full" and out["files"][0]["hr_profile"] is False
    browser.app.open_session({"samples": [{"file": "Esempio_t0.mzML", "time": 0, "type": "sample"}]})
    run = browser.app.session.items[0].run
    assert run.mode == "file" and len(run.scans) == 1111
    code, text = browser.link_big("x.txt", str(mount / "data"))      # same checks as an upload
    assert code == 400 and "error" in json.loads(text)
    browser.app.remove_file("Esempio_t0.mzML")
    assert not (work / "Esempio_t0.mzML").exists() and (mount / "data").exists()    # the link goes to the bin, never the mounted file
