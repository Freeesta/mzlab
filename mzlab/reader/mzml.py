"""A small reader for mzML files (ProteoWizard / msconvert output, indexed or not).

The file is memory-mapped. One pass finds every <spectrum> and reads the header values (MS level,
retention time, polarity, precursor, collision energy, filter string). The m/z and intensity arrays
(base64, zlib or none, 32 or 64 bit) are decoded when needed. Chromatograms (MRM traces) are
counted but not read: this tool works on full-scan and product-ion scans.
"""
from __future__ import annotations

import base64
import mmap
import os
import re
import time
import zlib
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from ..i18n import UserError
from .profile import analyzer_from_components, mass_profile

_CV = {}
_ANALYZERS = ("FTMS", "ITMS", "TOFMS", "TQMS", "SQMS")      # first token of a Thermo filter string
_RX_REF = re.compile(r'<precursor\b[^>]*?spectrumRef="([^"]*)"')
_RX_ICREF = re.compile(r'<scan\b[^>]*?instrumentConfigurationRef="([^"]*)"')
_RX_ACT = re.compile(r"@(hcd|cid|etd|ecd|pqd|uvpd)", re.I)
_RX_PATH = re.compile(r"(\d+\.\d+)@([a-z]+)(\d+\.\d+)", re.I)       # one stage of the fragmentation path in a Thermo filter string
_RX_PREC = re.compile(r"<precursor\b([^>]*)>(.*?)</precursor>", re.S)      # not <precursorList>: \b does not split "precursorList"
_RX_SREF = re.compile(r'spectrumRef="([^"]*)"')


def _cv(header: str, accession: str):
    """The value of the cvParam with this accession, or None."""
    rx = _CV.get(accession)
    if rx is None:
        rx = _CV[accession] = re.compile(r'accession="%s"[^>]*?value="([^"]*)"' % re.escape(accession))
    m = rx.search(header)
    return m.group(1) if m else None


def _resolution(h: str, hs: str):
    """Mass resolving power of a scan: MS:1000800, or the userParam an mzML written from a Thermo .raw by mzLab carries."""
    r = _float(_cv(hs, "MS:1000800"))
    if r is None:
        m = _RX_OTF_RES.search(h)
        r = _float(m.group(1)) if m else None
    return r


def _float(s):
    try:
        return float(s)
    except (TypeError, ValueError):
        return None


_RX_OTF_RES = re.compile(r'<userParam name="opentfraw.resolution" value="([^"]*)"')      # resolving power in an mzML written from a Thermo .raw by mzlab (OpenTFRaw)
_RX_PCV = re.compile(r'accession="(MS:1000(?:827|828|829|744|041|045|042))"[^>]*?value="([^"]*)"')


def _precursor_element(e: str, ref: str | None) -> dict:
    """One <precursor>: isolation target and absolute limits, selected ion, charge, spectrumRef, activation, collision energy, intensity."""
    v = {acc: _float(val) for acc, val in reversed(_RX_PCV.findall(e))}        # reversed: the first occurrence wins
    tgt, off_lo, off_hi, ch = v.get("MS:1000827"), v.get("MS:1000828"), v.get("MS:1000829"), v.get("MS:1000041")
    return {"target": tgt, "lo": None if tgt is None or off_lo is None else tgt - off_lo, "hi": None if tgt is None or off_hi is None else tgt + off_hi,
            "selected": v.get("MS:1000744"), "charge": int(ch) if ch else None, "ref": ref,
            "act": "HCD" if 'accession="MS:1000422"' in e else "CID" if 'accession="MS:1000133"' in e else "ETD" if 'accession="MS:1000598"' in e else None,
            "ce": v.get("MS:1000045"), "pint": v.get("MS:1000042")}


def _filter_path(filt: str):
    """Fragmentation path of a Thermo filter string ("... ms3 317.0000@cid25.00 261.0000@cid30.00 [..]"): ((m/z, "CID", 25.0), ...), or None."""
    if "@" not in filt:
        return None
    return tuple((float(a), b.upper(), float(c)) for a, b, c in _RX_PATH.findall(filt)) or None


def _path_key(path) -> tuple:
    """A path compared on nominal masses (MSn filter strings carry 244.0000, the DDA ones 244.0744)."""
    return tuple((round(m), a, ce) for m, a, ce in path)


def _immediate(precs: list, path, level: int, level_of: dict):
    """The <precursor> that is the direct parent of the scan. Never trust the order: ThermoRawFileParser writes the nearest generation first,
    MSConvert can write it last, and the list is incomplete (generations without a spectrum in the file are missing)."""
    if not precs:
        return None
    if path:
        for e in precs:
            if any(v is not None and abs(v - path[-1][0]) <= 0.01 for v in (e["target"], e["selected"])):
                return e
    for e in precs:
        if e["ref"] and level_of.get(e["ref"]) == level - 1:
            return e
    return precs[0]


