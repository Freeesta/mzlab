"""Compares two `scandump/1` dumps (docs/agenti/scandump.md), field by field, aligned on the scan number `n`: A is the oracle (RFR), B ours.
A tool of the test bench, never part of the site: no calculation of the program lives here.
    python3 tools/rosina/scandump_compare.py A.jsonl.gz B.jsonl.gz [--tol hr|lr] [--out report.json]
    python3 tools/rosina/scandump_compare.py --aggregate DIR [--out table.json]
Per field: compared, equal, different, absent in A, absent in B, largest difference, first 5 examples; a field that only B gives is `extra`, not an error.
Standard library only. Thresholds of ROSINA.md F.2: RT 1e-4 min; m/z 0.5 ppm (hr) or 1e-3 Da (lr); intensities 1e-5 relative; text equal after normalization."""
import argparse
import gzip
import json
import re
import sys
from pathlib import Path

RT_TOL = 1e-4
REL_TOL = 1e-5
DA_TOL = 1e-3
PPM_TOL = 0.5
EXAMPLES = 5

# kind of each numeric field: rt (minutes), mz (ppm or Da by --tol), da (window widths: Da), rel (intensity-like, relative), exact (integer or flag)
KINDS = {
    "rt": "rt",
    "bp_mz": "mz", "mz_low": "mz", "mz_high": "mz", "reactions.precursor": "mz", "sps_precursors": "mz",
    "reactions.iso_width": "da", "reactions.iso_offset": "da",
    "tic": "rel", "bp_int": "rel", "reactions.energy": "rel", "reactions.supplemental": "rel", "faims_cv": "rel",
    "n_peaks": "exact", "level": "exact", "segment": "exact", "event": "exact", "master": "exact",
    "reactions.count": "exact", "reactions.multiple": "exact",
}
SCAN_FIELDS = ["rt", "level", "polarity", "analyzer", "mode", "filter", "scan_type", "segment", "event", "master", "tic", "bp_mz", "bp_int",
               "mz_low", "mz_high", "n_peaks", "faims_cv", "sps_precursors"]
REACTION_FIELDS = ["precursor", "iso_width", "iso_offset", "activation", "energy", "energy_unit", "multiple", "supplemental"]
FILE_FIELDS = ["format_version", "instrument", "controller", "n_scans"]


def normalize(text) -> str:
    """Text equal after normalization: blanks collapsed, case ignored, trailing colon of a label dropped."""
    return re.sub(r"\s+", " ", str(text)).strip().rstrip(":").strip().lower()


def load(path):
    """(file row, scans by n, peaks by n, chromatograms by id, other rows by type)."""
    opener = gzip.open if str(path).endswith(".gz") else open
    head, scans, peaks, chroms, other = None, {}, {}, {}, {}
    with opener(path, "rt", encoding="utf-8") as f:
        for line in f:
            if not line.strip():
                continue
            r = json.loads(line)
            t = r.get("type")
            if t == "file":
                head = r
            elif t == "scan":
                scans[r["n"]] = r
            elif t == "peaks":
                peaks[r["n"]] = r
            elif t == "chromatogram":
                chroms[r["id"]] = r
            else:
                other.setdefault(t, []).append(r)
    if head is None or head.get("schema") != "scandump/1":
        raise SystemExit(f"{path}: the first row is not a scandump/1 file row")
    return head, scans, peaks, chroms, other


class Field:
    def __init__(self, name):
        self.name = name
        self.compared = self.equal = self.different = self.absent_a = self.absent_b = 0
        self.max_diff = 0.0
        self.examples = []
        self.extra = False

    def add(self, n, a, b, diff=None):
        """One comparison of present values: `diff` is the size of the difference, `None` when equal."""
        self.compared += 1
        if diff is None:
            self.equal += 1
            return
        self.different += 1
        if isinstance(diff, (int, float)):
            self.max_diff = max(self.max_diff, diff)
        if len(self.examples) < EXAMPLES:
            self.examples.append({"n": n, "a": a, "b": b})

    def missing(self, n, a, b):
        if a is None:
            self.absent_a += 1
        else:
            self.absent_b += 1
        if len(self.examples) < EXAMPLES:
            self.examples.append({"n": n, "a": a, "b": b})

    def dump(self):
        extra = self.absent_a > 0 and self.compared == 0 and self.absent_b == 0
        return {"compared": self.compared, "equal": self.equal, "different": self.different, "absent_a": self.absent_a,
                "absent_b": self.absent_b, "max_diff": self.max_diff, "extra": extra, "examples": self.examples}


