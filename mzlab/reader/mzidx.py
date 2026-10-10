"""`.mzidx`: the index of one mzML file, written once and read in slices (specification: docs/agenti/mzidx.md).

It holds the table of the scans (everything `Scan` says) and, for every group of scans (level x polarity), the peaks sorted by m/z in
three flat arrays (m/z, intensity, position of the scan) plus the retention times and the TIC. A `PeakTable` built on it
(`MzIdx.table`) gives the arrays as `LazyArray` objects: slices are read from the file through a small LRU cache of blocks,
so the memory stays flat however many files are open.

    build(run, out)   write the index of `run` (a `Run`) to `out`, sorting the peaks by m/z in buckets so that memory stays bounded
    open(path)        -> MzIdx, the reader
"""
from __future__ import annotations

import builtins
import collections
import itertools
import json
import os
import struct
import tempfile

import numpy as np

from .mzml import PeakTable, Scan

MAGIC = b"MZIDX001"
VERSION = 1
HEADER = struct.Struct("<8sIIQQQQQ8x")          # magic, version, flags, meta_off, meta_len, scans_off, scans_len, total_size  (64 bytes)
BLOCK = 16384                                   # elements per block (also the spacing of the fence values)
ALIGN = 64
POLS = {None: "all", 0: "all", 1: "+", -1: "-"}

# The ceiling of the blocks kept in memory (all files together). One place; 32 MB on a tablet or a phone (set_cache_limit).
CACHE_BYTES = 64 << 20
CACHE_BYTES_SMALL = 32 << 20
BUILD_BYTES = 64 << 20                          # peaks held in memory while building, per section, before they are spilled to a temporary file
BATCH_PEAKS = 1 << 20
SCAN_COLUMNS = ["start", "end", "level", "rt", "polarity", "tic", "precursor", "collision_energy", "filter", "profile", "parent", "iso",
                "act", "res", "an", "pint", "path", "precursors", "charge", "native"]


class BlockCache:
    """Least-recently-used blocks of the lazy arrays, with a ceiling in bytes. Blocks of the file in use come back when read again."""

    def __init__(self, limit: int = CACHE_BYTES):
        self.limit = limit
        self.size = 0
        self._d: collections.OrderedDict = collections.OrderedDict()

    def get(self, key):
        v = self._d.get(key)
        if v is not None:
            self._d.move_to_end(key)
        return v

    def put(self, key, arr):
        if key in self._d:
            return
        self._d[key] = arr
        self.size += arr.nbytes
        while self.size > self.limit and len(self._d) > 1:
            _, old = self._d.popitem(last=False)
            self.size -= old.nbytes

    def drop(self, owner):
        """Forget the blocks of one array owner (when its file is closed)."""
        for k in [k for k in self._d if k[0][0] == owner]:
            self.size -= self._d.pop(k).nbytes

    def clear(self):
        self._d.clear()
        self.size = 0


CACHE = BlockCache()
_UID = itertools.count(1)          # one number per open index: the cache keys must not be reused by a later file


def set_cache_limit(small: bool = False, nbytes: int | None = None):
    """The ceiling of the block cache: 64 MB, or 32 MB on a tablet or phone (or an explicit number of bytes)."""
    CACHE.limit = nbytes if nbytes is not None else CACHE_BYTES_SMALL if small else CACHE_BYTES


