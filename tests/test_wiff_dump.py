"""tools/wiff_dump.py on a container written by tools/cfb_scrivi.py: the counts, the experiment types, a sample name with spaces and parentheses, the Log hidden."""
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import cfb_scrivi


def counted(s: str) -> bytes:
    b = s.encode("utf-16le")
    return struct.pack("<H", len(b)) + b


def header(code: int, prec: float) -> bytes:
    h = bytearray(180)
    struct.pack_into("<d", h, 42, prec)
    struct.pack_into("<I", h, 122, code)
    return bytes(h)


def test_dump(tmp_path):
    exp = "MethodSubtree/Method{}/DeviceMethod0/Period0/Experiment0/ExperimentHeader"
    cfb_scrivi.write({
        "SampleSubtree/SampleIdxTable": bytes(52),
        "SampleSubtree/Sample1/Idx": bytes(32 + 54 * 3),
        "SampleSubtree/Sample1/SampleDABE/DATA": counted("Prova_t15 (2)") + counted("Vial 7"),
        "SampleSubtree/Sample1/Log": b"Operator: Qualcuno\r\n",
        "SampleSubtree/Sample2/Idx": bytes(32 + 54 * 5),
        exp.format(1): header(15, 0.0),
        exp.format(2): header(11, 237.1),
    }, tmp_path / "x.wiff")
    run = lambda *a: subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_dump.py"), str(tmp_path / "x.wiff"), *a], capture_output=True, text=True, check=True).stdout
    out = run()
    assert "samples: 2" in out and "x.wiff.scan: missing" in out, out
    assert "sample 1: 3 scans; experiments: full scan" in out and "sample 2: 5 scans; experiments: MS2 237.10" in out, out
    assert "SampleDABE/DATA: Prova_t15 (2) | Vial 7" in out, out
    assert "SampleSubtree/SampleN/Log  x1" in out and "Qualcuno" not in out, out
    assert "Log: Operator: Qualcuno" in run("--log")
