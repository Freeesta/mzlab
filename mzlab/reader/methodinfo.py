"""Acquisition parameters stored in Analyst .dam (method) and .wiff (data) files.

Decoded from the binary streams of the OLE2 container (checked on files from Analyst 1.6.3, 3200 QTRAP):
ion source parameters (CUR, GS1, GS2, CAD, IS, TEM, ihe) and compound parameters (DP, EP, CEP, CE, ...).
Values are 32-bit floats stored after the UTF-16 parameter name. Anything not recognised is left out.
"""
from __future__ import annotations

import itertools
import re
import struct
import xml.etree.ElementTree as ET
from pathlib import Path

from ..i18n import message
from .ole import Ole

NAMES = {  # standard Analyst names (kept in English on purpose: they are the names used on the instrument and in papers)
    "CUR": ("Curtain gas (CUR)", "psi"), "GS1": ("Ion source gas 1 (GS1)", "psi"), "GS2": ("Ion source gas 2 (GS2)", "psi"),
    "CAD": ("Collision gas (CAD)", ""), "IS": ("IonSpray voltage (IS)", "V"),
    "TEM": ("Source temperature (TEM)", "°C"), "ihe": ("Interface heater (ihe)", ""),
    "DP": ("Declustering potential (DP)", "V"), "EP": ("Entrance potential (EP)", "V"), "CEP": ("Collision cell entrance potential (CEP)", "V"),
    "CE": ("Collision energy (CE)", "eV"), "CXP": ("Collision cell exit potential (CXP)", "V"), "CES": ("Collision energy spread (CES)", "eV"),
}
_SRC = re.compile(rb"((?:[A-Za-z0-9]\x00){2,6})")
_CMP = re.compile(rb"[\x02-\x0c]\x00((?:[A-Za-z0-9]\x00){2,4})")


def _num(v: float) -> float:
    return round(v, 3) if abs(v) < 1e5 else v


def _strings(data: bytes, minlen: int = 4) -> list[str]:
    return [m.decode("utf-16le") for m in re.findall(rb"(?:[\x20-\x7e]\x00){%d,}" % minlen, data)]


def _lc(ole: Ole, base: str) -> dict | None:
    """LC method (pump gradient, flow, PDA, oven) from the XML stored by Analyst in DeviceMethod1/VendorAppMethod.

    Returns {"gradient": [{"t", "b", "flow"}], "flow", "b0", "run_time", "pda": {...}, "oven", "pda_channels": [...]} or None.
    """
    key = base + "DeviceMethod1/VendorAppMethod"
    if key not in ole.streams:
        return None
    raw = ole.read(key)
    a = raw.find(b"<SCIEX_ANALYST_ACQUISITION_METHOD")
    b = raw.find(b"</SCIEX_ANALYST_ACQUISITION_METHOD>")
    if a < 0 or b < 0:
        return None
    try:
        root = ET.fromstring(raw[a:b + len(b"</SCIEX_ANALYST_ACQUISITION_METHOD>")].decode("utf-8", "replace"))
    except ET.ParseError:
        return None
    cats = {}
    for c in root.iter("Category"):
        title = (c.findtext("Title") or "").strip()
        par = {(p.findtext("Name") or "").strip(): ((p.findtext("Value") or "").strip(), (p.findtext("Unit") or "").strip()) for p in c.findall("Parameter")}
        tabs = {}
        for t in c.findall("Table"):
            rows = [[(r.findtext(f"Field{i}") or "").strip() for i in range(1, 9) if r.find(f"Field{i}") is not None] for r in t.findall("row")]
            tabs[(t.findtext("Name") or "").strip()] = [r for r in rows[2:] if r]    # skip header and dashes
        cats[title] = (par, tabs)

    def num(x):
        try:
            return float(x)
        except (TypeError, ValueError):
            return None
    pumps, ctl = cats.get("Pumps", ({}, {}))[0], cats.get("System Controller", ({}, {}))[1]
    flow0, b0 = num(pumps.get("Total Flow", ("", ""))[0]), num(pumps.get("B Concentration", ("", ""))[0])
    events = sorted(((num(r[0]), r[3], num(r[4])) for r in ctl.get("Time Program", []) if len(r) >= 5 and num(r[0]) is not None), key=lambda e: e[0])
    grad, flow, bconc, stop = [], flow0, b0, None
    if b0 is not None:
        grad.append({"t": 0.0, "b": b0, "flow": flow})
    for t, name, val in events:
        if name == "Pump B Conc." and val is not None:
            bconc = val
        elif name == "Total Flow" and val is not None:
            flow = val
        elif name == "Stop":
            stop = t
            continue
        else:
            continue
        if grad and grad[-1]["t"] == t:
            grad[-1].update(b=bconc, flow=flow)
        else:
            grad.append({"t": t, "b": bconc, "flow": flow})
    pda, pt = cats.get("PDA Detector", ({}, {}))
    pdav = lambda k: pda.get(k, ("", ""))[0]
    out = {"gradient": grad, "flow": flow0, "b0": b0, "run_time": stop if stop is not None else num(pdav("Run Time")),
           "pda": {"start": num(pdav("Start WL")), "stop": num(pdav("Stop WL")), "step": num(pdav("Wave Step")), "freq": num(pdav("Sampling Frequency")),
                   "lamp": pdav("Lamp"), "cell": num(pdav("Cell Temperature")), "slit": num(pdav("Slit Width"))},
           "pda_channels": [{"wl": num(r[0]), "bw": num(r[1])} for r in pt.get("DA Channel Array", []) if len(r) >= 2 and num(r[0]) is not None],
           "oven": num(cats.get("Oven", ({}, {}))[0].get("Temperature", ("", ""))[0])}
    return out


