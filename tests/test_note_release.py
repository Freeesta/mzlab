"""Names of the release files and the download table of the release notes (tools/note_release.py)."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import note_release  # noqa: E402


def test_names_are_uniform_and_sort_after_the_installers():
    n = note_release.nomi("1.0.0")
    assert n["windows"] == "mzLab-1.0.0-Windows.exe" and n["deb"] == "mzLab-1.0.0-Linux.deb"
    assert n["sbom"] == "mzLab-1.0.0-sicurezza-SBOM.zip" and n["sito"] == "mzLab-1.0.0-sito.zip"
    assert sorted(n.values()).index(n["sbom"]) > sorted(n.values()).index(n["mac"])


def test_table_links_every_installer_and_keeps_the_changelog():
    t = note_release.testo("v1.0.0", "Freeesta/mzlab", "### Added\n- x\n")
    assert t.startswith("| Windows | Mac | Linux |")
    for k in ("windows", "mac", "appimage", "deb"):
        assert f"https://github.com/Freeesta/mzlab/releases/download/v1.0.0/{note_release.nomi('1.0.0')[k]})" in t
    assert t.rstrip().endswith("- x")
