"""The low-resolution table view (`lr_table_view`): the criteria of `lr_criteri` measured on the native pipeline of the site, on synthetic series and on the real
ones when the private data are there. Aggregated numbers only (no names of compounds, no data); the document is `cancello_lr/1`, written in the work folder and
never committed by this tool.

Real data are read from the data repository (`dati/3200 qtrap/mzML`) or from `--serie-lr` and the criteria that need them stay `ok` null (skipped, never passed) without it.
The full scan is judged against reference ions: on the synthetic series the frozen ions of the former Python engine (`tests/golden/finder_lr_riferimento.json`);
the real series has no public reference, so its full scan criteria stay unmeasured (only the determinism of the run is measured)."""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

from dati_percorsi import percorso
from . import ROOT, cancello, lr_criteri as C, lr_nativo as N, lr_sintetici as S, nativo

sys.path.insert(0, str(ROOT / "TP_Mine" / "tests"))
FORMAT = "cancello_lr/1"
SYNTHETIC_NEUTRAL = ("C10H12N2O3S", 241.0641, 9.0)                    # the parent of lr_synth: neutral formula, [M+H]+ m/z, RT
SYNTHETIC_REFERENCE = ROOT / "tests" / "golden" / "finder_lr_riferimento.json"


def prefissa(criteria: list[dict], p: str) -> list[dict]:
    """The same criteria under another prefix (`lr_x` -> `<p>x`)."""
    return [{**c, "id": p + c["id"][3:]} for c in criteria]


def fullscan(files: list[dict], parent: tuple, work: Path, rif: list[dict], pairs_truth=None) -> tuple[list[dict], dict, dict]:
    """The full scan criteria on `files` ([{name, path}]) against the reference ions `rif` ([{mz, rt, kind, label}]): (criteria, details, sha of the run).
    `parent`: (neutral formula, m/z observed, RT)."""
    idx = N.sort_by_time(N.lr_indexes([Path(f["path"]) for f in files], work / "indici"))
    run = nativo.esegui(idx, parent[0], parent[1], parent[2], work / "nativo")
    pairs = pairs_truth if pairs_truth is not None else C.isomer_pairs(rif)
    crit, det = C.criteria_fullscan(rif, run.tabella.mz, run.tabella.rt, run.identita, pairs)
    det["crate"] = run.versione_crate
    return crit, det, run.sha


def synthetic_pairs(shift: float) -> list[tuple]:
    """The isomers of lr_synth that the profile separates (valley below 50 %), from its Gaussians, as the points the table must hold."""
    import lr_synth
    pool = [x for x in lr_synth._pool(60.0, False) if x[3] > 0]
    out = []
    for i, a in enumerate(pool):
        for b in pool[i + 1:]:
            if round(a[0]) == round(b[0]) and a[1] != b[1] and C.valley(a[1], a[2], a[3], b[1], b[2], b[3]) < C.VALLEY_MAX:
                out.append(((a[0] + shift, a[1]), (b[0] + shift, b[1])))
    return out


def mrm(mzml: list[Path], work: Path) -> tuple[list[dict], dict, dict]:
    """The MRM criteria on `mzml` (in this order): (criteria, details, sha of the run)."""
    r = N.run_mrm(mzml, work / "mrm")
    crit, det = C.criteria_mrm(N.read_mrm(r["tabella"], [p.stem for p in mzml]))
    return crit, det, r["sha"]


def sintetici(work: Path, seme: int = 5) -> tuple[list[dict], dict]:
    """The criteria on the synthetic series: lr_synth (full scan) and the six-level MRM series; both run twice."""
    import lr_synth
    files = lr_synth.write_series(work / "fullscan")
    neutral, mz, rt = SYNTHETIC_NEUTRAL
    parent = (neutral, mz + lr_synth.SHIFT, rt)
    rif = json.loads(SYNTHETIC_REFERENCE.read_text(encoding="utf-8"))["ioni"]
    cf, df, sha = fullscan(files, parent, work / "fs1", rif, synthetic_pairs(lr_synth.SHIFT))
    _, _, sha2 = fullscan(files, parent, work / "fs2", rif, synthetic_pairs(lr_synth.SHIFT))
    mzml = S.mrm_series(work / "mrm_file", seme)
    cm, dm, shm = mrm(mzml, work / "m1")
    shm2 = N.run_mrm(mzml, work / "m2")["sha"]
    crit = cm + cf + [cancello.crit("lr_determinismo", int(sha == sha2 and shm == shm2), 1, "==")]
    return prefissa(crit, "lr_sint_"), {"mrm": dm, "fullscan": df}


