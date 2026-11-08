"""controlla_rust.py --solo-fmt: the pre-push path judges fmt against the base (only NEW differences fail) and never touches the base."""
import importlib.util
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def load():
    spec = importlib.util.spec_from_file_location("controlla_rust", ROOT / "tools" / "controlla_rust.py")
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def test_new_fmt_blocked_and_base_allowed():
    m = load()
    base = {"clippy": {"a.rs::clippy::x": 3}, "fmt": ["old.rs"]}
    assert m.peggiori({"clippy": {}, "fmt": ["old.rs"]}, base) == []
    assert m.peggiori({"clippy": {}, "fmt": ["old.rs", "new.rs"]}, base) == ["NEW fmt difference: new.rs (run `cargo fmt`)"]


def test_solo_fmt_non_riscrive_la_base():
    r = subprocess.run([sys.executable, str(ROOT / "tools" / "controlla_rust.py"), "--solo-fmt", "--aggiorna"], capture_output=True, text=True)
    assert r.returncode != 0 and "cannot rewrite the base" in r.stderr


def test_pre_pr_fast_uses_the_base():
    testo = (ROOT / "tools" / "pre_pr.py").read_text(encoding="utf-8")
    assert '"--solo-fmt"' in testo and '"cargo", "fmt", "--all", "--check"' not in testo
