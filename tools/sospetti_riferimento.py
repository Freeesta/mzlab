"""Writes the synthetic mini suspect index and the answers of the Python reader (for the Rust parity test, crates/mzlab-feat/tests/suspects.rs).

    python3 tools/sospetti_riferimento.py DIR      # DIR/mini.mzsus and DIR/casi.json
"""
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import genera_sospetti as gs  # noqa: E402
from sospetti_prova import make_sources  # noqa: E402


def main(out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        fonti, keys, mass, _ = make_sources(Path(tmp))
        raw = gs.genera(fonti, Path(tmp), scarica=False)
    (out / "mini.mzsus").write_bytes(raw)
    s = gs.leggi_sospetti(raw)
    qs = [(mass[k] * f, ppm) for k in keys for f, ppm in ((1.0, 0.0), (1.0, 5.0), (1 + 4e-6, 5.0), (1 + 6e-6, 5.0), (1.0, 5000.0))] + [(50.0, 5.0), (700.0, 5.0)]
    casi = [{"mass": m, "ppm": ppm, "hits": [{"key": h.inchikey14, "mass": h.mass, "formula": h.formula, "sources": sum(gs.SOURCE_BITS[x] for x in h.sources),
                                              "pubmed": h.pubmed, "patents": h.patents, "reaction": h.reaction} for h in s.cerca(m, ppm)]} for m, ppm in qs]
    (out / "casi.json").write_text(json.dumps({"rows": s.n, "casi": casi}), encoding="utf-8")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
