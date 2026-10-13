"""Download with SHA-256 verification and conversion records of the benchmark (no network, no vendor converter: file:// URLs and a fake parser)."""
import hashlib
import json
import stat
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from bench import converti as C  # noqa: E402
from bench import scarica as D  # noqa: E402


def voce(tmp_path, nome, dati, sha=None, byte=None):
    src = tmp_path / "src"
    src.mkdir(exist_ok=True)
    (src / nome).write_bytes(dati)
    return {"nome": nome, "url": (src / nome).as_uri(), "sha256": sha or hashlib.sha256(dati).hexdigest(), "byte": len(dati) if byte is None else byte}


def test_download_verifies_and_keeps(tmp_path):
    v = voce(tmp_path, "a.raw", b"abc" * 1000)
    out = tmp_path / "out"
    p = D.scarica(v, out)
    assert p.read_bytes() == b"abc" * 1000 and not list(out.glob("*.part"))
    (tmp_path / "src" / "a.raw").write_bytes(b"changed")           # a file with the right hash is not downloaded again
    assert D.scarica(v, out) == p


def test_download_rejects_wrong_hash_or_size(tmp_path):
    out = tmp_path / "out"
    with pytest.raises(SystemExit, match="SHA-256"):
        D.scarica(voce(tmp_path, "b.raw", b"xyz", sha="0" * 64), out)
    assert not (out / "b.raw").exists()
    with pytest.raises(SystemExit, match="size"):
        D.scarica(voce(tmp_path, "c.raw", b"xyz", byte=7), out)
    assert not (out / "c.raw").exists()


def test_entries_and_filters(tmp_path):
    m = {"file_grezzi": [{"nome": "a"}, {"nome": "b"}], "file_verita": [{"nome": "t"}]}
    assert [v["nome"] for v in D.voci(m)] == ["a", "b", "t"]
    assert [v["nome"] for v in D.voci(m, ("file_grezzi",), ["b"])] == ["b"]
    with pytest.raises(SystemExit):
        D.voci(m, solo=["zz"])


def test_main_delete(tmp_path):
    (tmp_path / "x.raw").write_bytes(b"1")
    assert D.main(["m.json", str(tmp_path), "--elimina", "x.raw", "nonexistent.raw"]) == 0 and not (tmp_path / "x.raw").exists()


def test_conversion_record(tmp_path):
    fake = tmp_path / "fake"
    fake.write_text('#!/bin/sh\nif [ "$1" = "--version" ]; then echo 9.9.9; exit 0; fi\n'
                    'while [ $# -gt 0 ]; do case "$1" in -i) i="$2";; -o) o="$2";; esac; shift; done\n'
                    'b=$(basename "$i" .raw); echo "<mzML/>" > "$o/$b.mzML"\n')
    fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
    raw = tmp_path / "raw"
    raw.mkdir()
    (raw / "S1.raw").write_bytes(b"raw1")
    (raw / "S2.raw").write_bytes(b"raw2")
    out = tmp_path / "mz"
    assert C.main([str(raw), str(out), "--parser", str(fake), "--elimina-raw"]) == 0
    rec = json.loads((out / "converti.json").read_text())
    assert rec["convertitore"]["versione"] == "9.9.9" and sorted(rec["file"]) == ["S1.raw", "S2.raw"]
    assert rec["file"]["S1.raw"]["raw_sha256"] == hashlib.sha256(b"raw1").hexdigest()
    assert rec["file"]["S1.raw"]["mzml_sha256"] == hashlib.sha256(b"<mzML/>\n").hexdigest()
    assert not list(raw.glob("*.raw"))


def test_conversion_failure_is_reported(tmp_path):
    bad = tmp_path / "bad"
    bad.write_text("#!/bin/sh\nexit 3\n")
    bad.chmod(bad.stat().st_mode | stat.S_IEXEC)
    (tmp_path / "S1.raw").write_bytes(b"r")
    with pytest.raises(SystemExit, match="failed"):
        C.converti(str(bad), tmp_path / "S1.raw", tmp_path / "o")


def test_runner_parameters_and_shared_helpers(tmp_path):
    from bench import corri
    assert corri.parametri("asari", "predefinita")["mode"] == "pos"
    assert "FeatureFindingMetabo" in corri.parametri("openms", "predefinita")
    (tmp_path / "B.mzML").write_text("")
    (tmp_path / "A.mzML").write_text("")
    assert [f.stem for f in corri.file_mzml(tmp_path)] == ["A", "B"] and [f.stem for f in corri.file_mzml(tmp_path, ["B"])] == ["B"]
    with pytest.raises(SystemExit):
        corri.file_mzml(tmp_path / "vuota")
