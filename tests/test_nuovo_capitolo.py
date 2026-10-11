import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import nuovo_capitolo  # noqa: E402
import tema_testa  # noqa: E402


def test_skeleton_written_in_out_folder(tmp_path, capsys):
    assert nuovo_capitolo.main(["25", "spe", "Estrazione (SPE)", "--parte", "I bis. Campione", "--out", str(tmp_path)]) == 0
    html = (tmp_path / "25-spe.html").read_text(encoding="utf-8")
    assert '<html lang="it" data-esteso>' in html
    assert '<meta name="robots" content="noindex">' in html
    assert "source-card:" in html and "verification: draft" in html and "exam-level:" in html
    assert "(25.1)" in html and "Figura 25.1" in html and 'id="fd-25-0"' in html and 'aria-describedby="fd-25-0"' in html
    assert tema_testa.inject(html, "teoria") == html          # theme snippet exactly as in the other pages
    out = capsys.readouterr().out
    assert '["25-spe.html", "25", "Estrazione (SPE)", "I bis. Campione"],' in out


def test_refuses_to_overwrite_and_bad_slug(tmp_path):
    args = ["25", "spe", "T", "--out", str(tmp_path)]
    assert nuovo_capitolo.main(args) == 0
    assert nuovo_capitolo.main(args) == 1
    assert nuovo_capitolo.main(args + ["--force"]) == 0
    p = subprocess.run([sys.executable, str(ROOT / "tools" / "nuovo_capitolo.py"), "25", "Bad Slug", "T", "--out", str(tmp_path)], capture_output=True)
    assert p.returncode != 0
