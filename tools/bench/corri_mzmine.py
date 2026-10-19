"""MZmine 3.9 (headless batch): mzML import, centroid mass detection, ADAP chromatograms, local-minimum resolving, join alignment, gap filling, CSV export -> `bench/1`.
See `corri.py`. MZmine needs an exact OpenJDK 20: `--mzmine DIR` (the `bin/MZmine` of the bioconda package) and `--java-home DIR` or `MZMINE_HOME`/`JAVA_HOME`."""
from __future__ import annotations

import hashlib
import os
import re
import subprocess
import sys
import tarfile
import urllib.request
from pathlib import Path
from xml.sax.saxutils import escape

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import adatta, corri  # noqa: E402

BIOCONDA_URL = "https://conda.anaconda.org/bioconda/noarch/mzmine-3.9.0-hdfd78af_0.tar.bz2"
BIOCONDA_SHA256 = "d596078684c7e23af7c7e350dded437ecbf6b2851aadc856c318c3cac2645504"
JDK20_URL = "https://api.adoptium.net/v3/binary/latest/20/ga/linux/x64/jdk/hotspot/normal/eclipse"
CASA_39 = Path(os.environ.get("MZMINE39_DIR") or Path.home() / ".cache" / "mzlab" / "mzmine39")

M = "io.github.mzmine.modules."
FL = '<parameter name="Feature lists" type="BATCH_LAST_FEATURELISTS"/>'
RAW = '<parameter name="Raw data files" type="BATCH_LAST_FILES"/>'


def _scarica(url: str, dest: Path, sha256: str | None = None) -> None:
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "mzlab-bench"}), timeout=600) as r, open(dest, "wb") as f:
        while blocco := r.read(1 << 20):
            f.write(blocco)
    if sha256 and hashlib.sha256(dest.read_bytes()).hexdigest() != sha256:
        dest.unlink()
        raise SystemExit(f"sha256 of {url} differs from {sha256}")


def installa_39(cartella: Path | str = CASA_39) -> tuple[Path, Path]:
    """(mzmine folder with `bin/MZmine`, JAVA_HOME): mzmine 3.9.0 from the bioconda package (sha256 checked) and the OpenJDK 20 of Adoptium, in `cartella`.
    Nothing is downloaded when both are already there; the archives are deleted after the extraction to spare the disk."""
    cartella = Path(cartella)
    mm, jdk = cartella / "mzmine", cartella / "jdk"
    cartella.mkdir(parents=True, exist_ok=True)
    if not (mm / "bin" / "MZmine").exists():
        tar = cartella / "mzmine.tar.bz2"
        _scarica(BIOCONDA_URL, tar, BIOCONDA_SHA256)
        with tarfile.open(tar, "r:bz2") as t:
            t.extractall(mm, filter="data")
        tar.unlink()
        (mm / "bin" / "MZmine").chmod(0o755)
    if not list(jdk.glob("*/bin/java")):
        tar = cartella / "jdk20.tar.gz"
        _scarica(JDK20_URL, tar)
        with tarfile.open(tar, "r:gz") as t:
            t.extractall(jdk, filter="data")
        tar.unlink()
    return mm, next(jdk.glob("*/bin/java")).parents[1]


def _tolleranza_mz(nome: str, comuni: bytes) -> bool:
    """`m/z tolerance (<title>)`: the name is built from the title of a `ToleranceType`."""
    m = re.fullmatch(r"m/z tolerance \((.+)\)", nome)
    return bool(m) and m.group(1).encode() in comuni


def parametri_non_letti(xml: Path | str, mzmine: Path | str) -> list[str]:
    """The parameters of a batch that mzmine would ignore: mzmine reads the batch without a word for a parameter whose name is not in the parameter classes of the module
    (the log never lists the values it applied), so the names of every step are searched in the classes of the package of its module and of the packages below it.
    One line `<module>: <name>` per unknown name; an empty list = every name exists."""
    import zipfile
    from xml.etree import ElementTree as ET
    lib = Path(mzmine) / "lib"
    jar = zipfile.ZipFile(next(iter(lib.glob("work-*.jar")), None) or next((lib / "app").glob("mzmine-community-*.jar")))      # 3.9: work-*.jar; 4.x: app/mzmine-community-*.jar
    classi = [n for n in jar.namelist() if n.endswith("Parameters.class")]
    comuni = b"".join(jar.read(n) for n in jar.namelist() if n.startswith("io/github/mzmine/parameters/") and n.endswith(".class"))      # names of the shared parameter types
    out = []
    for passo in ET.parse(xml).getroot().iter("batchstep"):
        modulo = passo.get("method", "")
        pacchetto = modulo.replace(".", "/").rsplit("/", 1)[0] + "/"
        testo = comuni + b"".join(jar.read(n) for n in classi if n.startswith(pacchetto) or pacchetto.startswith(n.rsplit("/", 1)[0] + "/"))      # the package and the ones above it
        for p in passo.iter("parameter"):
            nome = p.get("name", "")
            if nome and nome.encode() not in testo and not _tolleranza_mz(nome, comuni):
                out.append(f"{modulo.rsplit('.', 1)[-1]}: {nome}")
    return out


