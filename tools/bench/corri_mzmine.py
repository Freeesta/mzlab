"""MZmine 3.9 (headless batch): mzML import, centroid mass detection, ADAP chromatograms, local-minimum resolving, join alignment, gap filling, CSV export -> `bench/1`.
See `corri.py`. MZmine needs an exact OpenJDK 20: `--mzmine DIR` (the `bin/MZmine` of the bioconda package) and `--java-home DIR` or `MZMINE_HOME`/`JAVA_HOME`."""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path
from xml.sax.saxutils import escape

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import adatta, corri  # noqa: E402

M = "io.github.mzmine.modules."
FL = '<parameter name="Feature lists" type="BATCH_LAST_FEATURELISTS"/>'
RAW = '<parameter name="Raw data files" type="BATCH_LAST_FILES"/>'


def mz_tol(nome: str, ass: float, ppm: float) -> str:
    return f'<parameter name="{nome}"><absolutetolerance>{ass}</absolutetolerance><ppmtolerance>{ppm}</ppmtolerance></parameter>'


def batch_xml(files: list[Path], uscita: Path, par: dict) -> str:
    a, c, r, j, g = par["ADAPChromatogramBuilder"], par["MassDetection"], par["MinimumSearchResolver"], par["JoinAligner"], par["GapFilling"]
    pol = par["polarita"].upper()
    scans = f'<parameter name="Scans"><parameter name="Polarity">{pol}</parameter><parameter name="MS level filter" selected="MS1, level = 1">1</parameter></parameter>'
    passi = [
        (M + "io.import_rawdata_mzml.MSDKmzMLImportModule",
         "<parameter name=\"File names\">" + "".join(f"<file>{escape(str(f.resolve()))}</file>" for f in files) + "</parameter>"),
        (M + "dataprocessing.featdet_massdetection.MassDetectionModule",
         RAW + scans + f'<parameter name="Mass detector" selected="Centroid"><module name="Centroid"><parameter name="Noise level">{c["noise_level"]}</parameter>'
         '<parameter name="Detect isotope signals below noise level" selected="false"/></module></parameter>'
         '<parameter name="Denormalize fragment scans (traps)">false</parameter>'),
        (M + "dataprocessing.featdet_adapchromatogrambuilder.ModularADAPChromatogramBuilderModule",
         RAW + scans + f'<parameter name="Minimum consecutive scans">{a["min_consecutive_scans"]}</parameter>'
         f'<parameter name="Minimum intensity for consecutive scans">{a["group_intensity_threshold"]}</parameter>'
         f'<parameter name="Minimum absolute height">{a["min_highest_intensity"]}</parameter>'
         + mz_tol("Scan to scan accuracy (m/z)", a["mz_tol_abs"], a["mz_tol_ppm"]) + '<parameter name="Suffix">chromatograms</parameter>'),
        (M + "dataprocessing.featdet_chromatogramdeconvolution.minimumsearch.MinimumSearchFeatureResolverModule",
         FL + '<parameter name="Suffix">resolved</parameter><parameter name="Original feature list">REMOVE</parameter>'
         '<parameter name="MS/MS scan pairing" selected="false"/><parameter name="Dimension">Retention time</parameter>'
         f'<parameter name="Minimum scans (data points)">{r["min_data_points"]}</parameter>'
         f'<parameter name="Chromatographic threshold">{r["chromatographic_threshold"]}</parameter>'
         f'<parameter name="Minimum search range RT/Mobility (absolute)">{r["search_rt_range"]}</parameter>'
         f'<parameter name="Minimum relative height">{r["min_relative_height"]}</parameter>'
         f'<parameter name="Minimum absolute height">{r["min_absolute_height"]}</parameter>'
         f'<parameter name="Min ratio of peak top/edge">{r["min_ratio"]}</parameter>'
         f'<parameter name="Peak duration range (min/mobility)"><min>{r["duration_min"]}</min><max>{r["duration_max"]}</max></parameter>'),
        (M + "dataprocessing.align_join.JoinAlignerModule",
         FL + '<parameter name="Feature list name">Aligned feature list</parameter>' + mz_tol("m/z tolerance (sample-to-sample)", j["mz_tol_abs"], j["mz_tol_ppm"])
         + f'<parameter name="Weight for m/z">{j["weight_mz"]}</parameter>'
         f'<parameter name="Retention time tolerance" unit="MINUTES">{j["rt_tol"]}</parameter>'
         f'<parameter name="Weight for RT">{j["weight_rt"]}</parameter>'
         '<parameter name="Require same charge state">false</parameter><parameter name="Require same ID">false</parameter>'
         '<parameter name="Mobility tolerance" selected="false"/><parameter name="Mobility weight">1.0</parameter><parameter name="Original feature list">REMOVE</parameter>'
         '<parameter name="Compare isotope pattern" selected="false"/><parameter name="Compare spectra similarity" selected="false"/>'),
        (M + "dataprocessing.gapfill_peakfinder.multithreaded.MultiThreadPeakFinderModule",
         FL + '<parameter name="Name suffix">gap-filled</parameter>'
         f'<parameter name="Intensity tolerance">{g["intensity_tolerance"]}</parameter>' + mz_tol("m/z tolerance (sample-to-sample)", g["mz_tol_abs"], g["mz_tol_ppm"])
         + f'<parameter name="Retention time tolerance" unit="MINUTES">{g["rt_tol"]}</parameter>'
         f'<parameter name="Minimum scans (data points)">{g["min_data_points"]}</parameter>'),
        (M + "io.export_features_csv.CSVExportModularModule",
         FL + f'<parameter name="Filename"><current_file>{escape(str(uscita))}</current_file></parameter>'
         '<parameter name="Field separator">,</parameter><parameter name="Identification separator">;</parameter>'
         '<parameter name="Filter rows">ALL</parameter>'),
    ]
    corpo = "".join(f'<batchstep method="{m}" parameter_version="{v}">{p}</batchstep>\n' for m, p in passi for v in [2 if 'MinimumSearch' in m else 1])
    return f'<?xml version="1.0" encoding="UTF-8"?>\n<batch mzmine_version="3.9.0">\n{corpo}</batch>\n'


