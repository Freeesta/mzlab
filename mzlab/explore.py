"""Exploration of mzML files: chromatograms, mass spectra and ion chromatograms on demand.

Nothing is computed that the user did not ask for: the page starts from the total ion chromatogram and the
students choose what to extract. Unit resolution data: windows are in Da.
"""
from __future__ import annotations

import re
from pathlib import Path

import numpy as np

from .bigfiles import hr_profile
from .differenza import subtract_spectrum
from .i18n import UserError
from .project import guess_sample
from .reader.mzml import Run
from .reader.profile import LOW, mass_profile

MAP_POINTS_MAX = 200_000               # most points the sticks of the 3D viewer can ask for
HR_CELL = float(np.log1p(3e-6))        # high resolution: centroids closer than 3 ppm are the same peak when scans are averaged


_KEYRX = re.compile(r"^(\w+)\s+([+-])\s")


def scan_key(s) -> str:
    """Name of the TYPE of a scan (what the filter menu of a cell lists): the Thermo filter without the precursor mass and the scan range for the
    survey and the DDA product ions («FTMS + p ESI Full ms», «FTMS + ms2 @hcd»), the whole path for MSn («FTMS + ms4 317 > 261 > 244 @cid35»)."""
    f = s.filter or ""
    if s.level <= 1:
        return re.sub(r"\s*\[[^\]]*\]\s*$", "", f) or "ms1"
    m = _KEYRX.match(f)
    pre = f"{m.group(1)} {m.group(2)} " if m else ""
    act = (s.act or (s.path[-1][1] if s.path else "") or "").lower()
    if s.path and len(s.path) > 1:
        return f"{pre}ms{s.level} " + " > ".join(str(round(x[0])) for x in s.path) + f" @{act}{s.path[-1][2]:g}"
    return f"{pre}ms{s.level} @{act}".rstrip(" @")


