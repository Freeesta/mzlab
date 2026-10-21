"""`teoria-blocchi` on the real Teoria: rebuilds the blocks in a temporary folder, checks schema, count, size, determinism, the --controlla exit code,
and that no text of a hidden answer (details.q) reaches the blocks."""
import json
import re
import subprocess
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
TEORIA = ROOT / "mzlab/web/teoria"
SCHEMA = json.loads((ROOT / "docs/schemi/teoria-blocchi-1.json").read_text())
MEASURED = 582


def binary(*args):
    return subprocess.run(["cargo", "run", "-q", "-p", "mzlab-tutor", "--bin", "teoria-blocchi", "--", *args], cwd=ROOT, capture_output=True, text=True)


@pytest.fixture(scope="module")
def out(tmp_path_factory):
    f = tmp_path_factory.mktemp("blocchi") / "blocchi-1.json"
    r = binary(str(TEORIA), str(f))
    assert r.returncode == 0, r.stderr[-2000:]
    return f


def test_schema_count_and_size(out):
    data = json.loads(out.read_text())
    Draft202012Validator(SCHEMA).validate(data)
    assert abs(data["n"] - MEASURED) <= MEASURED * 0.10
    assert out.stat().st_size <= 400_000


def test_same_bytes_and_controlla(out, tmp_path):
    again = tmp_path / "again.json"
    assert binary(str(TEORIA), str(again)).returncode == 0
    assert again.read_bytes() == out.read_bytes()
    assert binary(str(TEORIA), str(out), "--controlla").returncode == 0
    stale = tmp_path / "stale.json"
    stale.write_text(out.read_text().replace("Capitolo 1", "Capitolo X", 1))
    assert binary(str(TEORIA), str(stale), "--controlla").returncode == 1


def test_no_answer_text_in_blocks(out):
    texts = " ".join(b["text"] for b in json.loads(out.read_text())["blocks"])
    checked = 0
    for page in TEORIA.glob("*.html"):
        for det in re.findall(r'<details class="q">(.*?)</details>', page.read_text(), re.S):
            answer = re.sub(r"<[^>]+>", "", det.split("</summary>", 1)[-1]).strip()
            probe = " ".join(answer.split())[:60]
            if len(probe) == 60:
                assert probe not in texts, probe
                checked += 1
    assert checked > 40
