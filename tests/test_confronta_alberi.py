"""tools/confronta_alberi.py: identical trees, a changed byte, a missing file, and the commit placeholder."""
import json
import subprocess
import sys
from pathlib import Path

TOOL = Path(__file__).resolve().parent.parent / "tools" / "confronta_alberi.py"


def site(root: Path, commit: str, wasm: bytes = b"\0asm", extra: bool = False) -> Path:
    (root / "static").mkdir(parents=True)
    (root / "static" / "build.json").write_text(json.dumps({"commit": commit, "version": "1"}))
    (root / "static" / "version.js").write_text(f'window.MZLAB_COMMIT = "{commit}";')
    (root / "static" / "m.wasm").write_bytes(wasm)
    if extra:
        (root / "extra.txt").write_text("x")
    return root


def run(a: Path, b: Path) -> subprocess.CompletedProcess:
    return subprocess.run([sys.executable, str(TOOL), str(a), str(b)], capture_output=True, text=True, encoding="utf-8")


def test_only_the_commit_differs(tmp_path):
    r = run(site(tmp_path / "a", "a" * 40), site(tmp_path / "b", "b" * 40))
    assert r.returncode == 0 and "IDENTICAL" in r.stdout


def test_changed_and_missing_files(tmp_path):
    r = run(site(tmp_path / "a", "a" * 40), site(tmp_path / "b", "b" * 40, wasm=b"\0asn", extra=True))
    assert r.returncode == 1
    assert "different: static/m.wasm" in r.stdout and "only in B: extra.txt" in r.stdout
