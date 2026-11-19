"""Every mzML of tools/dati_sintetici.py passes the PSI-MS validator of the engine (`mzlab-cli file validate`), so the converter and the e2e uploads
see valid files. The one exception is the file that is broken on purpose (unknown instrument model, HR_NAMES["broken"])."""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tests"))
import cargo_util  # noqa: E402
import dati_sintetici as ds  # noqa: E402


def test_synthetic_files_are_valid_mzml(tmp_path):
    out = ds.make(tmp_path)
    files = sorted(p for p in out.rglob("*.mzML") if p.name != ds.HR_NAMES["broken"])
    assert len(files) > 40
    bad = []
    for i in range(0, len(files), 20):
        r = cargo_util.cli("file", "validate", *map(str, files[i:i + 20]), "--json")
        import json
        bad += [f["file"] for f in json.loads(r.stdout) if not f["valid"]]
    assert not bad, f"invalid synthetic mzML: {bad}: mzlab-cli file validate FILE shows why"