# Files above this size, or mounted from a browser Blob (WORKERFS, which cannot be memory-mapped), are read with _FileBuf
FILE_MODE_BYTES = 300 << 20
WORKERFS_ROOT = "/big"


class _FileBuf:
    """The small part of the mmap interface the reader uses (find, slicing, len, close), over a plain file.

    A Blob mounted with WORKERFS in the browser cannot be mapped, and a very large file is better not mapped at all.
    Searches run inside a read-ahead window (CACHE bytes), so walking through the spectra reads the file about once.
    """
    CACHE = 16 << 20

    def __init__(self, path):
        self._fh = open(path, "rb")
        self._fh.seek(0, os.SEEK_END)
        self._n = self._fh.tell()
        self._off, self._buf = 0, b""

    def __len__(self):
        return self._n

    def _read(self, a, b):
        self._fh.seek(a)
        return self._fh.read(b - a)

    def _window(self, pos, need):
        """Make the read-ahead window cover [pos, pos + need) (or reach the end of the file)."""
        end = self._off + len(self._buf)
        if self._off <= pos and (pos + need <= end or end >= self._n):
            return
        self._off = pos
        self._buf = self._read(pos, min(self._n, pos + max(self.CACHE, need)))

    def find(self, pat, start=0, end=None):
        end = self._n if end is None else min(end, self._n)
        pos, keep = max(start, 0), len(pat) - 1
        while pos + len(pat) <= end:
            self._window(pos, len(pat))
            stop = self._off + len(self._buf)
            i = self._buf.find(pat, pos - self._off, min(end, stop) - self._off)
            if i >= 0:
                return self._off + i
            if stop >= end or stop >= self._n:
                return -1
            pos = stop - keep            # overlap, so a pattern across two windows is found
        return -1

    def __getitem__(self, key):
        if not isinstance(key, slice):
            return self._read(key, key + 1)[0]
        a, b, _ = key.indices(self._n)
        if b <= a:
            return b""
        end = self._off + len(self._buf)
        if self._off <= a and b <= end:
            return self._buf[a - self._off:b - self._off]
        return self._read(a, b)

    def close(self):
        self._fh.close()
        self._buf = b""

    def release(self):
        """Drop the read-ahead window (the next read fills it again)."""
        self._off, self._buf = 0, b""


@dataclass
class Scan:
    index: int
    native: str
    start: int
    end: int
    level: int
    rt: float              # minutes
    polarity: int          # +1, -1, 0
    tic: float
    precursor: float | None
    collision_energy: float | None
    filter: str
    profile: bool = False  # the scan is a profile spectrum (MS:1000128), not centroids (MS:1000127)
    # --- fields for high resolution / DDA (None = the file does not say) ---
    parent: int | None = None          # index of the Full Scan the MS2 was triggered from (spectrumRef; otherwise the previous MS1 of the same polarity)
    iso: tuple | None = None           # isolation window (low, high) in m/z, absolute
    act: str | None = None             # activation: "HCD", "CID", "ETD"...
    res: float | None = None           # mass resolving power of the scan (MS:1000800)
    an: str | None = None              # analyzer: FTMS / ITMS / TOFMS / TQMS / SQMS, or "?" (see Run._analyzer)
    pint: float | None = None          # intensity of the selected ion in the survey scan (MS:1000042)
    # --- MSn (levels >= 2): the whole fragmentation path and every <precursor> of the file ---
    path: tuple | None = None          # ((m/z, "HCD"|"CID"|..., collision energy), ...) from the filter string, one item per stage; None = the filter has none
    precursors: list = field(default_factory=list)   # every <precursor>, in file order: {target, lo, hi, selected, charge, ref, act, ce, pint}
    charge: int | None = None          # charge of the immediate precursor, when the file says it


