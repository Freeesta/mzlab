"""A synthetic mzML written the way msconvert writes a Thermo DDA run (indexedmzML wrapper with the index of the offsets and the SHA-1, an id that is not the file name, a sample with an empty name and an
id that XML escapes, an Orbitrap Exploris 120, centroid MS1 and MS2 with zlib arrays). All numbers are invented. For the tests of the loading errors of the big mzML files:
    python3 tools/mzml_msconvert_sintetico.py OUT.mzML [--scans N] [--peaks1 N] [--peaks2 N] [--variant NAME]
`variante` changes one thing at a time (see VARIANTI) so that a failing file points to the cause."""
import argparse
import base64
import hashlib
import zlib
from pathlib import Path

import numpy as np

VARIANTI = {
    "base": "the file as msconvert writes it: wrapper, SHA-1, a run id that is not the file name, sample `_x0038_` with an empty name",
    "senza_wrapper": "no indexedmzML wrapper (a plain mzML)",
    "indice_falso": "the wrapper whose offsets point to the wrong places",
    "senza_checksum": "the wrapper without the SHA-1 of the file",
    "sample_normale": "a sample with a normal id and name",
    "accessione_ignota": "a cvParam with an accession that the program does not know",
    "location_speciale": "the location of the source file with spaces, an escaped ampersand and accents, as a Windows path can have",
    "id_come_file": "the id of the run equals the file name",
}

NS = "http://psi.hupo.org/ms/mzml"
UN_MZ = ' unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"'
UN_MIN = ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"'


def _cv(acc: str, name: str, value="", unit: str = "") -> str:
    return f'<cvParam cvRef="MS" accession="{acc}" name="{name}" value="{value}"{unit}/>'


def _array(values: np.ndarray, f64: bool, acc: str, name: str) -> str:
    raw = np.asarray(values, dtype="<f8" if f64 else "<f4").tobytes()
    b64 = base64.b64encode(zlib.compress(raw, 1)).decode()
    bits = _cv("MS:1000523", "64-bit float") if f64 else _cv("MS:1000521", "32-bit float")
    unit = UN_MZ if f64 else ""
    return f'<binaryDataArray encodedLength="{len(b64)}">{bits}{_cv("MS:1000574", "zlib compression")}{_cv(acc, name, "", unit)}<binary>{b64}</binary></binaryDataArray>'


def _spettro(i: int, rng, peaks1: int, peaks2: int, ms1_ogni: int, strano: bool) -> tuple[str, str]:
    """(id, xml) of the spectrum `i`: an MS1 every `ms1_ogni` scans, MS2 of the strongest ions of the last MS1 for the others."""
    livello = 1 if i % ms1_ogni == 0 else 2
    n = peaks1 if livello == 1 else peaks2
    mz = np.sort(rng.uniform(100, 1000, n)) if livello == 1 else np.sort(rng.uniform(50, 600, n))
    it = rng.lognormal(10, 1.5, n).astype(np.float32)
    sid = f"controllerType=0 controllerNumber=1 scan={i + 1}"
    rt = 0.0005 * i
    prec = ""
    if livello == 2:
        p = 150 + (i * 7919) % 700
        prec = (f'<precursorList count="1"><precursor spectrumRef="controllerType=0 controllerNumber=1 scan={i - (i % ms1_ogni) + 1}"><isolationWindow>{_cv("MS:1000827", "isolation window target m/z", p, UN_MZ)}</isolationWindow>'
                f'<selectedIonList count="1"><selectedIon>{_cv("MS:1000744", "selected ion m/z", p, UN_MZ)}{_cv("MS:1000041", "charge state", 1)}</selectedIon></selectedIonList>'
                f'<activation>{_cv("MS:1000422", "beam-type collision-induced dissociation")}{_cv("MS:1000045", "collision energy", 30)}</activation></precursor></precursorList>')
    extra = _cv("MS:9999999", "an accession from a newer ontology", 1) if strano else ""
    xml = (f'<spectrum index="{i}" id="{sid}" defaultArrayLength="{n}">'
           + _cv("MS:1000511", "ms level", livello) + (_cv("MS:1000579", "MS1 spectrum") if livello == 1 else _cv("MS:1000580", "MSn spectrum")) + _cv("MS:1000127", "centroid spectrum")
           + _cv("MS:1000130", "positive scan") + _cv("MS:1000504", "base peak m/z", f"{mz[int(np.argmax(it))]:.5f}") + _cv("MS:1000505", "base peak intensity", f"{float(it.max()):.4f}")
           + _cv("MS:1000285", "total ion current", f"{float(it.sum()):.4f}") + extra
           + f'<scanList count="1">{_cv("MS:1000795", "no combination")}<scan>{_cv("MS:1000016", "scan start time", f"{rt:.6f}", UN_MIN)}'
           + _cv("MS:1000512", "filter string", f"FTMS + p NSI Full ms{'' if livello == 1 else '2 %d.0000@hcd30.00' % (150 + (i * 7919) % 700)} [100.0000-1000.0000]")
           + f'<scanWindowList count="1"><scanWindow>{_cv("MS:1000501", "scan window lower limit", 100)}{_cv("MS:1000500", "scan window upper limit", 1000)}</scanWindow></scanWindowList></scan></scanList>'
           + prec + f'<binaryDataArrayList count="2">{_array(mz, True, "MS:1000514", "m/z array")}{_array(it, False, "MS:1000515", "intensity array")}</binaryDataArrayList></spectrum>\n')
    return sid, xml


