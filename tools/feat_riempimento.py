"""Reference answers of `tpmine.hr.features.fill_gaps` for the parity test of `crates/mzlab-feat/src/gapfill.rs`.

    python3 tools/feat_riferimento.py OUT --solo riempimento

Generates the small synthetic identity series (invented ions), builds the `.mzidx` of every file (OUT/ri_<name>.mzidx), detects and aligns them with the
Python code and writes `riempimento.json`: the alignment (m/z, area, apex RT, width of the peak), the RT map and what `fill_gaps` makes of the
zero cells with and without the map (area and filled cells). The Rust test gives the same alignment and the same index files to `gapfill::fill_file`.
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from mzlab.reader import mzidx
from mzlab.reader.mzml import Run
from tpmine.hr import features, rtalign


def _flat(a) -> list:
    return [None if np.isnan(v) else float(v) for v in np.asarray(a, float).ravel()]


def _copy(al: features.Alignment) -> features.Alignment:
    return features.Alignment(al.mz.copy(), al.rt.copy(), al.area.copy(), al.height.copy(), al.apex_rt.copy(), al.left.copy(), al.right.copy(), al.n_files)


def un_file_riempimento(out: Path, golden: Path | None) -> int:
    from valida import sintetici_identita as si
    d = out / "riempimento"
    si.genera(7, "piccolo", d)
    runs, feats = [], []
    for f in sorted(d.glob("SIN_*.mzML")):
        ix = out / f"ri_{f.stem}.mzidx"
        run = Run(f, index_path=ix)
        mzidx.build(run, ix)
        runs.append(run)
        feats.append(features.detect_file(run, 1))
    al = features.align(feats)
    rows = np.arange(len(al))
    tables = lambda k: runs[k].table(1, 1)                                                    # noqa: E731
    rmap = rtalign.build(al)
    with_map, no_map = _copy(al), _copy(al)
    fm = features.fill_gaps(with_map, rows, tables, range(al.n_files), rtmap=rmap)
    fn = features.fill_gaps(no_map, rows, tables, range(al.n_files))
    doc = {"files": [p.name for p in sorted(out.glob("ri_*.mzidx"))], "n_files": al.n_files, "mz": al.mz.tolist(), "rt": al.rt.tolist(),
           "area": al.area.ravel().tolist(), "apex_rt": _flat(al.apex_rt), "width": _flat(al.right - al.left),
           "rtmap": {"ok": bool(rmap.ok), "anchors": int(rmap.n_anchors), "grid": rmap.grid.tolist(), "shift": rmap.shift.ravel().tolist(), "rms": rmap.rms.tolist()},
           "with_map": {"area": with_map.area.ravel().tolist(), "filled": fm.ravel().astype(int).tolist()},
           "no_map": {"area": no_map.area.ravel().tolist(), "filled": fn.ravel().astype(int).tolist()}}
    (out / "riempimento.json").write_text(json.dumps(doc, separators=(",", ":")), encoding="utf-8")
    for r in runs:
        r.close()
    return int(fm.sum() + fn.sum())
