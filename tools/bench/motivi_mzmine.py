"""Why a row of mzmine has no row of ours, and the other way round (a tool of measure, not a function of the product).

`classifica` takes the two aligned tables (ours in `Mode::Ion`, mzmine's after the gap filling), the lists of features of every file before the alignment and says, for
each row without a partner under the common floor of height, in which stage it was lost. The reasons are those of §5.8 of `docs/studi/ATTRIBUTIO.md`:
`below` (under the floor in one of the two), `family` (absorbed in a family: a member of a row of ours is the row of mzmine), `background` (a trace of mzmine longer than
`fondo_min` minutes, which we discard), `split` (mzmine cuts at a zero or at 1 min and we do not: our feature of the file is one and wider), `align` (we have the feature
in the file, the alignment put it elsewhere), `other`. The pairs with the areas more than twice apart are counted apart as `area`.
    python3 tools/bench/motivi_mzmine.py MZML_DIR WORK_DIR [--rapporto FILE.md] [--altezza-min H] [--solo STEM ...]"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np

sys.path[:0] = [str(Path(__file__).resolve().parents[1])]
from bench import corri, corri_mzmine  # noqa: E402
from valida import PPM, ROOT, nativo  # noqa: E402
from valida import cancello_misure as M  # noqa: E402
from valida.tabelle import Tabella, leggi_mzmine  # noqa: E402

RT_TOL = 0.1
MOTIVI = ("below", "family", "background", "split", "align", "other")
FATTORE_AREA = 2.0


@dataclass
class ListaFile:
    """The features of one file before the alignment."""
    mz: np.ndarray
    rt: np.ndarray
    rt_inizio: np.ndarray
    rt_fine: np.ndarray


def _vicini(mz_lista, rt_lista, mz: float, rt: float, rt_tol: float = RT_TOL) -> np.ndarray:
    """Indices of the points within PPM and `rt_tol` of (mz, rt)."""
    return np.flatnonzero((np.abs(mz_lista - mz) <= mz * PPM * 1e-6) & (np.abs(rt_lista - rt) <= rt_tol))


def _nelle_colonne(tab: Tabella, riga: int, colonne) -> list[int]:
    return [k for k in colonne if tab.area[riga, k] > 0]


def _below(tab_altra: Tabella, mz: float, rt: float, colonne, altezza_min: float) -> bool:
    """The other table has the row but under the floor."""
    i = _vicini(tab_altra.mz, tab_altra.rt, mz, rt)
    return bool(len(i)) and float(tab_altra.massimo_altezza(colonne)[i].max()) < altezza_min


def _family(membri, mz: float, rt: float) -> bool:
    """A member (not the first) of some row of ours is at (mz, rt)."""
    if membri is None or len(membri[1]) == 0:
        return False
    inizio, m_mz, m_rt = membri
    primo = np.zeros(len(m_mz), bool)
    primo[inizio[:-1][inizio[:-1] < len(m_mz)]] = True
    i = np.flatnonzero(~primo & (np.abs(m_mz - mz) <= mz * PPM * 1e-6) & (np.abs(m_rt - rt) <= RT_TOL))
    return bool(len(i))


def _fondo(adap: list[ListaFile] | None, colonne_riga: list[int], mz: float, rt: float, fondo_min: float) -> bool:
    if adap is None:
        return False
    for k in colonne_riga:
        L = adap[k]
        i = _vicini(L.mz, L.rt, mz, rt, fondo_min)
        if len(i) and float(np.max(L.rt_fine[i] - L.rt_inizio[i])) >= fondo_min:
            return True
    return False


def _contiene(nat: ListaFile, mm: ListaFile, mz: float, rt: float) -> bool:
    """A feature of ours at this m/z holds the RT and two or more features of mzmine of the same file: mzmine cut a peak that we keep whole."""
    for j in np.flatnonzero((np.abs(nat.mz - mz) <= mz * PPM * 1e-6) & (nat.rt_inizio <= rt + 1e-9) & (nat.rt_fine >= rt - 1e-9)):
        dentro = (np.abs(mm.mz - mz) <= mz * PPM * 1e-6) & (mm.rt >= nat.rt_inizio[j] - 1e-9) & (mm.rt <= nat.rt_fine[j] + 1e-9)
        if int(dentro.sum()) >= 2:
            return True
    return False


def _split_mm(nat_file: list[ListaFile], mm_file: list[ListaFile], colonne_riga: list[int], mz: float, rt: float) -> bool:
    return any(_contiene(nat_file[k], mm_file[k], mz, rt) for k in colonne_riga)


def _align(nat_file: list[ListaFile], colonne_riga: list[int], mz: float, rt: float) -> bool:
    """We have a feature at (mz, RT) in at least half of the files of the row of mzmine: the alignment took it elsewhere."""
    con = sum(1 for k in colonne_riga if len(_vicini(nat_file[k].mz, nat_file[k].rt, mz, rt)))
    return bool(colonne_riga) and 2 * con >= len(colonne_riga)


def _split_nat(mm_file: list[ListaFile], nat_file: list[ListaFile], colonne_riga: list[int], mz: float, rt: float) -> bool:
    """Our feature of the file is wide and holds two or more features of mzmine."""
    return any(_contiene(nat_file[k], mm_file[k], mz, rt) for k in colonne_riga)


def classifica(nativa: Tabella, mzmine: Tabella, colonne, altezza_min: float, nat_file: list[ListaFile], mm_file: list[ListaFile], adap: list[ListaFile] | None = None,
               membri=None, fondo_min: float = 5.0) -> dict:
    """{"mzmine": {reason: [rows of mzmine without a partner]}, "nativa": {reason: [rows of ours without a partner]}, "area": [(row mzmine, row ours)], "senza_feature": [rows of mzmine in
    `other` of which we have no feature in any file, i.e. lost by the detection]}; the rows are indices of the full tables. `adap`: the chromatograms of mzmine per file (None = the reason `background` is not looked for)."""
    colonne = list(colonne)
    fm, fn, ia, ib = M.abbina_ioni(nativa, mzmine, colonne, altezza_min)
    senza_m = np.setdiff1d(np.arange(len(fm)), ia)
    senza_n = np.setdiff1d(np.arange(len(fn)), ib)
    out: dict = {"mzmine": {m: [] for m in MOTIVI}, "nativa": {m: [] for m in MOTIVI}, "area": [], "senza_feature": []}
    for r in fm[senza_m].tolist():
        mz, rt = float(mzmine.mz[r]), float(mzmine.rt[r])
        cr = _nelle_colonne(mzmine, r, colonne)
        if _below(nativa, mz, rt, colonne, altezza_min):
            m = "below"
        elif _family(membri, mz, rt):
            m = "family"
        elif _fondo(adap, cr, mz, rt, fondo_min):
            m = "background"
        elif _split_mm(nat_file, mm_file, cr, mz, rt):
            m = "split"
        elif _align(nat_file, cr, mz, rt):
            m = "align"
        else:
            m = "other"
            if not any(len(_vicini(L.mz, L.rt, mz, rt, 2 * RT_TOL)) for L in nat_file):
                out["senza_feature"].append(r)
        out["mzmine"][m].append(r)
    for r in fn[senza_n].tolist():
        mz, rt = float(nativa.mz[r]), float(nativa.rt[r])
        cr = _nelle_colonne(nativa, r, colonne)
        alt = np.flatnonzero((np.abs(mzmine.mz[fm] - mz) <= mz * PPM * 1e-6) & (np.abs(mzmine.rt[fm] - rt) <= 2 * RT_TOL))
        if _below(mzmine, mz, rt, colonne, altezza_min):
            m = "below"
        elif membri is not None and any(len(_vicini(mzmine.mz[fm], mzmine.rt[fm], float(a), float(b))) for a, b in _membri_di(membri, r)):
            m = "family"
        elif _split_nat(mm_file, nat_file, cr, mz, rt):
            m = "split"
        elif len(alt) >= 2:
            m = "align"
        else:
            m = "other"
        out["nativa"][m].append(r)
    am, an = mzmine.massimo(colonne) * M.MIN_A_S, nativa.massimo(colonne)
    for i, j in zip(ia.tolist(), ib.tolist()):
        a, b = am[fm[i]], an[fn[j]]
        if a > 0 and b > 0 and abs(np.log2(b / a)) > np.log2(FATTORE_AREA):
            out["area"].append((int(fm[i]), int(fn[j])))
    return out


def _membri_di(membri, riga: int):
    inizio, m_mz, m_rt = membri
    return list(zip(m_mz[inizio[riga] + 1:inizio[riga + 1]], m_rt[inizio[riga] + 1:inizio[riga + 1]])) if len(inizio) > riga + 1 else []


def conteggi(esito: dict) -> dict:
    return {"mzmine": {m: len(v) for m, v in esito["mzmine"].items()}, "nativa": {m: len(v) for m, v in esito["nativa"].items()}, "area": len(esito["area"]), "senza_feature": len(esito["senza_feature"])}


# ---- the run on a folder of mzML (the real series of the private repository, or the synthetic gold files) ----

def _lista_nativa(cartella: Path, nomi: list[str]) -> list[ListaFile]:
    out = []
    for n in nomi:
        _, f = nativo.apri((cartella / f"{n}.mzfeatf").read_bytes())
        out.append(ListaFile(f["mz"].astype(float), f["rt"].astype(float), f["left"].astype(float), f["right"].astype(float)))
    return out


def _tabella_nativa(cartella: Path, nomi: list[str]):
    meta, a = nativo.apri((cartella / "series.mzfeat").read_bytes())
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k)
    area = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["area"].reshape(n, k)))
    altezza = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["height"].reshape(n, k).astype(float)))
    membri = (a["member_start"].astype(int), a["member_mz"].astype(float), a["member_rt"].astype(float)) if len(a["member_start"]) else None
    return Tabella("nativo", [Path(x).stem for x in nomi], a["mz"].astype(float), a["rt"].astype(float), area, altezza=altezza), membri


def _lista_mzmine(fasi: Path, prefisso: str, stems: list[str]) -> list[ListaFile]:
    import csv
    out = []
    for s in stems:
        p = next(fasi.glob(f"{prefisso}_{s}.mzML_*.csv"))
        with open(p, newline="", encoding="utf-8") as f:
            r = list(csv.DictReader(f))
        out.append(ListaFile(*(np.array([float(x[c]) for x in r]) for c in ("mz", "rt", "rt_range:min", "rt_range:max"))))
    return out


def corri_nativo(idx: list[str], uscita: Path, ricetta: str) -> None:
    uscita.mkdir(parents=True, exist_ok=True)
    nativo.costruisci_esempio()
    subprocess.run([str(nativo.ESEMPIO), str(uscita), *idx, *(["--recipe", ricetta] if ricetta else [])], cwd=ROOT, check=True, capture_output=True)


def misura(mzml: Path, lavoro: Path, altezza_min: float, solo: list[str] | None = None, ciclo: float | None = None) -> dict:
    """The old and the new protocol, by file and for the series, and the reasons; everything is written in `lavoro`."""
    fs = corri.file_mzml(mzml, solo)
    stems = [f.stem for f in fs]
    serie = [{"nome": s} for s in stems]
    idx = nativo.indici(fs, lavoro / "indici")
    nomi = [Path(x).name for x in idx]
    corri_nativo(idx, lavoro / "ion", json.dumps({"mode": "ion"}))
    corri_nativo(idx, lavoro / "famiglia", "")
    ion, membri = _tabella_nativa(lavoro / "ion", nomi)
    fam, _ = _tabella_nativa(lavoro / "famiglia", nomi)
    fasi = lavoro / "mzmine"
    fasi.mkdir(exist_ok=True)
    mm, jh = corri_mzmine.installa_39()
    par = corri.parametri("mzmine", "predefinita")
    xml = fasi / "fasi.xml"
    xml.write_text(corri_mzmine.batch_xml(fs, fasi / "finale.csv", par, fasi=fasi), encoding="utf-8")
    if (non_letti := corri_mzmine.parametri_non_letti(xml, mm)):
        raise SystemExit("mzmine would ignore: " + "; ".join(non_letti))
    r = corri_mzmine.esegui_batch(xml, mm, jh, fasi / "tmp")
    finale = fasi / "riempimento_Aligned_feature_list_gap-filled.csv"
    if r.returncode or not finale.exists():
        raise SystemExit(f"mzmine failed:\n{(r.stdout + r.stderr)[-2000:]}")
    tabella_mm = leggi_mzmine(finale, serie)
    colonne = list(range(len(stems)))
    out = {"altezza_min": altezza_min, "file": stems, "per_serie": {}, "per_file": {}}
    cfg = lambda c: {"vecchio": M.concordanza_mzmine(fam, tabella_mm, c, 50.0), "nuovo": M.concordanza_mzmine_ioni(ion, tabella_mm, c, altezza_min)}
    out["per_serie"] = cfg(colonne)
    out["per_file"] = {s: cfg([k]) for k, s in enumerate(stems)}
    nat_file = _lista_nativa(lavoro / "ion", nomi)
    ciclo = ciclo or _ciclo(fs[0])
    esito = classifica(ion, tabella_mm, colonne, altezza_min, nat_file, _lista_mzmine(fasi, "risolutore", stems), _lista_mzmine(fasi, "adap", stems), membri, 400 * ciclo)
    out["motivi"] = conteggi(esito)
    return out


def _ciclo(mzml: Path) -> float:
    """Median spacing in minutes of the MS1 scans of a file."""
    from mzlab.reader.mzml import Run
    run = Run(mzml)
    rt = np.array([x.rt for x in run.scans if x.level == 1], float)
    run.close()
    return float(np.median(np.diff(rt))) if len(rt) > 2 else 0.0125


def rapporto(o: dict) -> str:
    def riga(n, c):
        f = lambda x: "-" if x is None else f"{x:.3f}"
        return f"| {n} | {c.get('righe_mzmine', 0)} | {c.get('righe_native', 0)} | {f(c.get('richiamo'))} | {f(c.get('precisione'))} | {f(c.get('spearman'))} | {f(c.get('log2_mediano', c.get('log2_centrato')))} |"
    testa = "| | rows mzmine | rows ours | recall | precision | Spearman | median abs log2 |\n|---|---|---|---|---|---|---|"
    s = [f"# mzmine agreement (floor of height {o['altezza_min']:g})", "", "## Series", testa]
    s += [riga("old (family, area)", o["per_serie"]["vecchio"]), riga("new (ion, height, counts x s)", o["per_serie"]["nuovo"]), "", "## By file", testa]
    for f, c in o["per_file"].items():
        s += [riga(f + " old", c["vecchio"]), riga(f + " new", c["nuovo"])]
    m = o["motivi"]
    s += ["", "## Reasons of the rows without a partner (new protocol)", "", "| reason | rows of mzmine | rows of ours |", "|---|---|---|"]
    s += [f"| {k} | {m['mzmine'][k]} | {m['nativa'][k]} |" for k in MOTIVI]
    s += ["", f"Of the rows of mzmine in `other`, {m['senza_feature']} have no feature of ours in any file (lost by the detection).",
          f"Pairs with the areas more than {FATTORE_AREA:g} times apart: {m['area']}."]
    return "\n".join(s) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("mzml_dir")
    ap.add_argument("lavoro")
    ap.add_argument("--rapporto")
    ap.add_argument("--altezza-min", type=float, default=10 * M.FLOOR, help="common floor of height in counts (default 10 x the noise floor of the engine)")
    ap.add_argument("--ciclo", type=float, help="spacing of the MS1 scans in minutes (default: read from the first file)")
    ap.add_argument("--solo", nargs="*")
    a = ap.parse_args()
    lavoro = Path(a.lavoro)
    lavoro.mkdir(parents=True, exist_ok=True)
    o = misura(Path(a.mzml_dir), lavoro, a.altezza_min, a.solo, a.ciclo)
    (lavoro / "motivi.json").write_text(json.dumps(o, indent=1), encoding="utf-8")
    testo = rapporto(o)
    if a.rapporto:
        Path(a.rapporto).write_text(testo, encoding="utf-8")
    print(testo)


if __name__ == "__main__":
    main()