def scrivi(path, scans: int = 2000, peaks1: int = 4000, peaks2: int = 400, variante: str = "base", seed: int = 1, ms1_ogni: int = 6) -> int:
    """Writes the file and returns its size in bytes."""
    path = Path(path)
    rng = np.random.default_rng(seed)
    run_id = path.stem if variante == "id_come_file" else "Run_sintetico_ddaPOS"
    sample = '<sample id="1" name="s"/>' if variante == "sample_normale" else '<sample id="_x0038_" name=""/>'
    sample_ref = "1" if variante == "sample_normale" else "_x0038_"
    loc = "file:///D:/Dati%20Lab/Progetto A&amp;B/Perché strano" if variante == "location_speciale" else "file:///invented"
    avvolto = variante not in ("senza_wrapper",)
    testa = ('<?xml version="1.0" encoding="utf-8"?>\n'
             + (f'<indexedmzML xmlns="{NS}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="{NS} http://psidev.info/files/ms/mzML/xsd/mzML1.1.2_idx.xsd">\n' if avvolto else "")
             + f'<mzML xmlns="{NS}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="{NS} http://psidev.info/files/ms/mzML/xsd/mzML1.1.0.xsd" id="{run_id}" version="1.1.0">\n'
             '<cvList count="2"><cv id="MS" fullName="Proteomics Standards Initiative Mass Spectrometry Ontology" version="4.1.163" URI="https://raw.githubusercontent.com/HUPO-PSI/psi-ms-CV/master/psi-ms.obo"/>'
             '<cv id="UO" fullName="Unit Ontology" version="09:04:2014" URI="https://raw.githubusercontent.com/bio-ontology-research-group/unit-ontology/master/unit.obo"/></cvList>\n'
             '<fileDescription><fileContent>' + _cv("MS:1000579", "MS1 spectrum") + _cv("MS:1000580", "MSn spectrum") + _cv("MS:1000127", "centroid spectrum") + '</fileContent>'
             f'<sourceFileList count="1"><sourceFile id="RAW1" name="sintetico.raw" location="{loc}">' + _cv("MS:1000768", "Thermo nativeID format") + _cv("MS:1000563", "Thermo RAW format") + _cv("MS:1000569", "SHA-1", "0" * 40) + '</sourceFile></sourceFileList></fileDescription>\n'
             f'<sampleList count="1">{sample}</sampleList>\n'
             '<softwareList count="2"><software id="Xcalibur" version="5.0">' + _cv("MS:1000532", "Xcalibur") + '</software><software id="pwiz" version="3.0.24288">' + _cv("MS:1000615", "ProteoWizard software") + '</software></softwareList>\n'
             '<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1">' + _cv("MS:1003095", "Orbitrap Exploris 120") + _cv("MS:1000529", "instrument serial number", "INVENTED1")
             + '<componentList count="3"><source order="1">' + _cv("MS:1000073", "electrospray ionization") + '</source><analyzer order="2">' + _cv("MS:1000484", "orbitrap") + '</analyzer><detector order="3">' + _cv("MS:1000624", "inductive detector") + '</detector></componentList>'
             '<softwareRef ref="Xcalibur"/></instrumentConfiguration></instrumentConfigurationList>\n'
             '<dataProcessingList count="1"><dataProcessing id="pwiz_Reader_Thermo_conversion"><processingMethod order="0" softwareRef="pwiz">' + _cv("MS:1000544", "Conversion to mzML") + '</processingMethod></dataProcessing></dataProcessingList>\n'
             f'<run id="{run_id}" defaultInstrumentConfigurationRef="IC1" defaultSourceFileRef="RAW1" sampleRef="{sample_ref}" startTimeStamp="2026-01-01T00:00:00Z">\n'
             f'<spectrumList count="{scans}" defaultDataProcessingRef="pwiz_Reader_Thermo_conversion">\n')
    chi = '</spectrumList>\n<chromatogramList count="1" defaultDataProcessingRef="pwiz_Reader_Thermo_conversion">\n'
    coda_run = '</chromatogramList>\n</run>\n</mzML>\n'
    pezzi = [testa.encode()]
    pos = len(pezzi[0])
    offsets = []
    for i in range(scans):
        sid, xml = _spettro(i, rng, peaks1, peaks2, ms1_ogni, variante == "accessione_ignota" and i == scans // 2)
        offsets.append((sid, pos + xml.index("<spectrum ")))
        b = xml.encode(); pezzi.append(b); pos += len(b)
    t = np.arange(0, scans, 6) * 0.0005
    tic = (f'<chromatogram index="0" id="TIC" defaultArrayLength="{len(t)}">' + _cv("MS:1000235", "total ion current chromatogram")
           + f'<binaryDataArrayList count="2">{_array(t, True, "MS:1000595", "time array")}{_array(rng.lognormal(18, 0.3, len(t)), True, "MS:1000515", "intensity array")}</binaryDataArrayList></chromatogram>\n')
    mezzo = (chi + tic + coda_run).encode()
    chrom_off = pos + len(chi.encode()) + tic.index("<chromatogram ")
    pezzi.append(mezzo); pos += len(mezzo)
    if avvolto:
        idx_off = pos + (7 if variante == "indice_falso" else 0)
        lista = ('<indexList count="2"><index name="spectrum">' + "".join(f'<offset idRef="{s}">{o + (13 if variante == "indice_falso" else 0)}</offset>' for s, o in offsets)
                 + f'</index><index name="chromatogram"><offset idRef="TIC">{chrom_off}</offset></index></indexList>\n<indexListOffset>{idx_off}</indexListOffset>\n')
        cuerpo = b"".join(pezzi) + lista.encode()
        h = hashlib.sha1(cuerpo + b"<fileChecksum>").hexdigest()
        fine = b"" if variante == "senza_checksum" else f"<fileChecksum>{h}</fileChecksum>\n".encode()
        data = cuerpo + (fine if variante != "senza_checksum" else b"") + b"</indexedmzML>\n"
        path.write_bytes(data)
        return len(data)
    data = b"".join(pezzi)
    path.write_bytes(data)
    return len(data)


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("out")
    ap.add_argument("--scans", type=int, default=2000)
    ap.add_argument("--peaks1", type=int, default=4000)
    ap.add_argument("--peaks2", type=int, default=400)
    ap.add_argument("--variant", default="base", choices=sorted(VARIANTI))
    a = ap.parse_args()
    print(scrivi(a.out, a.scans, a.peaks1, a.peaks2, a.variant), "bytes")
