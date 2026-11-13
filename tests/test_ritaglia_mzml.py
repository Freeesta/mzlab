import base64
import importlib.util
import re
import zlib
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("crop_mzml", ROOT / "tools" / "crop_mzml.py")
rit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rit)


def _b(a, dt):
    return base64.b64encode(zlib.compress(np.asarray(a, dtype=dt).tobytes())).decode()


def _spectrum(i, level, rt, parent=None):
    prec = f'<precursorList count="1"><precursor spectrumRef="{parent}"/></precursorList>' if parent else ""
    return f"""        <spectrum index="{i}" id="scan={i + 1}" defaultArrayLength="2">
          <cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{level}"/>
          <cvParam cvRef="MS" accession="MS:1000796" name="spectrum title" value="secret_sample.raw scan {i}"/>
          <scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>
          {prec}
          <binaryDataArrayList count="2">
            <binaryDataArray><cvParam name="64-bit float"/><cvParam name="zlib compression"/><cvParam name="m/z array"/><binary>{_b([100.0, 200.0], "<f8")}</binary></binaryDataArray>
            <binaryDataArray><cvParam name="32-bit float"/><cvParam name="zlib compression"/><cvParam name="intensity array"/><binary>{_b([5.0, 7.0], "<f4")}</binary></binaryDataArray>
          </binaryDataArrayList>
        </spectrum>
"""


def _file():
    sp = [_spectrum(0, 1, 1.0), _spectrum(1, 2, 1.1, "scan=1"), _spectrum(2, 1, 5.0), _spectrum(3, 2, 5.1, "scan=3"), _spectrum(4, 2, 5.2, "scan=3"), _spectrum(5, 1, 9.0)]
    return ('<?xml version="1.0"?>\n<indexedmzML><mzML id="secret_sample">\n<sourceFileList count="1"><sourceFile id="R" name="secret_sample.raw" location="file:///C:/priv"/></sourceFileList>\n'
            '<run id="secret_sample" startTimeStamp="2020-01-01T00:00:00Z" defaultSourceFileRef="R">\n      <spectrumList count="6" defaultDataProcessingRef="dp">\n'
            + "".join(sp) + "      </spectrumList>\n<chromatogramList count=\"1\"><chromatogram id=\"TIC\"/></chromatogramList>\n</run></mzML><indexList/></indexedmzML>")


def test_the_window_keeps_the_scans_inside_with_their_parent_and_is_anonymous():
    out = rit.crop(_file(), 4.0, 6.0, "HRMS_x")
    assert re.search(r'<spectrumList count="3"', out)
    assert out.count("<spectrum ") == 3
    assert "scan=3" in out and "scan=4" in out and "scan=5" in out and "scan=1\"" not in out
    for s in ("secret", "raw", "C:/priv", "chromatogram", "indexList", "startTimeStamp", "spectrum title"):
        assert s not in out, s
    assert 'id="HRMS_x"' in out
    assert [int(i) for i in re.findall(r'<spectrum index="(\d+)"', out)] == [0, 1, 2]


def test_an_ms2_in_the_window_brings_its_ms1_parent_from_before_the_window():
    out = rit.crop(_file(), 5.05, 5.3, "x")
    assert out.count("<spectrum ") == 3 and 'id="scan=3"' in out
