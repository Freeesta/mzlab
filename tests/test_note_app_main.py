"""tools/note_app_main.py (notes and file names of the rolling pre-release) and the shape of .github/workflows/app-main.yml:
a pre-release that cannot pass for a release, one write-access job, no secret, no version tag."""
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import note_app_main as n  # noqa: E402

SHA = "0123456789abcdef0123456789abcdef01234567"
WF = (ROOT / ".github/workflows/app-main.yml").read_text(encoding="utf-8")
DESKTOP = (ROOT / ".github/workflows/desktop.yml").read_text(encoding="utf-8")


def test_notes_name_the_commit_the_dates_and_the_mac_steps():
    t = n.notes("Freeesta/mzlab", SHA, "2026-11-26", "2026-11-26T10:00Z")
    assert SHA in t and f"`{SHA[:12]}`" in t and "2026-11-26T10:00Z" in t and "Commit date: 2026-11-26" in t
    assert "Not a release" in t and "not published" in t
    assert "right-click" in t and "xattr -dr com.apple.quarantine" in t and "Open Anyway" in t
    for _, _, name in n.FILES:
        assert f"releases/download/app-prebuilt/{name}" in t
    assert "cli-prebuilt" in t and "For maintainers" not in t


def test_notes_refuse_a_short_commit():
    with pytest.raises(ValueError):
        n.notes("Freeesta/mzlab", "abc123", "d", "b")


def test_names_are_fixed_and_unique():
    names = [f[2] for f in n.FILES]
    assert len(set(names)) == len(names) and not any(re.search(r"\d+\.\d+\.\d+", x) for x in names)
    out = subprocess.run([sys.executable, str(ROOT / "tools/note_app_main.py"), "--names"], capture_output=True, text=True, check=True).stdout
    assert out.splitlines() == ["|".join(f) for f in n.FILES]


def test_cli_exit_codes():
    run = lambda *a: subprocess.run([sys.executable, str(ROOT / "tools/note_app_main.py"), *a], capture_output=True, text=True)  # noqa: E731
    assert run("--repo", "Freeesta/mzlab", "--sha", SHA).returncode == 0
    assert run("--repo", "Freeesta/mzlab", "--sha", "nope").returncode == 2
    assert run().returncode == 2


def test_artifacts_match_what_desktop_yml_uploads():
    # the upload step names them `mzlab-desktop-${{ runner.os }}`
    assert "name: mzlab-desktop-${{ runner.os }}" in DESKTOP
    assert {a for a, _, _ in n.FILES} == {"mzlab-desktop-macOS", "mzlab-desktop-Windows", "mzlab-desktop-Linux"}


def test_workflow_is_a_rolling_prerelease_not_a_release():
    assert "--prerelease --latest=false" in WF and "TAG: app-prebuilt" in WF
    assert not re.search(r"tags:|refs/tags|git tag|--latest(?!=false)|secrets\.", WF)
    assert "cancel-in-progress: true" in WF and "uses: ./.github/workflows/desktop.yml" in WF
    # only the publish job writes
    assert WF.count("contents: write") == 1 and WF.index("contents: write") > WF.index("  publish:")


def test_desktop_yml_has_no_push_trigger_that_would_cancel_the_call():
    on = DESKTOP.split("jobs:")[0]
    assert not re.search(r"^  push:", on, re.M) and "workflow_call" in on

