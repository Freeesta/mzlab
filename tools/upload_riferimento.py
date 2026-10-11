"""Reference answers of mzidx.builder_inputs (the scan-table arrays, `source` and `run` of the metadata) for crates/mzlab-core/tests/upload_parity.rs.

    python3 tools/upload_riferimento.py OUT [FILE.mzML ...]

Writes the synthetic files of tools/mzidx_riferimento.py in OUT/ and OUT/<stem>.upl.json; prints the "<mzML path>" lines.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import mzidx_riferimento as ref  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def main(argv) -> int:
    out = Path(argv[0])
    out.mkdir(parents=True, exist_ok=True)
    files = ref.synthetic(out) + [Path(a) for a in argv[1:]]
    for f in files:
        run = Run(f)
        a = mzidx.builder_inputs(run)
        d = {"levels": list(a[0]), "polarity": [v - 256 if v > 127 else v for v in a[1]], "rt": __import__("numpy").frombuffer(a[2], "<f8").tolist(),
             "tic": __import__("numpy").frombuffer(a[3], "<f8").tolist(), "scans": a[4].decode() if isinstance(a[4], bytes) else a[4], "source": json.loads(a[5]),
             "run": json.loads(a[6]), "hr1": a[7]}
        (out / (f.stem + ".upl.json")).write_text(json.dumps(d), encoding="utf-8")
        print(f)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
