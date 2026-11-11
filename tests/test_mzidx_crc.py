"""The `crc32` of each `.mzidx` section in the Python reader (mzlab/reader/mzidx.py, docs/agenti/rust-mzidx.md): the builder writes one sum per
array; an index without the key (written before the sums) still opens with the same tables; a byte changed in an array read whole is an error."""
import json
import sys
import zlib
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import dati_sintetici as ds  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    p = tmp_path_factory.mktemp("crc")
    ds.full_scan(p / "full.mzML", 0, np.random.default_rng(11))
    run = Run(p / "full.mzML", index_path=p / "full.mzidx")
    mzidx.build(run, p / "full.mzidx")
    run.close()
    return (p / "full.mzidx").read_bytes()


def meta_of(b: bytes) -> dict:
    h = mzidx.HEADER.unpack(b[:mzidx.HEADER.size])
    return json.loads(b[h[3]:h[3] + h[4]])


def with_meta(b: bytes, meta: dict) -> bytes:
    """The index with its metadata replaced by `meta` at the end of the file, the header pointing at it."""
    m = json.dumps(meta, separators=(",", ":")).encode()
    magic, ver, flags, _mo, _ml, so, sl, _t = mzidx.HEADER.unpack(b[:mzidx.HEADER.size])
    body = b + m
    return mzidx.HEADER.pack(magic, ver, flags, len(b), len(m), so, sl, len(body)) + body[mzidx.HEADER.size:]


def tables(path):
    ix = mzidx.open(path)
    try:
        t = ix.table(1)
        return np.asarray(t.rt).copy(), np.asarray(t.tic).copy(), np.asarray(t.mz[:]).copy()
    finally:
        ix.close()


def test_every_array_has_its_sum(built):
    for s in meta_of(built)["sections"]:
        for name, size in mzidx._sizes(s):
            at = s["off"][name]
            assert s["crc32"][name] == zlib.crc32(built[at:at + size]), name


def test_new_and_old_indexes_both_open_with_the_same_tables(built, tmp_path):
    old = meta_of(built)
    for s in old["sections"]:
        del s["crc32"]
    (tmp_path / "new.mzidx").write_bytes(built)
    (tmp_path / "old.mzidx").write_bytes(with_meta(built, old))
    for a, z in zip(tables(tmp_path / "new.mzidx"), tables(tmp_path / "old.mzidx")):
        assert np.array_equal(a, z)


@pytest.mark.parametrize("name", ["rt", "tic", "scan_ids", "sumint", "bp_mz", "bp_inten", "fence"])
def test_a_changed_byte_in_an_array_read_whole_is_an_error(built, tmp_path, name):
    at = meta_of(built)["sections"][0]["off"][name] + 3
    d = bytearray(built)
    d[at] ^= 0x10
    (tmp_path / "bad.mzidx").write_bytes(bytes(d))
    with pytest.raises(ValueError, match="damaged"):
        tables(tmp_path / "bad.mzidx")
