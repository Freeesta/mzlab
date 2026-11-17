"""Size of every built .wasm against its cap, for tools/ci_metrics.py: `python3 tools/ci_wasm_dimensioni.py <site-dir> [out.json]` writes
{"<file>": {"byte": N, "tetto": N|null}}. Caps are the `tetto` of tools/moduli/<name>.toml (docs/agenti/wasm-tetti.md); no cap = null."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
import moduli  # noqa: E402


def caps() -> dict[str, int]:
    """output folder (relative to site/static) -> cap, from tools/moduli/."""
    return {r["uscita"]: r["tetto"] for r in moduli.carica().values()}


def misura(site: Path) -> dict[str, dict]:
    cap = caps()
    out = {}
    for w in sorted(site.rglob("*.wasm")):
        rel = w.relative_to(site)
        if "vendor" in rel.parts:          # third-party modules (Ketcher's Indigo) have no cap of ours
            continue
        # the cap of a module is the one whose output folder is the folder of the file
        out[rel.as_posix()] = {"byte": w.stat().st_size, "tetto": next((t for u, t in cap.items() if rel.parent.as_posix().endswith(u)), None)}
    return out


if __name__ == "__main__":
    data = misura(Path(sys.argv[1]))
    if data:
        Path(sys.argv[2] if len(sys.argv) > 2 else "wasm-dimensioni.json").write_text(json.dumps(data, separators=(",", ":")), encoding="utf-8")
