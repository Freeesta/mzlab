"""A big synthetic profile mzML (zlib, 64-bit m/z, 32-bit intensity) for timing the readers: python3 tools/mzml_grande.py OUT SCANS POINTS. Invented numbers."""
import base64, sys, zlib
import numpy as np
out, n, p = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
rng = np.random.default_rng(7)
def arr(v, f64, acc, name):
    b = zlib.compress(np.asarray(v, dtype='<f8' if f64 else '<f4').tobytes(), 6)
    e = base64.b64encode(b).decode()
    return (f'<binaryDataArray encodedLength="{len(e)}"><cvParam cvRef="MS" accession="{"MS:1000523" if f64 else "MS:1000521"}" name="{"64" if f64 else "32"}-bit float"/>'
            f'<cvParam cvRef="MS" accession="MS:1000574" name="zlib compression"/><cvParam cvRef="MS" accession="{acc}" name="{name}"/><binary>{e}</binary></binaryDataArray>')
with open(out, "w") as f:
    f.write('<?xml version="1.0" encoding="utf-8"?>\n<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="PSI-MS" version="4" URI="x"/></cvList>'
            '<fileDescription><fileContent><cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum"/></fileContent></fileDescription>'
            '<softwareList count="1"><software id="s" version="1"><cvParam cvRef="MS" accession="MS:1000799" name="custom unreleased software tool" value="t"/></software></softwareList>'
            '<instrumentConfigurationList count="1"><instrumentConfiguration id="ic"><cvParam cvRef="MS" accession="MS:1000031" name="instrument model"/></instrumentConfiguration></instrumentConfigurationList>'
            '<dataProcessingList count="1"><dataProcessing id="dp"><processingMethod order="0" softwareRef="s"><cvParam cvRef="MS" accession="MS:1000544" name="Conversion to mzML"/></processingMethod></dataProcessing></dataProcessingList>'
            f'<run id="r" defaultInstrumentConfigurationRef="ic"><spectrumList count="{n}" defaultDataProcessingRef="dp">\n')
    grid = np.linspace(100, 1000, p)
    for i in range(n):
        mz = grid + rng.normal(0, 1e-4, p).cumsum() * 0 + i * 1e-3
        c = rng.uniform(100, 1000, 40)[:, None]
        it = (1e5 * np.exp(-((mz[None, :] - c) ** 2) / 0.02)).sum(0) + rng.gamma(2, 20, p)
        f.write(f'<spectrum index="{i}" id="scan={i+1}" defaultArrayLength="{p}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>'
                '<cvParam cvRef="MS" accession="MS:1000128" name="profile spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/>'
                f'<cvParam cvRef="MS" accession="MS:1000285" name="total ion current" value="{it.sum():.1f}"/>'
                f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{i*0.02:.4f}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>'
                f'<binaryDataArrayList count="2">{arr(mz,True,"MS:1000514","m/z array")}{arr(it,False,"MS:1000515","intensity array")}</binaryDataArrayList></spectrum>\n')
    f.write('</spectrumList></run></mzML>\n')