def extra(ap) -> None:
    ap.add_argument("--mzmine", default=os.environ.get("MZMINE_HOME", ""), help="folder with bin/MZmine")
    ap.add_argument("--java-home", default=os.environ.get("JAVA_HOME", ""))


def main() -> None:
    a = corri.argomenti(__doc__, extra)
    par = corri.parametri("mzmine", a.config)
    fs = corri.file_mzml(a.mzml_dir, a.solo)
    nat = corri.cartella_nativa(a.out_dir, "mzmine", a.config)
    uscita = nat / "feature_list.csv"
    for vecchio in nat.glob("feature_list*.csv"):
        vecchio.unlink()
    (nat / "batch.xml").write_text(batch_xml(fs, uscita, par), encoding="utf-8")
    env = dict(os.environ, JAVA_HOME=a.java_home, PATH=f"{a.java_home}/bin:{os.environ['PATH']}")
    r = subprocess.run([str(Path(a.mzmine) / "bin" / "MZmine"), "-b", str(nat / "batch.xml"), "-t", str(nat / "tmp")], capture_output=True, text=True, env=env)
    (nat / "mzmine.log").write_text(r.stdout + r.stderr, encoding="utf-8")
    if not uscita.exists():
        raise SystemExit(f"MZmine wrote no table (exit {r.returncode}):\n{(r.stderr or r.stdout)[-2000:]}")
    t = adatta.da_csv("mzmine", uscita, [f.stem for f in fs], a.dataset, a.config)
    print(corri.salva(t, a.out_dir, "3.9.0"), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
