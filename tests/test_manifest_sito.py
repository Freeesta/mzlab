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


def test_seal_sw_names_the_cache_after_the_site_and_stays_out_of_the_desktop_manifest(tmp_path):
    out = fake_site(tmp_path)
    (out / "sw.js").write_text('const APPV = "__APP__";', encoding="utf-8")
    (out / "static" / "esempi").mkdir()
    (out / "static" / "esempi" / "x.mzML").write_bytes(b"mz")
    bs.seal_sw(out)
    m = json.loads((out / "static" / "sw-manifest.json").read_text(encoding="utf-8"))
    app = re.search(r'APPV = "(\w{10})"', (out / "sw.js").read_text(encoding="utf-8")).group(1)
    assert m["app"] == app and m["nofill"] == ["static/esempi/x.mzML"]
    assert m["files"]["static/a.js"] == hashlib.sha256(b"var a;").hexdigest()
    (out / "static" / "a.js").write_bytes(b"var b;")       # a changed file (a .wasm added later) is a new cache name
    bs.seal_sw(out)
    assert json.loads((out / "static" / "sw-manifest.json").read_text(encoding="utf-8"))["app"] != app
    bs.write_manifest(out, "x")
    assert "static/sw-manifest.json" not in json.loads((out / "static" / "manifest.json").read_text(encoding="utf-8"))["files"]


def check_manifest_matches_disk(out):
    """Every manifest entry is the size and SHA-256 of the file on disk (the desktop app refuses the update otherwise); returns the manifest."""
    m = json.loads((out / "static" / "manifest.json").read_text(encoding="utf-8"))
    for rel, e in m["files"].items():
        data = (out / rel).read_bytes()
        assert (len(data), hashlib.sha256(data).hexdigest()) == (e["size"], e["sha256"]), rel
    return m


def test_build_json_step_rewrites_the_manifest_last(tmp_path):
    """pages.yml: build, the parts (.wasm) are copied in, then --build-json; the manifest must describe that final site, build.json included."""
    out = tmp_path / "site"
    import subprocess, sys
    run = lambda *a: subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(out), *a], check=True, capture_output=True, cwd=ROOT)
    run()
    check_manifest_matches_disk(out)
    (out / "static" / "wasm").mkdir(exist_ok=True)
    (out / "static" / "wasm" / "engine.wasm").write_bytes(b"\0asm\1\0\0\0" * 50)       # a part that arrives after the build
    run("--build-json")
    m = check_manifest_matches_disk(out)
    assert "static/wasm/engine.wasm" in m["files"] and "static/build.json" in m["files"]
    assert m["version"] == json.loads((out / "static" / "build.json").read_text(encoding="utf-8"))["commit"]


def test_installed_web_app_is_named_apart_from_the_desktop_app():
    """The PWA is «<name> Web» (name and short name); the desktop app keeps the plain name."""
    assert bs.WEB_APP_NAME == f"{bs.APP_NAME} Web"
    assert bs.MANIFEST["name"] == bs.MANIFEST["short_name"] == bs.WEB_APP_NAME
