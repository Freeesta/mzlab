"""test.yml's `browser` job runs the preparation of the e2e site and the native binaries as steps of their own, before verifica.py: the step list then shows what each phase costs."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TEXT = (ROOT / ".github/workflows/test.yml").read_text(encoding="utf-8")


def steps() -> list[str]:
    job = re.search(r"^  browser:\n(.*?)(?=^  [\w-]+:\n|\Z)", TEXT, re.M | re.S).group(1)
    return re.findall(r"^      - (?:id: \w+\n        )?(?:name: )?(.+)$", job, re.M)


def test_the_preparation_steps_come_before_the_smoke_tests():
    names = steps()
    smoke = names.index("Smoke tests")
    assert names.index("Site and engine of the e2e tests (tools/prepara_sito.py)") < names.index("Native binaries of the e2e tests") < smoke


def test_the_native_step_builds_what_verifica_precompiles():
    verifica = (ROOT / "tools/verifica.py").read_text(encoding="utf-8")
    assert "mzlab-cli" in verifica and "mzlab-feat-wasm" in verifica and '"--example", "native"' in verifica
    assert "cargo build --release -q -p mzlab-cli" in TEXT and "cargo build --release -q -p mzlab-feat-wasm --example native" in TEXT