# one MRM transition inside MassRangeEx: Q1, 0, Q3, dwell (ms), 0, length-prefixed UTF-16 name, then the compound parameters (DP first)
_MRM = re.compile(rb"(.{4})\x00{4}(.{4})(.{4})\x00{4}(.)\x00((?:[\x20-\x7e]\x00)*)\x04\x00D\x00P\x00", re.S)
_CE = re.compile(rb"\x04\x00C\x00E\x00(.{4})", re.S)


def _experiments(ole: Ole, base: str) -> list[dict]:
    """What each experiment of the method scans: MRM transitions, or the start/stop m/z of a scan.

    Decoded from MassRangeEx and checked against the .dam files and mzML of the course: the MRM layout reproduces
    Q1/Q3/dwell/CE of the mzML chromatograms exactly; for scans the start m/z is the first float and the stop m/z sits at
    byte 240 (long layout) or 44 (short layout). Other layouts are reported without a range instead of guessing.
    """
    out = []
    for i in itertools.count():  # every experiment the method stores, sixteen or more
        key = f"{base}DeviceMethod0/Period0/Experiment{i}/MassRangeEx/MassRangeEx"
        if key not in ole.streams:
            break
        d = ole.read(key)
        hits = [m for m in _MRM.finditer(d) if m.group(5) and len(m.group(5)) == m.group(4)[0]]
        ces = sorted({round(struct.unpack("<f", m.group(1))[0], 2) for m in _CE.finditer(d)})
        if hits:
            tr = []
            for j, m in enumerate(hits):
                end = hits[j + 1].start() if j + 1 < len(hits) else len(d)
                ce = _CE.search(d, m.end(), end)
                q1, q3, dw = (struct.unpack("<f", m.group(g))[0] for g in (1, 2, 3))
                tr.append({"q1": round(q1, 2), "q3": round(q3, 2), "dwell": round(dw, 1), "name": m.group(5).decode("utf-16le"),
                           "ce": round(struct.unpack("<f", ce.group(1))[0], 2) if ce else None})
            out.append({"index": i, "kind": "mrm", "transitions": tr})
            continue
        rng = None
        if len(d) >= 244:
            a = struct.unpack_from("<f", d, 40)[0]
            b = struct.unpack_from("<f", d, 240 if len(d) == 432 else 44)[0] if len(d) in (432, 236) else None
            if b is not None and 10 <= a < b <= 3000:
                rng = [round(a, 2), round(b, 2)]
        out.append({"index": i, "kind": "scan", "range": rng, "ce": ces})
    return out