@dataclass
class PeakTable:
    """Peaks of a set of scans, sorted by m/z, for fast extraction of ion chromatograms."""
    rt: np.ndarray                 # retention time of each scan (minutes), in scan order
    scan_ids: np.ndarray           # file index of each scan
    mz: np.ndarray                 # all peaks, sorted by m/z
    inten: np.ndarray
    pos: np.ndarray                # position (in rt) of the scan each peak belongs to
    tic: np.ndarray = field(default=None)
    _sum: np.ndarray = field(default=None, repr=False, compare=False)
    _base: tuple = field(default=None, repr=False, compare=False)

    CHUNK = 1 << 20

    def iter_chunks(self, step: int | None = None):
        """(mz, inten, pos) of the peaks in runs of `step` peaks, in m/z order: the way to walk a whole table without holding it (a table on a .mzidx is read piece by piece)."""
        step = step or self.CHUNK
        for a in range(0, len(self.mz), step):
            yield self.mz[a:a + step], self.inten[a:a + step], self.pos[a:a + step]

    def sum_by_scan(self) -> np.ndarray:
        """Summed intensity of every scan, added in the order of the table (so the same as np.bincount over the whole table), computed once."""
        if self._sum is None:
            n = len(self.rt)
            base, acc = np.arange(n), np.zeros(n)
            for _mz, it, pos in self.iter_chunks():
                acc = np.bincount(np.concatenate([base, pos]), weights=np.concatenate([acc, it]), minlength=n)   # the running sums first: the additions go on in the same order
            self._sum = acc
        return self._sum

    def base_peaks(self) -> tuple:
        """(m/z, intensity) of the highest peak of every scan (zeros for a scan without peaks); of equal peaks the one highest in the table, computed once."""
        if self._base is None:
            n = len(self.rt)
            m, y, have = np.zeros(n), np.zeros(n), np.zeros(n, dtype=bool)
            for mz, it, pos in self.iter_chunks():
                o = np.lexsort((it, pos))
                ps = pos[o]
                last = np.r_[np.flatnonzero(ps[1:] != ps[:-1]), len(ps) - 1] if len(ps) else np.zeros(0, dtype=np.int64)
                sc, cm, cy = ps[last], mz[o][last], it[o][last]
                better = ~have[sc] | (cy >= y[sc])
                m[sc[better]], y[sc[better]], have[sc[better]] = cm[better], cy[better], True
            self._base = (m, y)
        return self._base

    def xic(self, mz: float, tol_da: float) -> np.ndarray:
        """Summed intensity within mz +- tol_da in every scan (zeros where nothing is there)."""
        lo, hi = mz - tol_da, mz + tol_da
        if self.mz.dtype == np.float32:      # keep the limits float32 too, otherwise numpy would convert the whole table to float64
            lo, hi = np.float32(lo), np.float32(hi)
        a = np.searchsorted(self.mz, lo, side="left")
        b = np.searchsorted(self.mz, hi, side="right")
        if b <= a:
            return np.zeros(len(self.rt))
        return np.bincount(self.pos[a:b], weights=self.inten[a:b], minlength=len(self.rt))


