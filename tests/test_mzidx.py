"""`.mzidx` (mzlab/reader/mzidx.py, docs/agenti/mzidx.md): the arrays read from the index are IDENTICAL to the table built in memory from the mzML."""
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402

EXAMPLES = sorted((Path(__file__).resolve().parent.parent / "mzlab" / "web" / "esempi").glob("*.mzML"))


@pytest.fixture(scope="module")
def synth(tmp_path_factory):
    p = tmp_path_factory.mktemp("idx")
    ds.full_scan(p / "full.mzML", 0, np.random.default_rng(1))
    ds.full_scan(p / "neg.mzML", 0, np.random.default_rng(5), neg=True)
    ds.ms2(p / "ms2.mzML", np.random.default_rng(2))
    ds.mrm(p / "mrm.mzML", 1.0, np.random.default_rng(3), 1)
    ds.mixed_ida(p / "ida.mzML", np.random.default_rng(4))
    ds.mixed_polarity(p / "pol.mzML", np.random.default_rng(6))
    ds.hr_dda(p / "hr.mzML", np.random.default_rng(7))
    ds.full_scan_profile(p / "prof.mzML", 0, np.random.default_rng(8))
    return sorted(p.glob("*.mzML"))


def same_tables(run, ix_path, levels=None):
    for level in sorted({s.level for s in run.scans}):
        for pol in (None, 0, 1, -1):
            a, b = run.table_reference(level, pol), run.table(level, pol)
            for k in ("rt", "scan_ids", "tic"):
                assert np.array_equal(getattr(a, k), getattr(b, k)), (level, pol, k)
            for k in ("mz", "inten", "pos"):
                x, y = getattr(a, k), np.asarray(getattr(b, k))
                assert x.dtype == y.dtype and np.array_equal(x, y), (level, pol, k)
            if len(a.mz):
                lo, hi = float(a.mz[0]), float(a.mz[-1])
                for v in np.linspace(lo, hi, 7):
                    assert np.array_equal(a.xic(v, 0.5), b.xic(v, 0.5))
                    assert np.array_equal(a.mz.searchsorted([v, v + 0.1], "left"), b.mz.searchsorted([v, v + 0.1], "left"))
                    assert np.array_equal(a.mz.searchsorted([v, v + 0.1], "right"), b.mz.searchsorted([v, v + 0.1], "right"))
                assert np.array_equal(a.sum_by_scan(), np.bincount(a.pos, weights=a.inten, minlength=len(a.rt)))
                assert np.array_equal(b.sum_by_scan(), a.sum_by_scan())
                bm, by = a.base_peaks()
                o = np.lexsort((a.inten, a.pos))
                ps = a.pos[o]
                last = np.r_[np.flatnonzero(ps[1:] != ps[:-1]), len(ps) - 1]
                m = np.zeros(len(a.rt)); y = np.zeros(len(a.rt))
                m[ps[last]] = a.mz[o][last]; y[ps[last]] = a.inten[o][last]
                assert np.array_equal(bm, m) and np.array_equal(by, y) and all(np.array_equal(u, v) for u, v in zip(b.base_peaks(), (m, y)))
                assert b.mz[0] == a.mz[0] and b.mz[-1] == a.mz[-1] and len(b.mz) == len(a.mz)


@pytest.mark.parametrize("f", EXAMPLES, ids=lambda p: p.name)
def test_examples_identical(f, tmp_path):
    run = Run(f, index_path=tmp_path / "x.mzidx")
    same_tables(run, tmp_path / "x.mzidx")
    run.close()


def test_synthetic_identical(synth, tmp_path):
    for f in synth:
        run = Run(f, index_path=tmp_path / (f.stem + ".mzidx"))
        same_tables(run, tmp_path / (f.stem + ".mzidx"))
        run.close()


def test_spill_to_disk_and_small_batches_give_the_same_file(synth, tmp_path, monkeypatch):
    f = next(p for p in synth if p.name == "full.mzML")
    run = Run(f, index_path=tmp_path / "a.mzidx")
    mzidx.build(run, tmp_path / "a.mzidx")
    monkeypatch.setattr(mzidx, "BATCH_PEAKS", 5000)
    mzidx.build(run, tmp_path / "b.mzidx", budget=1 << 16)           # a budget so small that every batch is spilled to the temporary file
    A, B = mzidx.open(tmp_path / "a.mzidx"), mzidx.open(tmp_path / "b.mzidx")
    assert A.meta["sections"] == B.meta["sections"] or A.meta["sections"][0]["n_peaks"] == B.meta["sections"][0]["n_peaks"]
    for k in ("mz", "inten", "pos"):
        assert np.array_equal(np.asarray(getattr(A.table(1), k)), np.asarray(getattr(B.table(1), k)))
    A.close(); B.close()
    run.close()


