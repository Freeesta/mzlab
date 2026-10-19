"""The low-resolution table view (`vista_tabella_lr`): the criteria of `lr_criteri` measured on the native pipeline of the site, on synthetic series and on the real
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
FORMATO = "cancello_lr/1"
NEUTRA_SINTETICA = ("C10H12N2O3S", 241.0641, 9.0)                    # the parent of lr_synth: neutral formula, [M+H]+ m/z, RT
RIFERIMENTO_SINTETICO = ROOT / "tests" / "golden" / "finder_lr_riferimento.json"


def prefissa(criteri: list[dict], p: str) -> list[dict]:
    """The same criteria under another prefix (`lr_x` -> `<p>x`)."""
    return [{**c, "id": p + c["id"][3:]} for c in criteri]


def fullscan(files: list[dict], parent: tuple, lavoro: Path, rif: list[dict], coppie_verita=None) -> tuple[list[dict], dict, dict]:
    """The full scan criteria on `files` ([{name, path}]) against the reference ions `rif` ([{mz, rt, kind, label}]): (criteria, details, sha of the run).
    `parent`: (neutral formula, m/z observed, RT)."""
    idx = N.ordina_per_tempo(N.indici_lr([Path(f["path"]) for f in files], lavoro / "indici"))
    run = nativo.esegui(idx, parent[0], parent[1], parent[2], lavoro / "nativo")
    coppie = coppie_verita if coppie_verita is not None else C.coppie_isomeri(rif)
    crit, det = C.criteri_fullscan(rif, run.tabella.mz, run.tabella.rt, run.identita, coppie)
    det["crate"] = run.versione_crate
    return crit, det, run.sha


def coppie_sintetiche(shift: float) -> list[tuple]:
    """The isomers of lr_synth that the profile separates (valley below 50 %), from its Gaussians, as the points the table must hold."""
    import lr_synth
    pool = [x for x in lr_synth._pool(60.0, False) if x[3] > 0]
    out = []
    for i, a in enumerate(pool):
        for b in pool[i + 1:]:
            if round(a[0]) == round(b[0]) and a[1] != b[1] and C.valle(a[1], a[2], a[3], b[1], b[2], b[3]) < C.VALLE_MAX:
                out.append(((a[0] + shift, a[1]), (b[0] + shift, b[1])))
    return out


def mrm(mzml: list[Path], lavoro: Path) -> tuple[list[dict], dict, dict]:
    """The MRM criteria on `mzml` (in this order): (criteria, details, sha of the run)."""
    r = N.esegui_mrm(mzml, lavoro / "mrm")
    crit, det = C.criteri_mrm(N.leggi_mrm(r["tabella"], [p.stem for p in mzml]))
    return crit, det, r["sha"]


def sintetici(lavoro: Path, seme: int = 5) -> tuple[list[dict], dict]:
    """The criteria on the synthetic series: lr_synth (full scan) and the six-level MRM series; both run twice."""
    import lr_synth
    files = lr_synth.write_series(lavoro / "fullscan")
    neutra, mz, rt = NEUTRA_SINTETICA
    parent = (neutra, mz + lr_synth.SHIFT, rt)
    rif = json.loads(RIFERIMENTO_SINTETICO.read_text(encoding="utf-8"))["ioni"]
    cf, df, sha = fullscan(files, parent, lavoro / "fs1", rif, coppie_sintetiche(lr_synth.SHIFT))
    _, _, sha2 = fullscan(files, parent, lavoro / "fs2", rif, coppie_sintetiche(lr_synth.SHIFT))
    mzml = S.serie_mrm(lavoro / "mrm_file", seme)
    cm, dm, shm = mrm(mzml, lavoro / "m1")
    shm2 = N.esegui_mrm(mzml, lavoro / "m2")["sha"]
    crit = cm + cf + [cancello.crit("lr_determinismo", int(sha == sha2 and shm == shm2), 1, "==")]
    return prefissa(crit, "lr_sint_"), {"mrm": dm, "fullscan": df}


def trova_reale(cartella: str | None) -> Path | None:
    """The folder of the real mzML: `--serie-lr`, else `dati/3200 qtrap/mzML` of the data repository; None when it does not exist."""
    base = Path(cartella) if cartella else percorso("dati/3200 qtrap/mzML")
    return base if base and base.is_dir() else None


def reale(cartella: Path | None, parent: tuple | None, lavoro: Path) -> tuple[list[dict], dict]:
    """The criteria on the real series (`B_MRM-*`, `B_FullMass-*` + `B_MS2-*`); each part not found stays unmeasured. `parent`: (neutral formula, m/z observed, RT) of the full scan."""
    det: dict = {}
    mrm_f = sorted(cartella.rglob("B_MRM-*.mzML")) if cartella else []
    fs_f = sorted(cartella.rglob("B_FullMass-*.mzML")) + sorted(cartella.rglob("B_MS2-*.mzML")) if cartella else []
    if mrm_f:
        cm, det["mrm"], shm = mrm(mrm_f, lavoro / "mrm")
        cm2 = N.esegui_mrm(mrm_f, lavoro / "mrm2")["sha"] == shm
    else:
        (cm, _), cm2 = C.criteri_mrm(None), None
    cf, _ = C.criteri_fullscan(None, [], [], [], [])                  # no public reference ions for the real full scan
    if fs_f and parent:
        idx = N.ordina_per_tempo(N.indici_lr(fs_f, lavoro / "fs1" / "indici"))
        cf2 = nativo.esegui(idx, parent[0], parent[1], parent[2], lavoro / "fs1").sha == nativo.esegui(idx, parent[0], parent[1], parent[2], lavoro / "fs2").sha
    else:
        cf2 = None
    d = None if cm2 is None and cf2 is None else int(bool(cm2 is not False and cf2 is not False))
    return cm + cf + [cancello.crit("lr_determinismo", d, 1, "==")], det


def vista_tabella_lr(criteri: list[dict]) -> bool:
    """True when every criterion (real and synthetic) was measured and passes."""
    return bool(criteri) and all(c["ok"] is True for c in criteri)


def documento(criteri: list[dict], data: str, crate: str) -> dict:
    return {"criteri": criteri, "crate_version": crate, "data": data, "format": FORMATO, "vista_tabella_lr": vista_tabella_lr(criteri)}


def misura(cartella_reale: str | None, parent: tuple | None, lavoro: Path, data: str, seme: int = 5) -> tuple[dict, dict]:
    """The whole measure: synthetic series always, real ones when found. Returns (document, details)."""
    cs, ds = sintetici(lavoro / "sintetici", seme)
    base = trova_reale(cartella_reale)
    cr, dr = reale(base, parent, lavoro / "reale")
    det = {"sintetici": ds, "reale": dr, "dati_reali": "letti" if base else "saltato: nessuna cartella (MZLAB_DATI o --serie-lr)"}
    return documento(cr + cs, data, cancello.versione_crate()), det


def a_json(c: dict) -> str:
    return json.dumps(c, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def markdown(c: dict, det: dict) -> str:
    """The report, aggregated numbers only (ready for studio/validazione/FT_*.md)."""
    riga = lambda k: f"| `{k['id']}` | {'saltato' if k['ok'] is None else ('passato' if k['ok'] else 'NON PASSATO')} | {k['valore']} | {k['soglia']} |"      # noqa: E731
    out = [f"# Low-resolution table view ({c['data']}, crate {c['crate_version']})", "",
           f"`vista_tabella_lr`: **{'open' if c['vista_tabella_lr'] else 'closed'}**; real data: {det.get('dati_reali')}", "",
           "| criterion | result | value | threshold |", "|---|---|---|---|"] + [riga(k) for k in c["criteri"]] + ["", "## Details", "```json",
           json.dumps(det, indent=1, sort_keys=True, ensure_ascii=False, default=str), "```", ""]
    return "\n".join(out)


def scrivi(uscita: Path, c: dict, det: dict, json_path: str | None = None) -> list[Path]:
    uscita.mkdir(parents=True, exist_ok=True)
    pj, pm = uscita / "cancello_lr.json", uscita / "cancello_lr_rapporto.md"
    pj.write_text(a_json(c), encoding="utf-8")
    pm.write_text(markdown(c, det), encoding="utf-8")
    if json_path:
        Path(json_path).write_text(a_json(c), encoding="utf-8")
    return [pj, pm]
