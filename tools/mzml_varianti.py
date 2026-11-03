"""Synthetic mzML files, one per variant of the format seen in the wild (conformance matrix: docs/agenti/mzml-matrice.md). All numbers are invented.
    python3 tools/mzml_varianti.py OUT_DIR
Writes plain/indexed, 32/64-bit, raw/zlib, CRLF, UTF-8 BOM, gzip, MS3/MS4, profile, ion mobility, referenceableParamGroup, scanSettings, every kind of
chromatogram, and the hostile ones (truncated, empty, not XML, broken base64, absurd offsets). The tests of `mzlab-cli` (mzml_varianti_tests.rs) run them."""
import base64, gzip, os, re, sys, zlib

import numpy as np

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
def arr(v, f64, z, acc, name, unit=""):
    b = np.asarray(v, dtype='<f8' if f64 else '<f4').tobytes()
    if z: b = zlib.compress(b)
    return (f'<binaryDataArray encodedLength="{len(base64.b64encode(b))}"><cvParam cvRef="MS" accession="{"MS:1000523" if f64 else "MS:1000521"}" name="{"64" if f64 else "32"}-bit float"/>'
            f'<cvParam cvRef="MS" accession="{"MS:1000574" if z else "MS:1000576"}" name="{"zlib compression" if z else "no compression"}"/>'
            f'<cvParam cvRef="MS" accession="{acc}" name="{name}"/><binary>{base64.b64encode(b).decode()}</binary></binaryDataArray>')
def spec(i, lvl, f64, z, extra="", prof=False, prec=None, im=False, rpg=False):
    mz = np.linspace(100, 500, 40) + i*0.01; it = (np.arange(40)%7+1)*1000.0
    pre = ""
    if lvl > 1: pre = f'<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{prec or 300.5}"/></selectedIon></selectedIonList></precursor></precursorList>'
    arrays = arr(mz, f64, z, "MS:1000514", "m/z array") + arr(it, f64, z, "MS:1000515", "intensity array")
    n = 2
    if im: arrays += arr(np.linspace(0.8, 1.2, 40), f64, z, "MS:1002893", "mean ion mobility array"); n = 3
    ref = '<referenceableParamGroupRef ref="g1"/>' if rpg else (f'<cvParam cvRef="MS" accession="{"MS:1000128" if prof else "MS:1000127"}" name="x"/>')
    return (f'<spectrum index="{i}" id="scan={i+1}" defaultArrayLength="40">{ref}<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{lvl}"/>'
            f'<cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><cvParam cvRef="MS" accession="MS:1000285" name="total ion current" value="{it.sum()}"/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{i*0.1}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>'
            f'{pre}<binaryDataArrayList count="{n}">{arrays}</binaryDataArrayList></spectrum>')
def chrom(idx, kind, id_, f64, z, tacc="MS:1000595", yacc="MS:1000515", yname="intensity array", extra=""):
    t = np.arange(30)*0.1; y = np.abs(np.sin(t))*1000+10
    return (f'<chromatogram index="{idx}" id="{id_}" defaultArrayLength="30"><cvParam cvRef="MS" accession="{kind}" name="c"/>{extra}'
            f'<binaryDataArrayList count="2">{arr(t,f64,z,"MS:1000595","time array")}{arr(y,f64,z,yacc,yname)}</binaryDataArrayList></chromatogram>')
def doc(specs, chroms, rpg=False, scan_settings=False, nl="\n"):
    h = '<?xml version="1.0" encoding="utf-8"?>\n<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="PSI-MS" version="4.1.0" URI="x"/></cvList>'
    h += '<fileDescription><fileContent><cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum"/></fileContent></fileDescription>'
    if rpg: h += '<referenceableParamGroupList count="1"><referenceableParamGroup id="g1"><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/></referenceableParamGroup></referenceableParamGroupList>'
    h += '<softwareList count="1"><software id="s" version="1"><cvParam cvRef="MS" accession="MS:1000799" name="custom unreleased software tool" value="t"/></software></softwareList>'
    if scan_settings: h += '<scanSettingsList count="1"><scanSettings id="ss"><targetList count="1"><target><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="300"/></target></targetList></scanSettings></scanSettingsList>'
    h += '<instrumentConfigurationList count="1"><instrumentConfiguration id="ic"><cvParam cvRef="MS" accession="MS:1000031" name="instrument model"/></instrumentConfiguration></instrumentConfigurationList>'
    h += '<dataProcessingList count="1"><dataProcessing id="dp"><processingMethod order="0" softwareRef="s"><cvParam cvRef="MS" accession="MS:1000544" name="Conversion to mzML"/></processingMethod></dataProcessing></dataProcessingList>'
    h += f'<run id="r" defaultInstrumentConfigurationRef="ic"><spectrumList count="{len(specs)}" defaultDataProcessingRef="dp">' + "".join(specs) + '</spectrumList>'
    if chroms: h += f'<chromatogramList count="{len(chroms)}" defaultDataProcessingRef="dp">' + "".join(chroms) + '</chromatogramList>'
    h += '</run></mzML>'
    return h.replace("\n", nl) if nl != "\n" else h
