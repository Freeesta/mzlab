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
REASONS = ("below", "family", "background", "split", "align", "other")
AREA_FACTOR = 2.0


@dataclass
class ListaFile:
    """The features of one file before the alignment."""
    mz: np.ndarray
    rt: np.ndarray
    rt_inizio: np.ndarray
    rt_fine: np.ndarray


def _neighbors(mz_list, rt_list, mz: float, rt: float, rt_tol: float = RT_TOL) -> np.ndarray:
    """Indices of the points within PPM and `rt_tol` of (mz, rt)."""
    return np.flatnonzero((np.abs(mz_list - mz) <= mz * PPM * 1e-6) & (np.abs(rt_list - rt) <= rt_tol))


def _in_columns(tab: Tabella, row: int, columns) -> list[int]:
    return [k for k in columns if tab.area[row, k] > 0]


def _below(tab_other: Tabella, mz: float, rt: float, columns, min_height: float) -> bool:
    """The other table has the row but under the floor."""
    i = _neighbors(tab_other.mz, tab_other.rt, mz, rt)
    return bool(len(i)) and float(tab_other.massimo_altezza(columns)[i].max()) < min_height


def _family(members, mz: float, rt: float) -> bool:
    """A member (not the first) of some row of ours is at (mz, rt)."""
    if members is None or len(members[1]) == 0:
        return False
    start, m_mz, m_rt = members
    first = np.zeros(len(m_mz), bool)
    first[start[:-1][start[:-1] < len(m_mz)]] = True
    i = np.flatnonzero(~first & (np.abs(m_mz - mz) <= mz * PPM * 1e-6) & (np.abs(m_rt - rt) <= RT_TOL))
    return bool(len(i))


def _background(adap: list[ListaFile] | None, columns_row: list[int], mz: float, rt: float, fondo_min: float) -> bool:
    if adap is None:
        return False
    for k in columns_row:
        L = adap[k]
        i = _neighbors(L.mz, L.rt, mz, rt, fondo_min)
        if len(i) and float(np.max(L.rt_fine[i] - L.rt_inizio[i])) >= fondo_min:
            return True
    return False


def _contiene(nat: ListaFile, mm: ListaFile, mz: float, rt: float) -> bool:
    """A feature of ours at this m/z holds the RT and two or more features of mzmine of the same file: mzmine cut a peak that we keep whole."""
    for j in np.flatnonzero((np.abs(nat.mz - mz) <= mz * PPM * 1e-6) & (nat.rt_inizio <= rt + 1e-9) & (nat.rt_fine >= rt - 1e-9)):
        inside = (np.abs(mm.mz - mz) <= mz * PPM * 1e-6) & (mm.rt >= nat.rt_inizio[j] - 1e-9) & (mm.rt <= nat.rt_fine[j] + 1e-9)
        if int(inside.sum()) >= 2:
            return True
    return False


def _split_mm(nat_file: list[ListaFile], mm_file: list[ListaFile], columns_row: list[int], mz: float, rt: float) -> bool:
    return any(_contiene(nat_file[k], mm_file[k], mz, rt) for k in columns_row)


def _align(nat_file: list[ListaFile], columns_row: list[int], mz: float, rt: float) -> bool:
    """We have a feature at (mz, RT) in at least half of the files of the row of mzmine: the alignment took it elsewhere."""
    con = sum(1 for k in columns_row if len(_neighbors(nat_file[k].mz, nat_file[k].rt, mz, rt)))
    return bool(columns_row) and 2 * con >= len(columns_row)


def _split_nat(mm_file: list[ListaFile], nat_file: list[ListaFile], columns_row: list[int], mz: float, rt: float) -> bool:
    """Our feature of the file is wide and holds two or more features of mzmine."""
    return any(_contiene(nat_file[k], mm_file[k], mz, rt) for k in columns_row)