class Comparator:
    def __init__(self, tol):
        self.tol = tol
        self.fields = {}

    def field(self, name):
        return self.fields.setdefault(name, Field(name))

    def differ(self, kind, a, b):
        """None when equal within the threshold, else the size of the difference (in the unit of the threshold)."""
        if kind == "exact":
            return None if a == b else abs(float(a) - float(b)) if isinstance(a, (int, float)) and isinstance(b, (int, float)) else 1.0
        x, y = float(a), float(b)
        d = abs(x - y)
        if kind == "rt":
            return None if d <= RT_TOL else d
        if kind == "da" or (kind == "mz" and self.tol == "lr"):
            return None if d <= DA_TOL else d
        if kind == "mz":
            ppm = d / abs(y) * 1e6 if y else (0.0 if d == 0 else float("inf"))
            return None if ppm <= PPM_TOL else ppm
        rel = d / max(abs(x), abs(y)) if max(abs(x), abs(y)) else 0.0
        return None if rel <= REL_TOL else rel

    def value(self, name, n, a, b, kind=None):
        """Compares one scalar of the two dumps (None = absent)."""
        if a is None and b is None:
            return
        f = self.field(name)
        if a is None or b is None:
            f.missing(n, a, b)
            return
        kind = kind or KINDS.get(name)
        if kind:
            f.add(n, a, b, self.differ(kind, a, b))
        else:
            f.add(n, a, b, None if self.same_text(a, b) else 1.0)

    def same_text(self, a, b):
        if normalize(a) == normalize(b):
            return True
        try:  # numbers written as text (trailer): equal within the relative threshold
            return self.differ("rel", float(a), float(b)) is None
        except (TypeError, ValueError):
            return False

    def array(self, name, n, a, b, kind):
        """A list of numbers: the length is one comparison (`name.count`), then the elements one by one."""
        a, b = a or [], b or []
        if not a and not b:
            return
        self.value(name + ".count", n, len(a), len(b), "exact")
        for x, y in zip(a, b):
            self.value(name, n, x, y, kind)


class ScanComparator(Comparator):
    """The comparator applied to the rows of a dump: scans (with reactions and trailer), peaks and chromatograms."""

    def scan(self, n, a, b):
        for k in SCAN_FIELDS:
            if k == "sps_precursors":
                self.array(k, n, a.get(k), b.get(k), "mz")
            else:
                self.value(k, n, a.get(k), b.get(k))
        ra, rb = a.get("reactions") or [], b.get("reactions") or []
        if ra or rb:
            self.value("reactions.count", n, len(ra), len(rb), "exact")
        for x, y in zip(ra, rb):
            for k in REACTION_FIELDS:
                self.value("reactions." + k, n, x.get(k), y.get(k))
        self.by_level(n, ra, rb, a.get("level") or b.get("level"))
        ta, tb = a.get("trailer") or {}, b.get("trailer") or {}
        na = {normalize(k): v for k, v in ta.items()}
        nb = {normalize(k): v for k, v in tb.items()}
        for k in sorted(set(na) | set(nb)):
            self.value("trailer." + k, n, na.get(k), nb.get(k), None)

    def by_level(self, n, ra, rb, level):
        """MS3+ chains, step by step and per level (`reactions.<field>@ms3` ...): the same fields as above, and our unexplained `raw_double` / `raw_long2` of the
        earlier steps set against RFR's isolation width / offset (`reactions.iso_width~raw_double@ms3`) until a level shows them equal on the whole corpus."""
        if not level or level < 3:
            return
        for i, (x, y) in enumerate(zip(ra, rb)):
            for k in REACTION_FIELDS:
                self.value(f"reactions.{k}@ms{level}", n, x.get(k), y.get(k), KINDS.get("reactions." + k))
            if i < len(rb) - 1:
                self.value(f"reactions.iso_width~raw_double@ms{level}", n, x.get("iso_width"), y.get("raw_double"), "da")
                self.value(f"reactions.iso_offset~raw_long2@ms{level}", n, x.get("iso_offset"), y.get("raw_long2"), "da")

    def peaks(self, n, a, b):
        self.value("peaks.count", n, len(a["mz"]), len(b["mz"]), "exact")
        for x, y in zip(a["mz"], b["mz"]):
            self.value("peaks.mz", n, x, y, "mz")
        for x, y in zip(a["int"], b["int"]):
            self.value("peaks.int", n, x, y, "rel")

    def chromatogram(self, i, a, b):
        self.value("chromatogram.count", i, len(a["time"]), len(b["time"]), "exact")
        for x, y in zip(a["time"], b["time"]):
            self.value("chromatogram.time", i, x, y, "rt")
        for x, y in zip(a["values"], b["values"]):
            self.value("chromatogram.values", i, x, y, "rel")


