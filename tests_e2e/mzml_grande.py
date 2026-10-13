"""Big synthetic centroid mzML (Orbitrap-like DDA) for the tests of loading many large files together: random peaks (zlib does not shrink them), no real data.
write(path, n_ms1, n_ms2, peaks, seed): about 6.5 KB per scan with 420 peaks."""
import sys, zlib, base64, struct
import numpy as np
def arr(a):
    return base64.b64encode(zlib.compress(a.astype('<f8').tobytes(), 1)).decode()
def spec(i, level, rt, mz, it, prec=None):
    s = f'''<spectrum index="{i}" id="controllerType=0 controllerNumber=1 scan={i+1}" defaultArrayLength="{len(mz)}">
<cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum" value=""/>
<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{level}"/>
<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>
<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum" value=""/>
<cvParam cvRef="MS" accession="MS:1000285" name="total ion current" value="{it.sum():.1f}"/>
<scanList count="1"><cvParam cvRef="MS" accession="MS:1000795" name="no combination" value=""/><scan>
<cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt:.6f}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/>
<cvParam cvRef="MS" accession="MS:1000800" name="mass resolving power" value="60000"/>
<cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="FTMS + p ESI Full ms{'' if level==1 else '2 @hcd30.00'} [50.0000-550.0000]"/>
</scan></scanList>
'''
    if prec:
        s += f'<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{prec:.5f}"/></selectedIon></selectedIonList><activation><cvParam cvRef="MS" accession="MS:1000133" name="collision-induced dissociation" value=""/><cvParam cvRef="MS" accession="MS:1000045" name="collision energy" value="30"/></activation></precursor></precursorList>\n'
    s += f'''<binaryDataArrayList count="2">
<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/><cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/><cvParam cvRef="MS" accession="MS:1000514" name="m/z array" value="" unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"/><binary>{arr(mz)}</binary></binaryDataArray>
<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/><cvParam cvRef="MS" accession="MS:1000574" name="zlib compression" value=""/><cvParam cvRef="MS" accession="MS:1000515" name="intensity array" value="" unitCvRef="MS" unitAccession="MS:1000131" unitName="number of detector counts"/><binary>{arr(it)}</binary></binaryDataArray>
</binaryDataArrayList>
</spectrum>
'''
    return s
def write(path, n_ms1=1500, n_ms2=14500, peaks=420, seed=1, minutes=25.0):
    rng = np.random.default_rng(seed); n = n_ms1 + n_ms2; per = n_ms2 // n_ms1
    head = f'''<?xml version="1.0" encoding="utf-8"?>
<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">
<cvList count="2"><cv id="MS" fullName="PSI-MS" version="4.1.0" URI="x"/><cv id="UO" fullName="UO" version="1" URI="x"/></cvList>
<fileDescription><fileContent><cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum" value=""/></fileContent></fileDescription>
<softwareList count="1"><software id="pwiz" version="1"><cvParam cvRef="MS" accession="MS:1000615" name="ProteoWizard software" value=""/></software></softwareList>
<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1"><cvParam cvRef="MS" accession="MS:1002732" name="Orbitrap Fusion" value=""/></instrumentConfiguration></instrumentConfigurationList>
<dataProcessingList count="1"><dataProcessing id="dp"><processingMethod order="0" softwareRef="pwiz"><cvParam cvRef="MS" accession="MS:1000544" name="Conversion to mzML" value=""/></processingMethod></dataProcessing></dataProcessingList>
<run id="synth" defaultInstrumentConfigurationRef="IC1">
<spectrumList count="{n}" defaultDataProcessingRef="dp">
'''
    with open(path, 'w', encoding='utf-8') as f:
        f.write(head); i = 0
        for k in range(n_ms1):
            rt = minutes * i / n
            mz = np.sort(rng.uniform(50, 550, peaks)); it = rng.uniform(1e3, 1e6, peaks)
            f.write(spec(i, 1, rt, mz, it)); i += 1
            for _ in range(per):
                m2 = np.sort(rng.uniform(50, 500, peaks // 2)); i2 = rng.uniform(1e2, 1e5, peaks // 2)
                f.write(spec(i, 2, minutes * i / n, m2, i2, prec=float(mz[rng.integers(peaks)]))); i += 1
        f.write('</spectrumList>\n</run>\n</mzML>\n')
