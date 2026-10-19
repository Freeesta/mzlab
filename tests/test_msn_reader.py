"""WP0: the reader keeps the whole fragmentation path and every <precursor> of an MSn scan (synthetic files; the DDA files stay as they were)."""
import base64
import glob
import re
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from dati_percorsi import percorso  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def _arr(name, acc, v):
    b = base64.b64encode(np.asarray(v, "<f8").tobytes()).decode()
    return (f'<binaryDataArray encodedLength="{len(b)}"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>'
            f'<cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{acc}" name="{name}"/>'
            f'<binary>{b}</binary></binaryDataArray>')


def _prec(target, ref=None, ce=None, act="MS:1000133", charge=None, half=2.5, selected=None):
    r = f' spectrumRef="{ref}"' if ref else ""
    c = f'<cvParam cvRef="MS" accession="MS:1000041" value="{charge}" name="charge state"/>' if charge else ""
    e = f'<cvParam cvRef="MS" accession="MS:1000045" value="{ce}" name="collision energy"/>' if ce is not None else ""
    return (f'<precursor{r}><isolationWindow><cvParam cvRef="MS" accession="MS:1000827" value="{target}" name="isolation window target m/z"/>'
            f'<cvParam cvRef="MS" accession="MS:1000828" value="{half}" name="lower"/><cvParam cvRef="MS" accession="MS:1000829" value="{half}" name="upper"/></isolationWindow>'
            f'<selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" value="{target if selected is None else selected}" name="selected ion m/z"/>{c}'
            f'</selectedIon></selectedIonList><activation>{e}<cvParam cvRef="MS" accession="{act}" name="activation"/></activation></precursor>')


def _spec(i, level, rt, filt, precs=()):
    pl = f'<precursorList count="{len(precs)}">{"".join(precs)}</precursorList>' if precs else ""
    return (f'<spectrum id="scan={i}" index="{i - 1}" defaultArrayLength="2"><cvParam cvRef="MS" accession="MS:1000511" value="{level}" name="ms level"/>'
            f'<cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/>'
            f'<cvParam cvRef="MS" accession="MS:1000285" value="10" name="total ion current"/>'
            f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" value="{rt}" name="scan start time" unitName="minute"/>'
            f'<cvParam cvRef="MS" accession="MS:1000512" value="{filt}" name="filter string"/></scan></scanList>{pl}'
            f'<binaryDataArrayList count="2">{_arr("m/z array", "MS:1000514", [100.0, 200.0])}{_arr("intensity array", "MS:1000515", [1.0, 2.0])}</binaryDataArrayList></spectrum>')


def _write(path, specs):
    path.write_text('<?xml version="1.0"?><mzML><run id="r"><spectrumList count="%d">%s</spectrumList></run></mzML>' % (len(specs), "".join(specs)), encoding="utf-8")
    return path


F = "FTMS + p ESI Full ms%d %s [50.0000-340.0000]"


def _msn_file(tmp_path):
    """Direct-infusion style MSn, one generation per file order of a Thermo converter (nearest precursor first, list incomplete)."""
    p2 = "317.0000@cid25.00"
    p3 = p2 + " 261.0000@cid30.00"
    p4 = p3 + " 244.0000@cid35.00"
    p5 = p4 + " 200.0000@cid30.00"
    s = [
        _spec(1, 1, 0.1, "FTMS + p ESI Full ms [50.0000-340.0000]"),
        _spec(2, 2, 0.2, F % (2, p2), [_prec(317, "scan=1", 25)]),
        _spec(3, 4, 0.3, F % (4, p4), [_prec(244, None, 35)]),                                                                       # count 1, no ref, generations missing
        _spec(4, 4, 0.4, F % (4, p4), [_prec(244, None, 35)]),
        _spec(5, 5, 0.5, F % (5, p5), [_prec(200, "scan=4", 30), _prec(244, None, 35), _prec(317, None, 25)]),                       # count 3, first with ref
        _spec(6, 5, 0.6, F % (5, p5), [_prec(200, "scan=4", 30, charge=1), _prec(244, None, 35)]),
        _spec(7, 5, 0.7, F % (5, "317.0000@cid25.00 261.0000@cid30.00 244.0000@cid40.00 200.0000@cid30.00"), [_prec(200, None, 30), _prec(244, None, 40)]),     # other CE: other node
        _spec(8, 5, 0.8, F % (5, p5), [_prec(317, None, 25), _prec(244, None, 35), _prec(200, "scan=4", 30)]),                       # reverse order (MSConvert-like)
    ]
    return _write(tmp_path / "msn.mzML", s)


