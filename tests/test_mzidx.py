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
    assert not any(k.startswith("indice") and "mzidx" not in k for k in b.timing)       # the spectra were not walked again


def test_a_replaced_file_is_not_served_from_the_old_index(tmp_path, monkeypatch):
    monkeypatch.setenv("MZLAB_CACHE", str(tmp_path / "cache"))
    f = tmp_path / "s.mzML"
    ds.full_scan(f, 0, np.random.default_rng(1))
    r1 = Run(f); t1 = np.asarray(r1.table(1).mz)
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