def _method(ole: Ole, base: str) -> dict:
    """Decode one method whose streams live under `base` ('' for .dam, 'MethodSubtree/Method3/' for .wiff)."""
    out: dict = {"source": [], "compound": [], "file": {}}
    out["experiments"] = _experiments(ole, base)
    lc = _lc(ole, base)
    if lc:
        out["lc"] = lc
    tab = base + "DeviceMethod0/Period0/Experiment0/IonSourceParamsTable/"
    for i in range(24):
        key = f"{tab}Parameter{i}/ParameterData"
        if key not in ole.streams:
            break
        d = ole.read(key)
        m = _SRC.search(d[16:])
        if not m:
            continue
        name = m.group(1).decode("utf-16le")
        end = 16 + m.end()
        if end + 4 <= len(d) and name in NAMES:
            v = struct.unpack_from("<f", d, end)[0]
            lab, unit = NAMES[name]
            out["source"].append({"id": name, "label": lab, "value": _num(v), "unit": unit})
    key = base + "DeviceMethod0/Period0/Experiment0/MassRangeEx/MassRangeEx"
    if key in ole.streams:
        d = ole.read(key)
        seen = set()
        for m in _CMP.finditer(d):
            name = m.group(1).decode("utf-16le")
            if name in NAMES and name not in seen and m.end() + 4 <= len(d):
                seen.add(name)
                v = struct.unpack_from("<f", d, m.end())[0]
                lab, unit = NAMES[name]
                out["compound"].append({"id": name, "label": lab, "value": _num(v), "unit": unit})
    for fname in (base + "AcqMethodFileInfoStm", "FileRec_Str"):
        if fname in ole.streams:
            s = _strings(ole.read(fname))
            for t in s:
                tl = t.lower()
                if ".dam" in tl and "metodo" not in out["file"]:
                    mm = re.search(r"[A-Z]:\\.*?\.dam", t)
                    out["file"]["metodo"] = mm.group(0) if mm else t
                elif re.search(r"(monday|tuesday|wednesday|thursday|friday|saturday|sunday)", tl):
                    mm = re.search(r"[A-Z][a-z]+day .*?[AP]M", t)
                    out["file"].setdefault("data", mm.group(0).replace(": ", ":") if mm else t)
                elif tl.startswith("analyst"):
                    out["file"].setdefault("software", re.sub(r"[^\x20-\x7e]", " ", t).strip())
                elif t.startswith("LAB_") or t.endswith("-PC"):
                    out["file"].setdefault("computer", t)
            if out["file"]:
                break
    return out


def read_methods(path) -> list[dict]:
    """[{name, source, compound, file}] for a .dam (one method) or a .wiff (one per sample)."""
    path = Path(path)
    ole = Ole(path)
    try:
        subs = sorted({k.split("/")[1] for k in ole.streams if k.startswith("MethodSubtree/Method")},
                      key=lambda s: int(re.sub(r"\D", "", s) or 0))
        if subs:
            res = []
            for s in subs:
                m = _method(ole, f"MethodSubtree/{s}/")
                m["name"] = s.replace("Method", "Method ")
                m["sample"] = int(re.sub(r"\D", "", s) or 0)
                res.append(m)
            return res
        m = _method(ole, "")
        m["name"] = path.name
        m["sample"] = None
        return [m]
    finally:
        ole.close()


