"""Rust rule: every computation of the engine lives in Rust (crates/); the Python package mzlab/ is only the reference of the parity
tests, so no new .py file may appear in it. A file listed here may be deleted (then remove it from the list), never added to."""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = {
    "mzlab/__init__.py", "mzlab/analog.py", "mzlab/api.py", "mzlab/api_aiuti.py", "mzlab/app.py", "mzlab/bigfiles.py",
    "mzlab/chem/__init__.py", "mzlab/chem/composition.py", "mzlab/chem/contaminants.py", "mzlab/chem/elements.py",
    "mzlab/chem/isoscore.py", "mzlab/chem/msntree.py", "mzlab/chem/subformulas.py", "mzlab/demo.py", "mzlab/differenza.py",
    "mzlab/explore.py", "mzlab/i18n.py", "mzlab/ionfamily.py", "mzlab/peaks.py", "mzlab/project.py", "mzlab/qaqc.py",
    "mzlab/reader/__init__.py", "mzlab/reader/methodinfo.py", "mzlab/reader/mzidx.py", "mzlab/reader/mzml.py",
    "mzlab/reader/ole.py", "mzlab/reader/profile.py", "mzlab/statistica.py",
}


def test_no_new_python_in_the_package():
    out = subprocess.run(["git", "ls-files", "mzlab"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
    new = sorted(f for f in out if f.endswith(".py") and f not in REFERENCE)
    assert not new, f"new Python in mzlab/: write it in Rust (crates/) with its parity test: {new}"