def compare(path_a, path_b, tol="hr"):
    ha, sa, pa, ca, _ = load(path_a)
    hb, sb, pb, cb, _ = load(path_b)
    c = ScanComparator(tol)
    for k in FILE_FIELDS:
        c.value("file." + k, 0, ha.get(k), hb.get(k), "exact" if k in ("controller", "n_scans", "format_version") else None)
    both = sorted(set(sa) & set(sb))
    for n in both:
        c.scan(n, sa[n], sb[n])
    for n in sorted(set(pa) & set(pb)):
        c.peaks(n, pa[n], pb[n])
    for i in sorted(set(ca) & set(cb)):
        c.chromatogram(i, ca[i], cb[i])
    return {
        "schema": "scandump-compare/1",
        "tolerance": tol,
        "producer_a": ha.get("producer"),
        "producer_b": hb.get("producer"),
        "instrument": hb.get("instrument") or ha.get("instrument"),
        "scans": {"a": len(sa), "b": len(sb), "common": len(both), "only_a": len(set(sa) - set(sb)), "only_b": len(set(sb) - set(sa))},
        "peaks_rows": {"a": len(pa), "b": len(pb), "common": len(set(pa) & set(pb))},
        "chromatograms": {"a": len(ca), "b": len(cb), "common": len(set(ca) & set(cb))},
        "timing_ms": {"a": {k: ha.get(k) for k in ("open_ms", "read_all_ms", "read_one_ms")}, "b": {k: hb.get(k) for k in ("open_ms", "read_all_ms", "read_one_ms")}},
        "fields": {k: v.dump() for k, v in sorted(c.fields.items())},
    }


def aggregate(directory):
    """Sums the reports of a folder into a table field x instrument (counts only: no file name, so nothing private)."""
    table, instruments = {}, set()
    for p in sorted(Path(directory).glob("*.json")):
        rep = json.loads(p.read_text(encoding="utf-8"))
        if rep.get("schema") != "scandump-compare/1":
            continue
        ins = rep.get("instrument") or "?"
        instruments.add(ins)
        for k, v in rep["fields"].items():
            cell = table.setdefault(k, {}).setdefault(ins, {"compared": 0, "equal": 0, "different": 0, "absent_a": 0, "absent_b": 0, "files": 0, "max_diff": 0.0})
            for m in ("compared", "equal", "different", "absent_a", "absent_b"):
                cell[m] += v[m]
            cell["files"] += 1
            cell["max_diff"] = max(cell["max_diff"], v["max_diff"])
    return {"schema": "scandump-aggregate/1", "instruments": sorted(instruments), "fields": table}


def table_text(agg):
    """The aggregate as a Markdown table, `equal/compared` per cell."""
    ins = agg["instruments"]
    lines = ["| field | " + " | ".join(ins) + " |", "|---|" + "---|" * len(ins)]
    for k, row in sorted(agg["fields"].items()):
        cells = []
        for i in ins:
            c = row.get(i)
            cells.append("-" if c is None else f"{c['equal']}/{c['compared']}" + (f" (+{c['absent_a']}a,{c['absent_b']}b)" if c["absent_a"] or c["absent_b"] else ""))
        lines.append(f"| {k} | " + " | ".join(cells) + " |")
    return "\n".join(lines)


def summary(rep):
    out = [f"scans A {rep['scans']['a']} B {rep['scans']['b']} common {rep['scans']['common']}"]
    for k, v in rep["fields"].items():
        tag = "extra" if v["extra"] else f"{v['equal']}/{v['compared']}"
        out.append(f"  {k}: {tag}" + (f" different {v['different']} max {v['max_diff']:.3g}" if v["different"] else "") + (f" absent A {v['absent_a']} B {v['absent_b']}" if v["absent_a"] or v["absent_b"] else ""))
    return "\n".join(out)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("a", nargs="?", help="dump A (oracle)")
    ap.add_argument("b", nargs="?", help="dump B (ours)")
    ap.add_argument("--tol", choices=["hr", "lr"], default="hr")
    ap.add_argument("--out", help="write the report as JSON")
    ap.add_argument("--aggregate", metavar="DIR", help="sum the reports (*.json) of DIR into a field x instrument table")
    args = ap.parse_args(argv)
    if args.aggregate:
        agg = aggregate(args.aggregate)
        if args.out:
            Path(args.out).write_text(json.dumps(agg, indent=1, sort_keys=True), encoding="utf-8")
        print(table_text(agg))
        return 0
    if not (args.a and args.b):
        ap.error("two dumps are needed (or --aggregate DIR)")
    rep = compare(args.a, args.b, args.tol)
    if args.out:
        Path(args.out).write_text(json.dumps(rep, indent=1, sort_keys=True), encoding="utf-8")
    print(summary(rep))
    return 0


if __name__ == "__main__":
    sys.exit(main())