def check_against(data: dict, lab: dict) -> list[dict]:
    """Compare a method (.dam, from read_methods) with what an mzML says about its own acquisition (Item.method()).

    Returns rows {what, method, data, status} with status "ok", "diff" or "na" (cannot be checked). Only things that are
    stored in both places are compared: experiment type, m/z range, MS2 collision energies and precursors, MRM transitions
    (Q1, Q3, CE, dwell), run length and PDA. A match does not prove the method was the one used, a difference shows it was not.
    """
    exps = lab.get("experiments") or []
    rows: list[dict] = []
    if not exps:
        return rows

    def add(what, method, got, status):
        rows.append({"what": what, "method": method, "data": got, "status": status})
    kind = data.get("kind")
    m_mrm = all(e["kind"] == "mrm" for e in exps)
    m_name = "MRM" if m_mrm else message("chk.type.scan")
    d_name = {"full": message("chk.type.full"), "ms2": message("chk.type.ms2"), "mrm": "MRM"}.get(kind) or message("chk.type.unknown")
    add(message("chk.type"), m_name, d_name, "ok" if (kind == "mrm") == m_mrm and kind in ("full", "ms2", "mrm") else "diff")
    fmt = lambda v: f"{v:g}"  # noqa: E731
    if kind == "mrm" and m_mrm:
        m_tr = [t for e in exps for t in e["transitions"]]
        d_tr = list(data.get("transitions") or [])
        used = set()
        for t in m_tr:
            hit = next((i for i, x in enumerate(d_tr) if i not in used and x.get("q1") is not None and abs(x["q1"] - t["q1"]) <= 0.2 and abs(x["q3"] - t["q3"]) <= 0.2), None)
            mtxt = f"CE {fmt(t['ce'])} eV, dwell {fmt(t['dwell'])} ms" if t.get("ce") is not None else f"dwell {fmt(t['dwell'])} ms"
            if hit is None:
                add(message("chk.transition", q1=fmt(t["q1"]), q3=fmt(t["q3"])), mtxt, message("chk.absentFile"), "diff")
                continue
            used.add(hit)
            x = d_tr[hit]
            dw = x["dwell"] * 1000 if x.get("dwell") is not None and x["dwell"] < 5 else x.get("dwell")
            # the method of a negative-mode run stores the energy with its sign, the data its magnitude
            ok = (x.get("ce") is None or t.get("ce") is None or abs(abs(x["ce"]) - abs(t["ce"])) < 0.5) and (dw is None or abs(dw - t["dwell"]) < 1)
            add(message("chk.transition", q1=fmt(t["q1"]), q3=fmt(t["q3"])), mtxt, f"CE {fmt(x['ce'])} eV, dwell {fmt(dw)} ms" if x.get("ce") is not None and dw is not None else message("chk.present"), "ok" if ok else "diff")
        for i, x in enumerate(d_tr):
            if i not in used and x.get("q1") is not None:
                add(message("chk.transition", q1=fmt(x["q1"]), q3=fmt(x["q3"])), message("chk.absentMethod"), message("chk.presentFile"), "diff")
    elif kind in ("full", "ms2") and not m_mrm:
        rngs = [e.get("range") for e in exps]
        win = data.get("scan_window")
        if win and all(rngs):
            lo, hi = min(r[0] for r in rngs), max(r[1] for r in rngs)
            add(message("chk.massRange"), f"{fmt(lo)}-{fmt(hi)}", f"{fmt(win[0])}-{fmt(win[1])}", "ok" if abs(lo - win[0]) <= 1 and abs(hi - win[1]) <= 1 else "diff")
        elif win:
            add(message("chk.massRange"), message("chk.notDecodable"), f"{fmt(win[0])}-{fmt(win[1])}", "na")
        if kind == "ms2":
            ces = sorted({c for e in exps for c in e.get("ce", [])})
            dce = sorted(data.get("ce") or [])
            if ces and dce:
                add(message("chk.ce"), ", ".join(map(fmt, ces)), ", ".join(map(fmt, dce)), "ok" if len(ces) == len(dce) and all(abs(a - b) < 0.5 for a, b in zip(ces, dce)) else "diff")
            prec = data.get("precursors") or []
            if prec:
                add(message("chk.experiments"), str(len(exps)), message("chk.precursorsN", n=len(prec), list=", ".join(fmt(p) for p in prec)), "ok" if len(prec) == len(exps) else "diff")
    rt = (lab.get("lc") or {}).get("run_time")
    if rt and data.get("rt_max"):
        add(message("chk.runTime"), fmt(rt), message("chk.reachesTo", t=f"{data['rt_max']:.1f}"), "ok" if data["rt_max"] <= rt + 0.1 else "diff")
    pda = (lab.get("lc") or {}).get("pda")
    if pda:
        add(message("chk.pda"), message("chk.pdaOn"), message("chk.pdaSignal") if data.get("pda") else message("chk.pdaNone"), "ok" if data.get("pda") else "diff")
    return rows


def method_kind(lab: dict) -> str | None:
    """Experiment type a method (.dam) is for: 'mrm' (decoded from the file), 'ms2' or 'full' (only the NAME says: the binary does not tell a Q1
    scan from a product-ion scan), None when it cannot be said. Used to warn when the opened data are of another type."""
    import re
    exps = lab.get("experiments") or []
    if exps and all(e.get("kind") == "mrm" for e in exps):
        return "mrm"
    name = str(lab.get("name") or "").lower()
    if re.search(r"ms2|msms|ms-ms|epi\b|product|prodotti", name):
        return "ms2"
    if re.search(r"full|\bq1\b|\bems\b|\bms1\b|scan", name):
        return "full"
    return None