def indexed(body):
    # body has no wrapper; add indexedmzML with correct offsets
    b = body.replace('<?xml version="1.0" encoding="utf-8"?>\n', '')
    head = '<?xml version="1.0" encoding="utf-8"?>\n<indexedmzML xmlns="http://psi.hupo.org/ms/mzml">'
    full = head + b
    offs = [(m.group(1), len(full[:m.start()].encode())) for m in re.finditer(r'<spectrum index="\d+" id="([^"]+)"', full)]
    tail = '</mzML>'
    full = full.replace(tail, tail)  # keep
    lo = len(full.encode()) + len('</mzML>'.encode())*0
    idx = '<indexList count="1"><index name="spectrum">' + "".join(f'<offset idRef="{i}">{o}</offset>' for i, o in offs) + '</index></indexList>'
    ilo = len((full).encode()) 
    return full + idx + f'<indexListOffset>{ilo}</indexListOffset><fileChecksum>0</fileChecksum></indexedmzML>'
def w(name, s, mode='w', enc='utf-8'):
    p = os.path.join(out, name)
    if isinstance(s, bytes): open(p, 'wb').write(s)
    else: open(p, 'w', encoding=enc, newline='').write(s)
S = lambda **k: [spec(i, 1 if i%3 else 1, **k) for i in range(6)]
base = dict(f64=True, z=False)
for f64 in (True, False):
    for z in (False, True):
        w(f'plain_{"64" if f64 else "32"}_{"z" if z else "raw"}.mzML', doc([spec(i,1,f64,z) for i in range(6)], [chrom(0,"MS:1000235","TIC",f64,z)]))
w('indexed.mzML', indexed(doc(S(**base), [chrom(0,"MS:1000235","TIC",True,False)])))
w('crlf.mzML', doc(S(**base), [], nl="\r\n"))
w('bom.mzML', b'\xef\xbb\xbf' + doc(S(**base), []).encode())
w('gz.mzML.gz', gzip.compress(doc(S(**base), []).encode()))
ms = [spec(0,1,True,True), spec(1,2,True,True), spec(2,3,True,True,prec=250.1), spec(3,4,True,True), spec(4,1,True,True)]
w('ms3_4.mzML', doc(ms, []))
w('profile.mzML', doc([spec(i,1,True,True,prof=True) for i in range(6)], []))
w('mobility.mzML', doc([spec(i,1,True,True,im=True) for i in range(6)], []))
w('rpg.mzML', doc([spec(i,1,True,True,rpg=True) for i in range(6)], [], rpg=True))
w('scansettings.mzML', doc(S(**base), [], scan_settings=True))
srm = '<precursor><isolationWindow><cvParam cvRef="MS" accession="MS:1000827" name="isolation window target m/z" value="300.5"/></isolationWindow></precursor>'
w('chroms_all.mzML', doc(S(f64=True,z=True), [
  chrom(0,"MS:1000235","TIC",True,True), chrom(1,"MS:1000628","BPC",True,True),
  chrom(2,"MS:1001472","SRM",True,True,extra=f'<precursor><isolationWindow><cvParam cvRef="MS" accession="MS:1000827" name="t" value="300.5"/></isolationWindow></precursor><product><isolationWindow><cvParam cvRef="MS" accession="MS:1000827" name="t" value="150.2"/></isolationWindow></product>'),
  chrom(3,"MS:1001472","SIM",True,True),
  chrom(4,"MS:1000812","absorption",True,True,yacc="MS:1000515",yname="intensity array")]))
w('ms3_only_idx.mzML', doc(ms[:3], []))
full = doc(S(**base), []).encode()
w('truncated.mzML', full[:len(full)//2])
w('empty.mzML', b'')
w('notxml.mzML', b'hello world')
w('badb64.mzML', doc(S(**base), []).replace('<binary>', '<binary>!!!', 1))
w('bigoffsets.mzML', indexed(doc(S(**base), [])).replace('<offset idRef="scan=1">', '<offset idRef="scan=1">9999999999', 1))