def test_filter_path_and_immediate_precursor(tmp_path):
    r = Run(_msn_file(tmp_path))
    s = r.scans
    assert s[0].path is None and s[0].precursors == [] and s[0].precursor is None
    assert s[1].path == ((317.0, "CID", 25.0),) and s[1].precursor == 317 and s[1].iso == (314.5, 319.5)
    assert s[2].path == ((317.0, "CID", 25.0), (261.0, "CID", 30.0), (244.0, "CID", 35.0))
    assert s[2].precursor == 244 and len(s[2].precursors) == 1 and s[2].precursors[0]["ref"] is None and s[2].collision_energy == 35
    for k in (4, 7):                                                             # same path whatever the order of the <precursor> elements
        assert s[k].precursor == 200 and s[k].collision_energy == 30 and s[k].act == "CID" and s[k].iso == (197.5, 202.5)
    assert [e["target"] for e in s[4].precursors] == [200, 244, 317] and s[4].precursors[0]["ref"] == "scan=4"
    assert [e["target"] for e in s[7].precursors] == [317, 244, 200]
    assert s[5].charge == 1 and s[4].charge is None


def test_parent_is_the_scan_one_level_below_with_the_same_path(tmp_path):
    s = Run(_msn_file(tmp_path)).scans
    assert s[1].parent == 0                         # MS2: the Full Scan, as before
    assert s[2].parent is None                      # the MS3 generation is not in the file: normal in a direct infusion
    assert s[3].parent is None
    assert s[4].parent == 3 and s[5].parent == 3 and s[7].parent == 3        # the last MS4 scan with path[:-1]
    assert s[6].parent is None                      # another collision energy at the last stage... and no MS4 with 244@cid40 in the file


def test_without_filter_path_the_reference_and_the_level_decide(tmp_path):
    f = "FTMS + p ESI Full ms [50.0000-340.0000]"                               # a converter that drops the path
    s = [_spec(1, 1, 0.1, f), _spec(2, 2, 0.2, f, [_prec(317, "scan=1", 25)]),
         _spec(3, 3, 0.3, f, [_prec(317, None, 25), _prec(261, "scan=2", 30)])]       # the element that points to a level-2 scan is the immediate one
    r = Run(_write(tmp_path / "np.mzML", s)).scans
    assert r[2].path is None and r[2].precursor == 261 and r[2].parent == 1
    assert r[1].precursor == 317 and r[1].parent == 0


def test_dda_ms2_is_unchanged(tmp_path):
    f = "FTMS + p NSI Full ms2 317.1642@hcd38.33 [50.0000-340.0000]"
    s = [_spec(1, 1, 0.1, "FTMS + p NSI Full ms [50.0000-340.0000]"),
         _spec(2, 2, 0.2, f, [_prec(317.1642, "scan=1", 38.33, act="MS:1000422", half=0.75, selected=317.17)])]
    r = Run(_write(tmp_path / "dda.mzML", s)).scans[1]
    assert r.precursor == 317.17 and r.iso == pytest.approx((316.4142, 317.9142)) and r.act == "HCD" and r.parent == 0
    assert r.collision_energy == 38.33 and r.path == ((317.1642, "HCD", 38.33),)


def test_release_gives_back_the_read_ahead_window_and_reads_again(tmp_path):
    f = "FTMS + p NSI Full ms [50.0000-340.0000]"
    path = _write(tmp_path / "rel.mzML", [_spec(1, 1, 0.1, f), _spec(2, 1, 0.2, f)])
    for mode in ("file", "mmap"):
        r = Run(path, mode=mode)
        a = r.read(0)
        r.release()
        if mode == "file":
            assert r._mm._buf == b""
        b = r.read(1)
        r.release()
        assert np.array_equal(r.read(0)[0], a[0]) and len(b[0]) == len(a[0])
        r.close()


def _hrms():
    d = percorso("dati/HRMS")
    return d if d and d.is_dir() else None


@pytest.mark.skipif(_hrms() is None or not list(_hrms().glob("Orbitrap_*_DDApos_*.mzML")), reason="MZLAB_DATI / QQQ_DATI: files DDA not available")
def test_real_dda_precursor_and_isolation_as_before():
    """Independent oracle: the first selected ion / isolation window of the header, which is what the reader gave before WP0."""
    for f in sorted(glob.glob(str(_hrms() / "Orbitrap_*_DDApos_*.mzML"))):
        r = Run(f)
        n = 0
        for s in r.scans:
            if s.level != 2:
                continue
            h = r._mm[s.start:s.end].decode("utf-8", "replace")
            sel = float(re.search(r'accession="MS:1000744"[^>]*?value="([^"]*)"', h).group(1))
            tgt = float(re.search(r'accession="MS:1000827"[^>]*?value="([^"]*)"', h).group(1))
            lo = re.search(r'accession="MS:1000828"[^>]*?value="([^"]*)"', h)
            hi = re.search(r'accession="MS:1000829"[^>]*?value="([^"]*)"', h)
            assert s.precursor == sel and len(s.precursors) == 1 and s.path is not None and len(s.path) == 1
            assert s.iso == (tgt - float(lo.group(1)), tgt + float(hi.group(1)))
            n += 1
        assert n > 1000
