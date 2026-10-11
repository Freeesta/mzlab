"""Reference `.mzidx` files built by the Python builder, for the parity test of the Rust builder (crates/mzlab-core/tests/mzidx_parity.rs).

    python3 tools/mzidx_riferimento.py OUT [FILE.mzML ...]

Writes the synthetic mzML files (as tests/test_mzidx.py) in OUT/, builds OUT/<stem>.mzidx for them and for the given files, and prints
"<mzML path>" lines. No compound names.
"""
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import dati_sintetici as ds  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def synthetic(p: Path) -> list[Path]:
    ds.full_scan(p / "full.mzML", 0, np.random.default_rng(1))
    ds.full_scan(p / "neg.mzML", 0, np.random.default_rng(5), neg=True)
    ds.ms2(p / "ms2.mzML", np.random.default_rng(2))
    ds.mrm(p / "mrm.mzML", 1.0, np.random.default_rng(3), 1)
    ds.mixed_ida(p / "ida.mzML", np.random.default_rng(4))
    ds.mixed_polarity(p / "pol.mzML", np.random.default_rng(6))
    ds.hr_dda(p / "hr.mzML", np.random.default_rng(7))
    ds.full_scan_profile(p / "prof.mzML", 0, np.random.default_rng(8))
    return sorted(p.glob("*.mzML"))


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    files = synthetic(out) + [Path(a) for a in argv[1:]]
    for f in files:
        run = Run(f, index_path=out / (f.stem + ".mzidx"))
        mzidx.build(run, out / (f.stem + ".mzidx"))
        run.close()
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
