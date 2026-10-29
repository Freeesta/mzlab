"""One version everywhere (tools/versione.py), the notes of a release, and the SBOM of the JavaScript components (tools/sbom.py)."""
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import sbom  # noqa: E402
import versione  # noqa: E402


def test_all_the_files_say_the_same_version():
    assert versione.controlla() == []
    assert set(versione.versioni()) >= {"Cargo.toml", "crates/mzlab-desktop/tauri.conf.json", "CITATION.cff"}


def _copia(tmp_path, cambia=None):
    (tmp_path / "crates/mzlab-desktop").mkdir(parents=True)
    for f in ("Cargo.toml", "crates/mzlab-desktop/tauri.conf.json", "CITATION.cff", ".zenodo.json", "CHANGELOG.md"):
        (tmp_path / f).write_text((ROOT / f).read_text(encoding="utf-8"), encoding="utf-8")
    for f, (a, b) in (cambia or {}).items():
        p = tmp_path / f
        p.write_text(p.read_text(encoding="utf-8").replace(a, b, 1), encoding="utf-8")
    return tmp_path


def test_a_file_with_another_version_is_found(tmp_path):
    v = versione.versioni()["Cargo.toml"]
    root = _copia(tmp_path, {"CITATION.cff": (f'version: "{v}"', 'version: "9.9.9"')})
    assert any("differ" in e for e in versione.controlla(root))


def test_the_tag_must_match(tmp_path):
    root = _copia(tmp_path)
    v = versione.versioni()["Cargo.toml"]
    assert versione.controlla(root, tag=f"v{v}") == []
    assert versione.controlla(root, tag="v9.9.9")


def test_notes_come_from_the_section_of_the_version(tmp_path):
    root = _copia(tmp_path)
    (root / "CHANGELOG.md").write_text("# C\n\n## [Non rilasciato]\n- x\n\n## [1.2.3] - 2026-01-01\n- a\n- b\n\n## [1.2.2] - 2025-12-01\n- old\n", encoding="utf-8")
    assert versione.nota("v1.2.3", root) == "- a\n- b\n"
    with pytest.raises(SystemExit):
        versione.nota("v4.5.6", root)


def test_sbom_lists_what_ships_and_matches_the_vendor_readme():
    doc = sbom.sbom()
    assert doc["bomFormat"] == "CycloneDX" and doc["specVersion"] == "1.5"
    versions = {c["name"]: c.get("version") for c in doc["components"]}
    readme = (ROOT / "mzlab/web/vendor/README.md").read_text(encoding="utf-8")
    assert f"Ketcher {versions['ketcher']}" in readme and f"OpenChemLib JS {versions['openchemlib']}" in readme
    assert "pyodide" not in versions and "numpy" not in versions
    json.dumps(doc)


def test_build_site_names_the_exact_build(tmp_path):
    """version.js carries the full commit and static/build.json the hash of every .wasm, written again (--build-json) once the .wasm files are in place."""
    import hashlib
    import re
    import subprocess

    out = tmp_path / "site"
    subprocess.run([sys.executable, str(ROOT / "tools/build_site.py"), "--out", str(out)], check=True, cwd=ROOT, capture_output=True)
    commit = re.search(r'MZLAB_COMMIT = "([^"]+)"', (out / "static/version.js").read_text(encoding="utf-8")).group(1)
    assert re.fullmatch(r"[0-9a-f]{40}", commit) or commit == "dev"
    wasm = out / "static/wasm"
    wasm.mkdir(exist_ok=True)
    (wasm / "b_bg.wasm").write_bytes(b"\0asm-b")
    (out / "static/wasm-lib").mkdir()
    (out / "static/wasm-lib/a_bg.wasm").write_bytes(b"\0asm-a")
    subprocess.run([sys.executable, str(ROOT / "tools/build_site.py"), "--out", str(out), "--build-json"], check=True, cwd=ROOT)
    build = json.loads((out / "static/build.json").read_text(encoding="utf-8"))
    assert build["commit"] == commit and re.fullmatch(r"\d+\.\d+\.\d+", build["version"])
    assert build["cargo_lock_sha256"] == hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest()
    assert list(build["modules"]) == ["wasm-lib/a_bg.wasm", "wasm/b_bg.wasm"]
    for name, sha in build["modules"].items():
        assert sha == hashlib.sha256((out / "static" / name).read_bytes()).hexdigest()