def test_reopening_reads_the_scan_table_from_the_index(synth, tmp_path):
    f = next(p for p in synth if p.name == "ida.mzML")
    a = Run(f, index_path=tmp_path / "i.mzidx")
    a.table(1)
    assert a._idx
    b = Run(f, index_path=tmp_path / "i.mzidx")
    assert b._idx and [s.__dict__ for s in a.scans] == [s.__dict__ for s in b.scans]
    assert b.sha256 == a.sha256 and b.n_chromatograms == a.n_chromatograms and b.hr1 == a.hr1
    assert a.read(3)[0].tolist() == b.read(3)[0].tolist()
    assert "costruzione .mzidx" not in b.timing and "indice" in b.timing        # no new index, no walk through the spectra


def test_a_replaced_file_is_not_served_from_the_old_index(tmp_path, monkeypatch):
    monkeypatch.setenv("MZLAB_CACHE", str(tmp_path / "cache"))
    f = tmp_path / "s.mzML"
    ds.full_scan(f, 0, np.random.default_rng(1))
    r1 = Run(f); t1 = np.asarray(r1.table(1).mz)
    r1.close()                                  # Windows cannot rewrite a file that is still mapped
    ds.full_scan(f, 15, np.random.default_rng(9))
    r2 = Run(f); t2 = np.asarray(r2.table(1).mz)
    assert len(t1) != len(t2) or not np.array_equal(t1, t2)
    assert np.array_equal(t2, np.asarray(r2.table_reference(1).mz))


def test_a_broken_index_is_ignored(synth, tmp_path):
    f = synth[0]
    p = tmp_path / "z.mzidx"
    p.write_bytes(b"MZIDX001" + b"\0" * 100)
    r = Run(f, index_path=p)
    assert not r._idx and r.scans
    assert np.array_equal(np.asarray(r.table(1).mz), r.table_reference(1).mz)


def test_cache_ceiling_is_one_constant():
    assert mzidx.CACHE_BYTES == 64 << 20 and mzidx.CACHE_BYTES_SMALL == 32 << 20 and mzidx.CACHE.limit == mzidx.CACHE_BYTES
    mzidx.set_cache_limit(small=True)
    assert mzidx.CACHE.limit == 32 << 20
    mzidx.set_cache_limit()
    assert mzidx.CACHE.limit == 64 << 20


def test_blocks_in_memory_stay_under_the_ceiling(synth, tmp_path):
    f = next(p for p in synth if p.name == "full.mzML")
    r = Run(f, index_path=tmp_path / "c.mzidx")
    t = r.table(1)
    mzidx.set_cache_limit(nbytes=3 * mzidx.BLOCK * 8)
    try:
        for v in np.linspace(float(t.mz[0]), float(t.mz[-1]), 40):
            t.xic(v, 0.3)
            assert mzidx.CACHE.size <= 3 * mzidx.BLOCK * 8 + mzidx.BLOCK * 8        # the block just read may sit above the ceiling until the next one comes
    finally:
        mzidx.set_cache_limit()
    r.close()


def test_indexlist_gives_the_same_scans_as_walking_the_file(synth):
    for f in synth:
        r = Run(f)
        if r._spectrum_offsets() is None:
            continue
        r2 = Run.__new__(Run)
        r2.__dict__.update(r.__dict__)
        r2.scans = []
        r2._spectrum_offsets = lambda: None
        r2._index()
        assert [s.__dict__ for s in r.scans] == [s.__dict__ for s in r2.scans], f.name


