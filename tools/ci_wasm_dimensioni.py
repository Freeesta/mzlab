"""Size of every built .wasm against its cap, for tools/ci_metrics.py: `python3 tools/ci_wasm_dimensioni.py <site-dir> [out.json]` writes
{"<file>": {"byte": N, "tetto": N|null}}. Caps are the `limit_*` variables of tools/build_*_wasm.sh (docs/agenti/wasm-tetti.md); no cap = null."""
import json
import re
import sys
from pathlib import Path

RADICE = Path(__file__).resolve().parent


def tetti() -> dict[str, int]:
    out = {}
    for sh in sorted(RADICE.glob("build_*_wasm.sh")):
        for nome, val in re.findall(r"^\s*limit_(\w+)=(\d+)", sh.read_text(encoding="utf-8"), re.M):
            out[nome] = int(val)
    return out


def misura(sito: Path) -> dict[str, dict]:
    cap = tetti()
    out = {}
    for w in sorted(sito.rglob("*.wasm")):
        if "vendor" in w.relative_to(sito).parts:          # third-party modules (Ketcher's Indigo) have no cap of ours
            continue
        # the cap of a module is the limit_<name> whose name appears in its folder or file name
        chiavi = [k for k in cap if k in (w.parent.name, w.stem) or k == w.parent.name.replace("-", "_")]
        out[w.relative_to(sito).as_posix()] = {"byte": w.stat().st_size, "tetto": cap[chiavi[0]] if chiavi else None}
    return out


if __name__ == "__main__":
    dati = misura(Path(sys.argv[1]))
    if dati:
        Path(sys.argv[2] if len(sys.argv) > 2 else "wasm-dimensioni.json").write_text(json.dumps(dati, separators=(",", ":")), encoding="utf-8")