class Item:
    def __init__(self, file: str, label: str | None, time, typ: str, path: Path, conc: float | None = None, cunit: str | None = None,
                 part: dict | None = None, run: Run | None = None):
        # part = {"mode": "ms1" | "ms2" | "mrm", "pol": 1 | -1 | None, "tag": "MS2"}: one experiment of a MIXED file (survey + product ions, MRM + EPI, + and - polarity).
        # The parts of a file share one Run (the file is read once); `file` is then "name.mzML#tag", the name of the physical file is `path.name`.
        self.file, self.time, self.type, self.path, self.conc, self.cunit = file, time, typ, path, conc, cunit
        self.rep: str | None = None           # replicate group typed by the user (None = the automatic one: same type, time and concentration)
        self.part = part
        base = label or Path(path).stem
        self.label = base + (f" · {part['tag']}" if part and not base.endswith(part["tag"]) else "")
        self.run = run or Run(path)
        self.sha256 = getattr(self.run, "sha256", "")
        self.pol = part.get("pol") if part else None
        self.sc = [s for s in self.run.scans if self._match(s)]
        self._bpc: dict = {}
        self._tabs: dict = {}

    def _match(self, s) -> bool:
        p = self.part
        if not p:
            return True
        if p["mode"] == "mrm" or (p.get("pol") not in (None, 0) and s.polarity not in (p["pol"], 0)):
            return False
        return s.level == 1 if p["mode"] == "ms1" else s.level >= 2

    def _keys(self) -> dict:
        """scan index -> scan_key, for every scan of the physical file (computed once, shared by the parts of a file)."""
        d = self.run.__dict__.get("_skeys")
        if d is None:
            d = self.run.__dict__["_skeys"] = {sc.index: scan_key(sc) for sc in self.run.scans}
        return d

    def filter_groups(self) -> list[dict]:
        """The scan types of the physical file with their counts (one filter = one scan type): [{key, level, n, rt0, rt1, pol}], the survey first."""
        keys, out = self._keys(), {}
        for sc in self.run.scans:
            g = out.get(keys[sc.index])
            if g is None:
                out[keys[sc.index]] = {"key": keys[sc.index], "level": sc.level, "n": 1, "rt0": float(sc.rt), "rt1": float(sc.rt), "pol": sc.polarity}
            else:
                g["n"] += 1; g["rt0"] = min(g["rt0"], float(sc.rt)); g["rt1"] = max(g["rt1"], float(sc.rt))
        for g in out.values():
            g["rt0"], g["rt1"] = round(g["rt0"], 4), round(g["rt1"], 4)
        return sorted(out.values(), key=lambda g: (g["level"], -g["n"], g["key"]))

    def _fkeep(self, ids, filt):
        """Boolean mask over scan indexes `ids`: the scans of the type `filt` (a scan_key as /api/filters lists it, or a whole Thermo filter string)."""
        keys, sc = self._keys(), self.run.scans
        return np.array([keys[int(i)] == filt or sc[int(i)].filter == filt for i in ids], dtype=bool)

    def _tbl(self, level: int):
        """Peak table of this experiment (level and, for a part of a file with both polarities, the polarity)."""
        return self.run.table(level, self.pol)

    def spectrum_mode(self, level: int | None = None) -> str:
        """'profile', 'centroid' or 'mixed': read from the scans themselves (MS:1000128 / MS:1000127), not from the file name."""
        sc = [s for s in self.sc if level is None or s.level == level] or self.sc
        n = sum(1 for s in sc if s.profile)
        return "profile" if sc and n == len(sc) else "centroid" if n == 0 else "mixed"

    def _step(self, level: int):
        """Point spacing of a profile spectrum (Da), None for centroids: profile points are summed on their own grid, not in bins of 0.1 Da."""
        cache = self.__dict__.setdefault("_steps", {})
        if level not in cache:
            ids = [s.index for s in self.sc if s.level == level and s.profile]
            step = None
            if ids and self.spectrum_mode(level) == "profile":
                for i in ids[len(ids) // 2:len(ids) // 2 + 20]:
                    mz, _ = self.run.read(i)
                    if len(mz) > 20:
                        d = np.diff(mz)
                        step = float(np.round(d[d > 1e-4].min(), 4))                  # the shortest gap = the point spacing (the converter leaves out zeros, so other gaps are multiples)
                        break
            cache[level] = step if step and step > 0 else None
        return cache[level]

    def _cells(self, mz, bin_da: float, level: int, hr: bool = False):
        """Cell of each point: bins of bin_da Da for centroids (cells of 3 ppm in high resolution), the point grid itself for profile spectra."""
        st = self._step(level)
        if st:
            return np.rint(mz / st).astype(np.int64)
        if hr:
            return np.floor(np.log(np.maximum(mz, 1e-9)) / HR_CELL).astype(np.int64)
        return np.floor(mz / bin_da).astype(np.int64)

    def info(self) -> dict:
        r = self.run
        ms1 = [s for s in self.sc if s.level == 1]
        ms2 = [s for s in self.sc if s.level >= 2]
        t1 = self._tbl(1) if ms1 else None
        t2 = self._tbl(2) if ms2 else None
        pol = sorted({s.polarity for s in self.sc if s.polarity})
        out = {"file": self.file, "label": self.label, "sha256": getattr(self, "sha256", "") or getattr(r, "sha256", ""),
                "time": self.time, "type": self.type, "conc": self.conc, "cunit": self.cunit, "rep": self.rep,
                "scans": len(self.sc), "ms1": len(ms1), "ms2": len(ms2),
                "rt_min": float(min((s.rt for s in self.sc), default=0.0)),
                "rt_max": float(max((s.rt for s in self.sc), default=0.0)),
                "mz_min": float(t1.mz[0]) if t1 is not None and len(t1.mz) else None,
                "mz_max": float(t1.mz[-1]) if t1 is not None and len(t1.mz) else None,
                "mz2_min": float(t2.mz[0]) if t2 is not None and len(t2.mz) else None,
                "mz2_max": float(t2.mz[-1]) if t2 is not None and len(t2.mz) else None,
                "polarity": "positive" if pol == [1] else "negative" if pol == [-1] else "mixed" if pol else "unknown",
                "precursors": sorted({round(s.precursor, 1) for s in ms2 if s.precursor}), "spectrum_mode": self.spectrum_mode(),
                "ms2_exps": self._ms2_exps(ms2), "chromatograms": r.n_chromatograms, "kind": self.kind(),
                "srm": sum(1 for c in r.chromatograms() if c["kind"] == "srm"), "pda": self.has_pda()}
        out.update(self.hr_info())
        if self.part:
            out["part"] = self.part["tag"]
            if self.part["mode"] == "ms2":                # when each product-ion scan started and from which precursor (the triangles on the survey chromatogram)
                out["ms2_events"] = [[round(float(s.rt), 4), round(float(s.precursor), 1) if s.precursor else None] for s in ms2]
        if not self.sc:                                # MRM file: no scans, take times and polarity from the chromatograms
            rng = self._srm_rt_range()
            if rng:
                out["rt_min"], out["rt_max"] = rng
            out["polarity"] = self._header_polarity()
        return out

    def hr_info(self) -> dict:
        """High-resolution facts of the PHYSICAL file (both levels, whichever part this item is): mass profile of the survey (`prof1`) and of the
        product-ion scans (`prof2`), instrument, resolving power (median), relative collision energy (Thermo), DDA.

        Anything wrong here must never stop the file from opening: on an error the answer is today's low-resolution profile and `hr_err` says why."""
        try:
            r = self.run
            sc1 = [s for s in r.scans if s.level == 1]
            sc2 = [s for s in r.scans if s.level > 1]
            med = lambda v: float(np.median(v)) if len(v) else None
            md = r.__dict__.get("_md")
            if md is None:
                md = r.__dict__["_md"] = r.metadata()
            return {"prof1": mass_profile(sc1), "prof2": mass_profile(sc2), "instrument": md.get("instrument", ""),
                    "res1": med([s.res for s in sc1 if s.res]), "res2": med([s.res for s in sc2 if s.res]), "nce": bool(r.nce),
                    "dda": bool(sc1 and sc2 and all(s.parent is not None for s in sc2)), "infusion": infusion_like(r.scans), "max_level": max((s.level for s in r.scans), default=1),
                    "acq": self._acq(sc1, sc2), "mode1": "profile" if any(s.profile for s in sc1[:50]) else "centroid" if sc1 else None,
                    "mode2": "profile" if any(s.profile for s in sc2[:50]) else "centroid" if sc2 else None}
        except Exception as e:  # noqa: BLE001 -- the fallback is the behaviour of today
            return {"prof1": dict(LOW), "prof2": dict(LOW), "instrument": "", "res1": None, "res2": None, "nce": False, "dda": False, "infusion": False,
                    "hr_err": f"{type(e).__name__}: {e}"[:160]}

    @staticmethod
    def _acq(sc1, sc2) -> list[str]:
        """What the file is, from its scans: MS1, SIM, DDA / DIA / AIF / PRM (product ions), MSn (more than one stage of fragmentation)."""
        out = ["MS1"] if sc1 else []
        if any("sim" in (s.filter or "").lower().split() for s in sc1):
            out.append("SIM")
        if not sc2:
            return out
        wins = [s.iso for s in sc2 if s.iso]
        if max(s.level for s in sc2) >= 3:
            out.append("MSn")                                           # infusion with several stages of fragmentation: not a DIA or PRM scheme
            return out
        if not wins:
            out.append("AIF")                                           # product ions without an isolation window: all ions fragmented together
        else:
            width = float(np.median([hi - lo for lo, hi in wins])); centres = [round((lo + hi) / 2, 1) for lo, hi in wins]
            n, d = len(centres), len(set(centres))
            if width >= 5 and n / max(d, 1) >= 3:
                out.append("DIA")                                       # wide windows that come back cycle after cycle
            elif sc1 and all(s.parent is not None for s in sc2):
                out.append("DDA")
            elif n / max(d, 1) >= 5 and max(s.level for s in sc2) == 2:
                out.append("PRM")                                       # the same few targets again and again
            else:
                out.append("MS2")
        return out

    # ------------------------------------------------------------------ what the file says (information bar of the bench)
    def scan_header(self, sid: int) -> dict:
        """Every name / value pair of the header of one scan as the mzML has it (the cvParam and userParam up to the arrays), plus the known fields."""
        r = self.run
        if not 0 <= sid < len(r.scans):
            raise UserError("err.scan.outside", text="scan outside the file")
        sc = r.scans[sid]
        head = bytes(r._mm[sc.start:sc.end]).decode("utf-8", "replace").split("<binaryDataArrayList", 1)[0]
        pairs = []
        for m in re.finditer(r"<(cvParam|userParam)\b([^>]*?)/?>", head):
            a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(2)))
            if a.get("name"):
                pairs.append({"name": a["name"], "value": a.get("value", ""), "unit": a.get("unitName", "")})
        return {"sid": sid, "no": self._no(sc), "filter": sc.filter, "rt": round(float(sc.rt), 4), "level": sc.level,
                "polarity": {1: "positive", -1: "negative"}.get(sc.polarity, "?"), "profile": bool(sc.profile),
                "prec": round(float(sum(sc.iso) / 2), 5) if sc.iso else round(float(sc.precursor), 5) if sc.precursor else None,
                "iso": [round(float(sc.iso[0]), 4), round(float(sc.iso[1]), 4)] if sc.iso else None, "act": sc.act, "ce": sc.collision_energy,
                "res": sc.res, "analyzer": sc.an, "tic": sc.tic, "key": scan_key(sc), "pairs": pairs}

    def scan_table(self, level: int, filt: str | None = None) -> dict:
        """The scans of one level (and scan type), in columns: file index, scan number, RT, TIC, precursor, base peak m/z and intensity."""
        t = self._tbl(level)
        ids = t.scan_ids
        if filt and len(ids):
            ids = ids[self._fkeep(ids, filt)]
        sc = self.run.scans
        n = len(ids)
        bpm = np.zeros(n); bpi = np.zeros(n)
        if n and len(t.pos):
            m, y = t.base_peaks()                                     # per scan: the highest peak (computed once per table)
            at = {int(i): k for k, i in enumerate(t.scan_ids)}
            sel = np.array([at[int(i)] for i in ids])
            bpm, bpi = m[sel], y[sel]
        pr = lambda x: (float(sum(x.iso) / 2) if x.iso else float(x.precursor) if x.precursor else None)
        return {"level": level, "n": int(n), "sid": [int(i) for i in ids], "no": [self._no(sc[int(i)]) for i in ids],
                "rt": [round(float(sc[int(i)].rt), 4) for i in ids], "tic": [float(sc[int(i)].tic) for i in ids],
                "prec": [None if pr(sc[int(i)]) is None else round(pr(sc[int(i)]), 5) for i in ids],
                "bpmz": [round(float(v), 5) for v in bpm], "bpint": [round(float(v), 1) for v in bpi]}

    def file_info(self) -> dict:
        """What the mzML says about the file and the instrument (not the status log nor the method of Xcalibur: they are not in an mzML)."""
        r = self.run
        md = r.__dict__.get("_md") or r.metadata()
        head = bytes(r._mm[:max(r._mm.find(b"<run "), 0) + 3000]).decode("utf-8", "replace")
        ts = re.search(r'<run\b[^>]*startTimeStamp="([^"]*)"', head)
        proc = re.findall(r'<dataProcessing id="([^"]*)"', head)
        conv = [m.group(1) for m in re.finditer(r'<processingMethod\b[^>]*softwareRef="([^"]*)"', head)]
        sc = r.scans
        return {"file": Path(self.path).name, "instrument": md.get("instrument", ""), "serial": md.get("serial", ""), "components": md.get("components", []),
                "analyzers": md.get("analyzers", []), "software": md.get("software", []), "source_files": md.get("source_files", []),
                "conversion": sorted(set(conv)), "processing": proc, "start": ts.group(1) if ts else "", "scans": len(sc),
                "rt": [round(float(min((s.rt for s in sc), default=0.0)), 3), round(float(max((s.rt for s in sc), default=0.0)), 3)],
                "levels": {str(l): sum(1 for s in sc if s.level == l) for l in sorted({s.level for s in sc})}, "filters": self.filter_groups(),
                "mz": [self.__dict__.get("_mzr") or None][0]}

    def scan_params(self) -> dict | None:
        """What the scans say about how they were acquired (the method itself is not in the mzML): for the window «Metodo» of a high-resolution / DDA
        file that has no .dam. Survey: scan window, resolving power, injection time; product ions: activation, collision energy (relative NCE for Thermo),
        isolation width, resolving power, number and number per cycle. None for a file that is not high resolution."""
        try:
            r = self.run
            sc1 = [x for x in r.scans if x.level == 1]
            sc2 = [x for x in r.scans if x.level > 1]
            if not (sc1 and (self.profile(1)["hr"] or self.profile(2)["hr"])):
                return None
            med = lambda v: float(np.median(v)) if len(v) else None
            def sample(sc):
                return sc[:: max(1, len(sc) // 300)]
            def heads(sc):
                for x in sample(sc):
                    h = r._mm[x.start:x.end].decode("utf-8", "replace")
                    yield h[:h.find("<binaryDataArrayList")]
            def vals(sc, acc):
                out = []
                for h in heads(sc):
                    m = re.search(r'accession="%s"[^>]*?value="([^"]*)"' % acc, h)
                    if m:
                        try:
                            out.append(float(m.group(1)))
                        except ValueError:
                            pass
                return out
            lo, hi = vals(sc1, "MS:1000501"), vals(sc1, "MS:1000500")
            ms1 = {"n": len(sc1), "window": [min(lo), max(hi)] if lo and hi else None, "res": med([x.res for x in sc1 if x.res]), "inject": med(vals(sc1, "MS:1000927")), "an": self.profile(1)["an"]}
            ms2 = None
            if sc2:
                ms2 = {"n": len(sc2), "per_cycle": round(len(sc2) / len(sc1), 2), "act": sorted({x.act for x in sc2 if x.act}), "nce": bool(r.nce),
                       "ce": sorted({x.collision_energy for x in sc2 if x.collision_energy is not None}),
                       "iso": sorted({round((x.iso[1] - x.iso[0]) / 2, 3) for x in sc2 if x.iso})[:4], "res": med([x.res for x in sc2 if x.res]),
                       "inject": med(vals(sc2, "MS:1000927")), "an": self.profile(2)["an"]}
                lo2 = vals(sc2[:50], "MS:1000501")
                ms2["window_lo"] = min(lo2) if lo2 else None      # lower limit of the MS2 scan window (ion traps cut the spectrum there)
            return {"ms1": ms1, "ms2": ms2, "instrument": self.hr_info().get("instrument", "")}
        except Exception as e:  # noqa: BLE001 -- the window «Metodo» works without it
            return {"error": f"{type(e).__name__}: {e}"[:120]}

    def profile(self, level: int, hr: bool = True) -> dict:
        """Mass profile (decimals, tolerance) of the scans of this level of the physical file; today's profile if anything fails or if the
        page switched high resolution off (hr=False)."""
        if not hr:
            return dict(LOW)
        cache = self.__dict__.setdefault("_profs", {})            # the scans never change: the profile of a level is worked out once (a block of 60 spectra asks for it 120 times)
        key = level == 1
        if key not in cache:
            try:
                cache[key] = mass_profile([s for s in self.run.scans if (s.level == 1) == key])
            except Exception:  # noqa: BLE001
                return dict(LOW)
        return dict(cache[key])

    def hr_on(self, level: int, hr: bool = True) -> bool:
        """True when the centroids of this level are read as high resolution: nothing is merged into bins of 0.1 Da (profile-mode spectra keep their own grid)."""
        return bool(hr) and self._step(level) is None and self.profile(level)["hr"]

    def ptol(self, level: int, precursor, hr: bool = True) -> float:
        """Half width (Da) used to say that a scan has THIS precursor: 0.6 for unit resolution, the profile tolerance in ppm (at least 1 mDa) for high resolution."""
        if level > 1 and precursor and self.hr_on(level, hr):
            return max(float(precursor) * self.profile(level)["tol"] * 1e-6, 1e-3)
        return 0.6

    @staticmethod
    def _ms2_exps(ms2) -> list[dict]:
        """The MS2 experiments of a file: one per precursor (and collision energy), with the number of scans.

        A data-dependent file (IDA/DDA) has a different precursor at every cycle: precursors closer than 0.5 are one group
        (at most 1 m/z wide), named by their most frequent value.
        """
        cnt: dict = {}
        for s in ms2:
            k = (round(s.precursor, 1) if s.precursor else None, s.collision_energy)
            cnt[k] = cnt.get(k, 0) + 1
        rows = sorted(cnt.items(), key=lambda kv: (kv[0][0] or 0, kv[0][1] or 0))
        if len({k[0] for k in cnt}) <= 8:
            return [{"prec": k[0], "ce": k[1], "n": n} for k, n in rows]
        groups: list[dict] = []
        for (pr, ce), n in rows:
            g = groups[-1] if groups else None
            if pr is not None and g and g["lo"] is not None and pr - g["lo"] <= 0.5 and g["ce"] == ce:
                g["n"] += n; g["votes"][pr] = g["votes"].get(pr, 0) + n
            else:
                groups.append({"lo": pr, "ce": ce, "n": n, "votes": {pr: n}})
        return [{"prec": max(g["votes"], key=g["votes"].get), "ce": g["ce"], "n": g["n"]} for g in groups]

    def kind(self) -> str:
        """Experiment type: 'full' (Q1 scan), 'ms2' (product ion scans) or 'mrm' (SRM chromatograms only)."""
        r = self.run
        if self.part:
            return {"ms1": "full", "ms2": "ms2", "mrm": "mrm"}[self.part["mode"]]
        n1 = sum(1 for s in r.scans if s.level == 1)
        n2 = len(r.scans) - n1
        if n2:
            return "ms2"
        if n1:
            return "full"
        return "mrm" if any(c["kind"] == "srm" for c in r.chromatograms()) else "empty"

    def _header_polarity(self) -> str:
        """Polarity from the mzML header (used when there are no scans, e.g. MRM files)."""
        r = self.run
        head = r._mm[:r._mm.find(b"</run>")]
        pos, neg = b'accession="MS:1000130"' in head, b'accession="MS:1000129"' in head
        return "positive" if pos and not neg else "negative" if neg and not pos else "mixed" if pos and neg else "unknown"

    def _srm_rt_range(self):
        """(min, max) retention time in minutes over the SRM chromatograms, or None."""
        r, lo, hi = self.run, None, None
        for c in r.chromatograms():
            if c["kind"] != "srm":
                continue
            t, _ = r.chromatogram(c["index"])
            if len(t):
                lo = float(t.min()) if lo is None else min(lo, float(t.min()))
                hi = float(t.max()) if hi is None else max(hi, float(t.max()))
        return None if lo is None else (lo, hi)

    def mrm(self) -> list[dict]:
        out = []
        for c in self.run.chromatograms():
            if c["kind"] != "srm":
                continue
            t, y = self.run.chromatogram(c["index"])
            out.append({"id": c["index"], "q1": c.get("q1"), "q3": c.get("q3"), "ce": c.get("ce"), "name": c.get("name", ""),
                        "dwell": c.get("dwell"), "rt": [round(float(v), 4) for v in t], "y": [round(float(v), 1) for v in y]})
        return out

    def method(self) -> dict:
        """What the mzML says about how the data were acquired (the source parameters are not stored in mzML)."""
        r, md = self.run, self.run.metadata()
        kind = self.kind()
        ms2 = [s for s in self.sc if s.level >= 2]
        rts = np.array([s.rt for s in self.sc if s.level == (1 if kind == "full" else 2)])
        cycle = float(np.median(np.diff(rts)) * 60) if len(rts) > 2 else None
        win = r.scan_window()
        res = {"file": self.file, "kind": kind, **md, "polarity": self.info()["polarity"] if kind != "mrm" else self._header_polarity(), "scans": len(self.sc),
               "rt_min": float(min((s.rt for s in self.sc), default=0.0)), "rt_max": float(max((s.rt for s in self.sc), default=0.0)),
               "pda": self.has_pda(), "cycle_s": cycle, "scan_window": list(win) if win else None,
               "filters": sorted({s.filter for s in self.sc if s.filter})[:6],
               "precursors": sorted({round(s.precursor, 2) for s in ms2 if s.precursor}),
               "ce": sorted({s.collision_energy for s in ms2 if s.collision_energy is not None}),
               "transitions": [{"q1": c.get("q1"), "q3": c.get("q3"), "ce": c.get("ce"), "name": c.get("name", ""), "dwell": c.get("dwell")}
                               for c in r.chromatograms() if c["kind"] == "srm"]}
        sp = self.scan_params()
        if sp:
            res["scan_params"] = sp
        return res

    # ------------------------------------------------------------------ chromatograms
    def has_pda(self) -> bool:
        return any(c["kind"] == "pda" for c in self.run.chromatograms())

    def total(self, kind: str = "tic", level: int = 1, mz0: float | None = None, mz1: float | None = None,
              prec: float | None = None, filt: str | None = None):
        """Chromatogram of the whole file. prec (MS2 files): only the scans of that precursor ion (None = all the precursors); filt: only the scans of that type."""
        rt, y = self._total(kind, level, mz0, mz1)
        if filt and len(rt) and kind != "pda" and self.kind() != "mrm":
            t = self._tbl(level)
            if len(t.scan_ids) == len(rt):
                fk = self._fkeep(t.scan_ids, filt)
                rt, y = rt[fk], y[fk]
                if prec is not None and level > 1 and len(rt):
                    ids = t.scan_ids[fk]
                    pr = np.array([sum(self.run.scans[i].iso)/2 if self.run.scans[i].iso else self.run.scans[i].precursor or -1.0 for i in ids])
                    sm = np.abs(pr - prec) <= 0.6
                    return rt[sm], y[sm]
                return rt, y
        if prec is not None and level > 1 and len(rt):
            t = self._tbl(level)
            pr = np.array([sum(self.run.scans[i].iso)/2 if self.run.scans[i].iso else self.run.scans[i].precursor or -1.0 for i in t.scan_ids])
            sm = np.abs(pr - prec) <= 0.6
            if len(sm) == len(rt):
                return rt[sm], y[sm]
        return rt, y

    def _total(self, kind: str, level: int, mz0, mz1):
        if kind == "pda":                       # UV trace recorded by the PDA/DAD (own time axis, not tied to the MS scans)
            for c in self.run.chromatograms():
                if c["kind"] == "pda":
                    return self.run.chromatogram(c["index"])
            return np.zeros(0), np.zeros(0)
        if self.kind() == "mrm":
            for c in self.run.chromatograms():
                if c["kind"] == kind or (kind == "tic" and c["kind"] == "tic"):
                    t, y = self.run.chromatogram(c["index"])
                    return t, y
            return np.zeros(0), np.zeros(0)
        t = self._tbl(level)
        if len(t.rt) == 0:
            return np.zeros(0), np.zeros(0)
        if mz0 is not None or mz1 is not None:          # only the ions inside [mz0, mz1] (None = open end)
            dt = t.mz.dtype.type
            a = 0 if mz0 is None else int(np.searchsorted(t.mz, dt(mz0), side="left"))
            b = len(t.mz) if mz1 is None else int(np.searchsorted(t.mz, dt(mz1), side="right"))
            pos, inten = t.pos[a:max(a, b)], t.inten[a:max(a, b)]
            if kind == "tic":
                return t.rt, np.bincount(pos, weights=inten, minlength=len(t.rt))
            out = np.zeros(len(t.rt))
            if len(pos):
                order = np.argsort(pos, kind="stable")
                ps, ys = pos[order], inten[order]
                starts = np.flatnonzero(np.r_[True, ps[1:] != ps[:-1]])
                out[ps[starts]] = np.maximum.reduceat(ys, starts)
            return t.rt, out
        if kind == "tic":
            return t.rt, t.sum_by_scan()
        return t.rt, t.base_peaks()[1]

    def xic(self, mz: float, tol: float, level: int = 1):
        t = self._tbl(level)
        return t.rt, t.xic(mz, tol)

    # ------------------------------------------------------------------ ion map (RT x m/z)
    def ionmap(self, level: int, grid: dict, agg: str = "max") -> np.ndarray:
        """Intensity per cell on a common RT x m/z grid, shape (nrt, nmz), float32.

        The grid (rt0, rt1, nrt, mz0, dmz, nmz) is shared by all files of a session, so maps can be compared or subtracted bin by bin.
        agg="max": the most intense peak that falls in the cell (a peak is never diluted by the width of the bin);
        agg="mean": the mean per scan of the scans that fall in the same RT bin (the difference A - B uses it).
        """
        agg = check_agg(agg)
        key = (agg, level, grid["rt0"], grid["rt1"], grid["nrt"], grid["mz0"], grid["dmz"], grid["nmz"])
        cache = self.__dict__.setdefault("_maps", {})
        if key in cache:
            return cache[key]
        nrt, nmz = grid["nrt"], grid["nmz"]
        t = self._tbl(level)
        out = np.zeros((nrt, nmz), dtype=np.float32)
        if len(t.rt) and len(t.mz):
            span = max(grid["rt1"] - grid["rt0"], 1e-9)
            scan_bin = np.clip(np.floor((t.rt - grid["rt0"]) / span * nrt).astype(np.int64), 0, nrt - 1)
            n_scans = np.bincount(scan_bin, minlength=nrt)
            size = nrt * nmz
            if agg == "mean":                                          # the peaks in pieces; the running sums go first so the additions keep their order
                cells, acc = np.arange(size), np.zeros(size)
                for mz, inten, pos in t.iter_chunks():
                    j = np.floor((mz - grid["mz0"]) / grid["dmz"]).astype(np.int64)
                    ok = (j >= 0) & (j < nmz)
                    acc = np.bincount(np.concatenate([cells, scan_bin[pos][ok] * nmz + j[ok]]), weights=np.concatenate([acc, inten[ok]]), minlength=size)
                out = (acc.reshape(-1, nmz) / np.maximum(n_scans, 1)[:, None]).astype(np.float32)
            else:                                                      # the maximum of the pieces is the maximum
                flat = np.zeros(size, dtype=np.float32)
                for mz, inten, pos in t.iter_chunks():
                    j = np.floor((mz - grid["mz0"]) / grid["dmz"]).astype(np.int64)
                    ok = (j >= 0) & (j < nmz)
                    flat = np.maximum(flat, _cells(scan_bin[pos][ok] * nmz + j[ok], inten[ok], size, agg, n_scans, nmz))
                out = flat.reshape(nrt, nmz)
        cache[key] = out
        return out

    def ionmap_region(self, level: int, rt0: float, rt1: float, nrt: int, mz0: float, mz1: float, nmz: int, agg: str = "max", log: bool = False) -> np.ndarray:
        """The same map as ionmap(), on a grid of its own over a region only (the true zoom of the map): shape (nrt, nmz), float32;
        RT bins of (rt1 - rt0) / nrt; m/z bins of (mz1 - mz0) / nmz, or equal steps of ln(m/z) with log=True (cells of a constant number of ppm)."""
        agg = check_agg(agg)
        nrt, nmz = max(1, int(nrt)), max(1, int(nmz))
        out = np.zeros((nrt, nmz), dtype=np.float32)
        t = self._tbl(level)
        if not len(t.rt) or not len(t.mz) or rt1 <= rt0 or mz1 <= mz0 or (log and mz0 <= 0):
            return out
        a, b = np.searchsorted(t.mz, mz0, side="left"), np.searchsorted(t.mz, mz1, side="left")
        scan_in = (t.rt >= rt0) & (t.rt < rt1)
        scan_bin = np.clip(np.floor((t.rt - rt0) / (rt1 - rt0) * nrt).astype(np.int64), 0, nrt - 1)
        if b > a:
            pos = t.pos[a:b]
            ok = scan_in[pos]
            i = scan_bin[pos[ok]]
            mz = t.mz[a:b][ok].astype(np.float64)
            f = np.log(mz / mz0) / np.log(mz1 / mz0) if log else (mz - mz0) / (mz1 - mz0)
            j = np.clip(np.floor(f * nmz).astype(np.int64), 0, nmz - 1)
            out = _cells(i * nmz + j, t.inten[a:b][ok], nrt * nmz, agg, np.bincount(scan_bin[scan_in], minlength=nrt), nmz).reshape(nrt, nmz)
        return out

    def map_background(self, level: int, mz0: float, mz1: float, nmz: int, log: bool = False, agg: str = "max") -> np.ndarray:
        """The constant background of the m/z cells of a region: for each cell the median along the WHOLE run, over the scans (at most 1500
        groups of consecutive scans), shape (nmz,). An ion present all along the run (solvent, column bleed, plasticisers) is a line of the map;
        subtracting this takes it away and leaves the peaks."""
        key = ("bg", agg, level, round(mz0, 4), round(mz1, 4), nmz, log)
        cache = self.__dict__.setdefault("_maps", {})
        if key in cache:
            return cache[key]
        t = self._tbl(level)
        out = np.zeros(nmz, dtype=np.float32)
        if len(t.rt) and len(t.mz) and mz1 > mz0 and not (log and mz0 <= 0):
            ns = len(t.rt)
            grp = max(1, int(np.ceil(ns / 1500)))
            a, b = np.searchsorted(t.mz, mz0, side="left"), np.searchsorted(t.mz, mz1, side="left")
            mz = t.mz[a:b].astype(np.float64)
            f = np.log(mz / mz0) / np.log(mz1 / mz0) if log else (mz - mz0) / (mz1 - mz0)
            j = np.clip(np.floor(f * nmz).astype(np.int64), 0, nmz - 1)
            ng = (ns + grp - 1) // grp
            full = _cells((t.pos[a:b] // grp) * nmz + j, t.inten[a:b], ng * nmz, agg, np.bincount(np.arange(ns) // grp, minlength=ng), nmz).reshape(ng, nmz)
            out = np.median(full, axis=0).astype(np.float32)
        cache[key] = out
        return out

    def map_points(self, level: int, rt0: float, rt1: float, mz0: float, mz1: float, cap: int = MAP_POINTS_MAX) -> dict:
        """The true points (RT, m/z, intensity) of a region for the sticks of the 3D viewer: the most intense ones up to cap.
        Returns {rt, mz, inten (arrays, most intense first), total (points in the region), n}."""
        cap = max(1, min(int(cap), MAP_POINTS_MAX))
        t = self._tbl(level)
        empty = {"rt": np.zeros(0, np.float32), "mz": np.zeros(0, np.float64), "inten": np.zeros(0, np.float32), "total": 0, "n": 0}
        if not len(t.rt) or not len(t.mz):
            return empty
        a, b = np.searchsorted(t.mz, mz0, side="left"), np.searchsorted(t.mz, mz1, side="right")
        rt = t.rt[t.pos[a:b]]
        idx = np.flatnonzero((rt >= rt0) & (rt <= rt1))
        total = int(len(idx))
        if not total:
            return empty
        y = t.inten[a:b][idx]
        if total > cap:
            idx = idx[np.argpartition(-y, cap - 1)[:cap]]
            y = t.inten[a:b][idx]
        idx = idx[np.argsort(-y, kind="stable")]
        return {"rt": rt[idx].astype(np.float32), "mz": t.mz[a:b][idx].astype(np.float64), "inten": t.inten[a:b][idx].astype(np.float32), "total": total, "n": int(len(idx))}

    # ------------------------------------------------------------------ marked points of the map (only measures: never a name or a verdict)
    def map_snap(self, level: int, rt: float, mz: float, dmz: float = 1.0, drt: float = 0.2, dbins: int = 1) -> dict | None:
        """The local maximum near a click on the map: the most intense peak within +-drt minutes and +-dbins bins of dmz around (rt, mz).
        rt = RT of that scan (the apex), mz = the m/z of the peak as the file has it (exact in high resolution)."""
        t = self._tbl(level)
        if not len(t.rt) or not len(t.mz):
            return None
        lo, hi = (np.floor(mz / dmz) - dbins) * dmz, (np.floor(mz / dmz) + dbins + 1) * dmz
        a, b = np.searchsorted(t.mz, lo, side="left"), np.searchsorted(t.mz, hi, side="left")
        if b <= a:
            return None
        pos = t.pos[a:b]
        ok = np.abs(t.rt[pos] - rt) <= drt
        if not ok.any():
            return None
        k = np.flatnonzero(ok)[np.argmax(t.inten[a:b][ok])]
        return {"rt": float(t.rt[pos[k]]), "mz": float(t.mz[a + k])}

    def map_measure(self, level: int, rt: float, mz: float, tol: float, ref: "Item | None" = None, rt_tol: float = 0.1) -> dict:
        """Measures of one marked point: intensity at the apex in this file (A) and the local maximum within +-rt_tol in the reference (B)
        on the XIC mz +- tol; S/N of A = (apex - median of the XIC) / (1.4826 x MAD of the XIC)."""
        rts, ya = self.xic(mz, tol, level)
        out = {"ia": 0.0, "ib": None, "sn": None}
        if len(rts):
            i = int(np.argmin(np.abs(rts - rt)))
            out["ia"] = float(ya[i])
            med = float(np.median(ya))
            mad = float(np.median(np.abs(ya - med))) * 1.4826
            out["sn"] = (out["ia"] - med) / mad if mad > 0 else None
        if ref is not None:
            rb, yb = ref.xic(mz, tol, level)
            w = np.abs(rb - rt) <= rt_tol
            out["ib"] = float(yb[w].max()) if w.any() else 0.0
        return out

    # ------------------------------------------------------------------ spectra
    def _binned(self, rt0, rt1, level=1, precursor=None, prec_tol=0.6, bin_da=0.1, hr=False, raw1=False, filt=None):
        """Mean spectrum in [rt0, rt1] on a fixed bin grid: (bin ids, centroid m/z, mean intensity, n scans).
        hr: high-resolution centroids, cells of 3 ppm (and with raw1 a window holding ONE scan gives that scan's own centroids)."""
        hr = self.hr_on(level, hr)
        e = (np.zeros(0, dtype=np.int64), np.zeros(0), np.zeros(0))
        t = self._tbl(level)
        if len(t.rt) == 0:
            return (*e, 0)
        ok = (t.rt >= min(rt0, rt1)) & (t.rt <= max(rt0, rt1))
        if level > 1 and precursor is not None:
            prec = np.array([sum(self.run.scans[i].iso)/2 if self.run.scans[i].iso else self.run.scans[i].precursor or -1.0 for i in t.scan_ids])
            ok &= np.abs(prec - precursor) <= prec_tol
        if filt:
            ok &= self._fkeep(t.scan_ids, filt)
        n = int(ok.sum())
        if n == 0:
            return (*e, 0)
        parts = [(mz[m], y[m]) for mz, y, pos in t.iter_chunks() for m in (ok[pos],)]
        mz, y = (np.concatenate([p[0] for p in parts]), np.concatenate([p[1] for p in parts])) if parts else (np.zeros(0), np.zeros(0))
        if len(mz) == 0:
            return (*e, n)
        if hr and raw1 and n == 1:                                  # one scan = its own centroids, exactly (read from the file: the peak table of a high-resolution file is float32)
            mz, y = self.run.read(int(t.scan_ids[np.flatnonzero(ok)[0]]))
            return np.arange(len(mz)), mz, y, 1
        b = self._cells(mz, bin_da, level, hr)
        u, inv = np.unique(b, return_inverse=True)
        sy = np.bincount(inv, weights=y)
        smz = np.bincount(inv, weights=y * mz) / np.maximum(sy, 1e-12)
        return u, smz, sy / n, n

    # ------------------------------------------------------------------ DDA and single scans as they are in the file
    @staticmethod
    def _no(s) -> int:
        """Scan number as the instrument software shows it: the 'scan=N' of the id (Thermo), otherwise the position in the file + 1."""
        m = re.search(r"scan=(\d+)", s.native or "")
        return int(m.group(1)) if m else s.index + 1

    def dda(self, hr: bool = True) -> dict:
        """Every MS2 of the physical file with its parent, isolation window and activation, in columns, plus the survey scans (sid = index in the file).

        `prec` is the selected ion of the MS2 (decimals of the MS2 profile + 1); `tgt` the centre of the isolation window = where the ion sits in the survey scan."""
        r = self.run
        ms2 = [s for s in r.scans if s.level > 1]
        d = self.profile(2, hr)["dec"] + 1
        ms1 = [s for s in r.scans if s.level == 1]
        by = {s.index: s for s in r.scans}
        rt = lambda s: round(float(s.rt), 4)
        return {"sid": [s.index for s in ms2], "rt": [rt(s) for s in ms2], "no": [self._no(s) for s in ms2],
                "prec": [round(float(s.precursor), d) if s.precursor else None for s in ms2],
                "tgt": [round(float(sum(s.iso) / 2), d) if s.iso else None for s in ms2],
                "parent": [s.parent for s in ms2], "prt": [rt(by[s.parent]) if s.parent in by else None for s in ms2],
                "lo": [round(float(s.iso[0]), d) if s.iso else None for s in ms2], "hi": [round(float(s.iso[1]), d) if s.iso else None for s in ms2],
                "act": [s.act for s in ms2], "ce": [s.collision_energy for s in ms2], "pint": [s.pint for s in ms2],
                "ms1": {"sid": [s.index for s in ms1], "rt": [rt(s) for s in ms1]}, "nce": bool(r.nce)}

    def scan(self, sid: int, hr: bool = True) -> dict:
        """One scan exactly as the file has it: its own centroids, no merging (what the DDA panels show)."""
        r = self.run
        if not 0 <= sid < len(r.scans):
            raise UserError("err.scan.outside", text="scan outside the file")
        s = r.scans[sid]
        mz, y = r.read(sid)
        d = self.profile(s.level, hr)["dec"] + 1
        return {"sid": sid, "no": self._no(s), "parent_no": self._no(r.scans[s.parent]) if s.parent is not None else None,
                "rt": round(float(s.rt), 4), "level": s.level, "mz": [round(float(v), d) for v in mz], "y": [round(float(v), 1) for v in y],
                "prec": round(float(sum(s.iso) / 2), d) if s.iso else round(float(s.precursor), d) if s.precursor else None, "lo": round(float(s.iso[0]), d) if s.iso else None, "hi": round(float(s.iso[1]), d) if s.iso else None,
                "act": s.act, "ce": s.collision_energy, "nce": bool(r.nce), "res": s.res, "an": s.an, "parent": s.parent, "filter": s.filter,
                "profile": bool(s.profile)}

    def scan_avg(self, sids: list[int], hr: bool = True) -> dict:
        """Mean of some scans of the same level: cells of 3 ppm for a high-resolution profile, 0.1 Da otherwise; intensity = mean per scan, m/z weighted."""
        r = self.run
        sids = [i for i in sids if 0 <= i < len(r.scans)]
        if not sids:
            raise UserError("err.scan.noneValid", text="no valid scan")
        level = r.scans[sids[0]].level
        prof = self.profile(level, hr)
        parts = [r.read(i) for i in sids]
        mz = np.concatenate([p[0] for p in parts]) if parts else np.zeros(0)
        y = np.concatenate([p[1] for p in parts]) if parts else np.zeros(0)
        if len(mz):
            cell = np.floor(np.log(np.maximum(mz, 1e-9)) / np.log1p(3e-6)).astype(np.int64) if prof["hr"] else np.floor(mz / 0.1).astype(np.int64)
            u, inv = np.unique(cell, return_inverse=True)
            sy = np.bincount(inv, weights=y)
            mz, y = np.bincount(inv, weights=y * mz) / np.maximum(sy, 1e-12), sy / len(sids)
        d = prof["dec"] + 1
        return {"sids": sids, "level": level, "n": len(sids), "rt": round(float(np.mean([r.scans[i].rt for i in sids])), 4),
                "mz": [round(float(v), d) for v in mz], "y": [round(float(v), 1) for v in y]}

    def scan_count(self, level: int = 1, precursor: float | None = None, prec_tol: float = 0.6, filt: str | None = None) -> int:
        """Number of scans of that level (and precursor) = the length of the chromatogram of the same selection."""
        return len(self._scan_ids(level, precursor, prec_tol, filt)[0])

    def _scan_ids(self, level, precursor, prec_tol, filt=None):
        t = self._tbl(level)
        ids, rt = t.scan_ids, t.rt
        if filt and len(ids):
            fk = self._fkeep(ids, filt)
            ids, rt = ids[fk], rt[fk]
        if level > 1 and precursor is not None and len(ids):
            pr = np.array([sum(self.run.scans[i].iso)/2 if self.run.scans[i].iso else self.run.scans[i].precursor or -1.0 for i in ids])
            sm = np.abs(pr - precursor) <= prec_tol
            ids, rt = ids[sm], rt[sm]
        return ids, rt

    def window_scans(self, rt0: float, rt1: float, level: int = 1, precursor: float | None = None, prec_tol: float = 0.6, filt: str | None = None) -> dict:
        """Which scans a time window holds: positions (in the chromatogram of the same level / precursor) of the first and the last one, and the total."""
        ids, rt = self._scan_ids(level, precursor, prec_tol, filt)
        ok = np.where((rt >= min(rt0, rt1)) & (rt <= max(rt0, rt1)))[0]
        out = {"i0": int(ok[0]) if len(ok) else None, "i1": int(ok[-1]) if len(ok) else None, "n": int(len(ids))}
        if len(ok) == 1:                                   # a window holding one scan: which one (index in the file), for the DDA panels
            out["sid"] = int(ids[ok[0]])
        return out

    def scans(self, i0: int, i1: int, level: int = 1, precursor: float | None = None, prec_tol: float = 0.6, bin_da: float = 0.1, hr: bool = False,
              filt: str | None = None) -> list[dict]:
        """Binned spectra of scans i0..i1 (inclusive; position in the chromatogram of the same level/precursor), one by one.

        Reads the arrays of those scans straight from the file (no filtering of the whole peak table) and bins them exactly as `spectrum`
        does for a window holding that single scan: same bins of bin_da Da, same intensity-weighted m/z.
        """
        ids, rt = self._scan_ids(level, precursor, prec_tol, filt)
        out, hr = [], self.hr_on(level, hr)
        for i in range(max(i0, 0), min(i1, len(ids) - 1) + 1):
            mz, y = self.run.read(int(ids[i]))
            if len(mz) and not hr:                                  # high resolution: the scan is shown with its own centroids
                b = self._cells(mz, bin_da, level)
                u, inv = np.unique(b, return_inverse=True)
                sy = np.bincount(inv, weights=y)
                mz, y = np.bincount(inv, weights=y * mz) / np.maximum(sy, 1e-12), sy
            out.append({"i": i, "sid": int(ids[i]), "rt": float(rt[i]), "mz": mz, "y": y})
        return out

    def nearest_scan(self, rt: float, level: int = 1, precursor: float | None = None, filter: str | None = None) -> dict | None:
        """Returns the scan of the given filter/level/precursor closest to the requested RT."""
        t = self._tbl(level)
        ids = t.scan_ids
        if filter is not None:
            ids = [i for i in ids if self.run.scans[i].filter == filter or self._keys()[int(i)] == filter]
        elif precursor is not None:
            ids = [i for i in ids if abs((sum(self.run.scans[i].iso)/2 if self.run.scans[i].iso else self.run.scans[i].precursor or -100.0) - precursor) <= 0.6]
        if len(ids) == 0:
            return None
        rts = np.array([self.run.scans[i].rt for i in ids])
        idx = np.argmin(np.abs(rts - rt))
        s = self.run.scans[ids[idx]]
        return {"rt": float(s.rt), "sid": s.index, "filter": s.filter}

    def spectrum(self, rt0: float, rt1: float, level: int = 1, precursor: float | None = None,
                 prec_tol: float = 0.6, bin_da: float = 0.1, min_rel: float = 0.0, bg: dict | None = None, hr: bool = False, filt: str | None = None):
        """Mean spectrum of the scans in [rt0, rt1], peaks merged within bin_da.

        bg = {"item": Item, "rt0": .., "rt1": .., "factor": 1.0, "tol": 5.0 (ppm, high resolution), "clip": True} subtracts a reference spectrum
        (another time window, or the blank file, see differenza.subtract_spectrum): the same nominal mass in low resolution, +-tol ppm in high;
        what is only in the reference stays negative unless clip. Both spectra are mean intensity per scan, so windows of different length are comparable.
        """
        u, smz, sy, n = self._binned(rt0, rt1, level, precursor, prec_tol, bin_da, hr, raw1=bg is None, filt=filt)
        if bg is not None and len(u):
            bu, bmz, by, bn = bg["item"]._binned(bg["rt0"], bg["rt1"], level, precursor, prec_tol, bin_da, hr)
            if bn and len(bu):
                smz, sy = subtract_spectrum(smz, sy, bmz, by, float(bg.get("factor", 1.0)), unit=not self.hr_on(level, hr),
                                            tol=float(bg.get("tol", 5.0)), clip=bool(bg.get("clip", True)))
        if min_rel > 0 and len(sy):
            keep = sy >= min_rel * sy.max()
            smz, sy = smz[keep], sy[keep]
        return smz, sy, n


def check_agg(agg: str) -> str:
    if agg not in ("max", "mean"):
        raise UserError("err.map.agg", {"agg": str(agg)[:20]}, f"agg must be max or mean, not {agg!r}")
    return agg


def _cells(cell: np.ndarray, w: np.ndarray, size: int, agg: str, n_scans: np.ndarray, nmz: int) -> np.ndarray:
    """Intensity of every cell of a map from the cell number of each peak: the maximum (agg="max") or the mean per scan of the RT bin ("mean"), flat."""
    if agg == "mean":
        acc = np.bincount(cell, weights=w, minlength=size).reshape(-1, nmz)
        return (acc / np.maximum(n_scans, 1)[:, None]).astype(np.float32).ravel()
    out = np.zeros(size, dtype=np.float32)
    if len(cell):
        np.maximum.at(out, cell, w.astype(np.float32, copy=False))          # the maximum is the same in any order (and float32 rounding is monotonic): no sorting needed
    return out


def map_cells(hr: bool, mz0: float, mz1: float, nmz: int, log: bool = False) -> tuple:
    """The m/z cells of a region of the map: (mz0, mz1, nmz, log).
    High resolution: never more cells than the instrument tells apart (cells of HR_CELL = 3 ppm), so a wide view is coarse and a zoom reaches the
    resolution of the instrument; log=True gives cells of equal ppm. Low resolution: when the cells are wider than 0.5 Da they are nominal masses,
    [n - 0.2; n + 0.8] (as the XIC of the cursor), whole numbers of Da wide."""
    nmz = max(1, int(nmz))
    if hr:
        top = (np.log(mz1 / mz0) / HR_CELL) if log and mz0 > 0 else (mz1 - mz0) / (mz1 * 3e-6)
        return mz0, mz1, max(1, min(nmz, int(np.ceil(top)))), bool(log and mz0 > 0)
    if (mz1 - mz0) / nmz >= 0.5:
        lo, hi = float(np.floor(mz0 + 0.2) - 0.2), float(np.ceil(mz1 + 0.2) - 0.2)
        w = max(1, int(np.ceil((hi - lo) / nmz)))
        n = int(np.ceil((hi - lo) / w - 1e-9))
        return lo, lo + w * n, n, False
    return mz0, mz1, nmz, False


def subtract_constant(m: np.ndarray, bg: np.ndarray) -> np.ndarray:
    """Takes the constant background (one value per m/z column) away from a map (RT x m/z); what is below it stays 0."""
    return np.maximum(np.asarray(m, dtype=np.float32) - np.asarray(bg, dtype=np.float32)[None, :], 0).astype(np.float32)


def diff_tolerant(a: np.ndarray, b: np.ndarray, w: int) -> np.ndarray:
    """A - B with a tolerance in RT: B is replaced by its local maximum over +-w RT bins (axis 0), so a small drift of the retention time
    and the lines of the background (present in both) cancel instead of leaving a red/blue pair. w = 0: the plain difference."""
    a, b = np.asarray(a, dtype=np.float32), np.asarray(b, dtype=np.float32)
    if w > 0 and b.shape[0] > 1:
        p = np.pad(b, ((w, w), (0, 0)), mode="edge")
        b = np.lib.stride_tricks.sliding_window_view(p, 2 * w + 1, axis=0).max(axis=-1)
    return (a - b).astype(np.float32)


def rt_groups(points: list[dict], tol: float = 0.05) -> list[dict]:
    """Groups of marked points whose apexes are within +-tol minutes (chained: a point joins the group of the previous one when it is close
    to it). For each point: g = number of the group (1, 2, ... in RT order) and dm = m/z minus the m/z of the most intense point (ia) of its group."""
    order = sorted(range(len(points)), key=lambda i: points[i]["rt"])
    gid, g, last = {}, 0, None
    for i in order:
        if last is None or points[i]["rt"] - last > tol:
            g += 1
        gid[i], last = g, points[i]["rt"]
    out = [dict(p, g=gid[i]) for i, p in enumerate(points)]
    for k in set(gid.values()):
        mem = [p for p in out if p["g"] == k]
        top = max(mem, key=lambda p: p.get("ia") or 0)
        for p in mem:
            p["dm"] = p["mz"] - top["mz"]
    return out


def infusion_like(scans) -> bool:
    """A direct infusion as far as the scans tell: a single scan, or several scans that all sit at the same retention time."""
    if not scans:
        return False
    if len(scans) == 1:
        return True
    rts = [s.rt for s in scans]
    return max(rts) - min(rts) < 1e-6


def file_parts(run) -> list[dict]:
    """The experiments of a MIXED file (survey + product ions, MRM + EPI, positive + negative polarity), or [] when the file has only one.

    Each part is {"mode": "ms1" | "ms2" | "mrm", "pol": 1 | -1 | None, "tag": "MS1" | "MS2" | "MRM" (+ " pos" / " neg" when both polarities are there)}.
    """
    sc = run.scans
    split = len({s.polarity for s in sc if s.polarity in (1, -1)}) > 1
    parts = []
    for mode, name, test in (("ms1", "MS1", lambda s: s.level == 1), ("ms2", "MS2", lambda s: s.level >= 2)):
        sel = [s for s in sc if test(s)]
        if not sel:
            continue
        for pol in (sorted({s.polarity for s in sel if s.polarity in (1, -1)}, reverse=True) if split else [None]):
            parts.append({"mode": mode, "pol": pol, "tag": name + ((" pos" if pol == 1 else " neg") if split else "")})
    if any(c["kind"] == "srm" for c in run.chromatograms()):
        parts.append({"mode": "mrm", "pol": None, "tag": "MRM"})
    return parts if len(parts) > 1 else []


def mixed_text(run) -> list[dict]:
    """The parts of a mixed file for the loading table ([] for a file with a single experiment): [{"name", "pol", "nprec"}]; the page writes them."""
    parts = file_parts(run)
    if not parts:
        return []
    n_prec = len({round(s.precursor) for s in run.scans if s.level >= 2 and s.precursor})
    names = {"ms1": "Full Scan", "ms2": "MS2", "mrm": "MRM"}
    return [{"name": names[p["mode"]], "pol": p["pol"], "nprec": n_prec if p["mode"] == "ms2" and n_prec else None} for p in parts]


class Session:
    def __init__(self, samples: list[dict], root: Path):
        self.items: list[Item] = []
        self.root = root
        self._hr = self._lr = False
        self.add(samples)

    def add(self, samples: list[dict]) -> None:
        """Open more files in this session (the same checks as for the first ones). If they cannot be added the session stays as it was."""
        n_before, hr0, lr0 = len(self.items), self._hr, self._lr
        try:
            self._open(samples)
        except BaseException:
            for it in self.items[n_before:]:
                if all(o.run is not it.run for o in self.items[:n_before]):
                    it.run.close()
            del self.items[n_before:]
            self._hr, self._lr = hr0, lr0
            raise

    def _open(self, samples: list[dict]) -> None:
        root = self.root
        for s in samples:
            name, _, tag = str(s["file"]).partition("#")      # "x.mzML#MS2" = the MS2 part of a mixed file (as saved in the notebook)
            f = Path(name)
            label, t, typ = guess_sample(f.name)
            path = f if f.is_absolute() else root / f
            run = Run(path)
            parts = file_parts(run)
            common = (s.get("label") or label, s["time"] if s.get("time") is not None else t, s.get("type") or typ)
            if not parts:
                item = Item(name, *common[:1], common[1], common[2], path, s.get("conc"), s.get("cunit"), run=run)
                item.rep = s.get("rep") or None
                self.items.append(item)
                hr_info = item.hr_info()
            else:
                for p in ([q for q in parts if q["tag"] == tag] if tag else parts):
                    item = Item(f"{name}#{p['tag']}", common[0], common[1], common[2], path, s.get("conc"), s.get("cunit"), part=p, run=run)
                    item.rep = s.get("rep") or None
                    self.items.append(item)
                    hr_info = item.hr_info()
            if hr_info["prof1"]["hr"] or hr_info["prof2"]["hr"]:
                self._hr = True
            else:
                self._lr = True
            if self._hr and self._lr:
                raise UserError("err.hr.mix", text="low- and high-resolution files cannot be mixed")

    def info(self) -> list[dict]:
        return [it.info() for it in self.items]

    def set_meta(self, entries: list[dict]) -> int:
        """Change type, time (minutes), concentration, its unit and replicate group of files already open. Each entry names a file
        ('x.mzML' reaches every part of it) and carries only the fields to change; returns how many items changed."""
        n = 0
        for e in entries:
            name = str(e.get("file", ""))
            for it in self.items:
                if name not in (it.file, it.file.partition("#")[0]):
                    continue
                if e.get("type") in ("sample", "blank", "standard"):
                    it.type = e["type"]
                if "time" in e:
                    it.time = None if e["time"] in (None, "") else float(e["time"])
                if "conc" in e:
                    it.conc = None if e["conc"] in (None, "") else float(e["conc"])
                if "cunit" in e:
                    it.cunit = e["cunit"] or None
                if "rep" in e:
                    it.rep = str(e["rep"] or "").strip() or None
                n += 1
        return n

    def grid(self, level: int = 1, rt_bin: float = 0.04, dmz: float = 1.0) -> dict:
        """Common RT x m/z grid for the ion maps of every file that has scans of this level."""
        rts, mzs, hr = [], [], False
        for it in self.items:
            sc = [x for x in it.sc if x.level == level]
            if not sc:
                continue
            rts += [min(x.rt for x in sc), max(x.rt for x in sc)]
            t = it._tbl(level)
            if len(t.mz):
                mzs += [float(t.mz[0]), float(t.mz[-1])]
            hr = hr or bool(it.profile(level)["hr"])
        if not rts or not mzs:
            return {"rt0": 0.0, "rt1": 1.0, "nrt": 1, "mz0": 0.0, "dmz": dmz, "nmz": 1}
        rt0, rt1 = float(np.floor(min(rts))), float(np.ceil(max(rts)))
        nrt = int(min(600, max(50, round((rt1 - rt0) / rt_bin))))
        mz0 = float(np.floor(min(mzs))) - (0.0 if hr else 0.2)         # low resolution: bin n = [n - 0.2; n + 0.8], the nominal mass
        return {"rt0": rt0, "rt1": rt1, "nrt": nrt, "mz0": mz0, "dmz": dmz, "nmz": int(max(1, np.ceil((max(mzs) - mz0) / dmz + 1e-9)))}


def sniff(path: Path) -> dict:
    """What an mzML contains, read from the file itself (not from its name): {kind, scans, srm, polarity}."""
    it = Item(path.name, None, None, "sample", path)
    try:
        i = it.info()
        return {"kind": i["kind"], "scans": i["scans"], "srm": i["srm"], "polarity": i["polarity"], "mixed": mixed_text(it.run), "spectrum_mode": i["spectrum_mode"],
                "hr_profile": hr_profile(it.run)}
    finally:
        it.run.close()
