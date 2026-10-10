"""static/manifest.json of the site: what the desktop app compares to download only the files that changed."""
import hashlib
import importlib.util
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("build_site", ROOT / "tools" / "build_site.py")
bs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bs)


def fake_site(tmp_path):
    (tmp_path / "static" / "pyodide").mkdir(parents=True)
    (tmp_path / "index.html").write_text("<html></html>", encoding="utf-8")
    (tmp_path / "static" / "a.js").write_bytes(b"var a;")
    (tmp_path / "static" / "pyodide" / "p.wasm").write_bytes(b"\0asm" * 100)
    (tmp_path / "sw.js").write_text("sw", encoding="utf-8")
    return tmp_path


def test_manifest_lists_every_file_with_sha256_and_size(tmp_path):
    out = fake_site(tmp_path)
    bs.write_manifest(out, "abc1234")
    m = json.loads((out / "static" / "manifest.json").read_text(encoding="utf-8"))
    assert m["version"] == "abc1234"
    assert set(m["files"]) == {"index.html", "static/a.js", "static/pyodide/p.wasm"}   # no sw.js, not itself
    e = m["files"]["static/a.js"]
    assert e == {"sha256": hashlib.sha256(b"var a;").hexdigest(), "size": 6}
    assert re.fullmatch(r"\d+\.\d+\.\d+", m["desktop_min"])


def test_desktop_min_is_not_newer_than_the_binary():
    cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    native = tuple(int(x) for x in re.search(r'^version = "(\d+)\.(\d+)\.(\d+)"', cargo, re.M).groups())
    need = tuple(int(x) for x in (ROOT / "crates" / "mzlab-desktop" / "DESKTOP_MIN").read_text(encoding="utf-8").strip().split("."))
    assert need <= native


def test_every_path_is_plain_relative(tmp_path):
    bs.write_manifest(fake_site(tmp_path), "x")
    m = json.loads((tmp_path / "static" / "manifest.json").read_text(encoding="utf-8"))
    for rel in m["files"]:
        assert not rel.startswith("/") and ".." not in rel.split("/") and "\\" not in rel
