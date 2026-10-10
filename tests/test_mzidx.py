"""`.mzidx` (mzlab/reader/mzidx.py, docs/agenti/mzidx.md): the arrays read from the index are IDENTICAL to the table `Run.table` builds in memory."""
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


def same_tables(run, ix):
    assert [s.__dict__ for s in ix.scans()] == [s.__dict__ for s in run.scans]
    for level in sorted({s.level for s in run.scans}):
        for pol in (None, 1, -1):
            a, b = run.table(level, pol), ix.table(level, pol)
            for k in ("rt", "scan_ids", "tic"):
                assert np.array_equal(getattr(a, k), getattr(b, k)), (level, pol, k)
            for k in ("mz", "inten", "pos"):
                x, y = getattr(a, k), np.asarray(getattr(b, k))
                assert x.dtype == y.dtype and np.array_equal(x, y), (level, pol, k)
            if len(a.mz):
                for v in np.linspace(float(a.mz[0]), float(a.mz[-1]), 7):
                    assert np.array_equal(a.xic(v, 0.5), b.xic(v, 0.5))
                    for side in ("left", "right"):
                        assert np.array_equal(a.mz.searchsorted([v, v + 0.1], side), b.mz.searchsorted([v, v + 0.1], side))
                assert b.mz[0] == a.mz[0] and b.mz[-1] == a.mz[-1] and len(b.mz) == len(a.mz)


@pytest.mark.parametrize("f", EXAMPLES, ids=lambda p: p.name)
def test_examples_identical(f, tmp_path):
    run = Run(f)
    mzidx.build(run, tmp_path / "x.mzidx")
    ix = mzidx.open(tmp_path / "x.mzidx")
    same_tables(run, ix)
    ix.close(); run.close()


def test_synthetic_identical(synth, tmp_path):
    for f in synth:
        run = Run(f)
        mzidx.build(run, tmp_path / "x.mzidx")
        ix = mzidx.open(tmp_path / "x.mzidx")
        same_tables(run, ix)
        ix.close(); run.close()


def test_spill_to_disk_and_small_batches_give_the_same_arrays(synth, tmp_path, monkeypatch):
    run = Run(next(p for p in synth if p.name == "full.mzML"))
    mzidx.build(run, tmp_path / "a.mzidx")
    monkeypatch.setattr(mzidx, "BATCH_PEAKS", 5000)
    mzidx.build(run, tmp_path / "b.mzidx", budget=1 << 16)                 # a budget so small that every batch goes to the temporary file
    A, B = mzidx.open(tmp_path / "a.mzidx"), mzidx.open(tmp_path / "b.mzidx")
    for k in ("mz", "inten", "pos"):
        assert np.array_equal(np.asarray(getattr(A.table(1), k)), np.asarray(getattr(B.table(1), k)))
    same_tables(run, B)
    A.close(); B.close(); run.close()


def test_a_broken_or_truncated_index_does_not_open(synth, tmp_path):
    run = Run(synth[0])
    mzidx.build(run, tmp_path / "a.mzidx")
    data = (tmp_path / "a.mzidx").read_bytes()
    (tmp_path / "cut.mzidx").write_bytes(data[:-10])
    (tmp_path / "junk.mzidx").write_bytes(b"MZIDX001" + b"\0" * 100)
    for n in ("cut", "junk"):
        with pytest.raises(Exception):
            mzidx.open(tmp_path / f"{n}.mzidx")


def test_cache_ceiling_is_one_constant():
    assert mzidx.CACHE_BYTES == 64 << 20 and mzidx.CACHE_BYTES_SMALL == 32 << 20 and mzidx.CACHE.limit == mzidx.CACHE_BYTES
    mzidx.set_cache_limit(small=True)
    assert mzidx.CACHE.limit == 32 << 20
    mzidx.set_cache_limit()
    assert mzidx.CACHE.limit == 64 << 20


def test_blocks_in_memory_stay_under_the_ceiling(synth, tmp_path):
    run = Run(next(p for p in synth if p.name == "full.mzML"))
    mzidx.build(run, tmp_path / "c.mzidx")
    ix = mzidx.open(tmp_path / "c.mzidx")
    t = ix.table(1)
    mzidx.set_cache_limit(nbytes=3 * mzidx.BLOCK * 8)
    try:
        for v in np.linspace(float(t.mz[0]), float(t.mz[-1]), 40):
            t.xic(v, 0.3)
            assert mzidx.CACHE.size <= 4 * mzidx.BLOCK * 8
    finally:
        mzidx.set_cache_limit()
    ix.close(); run.close()


def test_indexlist_gives_the_same_scans_as_walking_the_file():
    seen = 0
    for f in EXAMPLES:
        r = Run(f)
        if r._spectrum_offsets() is None:
            continue
        seen += 1
        r2 = Run.__new__(Run)
        r2.__dict__.update(r.__dict__)
        r2.scans = []
        r2._spectrum_offsets = lambda: None
        r2._index()
        assert [s.__dict__ for s in r.scans] == [s.__dict__ for s in r2.scans], f.name
    assert seen