def test_the_active_file_keeps_its_blocks(synth, tmp_path):
    runs = [Run(f, index_path=tmp_path / f"{f.stem}.mzidx") for f in synth[:3]]
    ts = [r.table(1) for r in runs]
    runs[0].activate()
    mzidx.CACHE.clear()
    mzidx.set_cache_limit(nbytes=4 * mzidx.BLOCK * 8)
    try:
        ts[0].mz[0:10]
        ts[0].mz[5 * mzidx.BLOCK:5 * mzidx.BLOCK + 10] if len(ts[0].mz) > 6 * mzidx.BLOCK else None
        for t in ts[1:]:
            for k in range(min(8, len(t.mz) // mzidx.BLOCK + 1)):
                t.mz[k * mzidx.BLOCK:k * mzidx.BLOCK + 5]
        assert any(k[0][0][0] == runs[0]._idx.uid for k in mzidx.CACHE._d)           # the others filled the cache, the active file's block is still there
    finally:
        mzidx.set_cache_limit()
        mzidx.CACHE.active = None
    for r in runs:
        r.close()


def test_known_sha_and_registry(synth, tmp_path, monkeypatch):
    monkeypatch.setenv("MZLAB_CACHE", str(tmp_path / "cache"))
    f = synth[0]
    assert mzidx.known_sha(f) == ""
    r = Run(f)
    r.table(1)
    assert mzidx.known_sha(f) == r.sha256 != ""
    where = tmp_path / "elsewhere" / "x.mzidx"
    where.parent.mkdir()
    import shutil
    shutil.copy(r.index_path, where)
    r.close()
    monkeypatch.setitem(mzidx.REGISTRY, f.name, str(where))
    monkeypatch.setenv("MZLAB_CACHE", str(tmp_path / "empty"))
    r2 = Run(f)
    assert r2._idx and Path(r2.index_path) == where
    other = tmp_path / "o.mzidx"
    shutil.copy(where, other)
    assert r2.use_index(other) and Path(r2.index_path) == other
    assert np.array_equal(np.asarray(r2.table(1).mz), r2.table_reference(1).mz)
    r2.close()


def test_maps_spectra_and_totals_equal_the_in_memory_ones(synth, tmp_path):
    from mzlab.explore import Item
    f = next(p for p in synth if p.name == "full.mzML")
    a = Item(f.name, None, 0, "sample", f, run=Run(f, index_path=tmp_path / "m.mzidx"))
    b = Item(f.name, None, 0, "sample", f, run=Run(f, index_path=tmp_path / "n.mzidx"))
    b.run.table = b.run.table_reference                                    # the same file read the old way
    ta = a._tbl(1)
    grid = {"rt0": float(ta.rt[0]), "rt1": float(ta.rt[-1]), "nrt": 60, "mz0": 100.0, "dmz": 1.0, "nmz": 400}
    for agg in ("max", "mean"):
        assert np.array_equal(a.ionmap(1, grid, agg), b.ionmap(1, grid, agg)), agg
    assert np.array_equal(a.ionmap_region(1, 2.0, 10.0, 30, 150.0, 300.0, 120), b.ionmap_region(1, 2.0, 10.0, 30, 150.0, 300.0, 120))
    for kind in ("tic", "bpc"):
        assert all(np.array_equal(x, y) for x, y in zip(a.total(kind, 1), b.total(kind, 1)))
        assert all(np.array_equal(x, y) for x, y in zip(a.total(kind, 1, 150.0, 300.0), b.total(kind, 1, 150.0, 300.0)))
    r0, r1 = float(ta.rt[100]), float(ta.rt[160])
    sa, sb = a._binned(r0, r1, 1), b._binned(r0, r1, 1)
    assert all(np.array_equal(x, y) for x, y in zip(sa, sb))
    assert a.scan_table(1) == b.scan_table(1)
    a.run.close(); b.run.close()


def test_rounded_is_round_for_every_value():
    from mzlab.app import rounded
    rng = np.random.default_rng(5)
    cases = [rng.random(50000) * 1e6, rng.integers(0, 10**7, 50000).astype(float), (rng.random(50000) * 1e6).astype(np.float32), np.round(rng.random(50000) * 100, 3),
             np.arange(0, 2000) * 0.05, -rng.random(1000) * 50, np.array([0.25, 0.35, 2.5, 0.05, 0.15, 1e-9, 0.0, -0.0, -0.04, 123456.75, 0.45, 0.55, 1.005, np.inf, np.nan])]
    for a in cases:
        for nd in (1, 4):
            ref = [round(float(v), nd) for v in a]
            assert [repr(x) for x in ref] == [repr(x) for x in rounded(a, nd)]