def mz_tol(nome: str, ass: float, ppm: float) -> str:
    return f'<parameter name="{nome}"><absolutetolerance>{ass}</absolutetolerance><ppmtolerance>{ppm}</ppmtolerance></parameter>'


def batch_xml(files: list[Path], uscita: Path, par: dict, fasi: Path | None = None, isotopi: bool = False) -> str:
    """The batch. `fasi`: a folder where the feature lists are also exported after every stage (`<stage>_{}.csv`, one per list). `isotopi`: stop after the isotope grouper."""
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
         f'<parameter name="Min # of data points">{r["min_data_points"]}</parameter>'
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
    if fasi is not None or isotopi:
        passi = _con_fasi(passi, par, fasi, isotopi)
    corpo = "".join(f'<batchstep method="{m}" parameter_version="{v}">{p}</batchstep>\n' for m, p in passi for v in [2 if 'MinimumSearch' in m else 1])
    return f'<?xml version="1.0" encoding="UTF-8"?>\n<batch mzmine_version="3.9.0">\n{corpo}</batch>\n'


def _esporta(percorso: str) -> tuple[str, str]:
    return (M + "io.export_features_csv.CSVExportModularModule",
            FL + f'<parameter name="Filename"><current_file>{escape(percorso)}</current_file></parameter>'
            '<parameter name="Field separator">,</parameter><parameter name="Identification separator">;</parameter>'
            '<parameter name="Remove empty columns">false</parameter><parameter name="Filter rows">ALL</parameter>')


def _con_fasi(passi: list, par: dict, fasi: Path | None, isotopi: bool) -> list:
    """Adds the exports after every stage (and, for `isotopi`, the isotope grouper after the resolver and nothing after it)."""
    nomi = ["import", "massa", "adap", "risolutore", "join", "riempimento", "csv"]
    out = []
    for nome, passo in zip(nomi, passi):
        if nome in ("join", "riempimento") and isotopi:
            continue
        if nome == "csv":
            if isotopi:
                out.append((M + "dataprocessing.filter_isotopegrouper.IsotopeGrouperModule", FL + '<parameter name="Name suffix">deisotoped</parameter>'
                            + mz_tol("m/z tolerance (intra-sample)", 0.0015, 5.0) + '<parameter name="Retention time tolerance" unit="MINUTES">0.05</parameter>'
                            '<parameter name="Mobility tolerance" selected="false"/><parameter name="Monotonic shape">false</parameter>'
                            '<parameter name="Maximum charge">2</parameter><parameter name="Representative isotope">Most intense</parameter>'
                            '<parameter name="Never remove feature with MS2">false</parameter><parameter name="Original feature list">REMOVE</parameter>'))
                out.append(_esporta(str(fasi / "isotopi_{}.csv")))
            if fasi is None:
                out.append(passo)
            continue
        out.append(passo)
        if fasi is not None and nome in ("adap", "risolutore", "join", "riempimento") and not (isotopi and nome == "adap"):
            out.append(_esporta(str(fasi / f"{nome}_{{}}.csv")))
    return out


def esegui_batch(xml: Path, mzmine: Path | str, java_home: Path | str, tmp: Path) -> subprocess.CompletedProcess:
    """`bin/MZmine -b xml` without interface; the log (stdout and stderr) goes next to the batch as `<xml>.log`."""
    env = dict(os.environ, JAVA_HOME=str(java_home), PATH=f"{java_home}/bin:{os.environ['PATH']}")
    r = subprocess.run([str(Path(mzmine) / "bin" / "MZmine"), "-b", str(xml), "-t", str(tmp)], capture_output=True, text=True, env=env)
    xml.with_suffix(".log").write_text(r.stdout + r.stderr, encoding="utf-8")
    return r


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
    mm, jh = (Path(a.mzmine), Path(a.java_home)) if a.mzmine and a.java_home else installa_39()
    r = esegui_batch(nat / "batch.xml", mm, jh, nat / "tmp")
    if not uscita.exists():
        raise SystemExit(f"MZmine wrote no table (exit {r.returncode}):\n{(r.stderr or r.stdout)[-2000:]}")
    t = adatta.da_csv("mzmine", uscita, [f.stem for f in fs], a.dataset, a.config)
    print(corri.salva(t, a.out_dir, "3.9.0"), len(t["righe"]), "rows")


if __name__ == "__main__":
    main()