class Run:
    def __init__(self, path, mode="auto", index_path=None):
        """mode: "mmap", "file" (plain reads: Blobs mounted in the browser, very large files) or "auto". index_path: where the .mzidx of this file is (default: the cache folder)."""
        self.path = Path(path)
        if mode == "auto":
            big = self.path.stat().st_size > FILE_MODE_BYTES or os.path.realpath(self.path).replace("\\", "/").startswith(str(WORKERFS_ROOT).replace("\\", "/").rstrip("/") + "/")
            mode = "file" if big else "mmap"
        self.mode = mode
        if mode == "file":
            self._mm = _FileBuf(self.path)
        else:
            with open(self.path, "rb") as fh:
                self._mm = mmap.mmap(fh.fileno(), 0, access=mmap.ACCESS_READ)
        head = self._mm[:4096]
        if b"mzML" not in head:
            raise UserError("err.file.notMzml", {"name": self.path.name}, f"{self.path.name} does not look like an mzML file")
        self.scans: list[Scan] = []
        self._read_header()
        self.timing: dict = {}                       # seconds spent reading the file ("indice", "tabella MS1"...), shown by the ?perf meter
        self._tables: dict = {}
        self._idx = None                             # the open MzIdx (False: no index could be written, tables are built in memory)
        self.index_path = Path(index_path) if index_path else None
        t0 = time.perf_counter()
        if not self._load_index():
            self._index()
            self.timing["indice"] = round(time.perf_counter() - t0, 3)
            self.n_chromatograms = self._count(b"<chromatogram ", self.scans[-1].end if self.scans else 0)
            self.sha256 = self._compute_sha256()
        else:
            self.timing["indice"] = round(time.perf_counter() - t0, 3)          # the scan table taken from the .mzidx
        self.release()                                                           # the read-ahead window of a plain-read file (16 MB) is not kept between operations

    # ------------------------------------------------------------------ the .mzidx
    def _default_index_path(self) -> Path:
        from . import mzidx
        return Path(mzidx.index_path_for(self.path))

    def _load_index(self) -> bool:
        """Take the scan table from the index of this file when there is a valid one (same size and probe; same modification time too, outside the browser)."""
        from . import mzidx
        try:
            path = self.index_path or self._default_index_path()
            if not path.exists():
                return False
            ix = mzidx.open(path)
        except Exception:                                 # noqa: BLE001 -- a broken or old index is simply not used
            return False
        st, src = self.path.stat(), ix.source
        by_date = self.index_path is None and not mzidx.FLAT and path.name not in mzidx.REGISTRY.values()
        if src.get("size") != st.st_size or src.get("probe") != mzidx.probe(self) or (by_date and src.get("mtime_ns") and st.st_mtime_ns and src["mtime_ns"] != st.st_mtime_ns):
            ix.close()
            return False
        self.scans = ix.scans()
        self.nce, self.hr1 = bool(ix.meta["run"]["nce"]), bool(ix.meta["run"]["hr1"])
        self.n_chromatograms = ix.meta["run"]["n_chromatograms"]
        self.sha256 = src.get("sha256", "")
        self._idx, self.index_path = ix, path
        return True

    def use_index(self, path) -> bool:
        """Switch to the index at `path` (the host moved it: the browser puts it in OPFS). The tables are taken again from it; the old one is closed. False if it does not fit this file."""
        from . import mzidx
        try:
            ix = mzidx.open(path)
            if ix.source.get("size") != self.path.stat().st_size or ix.source.get("probe") != mzidx.probe(self):
                ix.close()
                return False
        except Exception:                                 # noqa: BLE001
            return False
        old, self._idx, self.index_path = self._idx, ix, Path(path)
        self._tables = {k: v for k, v in self._tables.items() if k[0] == "mem"}
        if old:
            old.close()
        return True

    def activate(self):
        """This is the file the student is looking at: its blocks are the last to leave the cache."""
        if self._idx:
            from . import mzidx
            mzidx.CACHE.active = self._idx.uid

    def _ensure_index(self):
        """The open index; built now (next to the others, in the cache folder) when the file has none yet. False when it cannot be written."""
        if self._idx is None:
            from . import mzidx
            try:
                path = self.index_path or self._default_index_path()
                if mzidx.REGISTRY.get(self.path.name) == str(path):           # the registered index does not fit this file (it is read-only anyway): build a new one beside the others
                    mzidx.REGISTRY.pop(self.path.name)
                    path = self._default_index_path()
                t0 = time.perf_counter()
                mzidx.build(self, path)
                self._idx, self.index_path = mzidx.open(path), path
                self.timing["costruzione .mzidx"] = round(time.perf_counter() - t0, 3)
            except Exception:                              # noqa: BLE001 -- no room or no folder: keep working from memory
                self._idx = False
        return self._idx

    def _compute_sha256(self) -> str:
        try:
            import hashlib
            h = hashlib.sha256()
            if hasattr(self._mm, "__len__") and hasattr(self._mm, "find") and not hasattr(self._mm, "CACHE"):
                h.update(self._mm)
            else:
                with open(self.path, "rb") as fh:
                    while chunk := fh.read(1 << 20):
                        h.update(chunk)
            return h.hexdigest()
        except Exception:
            return ""

    def _read_header(self):
        """Instrument configurations (analyzers of each), the default one, and whether the file comes from Thermo (relative collision energy)."""
        head = self._mm[:max(self._mm.find(b"<run "), 0) or 200000].decode("utf-8", "replace")
        self.nce = 'accession="MS:1000768"' in head                       # Thermo nativeID format: the "collision energy" is relative (NCE), not eV
        self._ic: dict = {}
        for m in re.finditer(r'<instrumentConfiguration\s+id="([^"]*)"(.*?)</instrumentConfiguration>', head, re.S):
            names = []
            for blk in re.findall(r"<analyzer\b.*?</analyzer>", m.group(2), re.S):
                names += re.findall(r'<cvParam [^>]*name="([^"]*)"', blk)
            self._ic[m.group(1)] = analyzer_from_components(names)
        mm = re.search(r'<run\b[^>]*defaultInstrumentConfigurationRef="([^"]*)"', self._mm[:self._mm.find(b"<run ") + 2000].decode("utf-8", "replace"))
        self._ic_default = (mm.group(1) if mm and mm.group(1) in self._ic else next(iter(self._ic), None))

    def _analyzer(self, filt: str, ic_ref: str | None) -> str:
        """Analyzer code of a scan: the first token of a Thermo filter string; else the analyzer of its (or the default) instrumentConfiguration; else "?"."""
        tok = filt.split(" ", 1)[0].upper() if filt else ""
        if tok in _ANALYZERS:
            return tok
        return self._ic.get(ic_ref or "", self._ic.get(self._ic_default or "", "?"))

    def _link_parents(self, refs: dict):
        """Parent of every MSn scan. MS2: the Full Scan its spectrumRef names; when that is missing, the last MS1 before it with the same polarity.
        MS3 and above: the last scan of the level below whose path is this path without its last stage (nominal masses); without a path, the scan the
        spectrumRef names if it is one level below; otherwise None (in a direct infusion the generations between are often not in the file)."""
        by_native = {s.native: s.index for s in self.scans}
        last: dict = {}
        last_path: dict = {}
        for s in self.scans:
            if s.level == 1:
                last[s.polarity] = s.index
                continue
            if s.level == 2:
                ref = refs.get(s.index)
                s.parent = by_native.get(ref) if ref else None
                if s.parent is None:
                    s.parent = last.get(s.polarity, last.get(0))
            elif s.path:
                s.parent = last_path.get((s.level - 1, _path_key(s.path[:-1])))
            else:
                j = by_native.get(refs.get(s.index))
                s.parent = j if j is not None and self.scans[j].level == s.level - 1 else None
            if s.path:
                last_path[(s.level, _path_key(s.path))] = s.index
        self.hr1 = bool(mass_profile([s for s in self.scans if s.level == 1])["hr"])        # high-resolution survey scans (used to size the peak table)

    def _count(self, pat: bytes, start: int = 0) -> int:
        n, pos = 0, start
        while True:
            pos = self._mm.find(pat, pos)
            if pos < 0:
                return n
            n, pos = n + 1, pos + 1

    def close(self):
        self._mm.close()
        if self._idx:
            self._idx.close()
            self._idx = None

    def release(self):
        """Give back the memory of a file read with plain reads (the read-ahead window); the index stays. A mapped file has nothing to give back."""
        if hasattr(self._mm, "release"):
            self._mm.release()

    # ------------------------------------------------------------------ index
    def _spectrum_offsets(self):
        """Start of every <spectrum> as the indexList of an indexedmzML says, or None (no indexList, or it does not match the file)."""
        mm, n = self._mm, len(self._mm)
        tail = mm[max(n - 2048, 0):n]
        m = re.search(rb"<indexListOffset>\s*(\d+)\s*</indexListOffset>", tail)
        if not m or int(m.group(1)) >= n:
            return None
        blob = mm[int(m.group(1)):n]
        a = re.search(rb'<index\s+name="spectrum"\s*>', blob)
        if not a:
            return None
        e = blob.find(b"</index>", a.end())
        offs = [int(x) for x in re.findall(rb"<offset[^>]*>\s*(\d+)\s*</offset>", blob[a.end():e if e > 0 else len(blob)])]
        if not offs or any(y <= x for x, y in zip(offs, offs[1:])) or mm[offs[0]:offs[0] + 10] != b"<spectrum " or mm[offs[-1]:offs[-1] + 10] != b"<spectrum ":
            return None
        return offs

    def _spans(self):
        """(start, end, header text) of every spectrum. With an indexList only the header of each spectrum is read (up to <binaryDataArrayList), not its data."""
        mm = self._mm
        offs = self._spectrum_offsets()
        if offs is not None:
            last = mm.find(b"</spectrum>", offs[-1])
            if last < 0:
                offs = None
        if offs is not None:
            nxt = offs[1:] + [last + len(b"</spectrum>") + 1]
            for a, limit in zip(offs, nxt):
                n = 4096
                while True:
                    chunk = mm[a:min(a + n, limit)]
                    c = chunk.find(b"<binaryDataArrayList")
                    if c >= 0 or a + n >= limit:
                        break
                    n *= 4
                if c < 0:
                    b = a + len(chunk)
                    e = chunk.rfind(b"</spectrum>")
                    b = a + e + len(b"</spectrum>") if e >= 0 else b
                    c = None
                else:
                    tail = mm[max(limit - 64, a):limit]
                    e = tail.rfind(b"</spectrum>")
                    b = max(limit - 64, a) + e + len(b"</spectrum>") if e >= 0 else limit
                yield a, b, chunk[:c if c is not None else len(chunk)].decode("utf-8", "replace")
            return
        pos = 0
        while True:
            a = mm.find(b"<spectrum ", pos)
            if a < 0:
                break
            b = mm.find(b"</spectrum>", a)
            if b < 0:
                break
            b += len(b"</spectrum>")
            c = mm.find(b"<binaryDataArrayList", a, b)
            yield a, b, mm[a:(c if c > 0 else b)].decode("utf-8", "replace")
            pos = b

    def _index(self):
        refs, level_of = {}, {}
        for a, b, h in self._spans():
            m = re.search(r'\sid="([^"]*)"', h)
            lvl = _cv(h, "MS:1000511")
            pol = 1 if 'accession="MS:1000130"' in h else -1 if 'accession="MS:1000129"' in h else 0
            rt = _float(_cv(h, "MS:1000016")) or 0.0
            # retention time unit: minutes unless the cvParam says seconds
            m_rt = re.search(r'accession="MS:1000016"[^>]*unitName="([^"]*)"', h)
            if m_rt and m_rt.group(1).startswith("second"):
                rt /= 60.0
            lvl_i = int(float(lvl)) if lvl else 1
            hs = h.split("<precursorList", 1)[0]                   # what belongs to the scan itself, not to its precursor (a "ms level" there is the parent's)
            filt = _cv(hs, "MS:1000512") or ""
            prec = iso = act = pint = path = imm = None
            precs, ce = [], None
            if lvl_i > 1:
                path = _filter_path(filt)
                precs = [_precursor_element(me.group(2), (_RX_SREF.search(me.group(1)) or [None, None])[1]) for me in _RX_PREC.finditer(h)]
                if not precs and ("MS:1000744" in h or "MS:1000827" in h):      # precursor values outside any <precursor> element
                    precs = [_precursor_element(h, None)]
                imm = _immediate(precs, path, lvl_i, level_of)
                if imm is not None:
                    prec = imm["selected"] if imm["selected"] is not None else imm["target"]
                    tgt = imm["target"] if imm["target"] is not None else prec
                    if tgt is not None:
                        lo = 0.5 if imm["lo"] is None or imm["target"] is None else imm["target"] - imm["lo"]
                        hi = 0.5 if imm["hi"] is None or imm["target"] is None else imm["hi"] - imm["target"]
                        iso = (tgt - lo, tgt + hi)
                    act, pint = imm["act"], imm["pint"]
                    ce = imm["ce"]
                if act is None:
                    ma = _RX_ACT.search(filt)
                    act = (path[-1][1] if path else ma.group(1).upper() if ma else None)
                ref = next((e["ref"] for e in precs if e["ref"]), None)
                if ref:
                    refs[len(self.scans)] = ref
            if ce is None and "MS:1000045" in h:
                ce = _float(_cv(h, "MS:1000045"))
            mi = _RX_ICREF.search(hs)
            self.scans.append(Scan(
                index=len(self.scans), native=m.group(1) if m else "", start=a, end=b,
                level=lvl_i, rt=rt, polarity=pol,
                tic=_float(_cv(h, "MS:1000285")) or 0.0, precursor=prec,
                collision_energy=ce, filter=filt,
                profile='accession="MS:1000128"' in h, iso=iso, act=act, res=_resolution(h, hs), pint=pint,
                an=self._analyzer(filt, mi.group(1) if mi else None),
                path=path, precursors=precs, charge=imm["charge"] if imm else None))
            if m:
                level_of[m.group(1)] = lvl_i
        self._link_parents(refs)

    # ------------------------------------------------------------------ arrays
    def read(self, i: int):
        """(m/z, intensity) arrays of scan i."""
        s = self.scans[i]
        chunk = self._mm[s.start:s.end]
        mz = it = np.zeros(0)
        for blk in re.finditer(rb"<binaryDataArray .*?</binaryDataArray>", chunk, re.S):
            t = blk.group(0)
            k = t.find(b"<binary>")
            head = t[:k].decode("utf-8", "replace")
            if any(x in head for x in ("MS:1002312", "MS:1002313", "MS:1002314")):
                raise UserError("err.file.numpress", text="Numpress compression is not supported: convert with zlib or none")
            data = t[k + 8:t.find(b"</binary>")]
            raw = base64.b64decode(data) if data else b""
            if raw and 'accession="MS:1000574"' in head:   # empty arrays (scans without peaks) carry the flag but no data
                raw = zlib.decompress(raw)
            arr = np.frombuffer(raw, dtype="<f4" if 'accession="MS:1000521"' in head else "<f8").astype(float)
            if 'accession="MS:1000514"' in head:
                mz = arr
            elif 'accession="MS:1000515"' in head:
                it = arr
        n = min(len(mz), len(it))
        return mz[:n], it[:n]

    # ------------------------------------------------------------------ chromatograms (TIC/BPC/SRM)
    def chromatograms(self) -> list[dict]:
        """Header of every <chromatogram>: id, kind (tic/bpc/pda/srm/other), Q1, Q3, collision energy, dwell time."""
        if getattr(self, "_chroms", None) is not None:
            return self._chroms
        out, mm, pos = [], self._mm, (self.scans[-1].end if self.scans else 0)       # the chromatogramList follows the spectrumList
        while True:
            a = mm.find(b"<chromatogram ", pos)
            if a < 0:
                break
            b = mm.find(b"</chromatogram>", a)
            if b < 0:
                break
            c = mm.find(b"<binaryDataArrayList", a, b)
            h = mm[a:(c if c > 0 else b)].decode("utf-8", "replace")
            cid = (re.search(r'\sid="([^"]*)"', h) or [None, ""])[1]
            d = {"index": len(out), "id": cid, "start": a, "end": b + len(b"</chromatogram>"), "kind": "other"}
            if cid == "TIC":
                d["kind"] = "tic"
            elif cid == "BPC":
                d["kind"] = "bpc"
            elif cid == "TWC" or 'accession="MS:1000812"' in h or 'accession="MS:1000813"' in h:       # total wavelength chromatogram = PDA/DAD signal summed over wavelengths
                d["kind"] = "pda"
            elif 'accession="MS:1003019"' in h or cid.startswith("Pump Pressure"):                      # pump pressure (Thermo files converted with msconvert)
                d["kind"] = "pressure"
            elif cid.startswith("SRM"):
                d["kind"] = "srm"
                for key, rx in (("q1", r"Q1=([0-9.]+)"), ("q3", r"Q3=([0-9.]+)"), ("ce", r"ce=([0-9.]+)")):
                    m = re.search(rx, cid)
                    d[key] = float(m.group(1)) if m else None
                m = re.search(r"name=(.*)$", cid)
                d["name"] = m.group(1).strip() if m else ""
                dw = re.search(r'name="?MS_dwell_time"?\s+value="([^"]*)"', h)
                d["dwell"] = _float(dw.group(1)) if dw else None
            elif not any(f'accession="{a}"' in h for a in ("MS:1000235", "MS:1000627", "MS:1000628", "MS:1001473", "MS:1001472")):
                d["kind"] = "analog"             # any other detector or device trace (flow, temperature...): it has no mass-spectrometer term
            n = re.search(r'defaultArrayLength="(\d+)"', h)
            d["n"] = int(n.group(1)) if n else None
            out.append(d)
            pos = b + 1
        self._chroms = out
        self.release()
        return out

    def chromatogram(self, i: int):
        """(rt in minutes, intensity) of chromatogram i."""
        d = self.chromatograms()[i]
        chunk = self._mm[d["start"]:d["end"]]
        arrs = []
        for blk in re.finditer(rb"<binaryDataArray .*?</binaryDataArray>", chunk, re.S):
            t = blk.group(0)
            k = t.find(b"<binary>")
            head = t[:k].decode("utf-8", "replace")
            raw = base64.b64decode(t[k + 8:t.find(b"</binary>")] or b"")
            if raw and 'accession="MS:1000574"' in head:   # empty arrays (scans without peaks) carry the flag but no data
                raw = zlib.decompress(raw)
            a = np.frombuffer(raw, dtype="<f4" if 'accession="MS:1000521"' in head else "<f8").astype(float)
            arrs.append((("MS:1000595" in head), a))
        t = next((a for is_t, a in arrs if is_t), np.zeros(0))
        y = next((a for is_t, a in arrs if not is_t), np.zeros(0))
        n = min(len(t), len(y))
        if 'unitName="second"' in chunk[:chunk.find(b"<binaryDataArrayList")].decode("utf-8", "replace") or b'name="time array" value="" unitCvRef="UO" unitAccession="UO:0000010"' in chunk:
            t = t / 60.0
        return t[:n], y[:n]

    def chromatogram_unit(self, i: int) -> str:
        """The unit written on the intensity array of chromatogram i (unitName), or ''."""
        d = self.chromatograms()[i]
        chunk = bytes(self._mm[d["start"]:d["end"]])
        for blk in re.finditer(rb"<binaryDataArray .*?</binaryDataArray>", chunk, re.S):
            t = blk.group(0)
            head = t[:t.find(b"<binary>")].decode("utf-8", "replace")
            m = re.search(r'unitName="([^"]*)"', head)
            if "MS:1000595" not in head and m:
                return m.group(1)
        return ""

    def metadata(self) -> dict:
        """Instrument, software and source file names from the mzML header."""
        head = self._mm[:self._mm.find(b"<run ")].decode("utf-8", "replace")
        def names(block):
            return re.findall(r'<cvParam [^>]*name="([^"]*)"', block)
        ic = re.search(r"<instrumentConfiguration.*?</instrumentConfiguration>", head, re.S)
        ser = re.search(r'name="instrument serial number" value="([^"]*)"', head)
        # the model: the first cvParam of the instrumentConfiguration (or of the referenceableParamGroup it points to) that is not the serial number
        # nor a component (Thermo files keep it in a referenceableParamGroup; the 3200 QTRAP writes it directly)
        own = ic.group(0).split("<componentList", 1)[0] if ic else ""
        pool = names(own)
        for ref in re.findall(r'<referenceableParamGroupRef\s+ref="([^"]*)"', own):
            g = re.search(r'<referenceableParamGroup\s+id="%s"(.*?)</referenceableParamGroup>' % re.escape(ref), head, re.S)
            if g:
                pool += names(g.group(1))
        pool = [n for n in pool if n != "instrument serial number"]
        src = re.findall(r'<sourceFile id="[^"]*" name="([^"]*)"', head)
        soft = re.findall(r'<software id="[^"]*" version="([^"]*)">\s*<cvParam [^>]*name="([^"]*)"', head)
        comp = []
        if ic:
            for kind in ("source", "analyzer", "detector"):
                for blk in re.findall(r"<%s .*?</%s>" % (kind, kind), ic.group(0), re.S):
                    comp.extend(names(blk))
        model = pool[0] if pool else ""
        return {"instrument": model, "serial": ser.group(1) if ser else "", "components": comp, "analyzers": sorted(set(self._ic.values())),
                "source_files": src, "software": [f"{n} {v}".strip() for v, n in soft]}

    def scan_window(self):
        """(lowest, highest) m/z of the scan windows, or None."""
        lo = hi = None
        for s in self.scans[:50]:
            h = self._mm[s.start:s.end].decode("utf-8", "replace")
            a, b = _cv(h, "MS:1000501"), _cv(h, "MS:1000500")
            if a and b:
                lo = _float(a) if lo is None else min(lo, _float(a))
                hi = _float(b) if hi is None else max(hi, _float(b))
        return (lo, hi) if lo is not None else None

    # ------------------------------------------------------------------ experiments
    def experiments(self) -> list[dict]:
        """Groups of scans: (level, polarity), and for MS2 also the precursor (rounded)."""
        groups: dict = {}
        for s in self.scans:
            key = (s.level, s.polarity, round(s.precursor) if (s.level > 1 and s.precursor) else None)
            g = groups.setdefault(key, {"level": s.level, "polarity": s.polarity, "precursor": key[2], "n": 0,
                                        "rt_min": s.rt, "rt_max": s.rt})
            g["n"] += 1
            g["rt_min"], g["rt_max"] = min(g["rt_min"], s.rt), max(g["rt_max"], s.rt)
        return sorted(groups.values(), key=lambda g: (g["level"], g["polarity"], g["precursor"] or 0))

    def table(self, level: int = 1, polarity: int | None = None) -> PeakTable:
        """Peak table of the scans of one level (and polarity, when the file has both): a view on the .mzidx of the file, built on the first call."""
        key = (level, polarity if polarity in (1, -1) else None)
        if key in self._tables:
            return self._tables[key]
        t0 = time.perf_counter()
        ix = self._ensure_index()
        t = ix.table(level, polarity) if ix else None
        if t is None:
            return self.table_reference(level, polarity)
        self._tables[key] = t
        self.timing[f"tabella MS{level}" + (f" ({'pos' if polarity == 1 else 'neg'})" if polarity in (1, -1) else "")] = round(time.perf_counter() - t0, 3)
        return t

    def table_reference(self, level: int = 1, polarity: int | None = None) -> PeakTable:
        """The same table built in memory straight from the mzML (the reference the index is tested against; also the fallback when no index can be written)."""
        key = ("mem", level, polarity)
        if key in self._tables:
            return self._tables[key]
        t0 = time.perf_counter()
        ids = [s.index for s in self.scans if s.level == level and (polarity in (None, 0) or s.polarity in (polarity, 0))]
        rt = np.array([self.scans[i].rt for i in ids])
        tic = np.array([self.scans[i].tic for i in ids])
        # high-resolution MS1 (Run.hr1): float32 is enough for m/z (0.06 ppm at m/z 500) and intensity, and takes 12 bytes per peak instead of 20
        f32 = level == 1 and bool(getattr(self, "hr1", False))
        dt = np.float32 if f32 else np.float64
        mzs, its, poss = [], [], []
        for p, i in enumerate(ids):
            mz, it = self.read(i)
            mzs.append(mz.astype(dt, copy=False))
            its.append(it.astype(dt, copy=False))
            poss.append(np.full(len(mz), p, dtype=np.int32))
        mz = np.concatenate(mzs) if mzs else np.zeros(0, dtype=dt)
        del mzs
        order = np.argsort(mz, kind="stable")
        mz = mz[order]
        inten = (np.concatenate(its) if its else np.zeros(0, dtype=dt))[order]
        del its
        pos = (np.concatenate(poss) if poss else np.zeros(0, dtype=np.int32))[order]
        del order, poss
        t = PeakTable(rt=rt, scan_ids=np.array(ids), mz=mz, inten=inten, pos=pos, tic=tic)
        self._tables[key] = t
        self.timing[f"tabella MS{level}" + (f" ({'pos' if polarity == 1 else 'neg'})" if polarity in (1, -1) else "")] = round(time.perf_counter() - t0, 3)
        return t
