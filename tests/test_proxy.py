"""The anonymous public proxy of the assistant (proxy/, docs/agenti/assistente-proxy.md): builds the `llm` .wasm once and runs `node --test proxy/test/`
(gates, closed body, ledger caps, settlement, rollover, no key in any output, no logging, CPU per request). Skipped without node, cargo, the wasm32 target
or wasm-bindgen."""
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture(scope="module")
def wasm_dir(tmp_path_factory):
    if not shutil.which("node"):
        pytest.skip("node not available")
    if not all(shutil.which(x) for x in ("cargo", "wasm-bindgen")):
        pytest.skip("cargo or wasm-bindgen not available (the ledger of the proxy is Rust)")
    out = tmp_path_factory.mktemp("proxy-wasm")
    try:
        for cmd in (["cargo", "build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", "mzlab-llm-wasm"],
                    ["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_llm_wasm.wasm"]):
            subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True)
    except (subprocess.CalledProcessError, OSError) as e:
        pytest.skip(f"the llm .wasm could not be built ({e})")
    return out


def test_proxy_node_suite(wasm_dir):
    env = {**os.environ, "MZLAB_LLM_WASM_DIR": str(wasm_dir)}
    p = subprocess.run(["node", "--test", "--test-reporter=tap", "proxy/test/"], cwd=ROOT, env=env, capture_output=True, text=True, timeout=600)
    out = p.stdout + p.stderr
    assert p.returncode == 0, out[-4000:]
    assert "# fail 0" in out and "# skipped 0" in out, out[-4000:]
    cpu = [line for line in out.splitlines() if "proxy CPU per request" in line]
    assert cpu, out[-2000:]
    print(cpu[0])


def test_no_key_shaped_literal_in_the_proxy():
    for f in (ROOT / "proxy").rglob("*"):
        if f.is_file() and not {"wasm", "node_modules", ".wrangler"} & set(f.relative_to(ROOT / "proxy").parts):
            assert "sk-ant" not in f.read_text(encoding="utf-8", errors="replace"), f
