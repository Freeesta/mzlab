"""tools/lotto_gemini.py: several files joined in one document, batches, size warning."""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import lotto_gemini as L  # noqa: E402


def test_unisci_headers_and_order(tmp_path):
    (tmp_path / "a.txt").write_text("alpha\n")
    (tmp_path / "b.txt").write_text("beta")
    doc, n = L.unisci(["a.txt", "b.txt"], tmp_path)
    assert doc.startswith("=== FILE: a.txt (1 di 2) — file separati nel progetto, qui uniti in un unico documento ===\nalpha\n=== FINE FILE ===")
    assert "=== FILE: b.txt (2 di 2)" in doc and doc.index("a.txt") < doc.index("b.txt")
    assert n == len(doc.encode())


def test_batches_in_markdown(tmp_path):
    (tmp_path / "x.txt").write_text("x")
    (tmp_path / "y.txt").write_text("y")
    md = "Prose line with spaces\n@lotto 1\nx.txt\nnot-a-file.txt\n@lotto 2\n`y.txt`\n"
    assert L.leggi_elenco(md, 1, tmp_path) == ["x.txt"]
    assert L.leggi_elenco(md, 2, tmp_path) == ["y.txt"]
    assert L.leggi_elenco(md, None, tmp_path) == ["x.txt", "y.txt"]


def test_cli_warns_over_limit_and_missing(tmp_path):
    big = ROOT / "tools" / "lotto_gemini.py"
    lst = tmp_path / "l.txt"
    lst.write_text("tools/lotto_gemini.py\n")
    r = subprocess.run([sys.executable, str(big), str(lst), "--max-kb", "1", "--strict", "-o", str(tmp_path / "o.txt")], capture_output=True, text=True)
    assert r.returncode == 1 and "WARNING" in r.stderr and (tmp_path / "o.txt").is_file()
    lst.write_text("tools/does_not_exist.py\n")
    r = subprocess.run([sys.executable, str(big), str(lst)], capture_output=True, text=True)
    assert r.returncode == 1 and "missing file" in r.stderr
