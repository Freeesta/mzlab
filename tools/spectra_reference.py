"""Reference answer of `api/spectra` on a unit-resolution centroid file, for the parity test of the binary fast path of the app and of the engine `.wasm`
(crates/mzlab-desktop/tests/fast_spectra_merge.rs).

    python3 tools/spectra_reference.py FILE.mzML

Prints one JSON object: the scans of `/api/spectra?k=0&level=1&i0=0&i1=5&bin=0.5` as Python answers it with `merge` omitted (AUTO) and with `merge=0`,
as {sid, mz, y} lists. The file is indexed in a temporary folder. No compound names.
"""
import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mzlab.api import dispatch  # noqa: E402
from mzlab.app import App  # noqa: E402
from mzlab.explore import Item  # noqa: E402
from mzlab.reader import mzidx  # noqa: E402
from mzlab.reader.mzml import Run  # noqa: E402


def main(argv) -> int:
    f = Path(argv[0])
    with tempfile.TemporaryDirectory() as tmp:
        idx = Path(tmp) / (f.stem + ".mzidx")
        run = Run(f, index_path=idx)
        mzidx.build(run, idx)
        run.close()
        item = Item(f.name, None, None, "x", f)
        app = App()
        app.session = type("S", (), {"items": [item]})()
        out = {}
        for name, extra in (("auto", {}), ("off", {"merge": "0"})):
            _status, _ctype, body, _h = dispatch(app, "GET", "/api/spectra", {"k": "0", "level": "1", "i0": "0", "i1": "5", "bin": "0.5", **extra})
            out[name] = [{"sid": s.get("sid", s.get("i")), "mz": s["mz"], "y": s["y"]} for s in json.loads(body)["scans"]]
    print(json.dumps(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
