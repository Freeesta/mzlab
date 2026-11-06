"""Synthetic files of the formats the «aperti» and «andi» readers take (no laboratory data, invented numbers): an mzML with MS-Numpress arrays or its zlib twin,
an mzXML with its index, an ANDI-MS .cdf. Used by tests_e2e/e2e_lettori_modulo.py (site) and crates/mzlab-desktop/tests/e2e_shim.py (desktop)."""
import base64, math, struct, zlib
from pathlib import Path

def nib(x):   # one integer of the numpress nibble code (halves, least significant first)
    x &= 0xffffffff; top = x & 0xf0000000
    if top == 0:
        l = next((i for i in range(8) if x & (0xf0000000 >> (4 * i))), 8)
        return [l] + [(x >> (4 * (i - l))) & 0xf for i in range(l, 8)]
    if top == 0xf0000000:
        l = next((i for i in range(8) if (x & (0xf0000000 >> (4 * i))) != (0xf0000000 >> (4 * i))), 7)
        return [l + 8] + [(x >> (4 * (i - l))) & 0xf for i in range(l, 8)]
    return [0] + [(x >> (4 * i)) & 0xf for i in range(8)]
def np_linear(v, fp):
    q = [int(x * fp + 0.5) for x in v]; out = struct.pack(">d", fp) + b"".join(struct.pack("<I", x & 0xffffffff) for x in q[:2]); h = []
    for i in range(2, len(q)): h += nib(q[i] - (q[i - 1] + (q[i - 1] - q[i - 2])))
    return out + bytes(((h[i] << 4) | (h[i + 1] if i + 1 < len(h) else 8)) for i in range(0, len(h), 2))
def np_slof(v, fp): return struct.pack(">d", fp) + b"".join(struct.pack("<H", int(math.log(x + 1) * fp + 0.5)) for x in v)
def b64(b): return base64.b64encode(b).decode()
def mzml_file(path, numpress, n=40):
    """n MS1 spectra; the arrays are numpress (linear + slof, zlib on top for the old accessions) or the same values as zlib-compressed doubles; the intensities are rounded to what slof keeps so both read alike"""
    def cv(a, nm): return f'<cvParam cvRef="MS" accession="{a}" name="{nm}" value=""/>'
    out = ['<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><cvList count="1"><cv id="MS" fullName="PSI-MS" version="4.1.0" URI="x"/></cvList>',
           '<fileDescription><fileContent>' + cv("MS:1000579", "MS1 spectrum") + '</fileContent></fileDescription>',
           '<instrumentConfigurationList count="1"><instrumentConfiguration id="IC1">' + cv("MS:1000031", "instrument model") + '</instrumentConfiguration></instrumentConfigurationList>',
           f'<run id="r" defaultInstrumentConfigurationRef="IC1"><spectrumList count="{n}" defaultDataProcessingRef="dp">']
    for k in range(n):
        mz = [100 + 0.37 * i + 0.001 * (i % 7) for i in range(30)]; it = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        tic = sum(it)
        if numpress:
            a_mz = b64(np_linear(mz, 1e7)); a_it = b64(np_slof(it, 1000.0))
            mzc = cv("MS:1002312", "MS-Numpress linear prediction compression") + cv("MS:1000523", "64-bit float")
            itc = cv("MS:1002314", "MS-Numpress short logged float compression") + cv("MS:1000523", "64-bit float")
        else:
            fp = 1000.0; it = [math.exp(int(math.log(x + 1) * fp + 0.5) / fp) - 1 for x in it]
            mz = [round(x * 1e7) / 1e7 for x in mz]
            a_mz = b64(zlib.compress(struct.pack("<30d", *mz))); a_it = b64(zlib.compress(struct.pack("<30d", *it)))
            mzc = cv("MS:1000574", "zlib compression") + cv("MS:1000523", "64-bit float"); itc = mzc
        out.append(f'<spectrum index="{k}" id="scan={k + 1}" defaultArrayLength="30">' + cv("MS:1000511", "ms level").replace('value=""', 'value="1"')
                   + cv("MS:1000130", "positive scan") + cv("MS:1000128", "profile spectrum")
                   + '<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="%.4f" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>' % (k * 0.05)
                   + '<binaryDataArrayList count="2"><binaryDataArray encodedLength="0">' + mzc + cv("MS:1000514", "m/z array") + f'<binary>{a_mz}</binary></binaryDataArray>'
                   + '<binaryDataArray encodedLength="0">' + itc + cv("MS:1000515", "intensity array") + f'<binary>{a_it}</binary></binaryDataArray></binaryDataArrayList></spectrum>')
    out.append('</spectrumList></run></mzML>')
    Path(path).write_text("".join(out), encoding="utf-8")

