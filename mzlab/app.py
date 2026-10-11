"""The engine behind the page: the files the student opened, the session and the notebook. Runs inside Pyodide in the browser (see browser.py)."""
from __future__ import annotations

import base64
import json
import os
import re
import shutil
import threading
import time
from pathlib import Path

import numpy as np

from .explore import Session, sniff
from .i18n import UserError, message
from .peaks import merge_unit, pad_zeros, profile_peaks
from .reader.methodinfo import check_against, method_kind, read_methods
from .project import guess_conc, guess_sample


UPLOAD_SUFFIXES = (".mzml", ".dam")


def _method_summary(m: dict | None) -> dict | str:
    """One line for the list of methods, as a message for the page ({"key", "params"}; "" when there is nothing to say): what the .dam acquires."""
    if not m or m.get("error"):
        return message("load.method.unreadable") if m else ""
    ex = m.get("experiments") or []
    if not ex:
        return ""
    if all(e["kind"] == "mrm" for e in ex):
        return message("load.method.mrm", n=sum(len(e["transitions"]) for e in ex))
    rng = [e["range"] for e in ex if e.get("range")]
    if len(ex) != 1:
        return message("load.method.scans", n=len(ex))
    return message("load.method.scanRange", lo=f"{rng[0][0]:g}", hi=f"{rng[0][1]:g}") if rng else message("load.method.scan")


MAX_UPLOAD = 4 << 30  # bytes: one mzML upload


def rounded(a, nd: int) -> list:
    """[round(float(v), nd) for v in a], much faster for long traces: rint(v * 10^nd) / 10^nd is the same double as round() except when v * 10^nd lies within rounding error
    of a half (the few such values go through round() itself)."""
    a = np.asarray(a, dtype=np.float64)
    if not len(a):
        return []
    sc = 10.0 ** nd
    s = a * sc
    if not np.isfinite(s).all() or np.abs(s).max() >= 1e15:
        return [round(float(v), nd) for v in a]
    out = (np.rint(s) / sc).tolist()
    for i in np.flatnonzero(np.abs(s - np.floor(s) - 0.5) < 1e-6 * np.maximum(1.0, np.abs(s))):
        out[int(i)] = round(float(a[i]), nd)
    return out


