"""The golden answers of the e2e (tests/golden/e2e) are keyed by the SHA-256 of the synthetic files: the files must come out byte-identical on every machine.
numpy picks its exp/log/sin code by CPU (AVX-512 or not) and the last bit may change; tools/dati_sintetici.py uses the scalar libm, checked here
with numpy's SIMD paths switched off in a child process, and the first upload of every golden file must find its key."""
import gzip
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402
import dati_sintetici_msn as dsm  # noqa: E402

GOLD = ROOT / "tests" / "golden" / "e2e"


def _files(out: Path) -> dict[str, Path]:
    """The first file each golden e2e uploads, written as the test writes it."""
    (out / "lr").mkdir(parents=True); (out / "hr").mkdir(); (out / "msn").mkdir()
    ds.full_scan(out / "lr" / "B_FullMass-t0.mzML", 0, np.random.default_rng(2026))                       # make(): first file of the 2026 stream
    ds.hr_dda(out / "hr" / ds.HR_NAMES["exploris"], np.random.default_rng(31), "exploris")                # tests_e2e/lib_hr.py synth()
    dsm.msn_infusion(out / "msn" / "MSN_infusion.mzML", np.random.default_rng(5))                         # tests_e2e/e2e_hr_albero.py
    lr, hr, msn = out / "lr" / "B_FullMass-t0.mzML", out / "hr" / ds.HR_NAMES["exploris"], out / "msn" / "MSN_infusion.mzML"
    return {"e2e_rust": lr, "e2e_lr": lr, "e2e_hr_mappa": hr, "e2e_hr_albero": msn}


def _key(p: Path) -> str:
    """tests_e2e/golden_ref.py: the key of the first call of a PyRef, the upload of a file."""
    c0 = hashlib.sha256(b"golden/1").hexdigest()
    return hashlib.sha256(json.dumps([c0, "upload", p.name, hashlib.sha256(p.read_bytes()).hexdigest()], sort_keys=True).encode()).hexdigest()


def _digests(out: Path) -> dict[str, str]:
    return {k: hashlib.sha256(p.read_bytes()).hexdigest() for k, p in _files(out).items()}


def test_golden_keys_found(tmp_path):
    for test, p in _files(tmp_path).items():
        st = json.loads(gzip.decompress((GOLD / f"{test}.json.gz").read_bytes()))
        assert _key(p) in st, (f"{test}: the synthetic {p.name} has no golden answer (tools/dati_sintetici*.py changed?): "
                               f"MZLAB_REGEN_GOLDEN=1 python3 tools/verifica.py --solo {test}")


def test_same_bytes_without_simd(tmp_path):
    code = (f"import sys, json; from pathlib import Path; sys.path.insert(0, {str(ROOT / 'tests')!r}); "
            f"import test_golden_sintetici as t; print(json.dumps(t._digests(Path(sys.argv[1]))))")
    env = dict(os.environ, NPY_DISABLE_CPU_FEATURES="X86_V3 X86_V4")       # baseline SSE only, as on a runner without AVX2/AVX-512
    r = subprocess.run([sys.executable, "-c", code, str(tmp_path / "base")], env=env, capture_output=True, text=True, check=True)
    assert json.loads(r.stdout.strip().splitlines()[-1]) == _digests(tmp_path / "simd"), "the synthetic files depend on numpy's SIMD code: use the scalar libm (_libm in tools/dati_sintetici.py)"