def mzxml_file(path, n=40):
    """n MS1 scans, 64-bit big-endian peaks in zlib, with the <index> at the end; returns (rt in minutes, TIC) of each"""
    body = ['<?xml version="1.0" encoding="ISO-8859-1"?>\n<mzXML xmlns="http://sashimi.sourceforge.net/schema_revision/mzXML_3.2"><msRun scanCount="%d" startTime="PT0S" endTime="PT%dS">\n' % (n, 3 * n),
            '<parentFile fileName="campione.raw" fileType="RAWData" fileSha1="0123456789abcdef0123456789abcdef01234567"/><msInstrument msInstrumentID="1"><msManufacturer category="msManufacturer" value="Test"/><msModel category="msModel" value="Modello X"/></msInstrument>\n']
    pos = []; tics = []; text = body[0] + body[1]
    for k in range(n):
        mz = [100 + 0.37 * i + 0.001 * (i % 7) for i in range(30)]; it = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        raw = b"".join(struct.pack(">dd", a, b) for a, b in zip(mz, it)); z = zlib.compress(raw); tics.append(sum(it))
        pos.append(len(text.encode("latin-1")))
        text += ('<scan num="%d" msLevel="1" peaksCount="30" polarity="+" scanType="Full" centroided="0" retentionTime="PT%.1fS" totIonCurrent="%r">' % (k + 1, 3.0 * k, sum(it))
                 + '<peaks precision="64" byteOrder="network" contentType="m/z-int" compressionType="zlib" compressedLen="%d">%s</peaks></scan>\n' % (len(z), b64(z)))
    text += "</msRun>\n"; ix = len(text.encode("latin-1"))
    text += '<index name="scan">' + "".join('<offset id="%d">%d</offset>' % (i + 1, o) for i, o in enumerate(pos)) + '</index>\n<indexOffset>%d</indexOffset>\n</mzXML>\n' % ix
    Path(path).write_bytes(text.encode("latin-1")); return tics

def cdf_file(path, n=40):
    """an ANDI-MS file (netCDF-3 classic) of n scans of 30 points, float m/z and intensities as record variables; returns the TIC of each scan"""
    def pad(b): return b + b"\0" * (-len(b) % 4)
    def nm(t): return struct.pack(">I", len(t)) + pad(t.encode())
    def atts(a):
        if not a: return struct.pack(">II", 0, 0)
        return struct.pack(">II", 0x0C, len(a)) + b"".join(nm(k) + struct.pack(">II", 2, len(v)) + pad(v.encode()) for k, v in a)
    tics = []; mz = []; it = []
    for k in range(n):
        y = [1000 * (1 + math.sin(i + k)) ** 2 + 50 for i in range(30)]
        y = [struct.unpack(">f", struct.pack(">f", v))[0] for v in y]
        mz += [100 + 0.37 * i for i in range(30)]; it += y; tics.append(sum(y))
    dims = [("point_number", 0), ("scan_number", n)]
    # (name, dim ids, nc_type, data bytes, record?)
    dbl = lambda v: b"".join(struct.pack(">d", x) for x in v); ints = lambda v: b"".join(struct.pack(">i", x) for x in v)
    flt = lambda v: b"".join(struct.pack(">f", x) for x in v)
    fixed = [("scan_acquisition_time", 6, dbl([k * 3.0 for k in range(n)])), ("total_intensity", 6, dbl(tics)),
             ("scan_index", 4, ints([30 * k for k in range(n)])), ("point_count", 4, ints([30] * n))]
    rec = [("mass_values", flt(mz)), ("intensity_values", flt(it))]
    def header(begins):
        h = b"CDF\x01" + struct.pack(">I", 30 * n) + struct.pack(">II", 0x0A, 2) + b"".join(nm(d) + struct.pack(">I", l) for d, l in dims)
        h += atts([("experiment_type", "Centroided Mass Spectrum"), ("test_ionization_polarity", "Positive Polarity")])
        h += struct.pack(">II", 0x0B, len(fixed) + len(rec))
        for (name, ty, data), b in zip(fixed, begins):
            h += nm(name) + struct.pack(">II", 1, 1) + atts([]) + struct.pack(">III", ty, len(data), b)
        for (name, data), b in zip(rec, begins[len(fixed):]):
            h += nm(name) + struct.pack(">II", 1, 0) + atts([]) + struct.pack(">III", 5, 4, b)
        return h
    pos = len(header([0] * 6)); begins = []
    for _, _, data in fixed: begins.append(pos); pos += len(data)
    begins += [pos, pos + 4]
    out = header(begins) + b"".join(d for _, _, d in fixed)
    for r in range(30 * n): out += rec[0][1][4 * r:4 * r + 4] + rec[1][1][4 * r:4 * r + 4]
    Path(path).write_bytes(out); return tics