class LazyArray(np.lib.mixins.NDArrayOperatorsMixin):
    """A one-dimensional array that lives in the `.mzidx` file: slices are read on demand; anything else (masks, operators, np.asarray) reads it whole."""
    ndim = 1

    def __init__(self, fh, offset: int, n: int, dtype, fence=None, owner=None):
        self._fh, self._off, self._n = fh, offset, n
        self.dtype = np.dtype(dtype)
        self._fence = fence
        self._owner = owner if owner is not None else id(self)
        self._id = (self._owner, offset)

    # -- the ndarray surface the callers use
    @property
    def shape(self):
        return (self._n,)

    @property
    def size(self):
        return self._n

    @property
    def nbytes(self):
        return self._n * self.dtype.itemsize

    def __len__(self):
        return self._n

    def __array__(self, dtype=None, copy=None):
        a = self._read(0, self._n)
        return a if dtype is None else a.astype(dtype, copy=False)

    def __array_ufunc__(self, ufunc, method, *inputs, **kw):
        inputs = tuple(np.asarray(x) if isinstance(x, LazyArray) else x for x in inputs)
        return getattr(ufunc, method)(*inputs, **kw)

    def astype(self, dtype, copy=True):
        return np.asarray(self).astype(dtype, copy=False)

    # -- reading
    def _read(self, a: int, b: int) -> np.ndarray:
        a, b = max(a, 0), min(b, self._n)
        out = np.empty(max(b - a, 0), dtype=self.dtype)
        if b > a:
            self._fh.seek(self._off + a * self.dtype.itemsize)
            mv = memoryview(out).cast("B")
            got = 0
            while got < len(mv):
                k = self._fh.readinto(mv[got:])
                if not k:
                    raise OSError("mzidx: unexpected end of file")
                got += k
        return out

    def _block(self, k: int) -> np.ndarray:
        key = (self._id, k)
        blk = CACHE.get(key)
        if blk is None:
            blk = self._read(k * BLOCK, (k + 1) * BLOCK)
            CACHE.put(key, blk)
        return blk

    def _slice(self, a: int, b: int) -> np.ndarray:
        if b <= a:
            return np.empty(0, dtype=self.dtype)
        if b - a >= 2 * BLOCK:                                   # a long run: straight from the file, not through (and not into) the cache
            return self._read(a, b)
        k0, k1 = a // BLOCK, (b - 1) // BLOCK
        if k0 == k1:
            return self._block(k0)[a - k0 * BLOCK:b - k0 * BLOCK].copy()
        parts = [self._block(k) for k in range(k0, k1 + 1)]
        parts[0] = parts[0][a - k0 * BLOCK:]
        parts[-1] = parts[-1][:b - k1 * BLOCK]
        return np.concatenate(parts)

    def __getitem__(self, key):
        if isinstance(key, slice):
            a, b, step = key.indices(self._n)
            if step == 1:
                return self._slice(a, b)
            return self._slice(a, b)[::step] if step > 0 else np.asarray(self)[key]
        if isinstance(key, (int, np.integer)):
            i = int(key) + (self._n if key < 0 else 0)
            if not 0 <= i < self._n:
                raise IndexError("index out of range")
            return self._block(i // BLOCK)[i % BLOCK]
        return np.asarray(self)[key]

    def searchsorted(self, v, side="left", sorter=None):
        """np.searchsorted on a sorted array, reading one block per value (the first value of every block is kept in memory)."""
        if self._n == 0:
            return np.zeros(np.shape(v), dtype=np.intp) if np.ndim(v) else np.intp(0)
        fence = self._fence
        k = np.maximum(np.searchsorted(fence, v, side=side) - 1, 0)
        if np.ndim(v) == 0:
            kk = int(k)
            return np.intp(kk * BLOCK + np.searchsorted(self._block(kk), v, side=side))
        v = np.asarray(v)
        out = np.empty(v.shape, dtype=np.intp)
        for kk in np.unique(k):
            sel = k == kk
            out[sel] = kk * BLOCK + np.searchsorted(self._block(int(kk)), v[sel], side=side)
        return out


# ---------------------------------------------------------------------------------------------------------------- reader
class MzIdx:
    def __init__(self, path):
        self.path = os.fspath(path)
        self._fh = builtins.open(self.path, "rb")
        try:
            head = self._fh.read(HEADER.size)
            if len(head) < HEADER.size:
                raise ValueError("mzidx: file too short")
            magic, ver, _flags, self._moff, self._mlen, self._soff, self._slen, total = HEADER.unpack(head)
            if magic != MAGIC or ver != VERSION:
                raise ValueError("mzidx: not an index of this version")
            if self._fh.seek(0, os.SEEK_END) != total:
                raise ValueError("mzidx: file truncated")
            self._fh.seek(self._moff)
            self.meta = json.loads(self._fh.read(self._mlen).decode("utf-8"))
        except Exception:
            self._fh.close()
            raise
        self._tables: dict = {}
        self.uid = next(_UID)

    @property
    def source(self) -> dict:
        return self.meta["source"]

    def scans(self) -> list[Scan]:
        """The scan table, exactly as the reader of the mzML produced it."""
        self._fh.seek(self._soff)
        rows = json.loads(self._fh.read(self._slen).decode("utf-8"))
        out = []
        for i, r in enumerate(rows):
            d = dict(zip(SCAN_COLUMNS, r))
            d["iso"] = tuple(d["iso"]) if d["iso"] is not None else None
            d["path"] = tuple(tuple(x) for x in d["path"]) if d["path"] is not None else None
            out.append(Scan(index=i, **d))
        return out

    def has(self, level: int, polarity=None) -> bool:
        return f"{level}/{POLS.get(polarity, '?')}" in self.meta["keys"]

    def table(self, level: int, polarity=None):
        """The `PeakTable` of one group of scans, or None when the index does not hold that group."""
        k = f"{level}/{POLS.get(polarity, '?')}"
        if k not in self.meta["keys"]:
            return None
        si = self.meta["keys"][k]
        if si in self._tables:
            return self._tables[si]
        s = self.meta["sections"][si]
        dt = np.dtype(s["dtype"])
        n, p = s["n_scans"], s["n_peaks"]
        owner = (self.uid, si)
        a = lambda name, count, dtype, fence=None: LazyArray(self._fh, s["off"][name], count, dtype, fence, owner)
        fence = a("fence", s["n_fence"], dt)._read(0, s["n_fence"]) if p else np.zeros(0, dtype=dt)
        rt, tic, ids = (a(x, n, d)._read(0, n) for x, d in (("rt", "<f8"), ("tic", "<f8"), ("scan_ids", "<i8")))
        t = PeakTable(rt=rt, scan_ids=ids, mz=a("mz", p, dt, fence), inten=a("inten", p, dt), pos=a("pos", p, "<i4"), tic=tic)
        self._tables[si] = t
        return t

    def close(self):
        for si in range(len(self.meta["sections"])):
            CACHE.drop((self.uid, si))
        self._tables.clear()
        self._fh.close()


def open(path) -> MzIdx:          # noqa: A001 -- the name the specification gives
    return MzIdx(path)


# ---------------------------------------------------------------------------------------------------------------- builder
def _pad(n: int) -> int:
    return -n % ALIGN


class _Sorter:
    """The peaks of one section, collected in buckets of 1 m/z unit and spilled to a temporary file when they pass the budget."""

    NB = 10000

    def __init__(self, dt, budget: int, tmpdir):
        self.dt, self.budget, self.tmpdir = np.dtype(dt), budget, tmpdir
        self.batch: list = []
        self.batch_n = 0
        self.mem: dict = collections.defaultdict(list)       # bucket -> [(mz, inten, pos), ...] in scan order
        self.held = 0
        self.spill = None
        self.spilled: dict = collections.defaultdict(list)   # bucket -> [(offset, count), ...]
        self.counts = np.zeros(self.NB, dtype=np.int64)
        self.n = 0

    def add(self, mz, it, pos_value: int):
        if len(mz):
            self.batch.append((mz, it, pos_value))
            self.batch_n += len(mz)
            if self.batch_n >= BATCH_PEAKS:
                self.flush()

    def flush(self):
        if not self.batch:
            return
        mz = np.concatenate([b[0] for b in self.batch])
        it = np.concatenate([b[1] for b in self.batch])
        pos = np.concatenate([np.full(len(b[0]), b[2], dtype=np.int32) for b in self.batch])
        self.batch, self.batch_n = [], 0
        bucket = np.clip(np.nan_to_num(np.floor(mz.astype(np.float64)), nan=self.NB - 1, posinf=self.NB - 1, neginf=0), 0, self.NB - 1).astype(np.int64)
        o = np.argsort(bucket, kind="stable")                 # stable: the scan order (and the order inside a scan) survives in every bucket
        bucket, mz, it, pos = bucket[o], mz[o], it[o], pos[o]
        cuts = np.flatnonzero(np.r_[True, bucket[1:] != bucket[:-1], True])
        for a, b in zip(cuts[:-1], cuts[1:]):
            self.mem[int(bucket[a])].append((mz[a:b], it[a:b], pos[a:b]))
            self.counts[int(bucket[a])] += b - a
        self.n += len(mz)
        self.held += mz.nbytes + it.nbytes + pos.nbytes
        if self.held > self.budget:
            self.spill_out()

    def spill_out(self):
        if self.spill is None:
            self.spill = tempfile.TemporaryFile(dir=self.tmpdir)
        self.spill.seek(0, os.SEEK_END)
        for b, parts in self.mem.items():
            mz = np.concatenate([p[0] for p in parts])
            it = np.concatenate([p[1] for p in parts])
            pos = np.concatenate([p[2] for p in parts])
            off = self.spill.tell()
            self.spill.write(mz.tobytes()); self.spill.write(it.tobytes()); self.spill.write(pos.tobytes())
            self.spilled[b].append((off, len(mz)))
        self.mem = collections.defaultdict(list)
        self.held = 0

    def buckets(self):
        """(bucket, mz, inten, pos) sorted by m/z, stable, in bucket order."""
        isz = self.dt.itemsize
        for b in sorted(set(self.mem) | set(self.spilled)):
            mzs, its, poss = [], [], []
            for off, c in self.spilled.get(b, ()):
                self.spill.seek(off)
                mzs.append(np.frombuffer(self.spill.read(c * isz), dtype=self.dt))
                its.append(np.frombuffer(self.spill.read(c * isz), dtype=self.dt))
                poss.append(np.frombuffer(self.spill.read(c * 4), dtype=np.int32))
            for m, i, p in self.mem.get(b, ()):
                mzs.append(m); its.append(i); poss.append(p)
            mz, it, pos = np.concatenate(mzs), np.concatenate(its), np.concatenate(poss)
            o = np.argsort(mz, kind="stable")
            yield b, mz[o], it[o], pos[o]

    def close(self):
        if self.spill is not None:
            self.spill.close()
            self.spill = None


def selections(run) -> list[tuple]:
    """[(level, polarity key, ids)] for every group of scans a `Run.table(level, polarity)` can be asked for."""
    out = []
    for level in sorted({s.level for s in run.scans}):
        for pol, name in ((None, "all"), (1, "+"), (-1, "-")):
            ids = [s.index for s in run.scans if s.level == level and (pol in (None, 0) or s.polarity in (pol, 0))]
            out.append((level, name, ids))
    return out


def build(run, out, tmpdir=None, budget: int | None = None, sha256: str | None = None) -> str:
    """Write the index of `run` to `out` (atomically: a temporary file next to it, then a rename). Returns the path."""
    out = os.fspath(out)
    budget = BUILD_BYTES if budget is None else budget
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    tmp = out + f".{os.getpid()}.tmp"
    # -- the sections: groups of scans with the same members share one
    sections, keys = [], {}
    for level, name, ids in selections(run):
        sig = (level, tuple(ids))
        si = next((i for i, s in enumerate(sections) if s["sig"] == sig), None)
        if si is None:
            si = len(sections)
            sections.append({"sig": sig, "level": level, "ids": ids})
        keys[f"{level}/{name}"] = si
    hr1 = bool(getattr(run, "hr1", False))
    member: dict = collections.defaultdict(list)      # scan index -> [(section, position in it)]
    for si, s in enumerate(sections):
        s["dtype"] = "<f4" if (s["level"] == 1 and hr1) else "<f8"
        s["sorter"] = _Sorter(s["dtype"], max(budget // max(len(sections), 1), 1 << 20), tmpdir)
        for p, i in enumerate(s["ids"]):
            member[i].append((si, p))
    try:
        # -- pass 1: decode every scan once, distribute the peaks to the buckets of the sections it belongs to
        for sc in run.scans:
            if not member.get(sc.index):
                continue
            mz, it = run.read(sc.index)
            for si, p in member[sc.index]:
                dt = sections[si]["dtype"]
                sections[si]["sorter"].add(mz.astype(dt, copy=False), it.astype(dt, copy=False), p)
        for s in sections:
            s["sorter"].flush()
        # -- layout
        pos = HEADER.size
        for s in sections:
            n, npk = len(s["ids"]), s["sorter"].n
            isz = np.dtype(s["dtype"]).itemsize
            nf = -(-npk // BLOCK)
            s["n_scans"], s["n_peaks"], s["n_fence"], s["off"] = n, npk, nf, {}
            for name, size in (("rt", 8 * n), ("tic", 8 * n), ("scan_ids", 8 * n), ("fence", isz * nf), ("mz", isz * npk), ("inten", isz * npk), ("pos", 4 * npk)):
                pos += _pad(pos)
                s["off"][name] = pos
                pos += size
        pos += _pad(pos)
        # -- pass 2: the data
        with builtins.open(tmp, "wb") as f:
            f.write(b"\0" * HEADER.size)
            for s in sections:
                ids = s["ids"]
                f.seek(s["off"]["rt"]); f.write(np.array([run.scans[i].rt for i in ids], dtype="<f8").tobytes())
                f.seek(s["off"]["tic"]); f.write(np.array([run.scans[i].tic for i in ids], dtype="<f8").tobytes())
                f.seek(s["off"]["scan_ids"]); f.write(np.array(ids, dtype="<i8").tobytes())
                dt = np.dtype(s["dtype"])
                done, fence = 0, []
                for _b, mz, it, ps in s["sorter"].buckets():
                    for g in range(-done % BLOCK, len(mz), BLOCK):      # global indexes that are multiples of BLOCK, inside this bucket
                        fence.append(mz[g])
                    f.seek(s["off"]["mz"] + done * dt.itemsize); f.write(mz.astype(dt, copy=False).tobytes())
                    f.seek(s["off"]["inten"] + done * dt.itemsize); f.write(it.astype(dt, copy=False).tobytes())
                    f.seek(s["off"]["pos"] + done * 4); f.write(ps.astype("<i4", copy=False).tobytes())
                    done += len(mz)
                f.seek(s["off"]["fence"]); f.write(np.array(fence, dtype=dt).tobytes())
                s["sorter"].close()
            f.seek(pos)
            scans_off = f.tell()
            rows = [[getattr(sc, c) for c in SCAN_COLUMNS] for sc in run.scans]
            blob = json.dumps(rows, separators=(",", ":")).encode("utf-8")
            f.write(blob)
            scans_len = len(blob)
            st = run.path.stat()
            meta = {"format": "mzidx", "version": VERSION, "block": BLOCK,
                    "source": {"name": run.path.name, "size": st.st_size, "mtime_ns": st.st_mtime_ns, "probe": probe(run),
                               "sha256": sha256 if sha256 is not None else getattr(run, "sha256", "")},
                    "run": {"nce": bool(run.nce), "hr1": hr1, "n_chromatograms": int(run.n_chromatograms)},
                    "keys": keys,
                    "sections": [{k: s[k] for k in ("level", "dtype", "n_scans", "n_peaks", "n_fence", "off")} for s in sections]}
            mblob = json.dumps(meta, separators=(",", ":")).encode("utf-8")
            meta_off = f.tell()
            f.write(mblob)
            total = f.tell()
            f.seek(0)
            f.write(HEADER.pack(MAGIC, VERSION, 0, meta_off, len(mblob), scans_off, scans_len, total))
        os.replace(tmp, out)
    except BaseException:
        for s in sections:
            s["sorter"].close()
        try:
            os.remove(tmp)
        except OSError:
            pass
        raise
    return out


# ---------------------------------------------------------------------------------------------------------------- cache folder
def cache_dir() -> str:
    """The folder of the indexes: $MZLAB_CACHE, else ~/.cache/mzlab (a temporary folder when that cannot be made)."""
    d = os.environ.get("MZLAB_CACHE") or os.path.join(os.path.expanduser("~"), ".cache", "mzlab")
    try:
        os.makedirs(d, exist_ok=True)
        if not os.access(d, os.W_OK):
            raise OSError(d)
        return d
    except OSError:
        d = os.path.join(tempfile.gettempdir(), "mzlab-cache")
        os.makedirs(d, exist_ok=True)
        return d


def index_name(path) -> str:
    """The file name of the index of an mzML: from its real path, size and modification time."""
    import hashlib
    p = os.path.realpath(path)
    st = os.stat(p)
    return hashlib.sha1(f"{p}|{st.st_size}|{st.st_mtime_ns}".encode("utf-8")).hexdigest()[:24] + ".mzidx"


def probe(run_or_mm) -> str:
    """A quick fingerprint of a file (size, first and last 64 KB): tells a file replaced by another of the same size and date."""
    import hashlib
    mm = run_or_mm._mm if hasattr(run_or_mm, "_mm") else run_or_mm
    n, h = len(mm), hashlib.sha1()
    h.update(str(n).encode())
    h.update(bytes(mm[:65536]))
    h.update(bytes(mm[max(n - 65536, 0):n]))
    return h.hexdigest()


def trim(folder: str, limit: int = 4 << 30) -> int:
    """Delete the oldest indexes of a folder until it holds at most `limit` bytes. Returns the bytes freed."""
    files = []
    for n in os.listdir(folder):
        if n.endswith(".mzidx"):
            p = os.path.join(folder, n)
            try:
                st = os.stat(p)
                files.append((st.st_mtime, st.st_size, p))
            except OSError:
                pass
    total, freed = sum(x[1] for x in files), 0
    for _m, sz, p in sorted(files):
        if total - freed <= limit:
            break
        try:
            os.remove(p)
            freed += sz
        except OSError:
            pass
    return freed