def classifica(native: Tabella, mzmine: Tabella, columns, min_height: float, nat_file: list[ListaFile], mm_file: list[ListaFile], adap: list[ListaFile] | None = None,
               members=None, fondo_min: float = 5.0) -> dict:
    """{"mzmine": {reason: [rows of mzmine without a partner]}, "nativa": {reason: [rows of ours without a partner]}, "area": [(row mzmine, row ours)], "senza_feature": [rows of mzmine in
    `other` of which we have no feature in any file, i.e. lost by the detection]}; the rows are indices of the full tables. `adap`: the chromatograms of mzmine per file (None = the reason `background` is not looked for)."""
    columns = list(columns)
    fm, fn, ia, ib = M.abbina_ioni(native, mzmine, columns, min_height)
    without_m = np.setdiff1d(np.arange(len(fm)), ia)
    without_n = np.setdiff1d(np.arange(len(fn)), ib)
    out: dict = {"mzmine": {m: [] for m in REASONS}, "nativa": {m: [] for m in REASONS}, "area": [], "senza_feature": []}
    for r in fm[without_m].tolist():
        mz, rt = float(mzmine.mz[r]), float(mzmine.rt[r])
        cr = _in_columns(mzmine, r, columns)
        if _below(native, mz, rt, columns, min_height):
            m = "below"
        elif _family(members, mz, rt):
            m = "family"
        elif _background(adap, cr, mz, rt, fondo_min):
            m = "background"
        elif _split_mm(nat_file, mm_file, cr, mz, rt):
            m = "split"
        elif _align(nat_file, cr, mz, rt):
            m = "align"
        else:
            m = "other"
            if not any(len(_neighbors(L.mz, L.rt, mz, rt, 2 * RT_TOL)) for L in nat_file):
                out["senza_feature"].append(r)
        out["mzmine"][m].append(r)
    for r in fn[without_n].tolist():
        mz, rt = float(native.mz[r]), float(native.rt[r])
        cr = _in_columns(native, r, columns)
        alt = np.flatnonzero((np.abs(mzmine.mz[fm] - mz) <= mz * PPM * 1e-6) & (np.abs(mzmine.rt[fm] - rt) <= 2 * RT_TOL))
        if _below(mzmine, mz, rt, columns, min_height):
            m = "below"
        elif members is not None and any(len(_neighbors(mzmine.mz[fm], mzmine.rt[fm], float(a), float(b))) for a, b in _members_of(members, r)):
            m = "family"
        elif _split_nat(mm_file, nat_file, cr, mz, rt):
            m = "split"
        elif len(alt) >= 2:
            m = "align"
        else:
            m = "other"
        out["nativa"][m].append(r)
    am, an = mzmine.massimo(columns) * M.MIN_A_S, native.massimo(columns)
    for i, j in zip(ia.tolist(), ib.tolist()):
        a, b = am[fm[i]], an[fn[j]]
        if a > 0 and b > 0 and abs(np.log2(b / a)) > np.log2(AREA_FACTOR):
            out["area"].append((int(fm[i]), int(fn[j])))
    return out


def _members_of(members, row: int):
    start, m_mz, m_rt = members
    return list(zip(m_mz[start[row] + 1:start[row + 1]], m_rt[start[row] + 1:start[row + 1]])) if len(start) > row + 1 else []


def conteggi(outcome: dict) -> dict:
    return {"mzmine": {m: len(v) for m, v in outcome["mzmine"].items()}, "nativa": {m: len(v) for m, v in outcome["nativa"].items()}, "area": len(outcome["area"]), "senza_feature": len(outcome["senza_feature"])}


# ---- the run on a folder of mzML (the real series of the private repository, or the synthetic gold files) ----

def _native_list(folder: Path, names: list[str]) -> list[ListaFile]:
    out = []
    for n in names:
        _, f = nativo.apri((folder / f"{n}.mzfeatf").read_bytes())
        out.append(ListaFile(f["mz"].astype(float), f["rt"].astype(float), f["left"].astype(float), f["right"].astype(float)))
    return out


def _native_table(folder: Path, names: list[str]):
    meta, a = nativo.apri((folder / "series.mzfeat").read_bytes())
    n, k = meta["rows"], meta["files"]
    stato = a["state"].reshape(n, k)
    area = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["area"].reshape(n, k)))
    altezza = np.where(stato == nativo.CELLA_ASSENTE, 0.0, np.nan_to_num(a["height"].reshape(n, k).astype(float)))
    members = (a["member_start"].astype(int), a["member_mz"].astype(float), a["member_rt"].astype(float)) if len(a["member_start"]) else None
    return Tabella("nativo", [Path(x).stem for x in names], a["mz"].astype(float), a["rt"].astype(float), area, altezza=altezza), members


def _list_mzmine(phases: Path, prefix: str, stems: list[str]) -> list[ListaFile]:
    import csv
    out = []
    for s in stems:
        p = next(phases.glob(f"{prefix}_{s}.mzML_*.csv"))
        with open(p, newline="", encoding="utf-8") as f:
            r = list(csv.DictReader(f))
        out.append(ListaFile(*(np.array([float(x[c]) for x in r]) for c in ("mz", "rt", "rt_range:min", "rt_range:max"))))
    return out


def run_native(idx: list[str], output: Path, recipe: str) -> None:
    output.mkdir(parents=True, exist_ok=True)
    nativo.costruisci_esempio()
    subprocess.run([str(nativo.ESEMPIO), str(output), *idx, *(["--recipe", recipe] if recipe else [])], cwd=ROOT, check=True, capture_output=True)


