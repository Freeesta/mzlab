"""OpenMS (pyOpenMS): FeatureFinderMetabo per file, pose-clustering alignment and KD linking -> consensus table -> `bench/1`. See `corri.py`."""
from __future__ import annotations

import csv
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import pyopenms as oms  # noqa: E402

from bench import adatta, corri  # noqa: E402


def _aplica(algo, values: dict) -> None:
    p = algo.getParameters()
    for k, v in values.items():
        p.setValue(k, v)
    algo.setParameters(p)


def find_feature(mzml: Path, par: dict) -> oms.FeatureMap:
    exp = oms.MSExperiment()
    oms.MzMLFile().load(str(mzml), exp)
    exp.sortSpectra(True)
    mtd, epd, ffm = oms.MassTraceDetection(), oms.ElutionPeakDetection(), oms.FeatureFindingMetabo()
    _aplica(mtd, par["MassTraceDetection"])
    _aplica(epd, par["ElutionPeakDetection"])
    _aplica(ffm, par["FeatureFindingMetabo"])
    traces = mtd.run(exp, 0)
    divise = epd.detectPeaks(traces)
    if epd.getParameters().getValue("width_filtering") == "auto":
        divise = epd.filterByPeakWidth(divise)
    fm = oms.FeatureMap()
    ffm.run(divise, fm)
    fm.setUniqueIds()
    fm.setPrimaryMSRunPath([mzml.name.encode()])
    return fm


def align_and_merge(maps: list, par: dict) -> oms.ConsensusMap:
    """Linking with the KD algorithm, whose own RT warping (`warp:enabled`) aligns the maps: pyOpenMS 3.6 `MapAlignmentAlgorithmPoseClustering.align` fails on these
    feature maps ("no data points for 'linear' model"), so the separate pose-clustering step is left out and the warping is the linker's."""
    kd = oms.FeatureGroupingAlgorithmKD()
    _aplica(kd, par["FeatureGroupingAlgorithmKD"])
    cons = oms.ConsensusMap()
    kd.group(maps, cons)
    return cons


def write_csv(cons: oms.ConsensusMap, names: list[str], path: Path) -> None:
    """The map index of a consensus handle is the position of the map in the list given to the linker, i.e. the order of `names`."""
    with open(path, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["mz", "rt"] + [f"intensity_{n}" for n in names])
        for c in cons:
            v = [0.0] * len(names)
            for fh in c.getFeatureList():
                v[fh.getMapIndex()] += float(fh.getIntensity())
            w.writerow([c.getMZ(), c.getRT()] + v)


def main() -> None:
    a = corri.argomenti(__doc__)
    par = corri.parameters("openms", a.config)
    fs = corri.file_mzml(a.mzml_dir, a.solo)
    maps = []
    for f in fs:
        print(f.name, flush=True)
        maps.append(find_feature(f, par))
    cons = align_and_merge(maps, par)
    nat = corri.native_folder(a.out_dir, "openms", a.config)
    write_csv(cons, [f.stem for f in fs], nat / "consensus.csv")
    t = adatta.da_csv("openms", nat / "consensus.csv", [f.stem for f in fs], a.dataset, a.config)
    print(corri.salva(t, a.out_dir, oms.__version__), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