def find_real(folder: str | None) -> Path | None:
    """The folder of the real mzML: `--serie-lr`, else `dati/3200 qtrap/mzML` of the data repository; None when it does not exist."""
    base = Path(folder) if folder else percorso("dati/3200 qtrap/mzML")
    return base if base and base.is_dir() else None


def real(folder: Path | None, parent: tuple | None, work: Path) -> tuple[list[dict], dict]:
    """The criteria on the real series (`B_MRM-*`, `B_FullMass-*` + `B_MS2-*`); each part not found stays unmeasured. `parent`: (neutral formula, m/z observed, RT) of the full scan."""
    det: dict = {}
    mrm_f = sorted(folder.rglob("B_MRM-*.mzML")) if folder else []
    fs_f = sorted(folder.rglob("B_FullMass-*.mzML")) + sorted(folder.rglob("B_MS2-*.mzML")) if folder else []
    if mrm_f:
        cm, det["mrm"], shm = mrm(mrm_f, work / "mrm")
        cm2 = N.run_mrm(mrm_f, work / "mrm2")["sha"] == shm
    else:
        (cm, _), cm2 = C.criteria_mrm(None), None
    cf, _ = C.criteria_fullscan(None, [], [], [], [])                  # no public reference ions for the real full scan
    if fs_f and parent:
        idx = N.sort_by_time(N.lr_indexes(fs_f, work / "fs1" / "indici"))
        cf2 = nativo.esegui(idx, parent[0], parent[1], parent[2], work / "fs1").sha == nativo.esegui(idx, parent[0], parent[1], parent[2], work / "fs2").sha
    else:
        cf2 = None
    d = None if cm2 is None and cf2 is None else int(bool(cm2 is not False and cf2 is not False))
    return cm + cf + [cancello.crit("lr_determinismo", d, 1, "==")], det


def lr_table_view(criteria: list[dict]) -> bool:
    """True when every criterion (real and synthetic) was measured and passes."""
    return bool(criteria) and all(c["ok"] is True for c in criteria)


def document(criteria: list[dict], data: str, crate: str) -> dict:
    return {"criteri": criteria, "crate_version": crate, "data": data, "format": FORMAT, "vista_tabella_lr": lr_table_view(criteria)}


def misura(real_folder: str | None, parent: tuple | None, work: Path, data: str, seme: int = 5) -> tuple[dict, dict]:
    """The whole measure: synthetic series always, real ones when found. Returns (document, details)."""
    cs, ds = sintetici(work / "sintetici", seme)
    base = find_real(real_folder)
    cr, dr = real(base, parent, work / "reale")
    det = {"sintetici": ds, "reale": dr, "dati_reali": "letti" if base else "saltato: nessuna cartella (MZLAB_DATI o --serie-lr)"}
    return document(cr + cs, data, cancello.crate_version()), det


def a_json(c: dict) -> str:
    return json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def markdown(c: dict, det: dict) -> str:
    """The report, aggregated numbers only (ready for studio/validazione/FT_*.md)."""
    row = lambda k: f"| `{k['id']}` | {'saltato' if k['ok'] is None else ('passato' if k['ok'] else 'NON PASSATO')} | {k['valore']} | {k['soglia']} |"      # noqa: E731
    out = [f"# Low-resolution table view ({c['data']}, crate {c['crate_version']})", "",
           f"`vista_tabella_lr`: **{'open' if c['vista_tabella_lr'] else 'closed'}**; real data: {det.get('dati_reali')}", "",
           "| criterion | result | value | threshold |", "|---|---|---|---|"] + [row(k) for k in c["criteri"]] + ["", "## Details", "```json",
           json.dumps(det, indent=1, sort_keys=True, ensure_ascii=False, default=str), "```", ""]
    return "\n".join(out)


def scrivi(output: Path, c: dict, det: dict, json_path: str | None = None) -> list[Path]:
    output.mkdir(parents=True, exist_ok=True)
    pj, pm = output / "cancello_lr.json", output / "cancello_lr_rapporto.md"
    pj.write_text(a_json(c), encoding="utf-8")
    pm.write_text(markdown(c, det), encoding="utf-8")
    if json_path:
        Path(json_path).write_text(a_json(c), encoding="utf-8")
    return [pj, pm]
