"""The ZIP writer of the data export (crates/mzlab-core/src/zip.rs): archives written by Rust are read back byte for byte with Python's zipfile and `unzip -t`. Skipped without cargo."""
import os, shutil, subprocess, zipfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture(scope="module")
def out(tmp_path_factory):
    if not shutil.which("cargo"):
        pytest.skip("cargo not available")
    d = tmp_path_factory.mktemp("zip")
    env = dict(os.environ, MZLAB_ZIP_OUT=str(d))
    r = subprocess.run(["cargo", "test", "--locked", "-p", "mzlab-core", "--lib", "zip::"], cwd=ROOT, env=env, capture_output=True, text=True)
    assert r.returncode == 0, r.stdout[-2000:] + r.stderr[-2000:]
    return d


def test_n_files_byte_for_byte(out):
    big = (out / "big.bin").read_bytes()
    with zipfile.ZipFile(out / "n.zip") as z:
        assert z.testzip() is None
        assert z.namelist() == ["manifesto.json", "vuoto.txt", "sotto/è.bin", "z"]
        assert z.read("manifesto.json") == b'{"v":1}'
        assert z.read("vuoto.txt") == b""
        assert z.read("sotto/è.bin") == big
        assert z.read("z") == b"\x00\xff\x00"
        assert all(i.compress_type == zipfile.ZIP_STORED for i in z.infolist())


def test_empty_archive_and_empty_file(out):
    with zipfile.ZipFile(out / "zero.zip") as z:
        assert z.namelist() == []
    with zipfile.ZipFile(out / "uno_vuoto.zip") as z:
        assert z.read("v") == b"" and z.testzip() is None


@pytest.mark.skipif(not shutil.which("unzip"), reason="unzip not available")
def test_unzip_accepts_them(out):
    for n in ("n.zip", "zero.zip", "uno_vuoto.zip"):
        r = subprocess.run(["unzip", "-t", str(out / n)], capture_output=True, text=True)
        # unzip exits 1 for a warning-only run and 0 for ok; an empty archive is "zipfile is empty" (exit 1) on Info-ZIP
        assert r.returncode in (0, 1), r.stdout + r.stderr
        if n == "n.zip":
            assert r.returncode == 0 and "No errors detected" in r.stdout
