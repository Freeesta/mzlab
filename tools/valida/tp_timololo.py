#!/usr/bin/env python3
"""Measures the interpretation of the rows (`interp/1`) on the real series of the private repository and writes the numbers in the PRIVATE repository
(`coordinamento/consegne/TABUI_TP_TIMOLOLO_MISURE.md`); the public one only names the metrics (`docs/agenti/misure.md`). Nothing here is a function of the
product. The literature products and sites of `scratchpad/timolol_truth.json` and `timolol_msn_truth.json` are hypotheses found by hand: agreement and
disagreement are listed to be reviewed, never counted as right or wrong. Run: `python3 tools/valida/tp_timololo.py [--out FILE]` (skips without the data)."""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
import dati_percorsi  # noqa: E402

PPM = 10.0
RT_TOL = 0.3


def run_pass() -> dict:
    """The rows interpreted by `cargo test` (tests/interp_dati.rs) as the JSON it writes in MZLAB_MISURE_OUT."""
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / "misure.json"
        env = {**os.environ, "MZLAB_MISURE_OUT": str(out)}
        cmd = ["cargo", "test", "--release", "-p", "mzlab-feat", "--features", "interp", "--test", "interp_dati"]
        subprocess.run(cmd, cwd=ROOT, env=env, check=True, capture_output=True)
        return json.loads(out.read_text()) if out.exists() else {}


def rts(t: dict) -> list[float]:
    v = t.get("rt_obs", t.get("rt"))
    if v is None:
        return []
    return [float(x) for x in (v if isinstance(v, list) else [v])]


def find(rows: list[dict], t: dict) -> list[dict]:
    """Rows within PPM of the m/z of a literature entry (and RT_TOL of one of its RT when it has any)."""
    mz = float(t["mz"])
    hit = [r for r in rows if abs(r["mz"] - mz) / mz * 1e6 <= PPM]
    times = rts(t)
    return [r for r in hit if not times or any(abs(r["rt"] - x) <= RT_TOL for x in times)]


def measure(data: dict, truth: dict) -> dict:
    rows = data.get("rows", [])
    tps = [t for t in truth.get("tps", []) if isinstance(t, dict) and t.get("mz") is not None]
    seen, region_ok, region_n, site_diff, no_evidence, new_rows = 0, 0, 0, [], [], []
    for t in tps:
        hit = find(rows, t)
        seen += bool(hit)
        want = t.get("region")
        if hit and want:
            name = t.get("id", t.get("name", "?"))
            # rows with no change proposed (empty confidence) or that cannot say (few fragments, change lost) have no region to compare: no evidence
            if all(r["confidence"] in ("", "cannot_say") for r in hit):
                no_evidence.append((name, want, sorted({x for r in hit for x in r.get("reasons", [])})))
                continue
            region_n += 1
            agree = any(want.lower() in (r.get("region") or "").lower() for r in hit)
            region_ok += agree
            if not agree:
                site_diff.append((name, want, sorted({r["confidence"] for r in hit})))
    known = {r["id"] for t in tps for r in find(rows, t)}
    new_rows = [r for r in rows if r["id"] not in known and r.get("delta")]
    dec = data.get("decoys") or {}
    return {"rows": len(rows), "tp_literature": len(tps), "tp_in_view": seen, "region_agree": region_ok, "region_checked": region_n,
            "site_disagreements": site_diff, "no_evidence": no_evidence, "new_candidates": new_rows, "decoys": dec,
            "fdr_decoys": (dec["survived"] / dec["rules"]) if dec.get("rules") else None}


def render(m: dict) -> str:
    out = ["# TT-R3: measures of the interpretation on the timolol series", "",
           "Hypotheses of the literature are not the truth: the lists below are to be reviewed by hand.", "",
           f"- rows interpreted: {m['rows']}",
           f"- literature products in view: {m['tp_in_view']} / {m['tp_literature']}",
           f"- regions in agreement: {m['region_agree']} / {m['region_checked']} (products with a position verdict)",
           f"- products with no evidence for a position (no change or cannot_say): {len(m['no_evidence'])}",
           f"- decoys: {m['decoys']}  (FDR {m['fdr_decoys']})",
           "- agreement with the mzmine gold: not measured here (the list comes from the local chat)", "", "## Position disagreements to review"]
    out += [f"- {a}: literature {b}, program {c}" for a, b, c in m["site_disagreements"]] or ["- none"]
    out += ["", "## No evidence (the engine cannot say; not disagreements)"]
    out += [f"- {a}: literature {b}, reasons {', '.join(c) or 'n/a'}" for a, b, c in m["no_evidence"]] or ["- none"]
    out += ["", "## New candidates (rows with a change that no literature entry explains)"]
    out += [f"- m/z {r['mz']:.4f}, RT {r['rt']:.2f}: {r['delta']} ({r['confidence']})" for r in m["new_candidates"]] or ["- none"]
    return "\n".join(out) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path)
    a = ap.parse_args()
    radice = dati_percorsi.radice()
    truth_path = dati_percorsi.percorso("scratchpad/timolol_truth.json")
    if radice is None or truth_path is None:
        print("SKIP tp_timololo: no data repository")
        return 0
    data = run_pass()
    if not data:
        print("SKIP tp_timololo: the test did not write the measures (series or parent missing)")
        return 0
    text = render(measure(data, json.loads(truth_path.read_text())))
    dest = a.out or radice / "coordinamento/consegne/TABUI_TP_TIMOLOLO_MISURE.md"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(text, encoding="utf-8")
    print(f"written {dest.name} in the private repository")
    return 0


if __name__ == "__main__":
    sys.exit(main())