def misura(mzml: Path, work: Path, min_height: float, solo: list[str] | None = None, ciclo: float | None = None) -> dict:
    """The old and the new protocol, by file and for the series, and the reasons; everything is written in `work`."""
    fs = corri.file_mzml(mzml, solo)
    stems = [f.stem for f in fs]
    series = [{"nome": s} for s in stems]
    idx = nativo.indici(fs, work / "indici")
    names = [Path(x).name for x in idx]
    run_native(idx, work / "ion", json.dumps({"mode": "ion"}))
    run_native(idx, work / "famiglia", "")
    ion, members = _native_table(work / "ion", names)
    fam, _ = _native_table(work / "famiglia", names)
    phases = work / "mzmine"
    phases.mkdir(exist_ok=True)
    mm, jh = corri_mzmine.installa_39()
    par = corri.parametri("mzmine", "predefinita")
    xml = phases / "fasi.xml"
    xml.write_text(corri_mzmine.batch_xml(fs, phases / "finale.csv", par, phases=phases), encoding="utf-8")
    if (not_read := corri_mzmine.parametri_non_letti(xml, mm)):
        raise SystemExit("mzmine would ignore: " + "; ".join(not_read))
    r = corri_mzmine.esegui_batch(xml, mm, jh, phases / "tmp")
    final = phases / "riempimento_Aligned_feature_list_gap-filled.csv"
    if r.returncode or not final.exists():
        raise SystemExit(f"mzmine failed:\n{(r.stdout + r.stderr)[-2000:]}")
    table_mm = leggi_mzmine(final, series)
    columns = list(range(len(stems)))
    out = {"altezza_min": min_height, "file": stems, "per_serie": {}, "per_file": {}}
    cfg = lambda c: {"vecchio": M.concordanza_mzmine(fam, table_mm, c, 50.0), "nuovo": M.concordanza_mzmine_ioni(ion, table_mm, c, min_height)}
    out["per_serie"] = cfg(columns)
    out["per_file"] = {s: cfg([k]) for k, s in enumerate(stems)}
    nat_file = _native_list(work / "ion", names)
    ciclo = ciclo or _cycle(fs[0])
    outcome = classifica(ion, table_mm, columns, min_height, nat_file, _list_mzmine(phases, "risolutore", stems), _list_mzmine(phases, "adap", stems), members, 400 * ciclo)
    out["motivi"] = conteggi(outcome)
    return out


def _cycle(mzml: Path) -> float:
    """Median spacing in minutes of the MS1 scans of a file."""
    from mzlab.reader.mzml import Run
    run = Run(mzml)
    rt = np.array([x.rt for x in run.scans if x.level == 1], float)
    run.close()
    return float(np.median(np.diff(rt))) if len(rt) > 2 else 0.0125


def rapporto(o: dict) -> str:
    def row(n, c):
        f = lambda x: "-" if x is None else f"{x:.3f}"
        return f"| {n} | {c.get('righe_mzmine', 0)} | {c.get('righe_native', 0)} | {f(c.get('richiamo'))} | {f(c.get('precisione'))} | {f(c.get('spearman'))} | {f(c.get('log2_mediano', c.get('log2_centrato')))} |"
    head = "| | rows mzmine | rows ours | recall | precision | Spearman | median abs log2 |\n|---|---|---|---|---|---|---|"
    s = [f"# mzmine agreement (floor of height {o['altezza_min']:g})", "", "## Series", head]
    s += [row("old (family, area)", o["per_serie"]["vecchio"]), row("new (ion, height, counts x s)", o["per_serie"]["nuovo"]), "", "## By file", head]
    for f, c in o["per_file"].items():
        s += [row(f + " old", c["vecchio"]), row(f + " new", c["nuovo"])]
    m = o["motivi"]
    s += ["", "## Reasons of the rows without a partner (new protocol)", "", "| reason | rows of mzmine | rows of ours |", "|---|---|---|"]
    s += [f"| {k} | {m['mzmine'][k]} | {m['nativa'][k]} |" for k in REASONS]
    s += ["", f"Of the rows of mzmine in `other`, {m['senza_feature']} have no feature of ours in any file (lost by the detection).",
          f"Pairs with the areas more than {AREA_FACTOR:g} times apart: {m['area']}."]
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
    work = Path(a.lavoro)
    work.mkdir(parents=True, exist_ok=True)
    o = misura(Path(a.mzml_dir), work, a.altezza_min, a.solo, a.ciclo)
    (work / "motivi.json").write_text(json.dumps(o, indent=1), encoding="utf-8")
    text = rapporto(o)
    if a.rapporto:
        Path(a.rapporto).write_text(text, encoding="utf-8")
    print(text)


if __name__ == "__main__":
    main()