class App:
    """The page lets the user drop files and open them for exploration; the work folder keeps the files and the notebook."""

    def __init__(self, workdir=None):
        self.workdir = Path(workdir) if workdir else None
        if self.workdir:
            self.workdir.mkdir(parents=True, exist_ok=True)
        self.lock = threading.Lock()
        self.session: Session | None = None
        if self.workdir:
            self._resume()

    def _resume(self):
        """Reopen the files that were open when the program was closed (list stored in taccuino.json)."""
        try:
            nb = self.notebook()
            smp = [x for x in nb.get("session", []) if (self.workdir / self.safe_name(x["file"])).exists()]
            if smp:
                self.open_session({"samples": smp})
        except Exception:  # noqa: BLE001 -- a broken notebook must never stop the program
            self.session = None

    def file_sha256(self, path: Path | str) -> str:
        p = Path(path)
        if not p.is_absolute() and self.workdir:
            p = self.workdir / p
        if not p.is_file():
            return ""
        st = p.stat()
        cache = self.__dict__.setdefault("_sha256_cache", {})
        key = (str(p), st.st_mtime, st.st_size)
        if key not in cache:
            import hashlib
            from .reader.mzidx import known_sha
            cache[key] = known_sha(p) if p.suffix.lower() == ".mzml" else ""          # an index of the file remembers it: no need to read the file again
            if not cache[key]:
                h = hashlib.sha256()
                with open(p, "rb") as fh:
                    while chunk := fh.read(1 << 20):
                        h.update(chunk)
                cache[key] = h.hexdigest()
        return cache[key]

    # ------------------------------------------------------------------ files and start
    def files(self) -> list[dict]:
        if not self.workdir:
            return []
        out = []
        for p in sorted(self.workdir.iterdir()):
            if p.suffix.lower() == ".mzml" and p.is_file():
                label, t, typ = guess_sample(p.name)
                c = guess_conc(p.name) if typ == "standard" else None
                out.append({"name": p.name, "size": p.stat().st_size, "sha256": self.file_sha256(p), "time": t, "type": typ, "conc": c[0] if c else None, "cunit": c[1] if c else None, **self._sniff(p)})
        return out

    def _sniff(self, p: Path) -> dict:
        """Experiment type read from the file content (cached by path and modification time); the name only gives a hint."""
        if p.suffix.lower() != ".mzml":
            return {"kind": None, "hint": None}
        key = (str(p), p.stat().st_mtime)
        cache = self.__dict__.setdefault("_sniff_cache", {})
        if key not in cache:
            try:
                r = sniff(p)
            except Exception:  # noqa: BLE001 -- an unreadable file is reported when it is opened
                r = {"kind": None}
            n = p.stem.lower()
            r["hint"] = next((h for h, rx in (("mrm", r"mrm|srm"), ("ms2", r"ms2|msms|ms-ms|product|epi"), ("full", r"full|q1|ems|scan")) if re.search(rx, n)), None)
            cache[key] = r
        return cache[key]

    def methods(self) -> list[dict]:
        """Acquisition methods (.dam) uploaded in the work folder, oldest first (name, size, one-line content)."""
        if not self.workdir:
            return []
        ps = sorted((p for p in self.workdir.iterdir() if p.suffix.lower() == ".dam" and p.is_file()), key=lambda p: p.stat().st_mtime)
        info = {m["name"]: m for m in self.lab_methods(ps)}
        return [{"name": p.name, "size": p.stat().st_size, "info": _method_summary(info.get(p.name))} for p in ps]

    def lab_methods(self, paths=None) -> list[dict]:
        """The parameters read from every uploaded .dam: [{name, source, compound, lc, file}]. Nothing is assumed when none is loaded."""
        out = []
        for p in (paths if paths is not None else [self.workdir / m["name"] for m in self.methods()]):
            key = (str(p), p.stat().st_mtime)
            cache = self.__dict__.setdefault("_dam_cache", {})
            if key not in cache:
                try:
                    r = read_methods(p)[0]
                    r["name"] = p.name
                    cache[key] = r
                except Exception as e:  # noqa: BLE001 -- a broken file must not break the page
                    cache[key] = {"name": p.name, "error": f"cannot read this file ({e})", "error_key": "err.dam.unreadable", "params": {"message": str(e)}, "source": [], "compound": [], "file": {}}
            out.append(cache[key])
        return out

    @staticmethod
    def safe_name(name: str) -> str:
        n = Path(name.replace("\\", "/")).name.strip()
        if not n or n.startswith(".") or Path(n).suffix.lower() not in UPLOAD_SUFFIXES:
            raise UserError("err.file.notAccepted", {"name": name}, f"file type not accepted: {name!r} (use .mzML, or a method .dam; convert a .wiff to .mzML first)")
        return re.sub(r"[\x00-\x1f<>:\"|?*]", "_", n)

    def save_upload(self, name: str, stream, length: int) -> str:
        if not self.workdir:
            raise UserError("err.upload.noWorkFolder", text="no work folder")
        n = self.safe_name(name)
        if length > MAX_UPLOAD:
            raise UserError("err.upload.tooLarge", text="file too large")
        if length > shutil.disk_usage(self.workdir).free - (200 << 20):
            raise UserError("err.upload.noSpace", text="not enough free disk space")
        dest = self.workdir / n
        tmp = dest.with_name(dest.name + ".part")
        left = length
        import hashlib
        hasher = hashlib.sha256()
        with open(tmp, "wb") as fh:
            while left > 0:
                chunk = stream.read(min(1 << 20, left))
                if not chunk:
                    break
                fh.write(chunk)
                hasher.update(chunk)
                left -= len(chunk)
        if left:
            tmp.unlink(missing_ok=True)
            raise UserError("err.upload.interrupted", text="upload interrupted")
        os.replace(tmp, dest)
        st = dest.stat()
        self.__dict__.setdefault("_sha256_cache", {})[(str(dest), st.st_mtime, st.st_size)] = hasher.hexdigest()
        return n

    def remove_file(self, name: str):
        """Never deleted: moved to _cestino inside the work folder."""
        n = self.safe_name(name)
        src = self.workdir / n
        trash = self.workdir / "_cestino"
        trash.mkdir(exist_ok=True)
        # an open session keeps the file memory-mapped: on Windows a mapped file cannot be moved, so release it first
        # (the file leaves the session too: it is gone from the work folder)
        if self.session:
            for it in list(self.session.items):
                if Path(it.file).name == n:
                    try:
                        it.run.close()
                    except Exception:  # noqa: BLE001 -- already closed or never opened
                        pass
                    self.session.items.remove(it)
        for f in (src,):
            if f.exists():
                os.replace(f, trash / f.name)

    def notebook_path(self) -> Path | None:
        return self.workdir / "taccuino.json" if self.workdir else None

    def notebook(self) -> dict:
        p = self.notebook_path()
        if p and p.exists():
            return json.loads(p.read_text(encoding="utf-8"))
        return {}

    def save_notebook(self, data: dict):
        p = self.notebook_path()
        if not p:
            raise UserError("err.notebook.noWorkFolder", text="no work folder: use the download button")
        tmp = p.with_suffix(".tmp")
        tmp.write_text(json.dumps(data), encoding="utf-8")
        os.replace(tmp, p)

    def open_session(self, payload: dict):
        """Open the files for exploration: no compound, no candidates, just the data."""
        samples = []
        for s_ in payload.get("samples", []):
            n = self.safe_name(s_["file"])
            if not (self.workdir / n).exists():
                raise UserError("err.session.fileNotFound", {"name": n}, f"file not found: {n}")
            t = s_.get("time")
            c = s_.get("conc")
            samples.append({"file": n, "label": s_.get("label"), "type": s_.get("type", "sample"),
                            "time": None if t in (None, "") else float(t), "conc": None if c in (None, "") else float(c), "cunit": s_.get("cunit"),
                            "rep": s_.get("rep")})
        if not samples:
            raise UserError("err.session.noFiles", text="add at least one file")
        if payload.get("append") and self.session:          # one more file in the session that is open (the page opens a list file by file)
            self.session.add(samples)
        else:
            self.session = Session(samples, self.workdir)

    def set_meta(self, payload: dict) -> dict:
        """Metadata of the open files (type, time in minutes, concentration, replicate group), changed from the sample table."""
        if not self.session:
            raise UserError("err.session.none", text="no session open")
        return {"changed": self.session.set_meta(list(payload.get("files", [])))}

    def _git_commit(self) -> str:
        cache = self.__dict__.setdefault("_git_commit_val", None)
        if cache is None:
            try:
                import subprocess
                p = subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, cwd=Path(__file__).parent.parent)
                self._git_commit_val = p.stdout.strip() or "dev"
            except Exception:
                self._git_commit_val = "dev"
        return self._git_commit_val

    def session_state(self) -> dict:
        return {"session": self.session.info() if self.session else None, "app": bool(self.workdir),
                "workdir": str(self.workdir) if self.workdir else None, "files": self.files(), "methods": self.methods(),
                "version": __import__("mzlab").__version__,
                "commit": self._git_commit()}

    def perf(self) -> dict:
        """How each open file was read (mode, size, seconds per step): for the ?perf meter of the page."""
        seen, out = set(), []
        for it in (self.session.items if self.session else []):
            if id(it.run) in seen:
                continue
            seen.add(id(it.run))
            out.append({"file": Path(it.file).name.partition("#")[0], "mode": it.run.mode, "size": it.path.stat().st_size, "indexed": bool(it.run._idx), "timing": dict(it.run.timing)})
        return {"files": out}

    def _item(self, k: int):
        if not self.session or not 0 <= k < len(self.session.items):
            raise UserError("err.session.noSuchFile", text="no such file in the session")
        return self.session.items[k]

    def chrom(self, k: int, kind: str, level: int, mz0: float | None = None, mz1: float | None = None, prec: float | None = None, filt: str | None = None) -> dict:
        item = self._item(k)
        item.run.activate()
        rt, y = item.total(kind, level, mz0, mz1, prec, filt)
        return {"rt": rounded(rt, 4), "y": rounded(y, 1)}

    def xic(self, ks: list[int], mz: float, tol: float, level: int) -> dict:
        out = []
        for k in ks:
            rt, y = self._item(k).xic(mz, tol, level)
            out.append({"k": k, "rt": rounded(rt, 4), "y": rounded(y, 1)})
        return {"mz": mz, "tol": tol, "traces": out}

    def qaqc_ion(self, ks: list[int], roles: list[str], mz: float, rt: float, win: float, tol: float, unit: str = "da",
                 level: int = 1, thr: float = 3.0, ratio: bool = False) -> dict:
        """QA/QC measures of ONE ion (an internal standard, a known ion) in every file of `ks`: apex RT, RT shift, area, area
        relative to the median, observed m/z and error in ppm (the m/z of the apex window, intensity weighted), plus the summary
        of the series (drift, CV, mass accuracy) and, with `ratio`, the sample / blank area ratio. tol in Da (unit "da") or ppm."""
        from . import qaqc
        if len(roles) != len(ks):
            raise UserError("err.qaqc.roles", text="one role per file is needed")
        tol_da = tol if unit == "da" else mz * tol * 1e-6
        rows = []
        for k in ks:
            item = self._item(k)
            item.run.activate()
            t = item._tbl(level)
            y = t.xic(mz, tol_da)
            pk = qaqc.peak_in_window(t.rt, y, rt, win)
            if not pk:
                rows.append(None)
                continue
            a, b = np.searchsorted(t.mz, mz - tol_da, side="left"), np.searchsorted(t.mz, mz + tol_da, side="right")
            sel = (t.pos[a:b] >= pk["i0"]) & (t.pos[a:b] <= pk["i1"])
            w = t.inten[a:b][sel].astype(float)
            mo = float(np.sum(t.mz[a:b][sel].astype(float) * w) / w.sum()) if w.sum() > 0 else None
            rows.append({"k": k, "rt": round(pk["apex"], 4), "area": pk["area"], "height": pk["height"],
                         "mz_obs": None if mo is None else round(mo, 5),
                         "dppm": None if (mo is None or unit == "da") else round(qaqc.ppm_error(mo, mz), 3)})
        out = qaqc.summarize(rows, roles, rt)
        if ratio:
            out["ratio"] = qaqc.sample_blank_ratios([r["area"] if r else 0.0 for r in rows], roles, thr)
        out.update({"mz": mz, "rt": rt, "win": win, "tol": tol, "unit": unit, "level": level})
        return out

    def subxic(self, k: int, formula: str, ppm: float = 5.0, min_mz: float = 100.0, z: int = 1) -> dict:
        """XIC that sums every sub-formula of `formula` (high resolution files only) and the measures of each one (chem/subformulas.py)."""
        from .chem.subformulas import measure, subformulas
        item = self._item(k)
        if not item.hr_on(1):
            raise UserError("err.subxic.lr", text="the combined XIC of the sub-formulas needs a high-resolution file")
        if not 0 < ppm <= 100:
            raise UserError("err.subxic.ppm", text="ppm must be between 0 and 100")
        f, subs = subformulas(formula, z, min_mz)
        if not subs:
            raise UserError("err.subxic.none", {"mz": min_mz}, "no sub-formula above the lower limit")
        return {"k": k, "ppm": ppm, "min_mz": min_mz, "z": z, **measure(item._tbl(1), f, subs, ppm)}

    def mrm(self, ks: list[int]) -> dict:
        return {"files": [{"k": k, "transitions": self._item(k).mrm()} for k in ks]}

    def method(self, k: int) -> dict:
        m = self._item(k).method()
        m["lab"] = [{**x, "check": check_against(m, x)} for x in self.lab_methods()]   # only what the user uploaded as .dam: no built-in method is assumed
        pick = {"full": "full", "ms2": "ms2", "mrm": "mrm"}.get(m.get("kind"), "")
        names = [x["name"].lower() for x in m["lab"]]
        hit = [i for i, n in enumerate(names) if pick and pick in n] or [i for i, n in enumerate(names) if m.get("kind") == "ms2" and "ms2" in n]
        m["lab_pick"] = hit[-1] if hit else (len(names) - 1 if names else None)
        # several .dam loaded: preselect the one that agrees best with this file (fewest differences), name as tie-break
        if len(m["lab"]) > 1:
            diffs = [sum(r["status"] == "diff" for r in x["check"]) for x in m["lab"]]
            best = min(diffs)
            if diffs[m["lab_pick"]] > best:
                m["lab_pick"] = [i for i, d in enumerate(diffs) if d == best][-1]
        return m

    def method_warnings(self) -> dict:
        """The loaded .dam methods whose experiment type (MRM / Full Scan / MS2) is not among the types of the open files."""
        if not self.session:
            return {"warnings": []}
        have = sorted({it.kind() for it in self.session.items})
        names = {"full": "Full Scan", "ms2": "MS2", "mrm": "MRM"}
        out = []
        for x in self.lab_methods():
            g = method_kind(x)
            if g and g not in have:
                out.append({"method": x.get("name"), "expects": names[g], "files": [names[h] for h in have if h in names]})
        return {"warnings": out}

    def origin(self, k: int, mz: float, parent: float, rt0: float | None = None, rt1: float | None = None, formula: str | None = None) -> dict:
        """"Da dove viene questo ione?": the evidence of mzlab.ionfamily for the ion at mz against the candidate precursor `parent`,
        over every full-scan file of the session (k = the file the student is looking at, used as the reference). No verdict."""
        from . import ionfamily
        if not self.session:
            raise UserError("err.session.none", text="no session open")
        full = [(i, it) for i, it in enumerate(self.session.items) if it.kind() != "mrm" and it.type != "blank" and len(it.run.table(1).rt)]
        if not full:
            raise UserError("err.origin.needFullScan", text="full scan files are needed")
        samples = [{"label": it.label, "time": it.time, "table": it.run.table(1), "key": str(it.path)} for _, it in full]
        ref = next((j for j, (i, _) in enumerate(full) if i == k), None)
        ms2 = None
        for it in self.session.items:                  # product-ion spectra of X and of P, if the student loaded an MS2 file
            if not any(x.level == 2 for x in it.run.scans):
                continue
            lo, hi = it.info()["rt_min"], it.info()["rt_max"]
            sx, sp = it.spectrum(lo, hi, 2, mz), it.spectrum(lo, hi, 2, parent)
            if sp[2]:
                ms2 = {"x": (sx[0], sx[1]) if sx[2] else None, "p": (sp[0], sp[1])}
                break
        rep = ionfamily.origin_report(samples, mz, parent, rt0, rt1, ref, formula_p=formula or None, top=25, ms2=ms2)   # 25 co-eluting ions are plenty to read (and the browser version is ~10x slower)
        rep["hr"] = any(it.profile(1)["hr"] for _, it in full)            # the page adds the list of proofs of mzFinder only for high-resolution files
        rep["pol"] = next((int(s.polarity) for _, it in full for s in it.run.scans if s.polarity), 1)
        if ms2 is not None:                                                # the strongest peaks of the product-ion spectra, for those proofs (tolerance of HR)
            def strongest(sp):
                if sp is None:
                    return None
                a, y = np.asarray(sp[0], float), np.asarray(sp[1], float)
                k = np.argsort(-y)[:40]
                return [a[k].tolist(), y[k].tolist()]
            rep["ms2_peaks"] = {"x": strongest(ms2["x"]), "p": strongest(ms2["p"])}
        return rep

    @staticmethod
    def _spec_arrays(item, level: int, mz, y, merge: bool, hr: bool = True) -> dict:
        """What _spec_json sends, as numpy arrays (not rounded, not listed): mode, mz/y (peaks) and, for profile spectra, pmz/py (the profile line)."""
        hr = item.hr_on(level, hr)
        if item.spectrum_mode(level) == "profile":
            pm, py = profile_peaks(mz, y)
            lm, ly = pad_zeros(mz, y, item._step(level))
            return {"mode": "profile", "mz": np.asarray(pm, float), "y": np.asarray(py, float), "pmz": np.asarray(lm, float), "py": np.asarray(ly, float)}
        if merge and not hr:
            mz, y = merge_unit(mz, y)
        return {"mode": item.spectrum_mode(level), "mz": np.asarray(mz, float), "y": np.asarray(y, float)}

    @staticmethod
    def _spec_json(item, level: int, mz, y, merge: bool, hr: bool = True) -> dict:
        """A spectrum for the front end. Centroids: the peaks (merged by unit window if asked). Profile: `mz`/`y` are the PEAKS (one per
        nominal mass, `peaks.profile_peaks`) and `pmz`/`py` the profile points, drawn as a line; labels, table, ruler and clicks use the peaks."""
        hr = item.hr_on(level, hr)                                  # high-resolution centroids: decimals of the profile + 1, never merged by nominal mass
        d = item.profile(level)["dec"] + 1 if hr else 3
        r3 = lambda a: [round(float(v), d) for v in a]
        r1 = lambda a: [round(float(v), 1) for v in a]
        if item.spectrum_mode(level) == "profile":
            pm, py = profile_peaks(mz, y)
            lm, ly = pad_zeros(mz, y, item._step(level))
            return {"mode": "profile", "mz": r3(pm), "y": r1(py), "pmz": r3(lm), "py": r1(ly)}
        if merge and not hr:
            mz, y = merge_unit(mz, y)
        return {"mode": item.spectrum_mode(level), "mz": r3(mz), "y": r1(y)}

    def nearest_scan(self, k: int, rt: float, filter: str | None = None, level: int = 1, precursor: float | None = None) -> dict:
        ans = self._item(k).nearest_scan(rt, level, precursor, filter)
        if not ans:
            raise UserError("err.scan.none", text="no scan in this file")
        return ans

    def filters(self, k: int) -> dict:
        """The scan types of the physical file of item k (what the filter menu of a cell lists), with their counts: see Item.filter_groups."""
        return {"filters": self._item(k).filter_groups()}

    def scaninfo(self, k: int, sid: int) -> dict:
        return self._item(k).scan_header(sid)

    def scanlist(self, k: int, level: int, filt: str | None = None) -> dict:
        return self._item(k).scan_table(level, filt)

    def fileinfo(self, k: int) -> dict:
        it = self._item(k)
        out = it.file_info()
        for lv in (1, 2):                                    # m/z range of the survey and of the product ions, as the page already knows it
            try:
                tb = it._tbl(lv)
                if len(tb.mz):
                    out.setdefault("mz_range", {})[str(lv)] = [round(float(tb.mz[0]), 4), round(float(tb.mz[-1]), 4)]
            except Exception:  # noqa: BLE001
                pass
        out.pop("mz", None)
        return out

    def spectrum(self, k: int, rt0: float, rt1: float, level: int, precursor, bin_da: float, bg=None, merge: bool = False, hr: bool = True, filt: str | None = None) -> dict:
        if bg is not None:
            bg = {**bg, "item": self._item(bg["k"])}
        item = self._item(k)
        pt = item.ptol(level, precursor, hr)
        mz, y, n = item.spectrum(rt0, rt1, level, precursor, prec_tol=pt, bin_da=bin_da, bg=bg, hr=hr, filt=filt)
        return {**self._spec_json(item, level, mz, y, merge, hr), "scans": n, **item.window_scans(rt0, rt1, level, precursor, pt, filt)}

    MAX_SPECTRA = 60       # scans per /api/spectra request

    def spectra(self, k: int, i0: int, i1: int, level: int, precursor, bin_da: float, merge: bool = False, hr: bool = True, filt: str | None = None) -> dict:
        """Single scans i0..i1 of file k (for scan-by-scan navigation); each one equals /api/spectrum on a window holding only that scan."""
        item = self._item(k)
        if item.kind() == "mrm":
            raise UserError("err.scan.mrm", text="this file is MRM: it contains only transition chromatograms")
        if i0 > i1:
            raise UserError("err.scan.range", text="invalid scan range: i0 is greater than i1")
        if i1 - i0 + 1 > self.MAX_SPECTRA:
            raise UserError("err.scan.maxRequest", {"n": self.MAX_SPECTRA}, f"at most {self.MAX_SPECTRA} scans per request")
        pt = item.ptol(level, precursor, hr)
        n = item.scan_count(level, precursor, pt, filt)
        if i0 < 0 or i0 >= n:
            raise UserError("err.scan.outsideN", {"n": n}, f"scan outside the file (the file has {n})")
        return {"n": n, "scans": [{"i": s["i"], "sid": s["sid"], "rt": round(s["rt"], 4), **self._spec_json(item, level, s["mz"], s["y"], merge, hr)}
                                  for s in item.scans(i0, i1, level, precursor, prec_tol=pt, bin_da=bin_da, hr=hr, filt=filt)]}

    def scanbin(self, k: int, i0: int, i1: int, level: int = 1, precursor=None, filt: str | None = None, hr: bool = True, merge: bool = False) -> bytes:
        """Single scans i0..i1 (at most 60) as one binary block, for navigation: 4 bytes (little endian) = length of the JSON header, the JSON header
        {n, scans: [{i, sid, rt, no, filter, mode, n, np, nl, tic}]}, then for every scan its peaks (m/z float64, intensity float32; n values each) and,
        for a profile scan, its profile line (pmz float64, py float32; np values each). nl = highest intensity, tic = sum of the intensities."""
        item = self._item(k)
        if item.kind() == "mrm":
            raise UserError("err.scan.mrm", text="this file is MRM: it contains only transition chromatograms")
        if i0 > i1:
            raise UserError("err.scan.range", text="invalid scan range: i0 is greater than i1")
        if i1 - i0 + 1 > self.MAX_SPECTRA:
            raise UserError("err.scan.maxRequest", {"n": self.MAX_SPECTRA}, f"at most {self.MAX_SPECTRA} scans per request")
        pt = item.ptol(level, precursor, hr)
        ids, rts = item._scan_ids(level, precursor, pt, filt)       # one scan filter = one scan type (a path of fragmentation, MS1, ...)
        n = len(ids)
        if i0 < 0 or i0 >= n:
            raise UserError("err.scan.outsideN", {"n": n}, f"scan outside the file (the file has {n})")
        i1 = min(i1, n - 1)
        if filt:
            rows = []
            for j in range(i0, i1 + 1):
                mz, y = item.run.read(int(ids[j]))
                rows.append({"i": j, "sid": int(ids[j]), "rt": float(rts[j]), "mz": mz, "y": y})
        else:
            rows = item.scans(i0, i1, level, precursor, prec_tol=pt, bin_da=0.1, hr=hr)
        head, body = [], []
        for r in rows:
            sc = item.run.scans[r["sid"]]
            a = self._spec_arrays(item, level, r["mz"], r["y"], merge, hr)
            npk = len(a["mz"]); npr = len(a["pmz"]) if "pmz" in a else 0
            head.append({"i": r["i"], "sid": r["sid"], "rt": round(r["rt"], 4), "no": item._no(sc), "filter": sc.filter, "mode": a["mode"], "n": npk, "np": npr,
                         "nl": float(a["y"].max()) if npk else 0.0, "tic": float(a["y"].sum()) if npk else 0.0})
            body.append(a["mz"].astype("<f8").tobytes()); body.append(a["y"].astype("<f4").tobytes())
            if npr:
                body.append(a["pmz"].astype("<f8").tobytes()); body.append(a["py"].astype("<f4").tobytes())
        hb = json.dumps({"n": n, "scans": head}).encode("utf-8")
        return len(hb).to_bytes(4, "little") + hb + b"".join(body)

    def dda(self, k: int, hr: bool = True) -> dict:
        """The MS2 scans of the physical file of item k with parent / isolation / activation (see Item.dda)."""
        return self._item(k).dda(hr)

    def scan(self, k: int, sid: int, hr: bool = True) -> dict:
        return self._item(k).scan(sid, hr)

    def msn_tree(self, k: int, formula: str | None = None, ppm: float = 5.0) -> dict:
        """Tree of the fragmentation paths of file k (MSn files; see chem/msntree.py)."""
        from .chem.msntree import msn_tree
        return msn_tree(self._item(k).run, formula or None, ppm)

    def scanavg(self, k: int, sids: list[int], hr: bool = True) -> dict:
        if len(sids) > 200:
            raise UserError("err.scan.maxAverage", {"n": 200}, "at most 200 scans per average")
        return self._item(k).scan_avg(sids, hr)

    def reset(self, fresh: bool = False):
        with self.lock:
            if self.session:
                for it in list(self.session.items):      # release the memory-mapped files before they go
                    try:
                        it.run.close()
                    except Exception:  # noqa: BLE001
                        pass
            if fresh and self.workdir:                   # "Nuova sessione": the files and the notebook of the page are dropped
                for f in self.workdir.iterdir():
                    if f.is_file():
                        f.unlink(missing_ok=True)
            self.session = None

    def state(self):
        return {"error": None, "phase": "start", "app": bool(self.workdir), "workdir": str(self.workdir) if self.workdir else None,
                "files": self.files(), "methods": self.methods(), "version": __import__("mzlab").__version__}
